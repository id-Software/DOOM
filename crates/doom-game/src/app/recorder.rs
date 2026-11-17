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
use doom_sim::demo::{DemoError, write_demo_episode1_v2};
use doom_sim::{BT_ATTACK, BT_CHANGE, BT_USE, BT_WEAPONMASK, TicCommand};
use std::fmt;

/// The longest session, in tics (decision S21: about 8.3 hours, 4 MB of commands).
pub const TIC_CAP: usize = 1 << 20;

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

    /// Records one tic and returns the stored command, the one to execute.
    pub fn record(&mut self, tic: RecordedTic) -> Result<TicCommand, RecordError> {
        if self.full() {
            return Err(RecordError::Cap);
        }
        if read_ticcmd_bytes(tic.demo) != Some(tic.executed) {
            return Err(RecordError::NotReread);
        }
        if !wire24_admits(&tic.executed) {
            return Err(RecordError::NotAdmitted);
        }
        self.commands.push(tic.executed);
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
