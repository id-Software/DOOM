//! The session recorder (decisions S3 and S21; M5b investigation §3.6): every session is a
//! version-110 demo of the commands the simulation executed.
//!
//! A command is never changed after it is recorded: [`Recorder::record`] takes the whole
//! [`RecordedTic`], checks that its executed command is the one re-read from its demo bytes
//! (`G_WriteDemoTiccmd`'s write and re-read, g_game.c:1506-1523) and inside wire 24's command
//! domain, stores it, and hands back the stored copy, which is the only command the session
//! passes to the simulation. The file is written by doom-sim's own writer
//! ([`write_demo_episode1_v2`]) from the same stored commands.

use crate::input::{RecordedTic, read_ticcmd_bytes};
use doom_sim::demo::{DemoError, parse_demo_episode1_v2, write_demo_episode1_v2};
use doom_sim::{BT_ATTACK, BT_CHANGE, BT_USE, BT_WEAPONMASK, TicCommand};
use std::fmt;

/// The longest session, in tics (decision S21: about 8.3 hours, 4 MB of commands).
pub const TIC_CAP: usize = 1 << 20;
/// The bytes of a demo's header (version, skill, episode, map, the four option bytes, the console
/// player and the four `playeringame` bytes: g_game.c:1555-1566), before the first tic's command.
pub const DEMO_HEADER_BYTES: usize = doom_sim::demo::DEMO_HEADER.len();

/// Why a tic could not be recorded. Nothing was stored.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordError {
    /// The session already holds the cap's number of tics.
    Cap,
    /// The executed command is not the one re-read from the demo bytes.
    NotReread,
    /// The executed command is outside wire 24's command domain (`forward` or `side` −128,
    /// buttons other than attack, use and a weapon change, or weapon bits without the change
    /// bit).
    NotAdmitted,
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Cap => "the session reached its tic cap",
            Self::NotReread => "the executed command is not the demo's re-read command",
            Self::NotAdmitted => "the command is outside the wire-24 domain",
        })
    }
}

/// Whether wire 24 admits `cmd` (the button and move checks of the episode-1 v2 policy; M4b).
pub fn wire24_admits(cmd: &TicCommand) -> bool {
    cmd.buttons & !(BT_ATTACK | BT_USE | BT_CHANGE | BT_WEAPONMASK) == 0
        && cmd.forward != i8::MIN
        && cmd.side != i8::MIN
        && (cmd.buttons & BT_CHANGE != 0 || cmd.buttons & BT_WEAPONMASK == 0)
}

/// The commands of one session, append-only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recorder {
    map: u8,
    skill: u8,
    cap: usize,
    commands: Vec<TicCommand>,
}

impl Recorder {
    /// An empty recording of `E1M<map>` at `skill`, bounded at `cap` tics.
    pub fn new(map: u8, skill: u8, cap: usize) -> Self {
        Self {
            map,
            skill,
            cap,
            commands: Vec::new(),
        }
    }

    /// The number of tics recorded.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether nothing is recorded.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Whether the cap is reached.
    pub fn full(&self) -> bool {
        self.commands.len() >= self.cap
    }

    /// The recorded commands.
    pub fn commands(&self) -> &[TicCommand] {
        &self.commands
    }

    /// The recorded commands as a [`super::replay`] accepts them.
    pub fn demo_commands(&self) -> DemoCommands {
        DemoCommands(self.commands.clone())
    }

    /// Records one tic and returns the stored command, the one to execute. A [`RecordedTic`]
    /// comes only from `G_BuildTiccmd` and its demo round trip, so the checks below never fail
    /// in the app; they stay as a second guard.
    pub fn record(&mut self, tic: RecordedTic) -> Result<TicCommand, RecordError> {
        if self.full() {
            return Err(RecordError::Cap);
        }
        if read_ticcmd_bytes(tic.demo()) != Some(tic.executed()) {
            return Err(RecordError::NotReread);
        }
        if !wire24_admits(&tic.executed()) {
            return Err(RecordError::NotAdmitted);
        }
        self.commands.push(tic.executed());
        Ok(self.commands[self.commands.len() - 1])
    }

    /// Removes the last recorded tic: the simulation refused it and did no work.
    pub(crate) fn drop_last(&mut self) {
        self.commands.pop();
    }

    /// The demo file's bytes: header, every recorded tic and the end marker.
    pub fn bytes(&self) -> Result<Vec<u8>, DemoError> {
        write_demo_episode1_v2(self.map, self.skill, &self.commands)
    }

    /// The bytes that turn the file of every tic but the last ([`Recorder::bytes`] before the
    /// last [`Recorder::record`]) into the file of every tic: the last tic's four demo bytes and
    /// then the end marker, written by doom-sim's own writer (the same writer and refusals as
    /// [`Recorder::bytes`]), with the byte offset at which they go, which is that of the old
    /// marker: [`DEMO_HEADER_BYTES`] + 4 × (tics before). `Ok(None)` when nothing is recorded.
    pub fn last_tic_tail(&self) -> Result<Option<(u64, [u8; 5])>, DemoError> {
        let Some(last) = self.commands.last() else {
            return Ok(None);
        };
        let one = write_demo_episode1_v2(self.map, self.skill, std::slice::from_ref(last))?;
        let mut tail = [0; 5];
        tail.copy_from_slice(&one[DEMO_HEADER_BYTES..]);
        let before = (self.commands.len() - 1) as u64;
        Ok(Some((DEMO_HEADER_BYTES as u64 + 4 * before, tail)))
    }
}

/// The commands [`super::replay`] accepts: those of a parsed version-110 demo (each `angle_turn`
/// a demo byte shifted left by 8, g_game.c:1501) or of a [`Recorder`]. Nothing else makes one;
/// commands at full resolution go to [`super::replay_full_resolution`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DemoCommands(Vec<TicCommand>);

impl DemoCommands {
    /// The commands of an episode-1 version-110 demo of `E1M<map>`, read by doom-sim's reader
    /// (`G_ReadDemoTiccmd`, g_game.c:1491-1503).
    pub fn parse(bytes: &[u8], map: u8) -> Result<Self, DemoError> {
        parse_demo_episode1_v2(bytes, map).map(|(_, d)| Self(d.commands))
    }

    /// The commands, in tic order.
    pub fn commands(&self) -> &[TicCommand] {
        &self.0
    }
}

/// A local date and time, used only for a session's file name.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Timestamp {
    /// The year.
    pub year: u16,
    /// The month, 1–12.
    pub month: u8,
    /// The day, 1–31.
    pub day: u8,
    /// The hour, 0–23.
    pub hour: u8,
    /// The minute.
    pub minute: u8,
    /// The second.
    pub second: u8,
}

/// The file name of a session (decision S21): `E1M<map>-skill<n>-<yyyymmdd-hhmmss>.lmp`.
pub fn session_name(map: u8, skill: u8, t: Timestamp) -> String {
    format!(
        "E1M{map}-skill{skill}-{:04}{:02}{:02}-{:02}{:02}{:02}.lmp",
        t.year, t.month, t.day, t.hour, t.minute, t.second
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::write_ticcmd_bytes;

    fn cmd(forward: i8, side: i8, angle_turn: i16, buttons: u8) -> TicCommand {
        TicCommand {
            forward,
            side,
            angle_turn,
            buttons,
        }
    }

    /// A tic whose demo bytes and executed command are those of `built`'s round trip.
    fn tic(built: TicCommand) -> RecordedTic {
        let demo = write_ticcmd_bytes(&built);
        RecordedTic::forged(
            built,
            demo,
            read_ticcmd_bytes(demo).expect("not the marker"),
        )
    }

    #[test]
    fn a_changed_or_inadmissible_command_is_never_stored() {
        let mut r = Recorder::new(1, 2, TIC_CAP);
        let t = tic(cmd(25, 0, 640, 0));
        let changed = |executed| RecordedTic::forged(t.built(), t.demo(), executed);
        // the full-resolution turn, not the re-read
        assert_eq!(
            r.record(changed(cmd(25, 0, 640, 0))),
            Err(RecordError::NotReread)
        );
        assert_eq!(
            r.record(changed(cmd(26, 0, 768, 0))),
            Err(RecordError::NotReread)
        );
        let marker = RecordedTic::forged(t.built(), [0x80, 0, 3, 0], t.executed());
        assert_eq!(r.record(marker), Err(RecordError::NotReread));
        for bad in [
            cmd(0, i8::MIN, 0, 0),
            cmd(0, 0, 0, 0x80), // BT_SPECIAL
            cmd(0, 0, 0, 0x40),
            cmd(0, 0, 0, 2 << 3), // weapon bits without BT_CHANGE
        ] {
            assert!(!wire24_admits(&bad));
            assert_eq!(r.record(tic(bad)), Err(RecordError::NotAdmitted), "{bad:?}");
        }
        assert!(r.is_empty());
        assert!(wire24_admits(&cmd(127, -127, -32768, 1 | 2 | 4 | 7 << 3)));
        assert!(!wire24_admits(&cmd(i8::MIN, 0, 0, 0)));
        r.record(t).expect("the round trip of a built command");
        assert_eq!(r.demo_commands().commands(), r.commands());
    }

    #[test]
    fn the_stored_command_is_the_reread_and_the_file_holds_its_bytes() {
        let mut r = Recorder::new(4, 3, TIC_CAP);
        let tics = [
            tic(cmd(25, -24, 640, 1)),
            tic(cmd(-50, 40, -320, 2)),
            tic(cmd(0, 0, 32767, 4 | 3 << 3)),
            tic(cmd(0, 0, -129, 0)),
        ];
        let mut bytes = vec![110, 3, 1, 4, 0, 0, 0, 0, 0, 1, 0, 0, 0];
        for t in tics {
            let executed = r.record(t).expect("recorded");
            assert_eq!(executed, t.executed());
            assert_eq!(executed.angle_turn & 0xff, 0);
            assert_eq!(write_ticcmd_bytes(&executed), t.demo());
            bytes.extend(t.demo());
        }
        bytes.push(0x80);
        assert_eq!(r.bytes().expect("written"), bytes);
        assert_eq!(r.commands()[0].angle_turn, 768);
        assert_eq!(r.commands()[2].angle_turn, -32768);
        let parsed = DemoCommands::parse(&bytes, 4).expect("parsed");
        assert_eq!(parsed, r.demo_commands());
    }

    #[test]
    fn the_cap_is_1_048_576_tics() {
        let mut r = Recorder::new(1, 2, TIC_CAP);
        let t = tic(TicCommand::default());
        for _ in 0..TIC_CAP {
            r.record(t).expect("below the cap");
        }
        assert!(r.full());
        assert_eq!(r.record(t), Err(RecordError::Cap));
        assert_eq!(r.len(), 1_048_576);
        assert_eq!(r.bytes().expect("written").len(), 13 + 4 * 1_048_576 + 1);
        let mut small = Recorder::new(1, 2, 1);
        assert!(!small.full());
        small.record(t).expect("one");
        assert_eq!(small.record(t), Err(RecordError::Cap));
    }
}
