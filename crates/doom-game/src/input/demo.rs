//! The demo round trip of `G_WriteDemoTiccmd` (g_game.c:1506-1523) and `G_ReadDemoTiccmd`
//! (g_game.c:1491-1503), as pure functions (decision S3).

use doom_sim::TicCommand;

/// `DEMOMARKER` (g_game.c:1488).
const DEMOMARKER: u8 = 0x80;

/// The four bytes `G_WriteDemoTiccmd` writes for `cmd` (g_game.c:1510-1513):
/// `forwardmove`, `sidemove`, `(angleturn+128)>>8` and `buttons`, each stored modulo 256.
/// `angleturn` is promoted to `int`, so `+128` cannot overflow, and gcc's `>>` of a negative
/// `int` is arithmetic, as Rust's is: the third byte is ⌊(angleturn + 128) / 256⌋ mod 256,
/// round to nearest with an exact half rounding up.
pub fn write_ticcmd_bytes(cmd: &TicCommand) -> [u8; 4] {
    [
        cmd.forward as u8,
        cmd.side as u8,
        ((i32::from(cmd.angle_turn) + 128) >> 8) as u8,
        cmd.buttons,
    ]
}

/// `G_ReadDemoTiccmd` (g_game.c:1491-1503) on four bytes: `None` when the first is
/// `DEMOMARKER`, the end of the demo (g_game.c:1493-1498); otherwise `forwardmove` and
/// `sidemove` as `signed char`, `angleturn = byte<<8` narrowed to `short` (gcc wraps values of
/// 32768 and up to negative) and `buttons` unchanged.
pub fn read_ticcmd_bytes(bytes: [u8; 4]) -> Option<TicCommand> {
    (bytes[0] != DEMOMARKER).then(|| decode(bytes))
}

fn decode(bytes: [u8; 4]) -> TicCommand {
    TicCommand {
        forward: bytes[0] as i8,
        side: bytes[1] as i8,
        angle_turn: (u16::from(bytes[2]) << 8) as i16,
        buttons: bytes[3],
    }
}

/// One recorded tic: the command `G_BuildTiccmd` built, the four demo bytes and the command
/// the game executes, which while recording is the command re-read from those bytes into the
/// same `ticcmd_t` (g_game.c:664-669, 1514, 1522; M5b investigation §1.2.2 fact 4).
///
/// Only [`super::BuiltCommand::record`] makes one, so every value comes from `G_BuildTiccmd`
/// and its round trip; the fields are read through the accessors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecordedTic {
    built: TicCommand,
    demo: [u8; 4],
    executed: TicCommand,
}

impl RecordedTic {
    /// The round trip of a built command. `G_ReadDemoTiccmd`'s marker test reads the
    /// `forwardmove` byte just written (g_game.c:1493); a built `forward` is within ±50
    /// (g_game.c:413-416), never −128, so the test cannot fire and the re-read always happens.
    /// `consistancy` and `chatchar`, which the re-read leaves alone, are not represented.
    pub(crate) fn of(built: TicCommand) -> Self {
        let demo = write_ticcmd_bytes(&built);
        Self {
            built,
            demo,
            executed: decode(demo),
        }
    }

    /// The built command, `angle_turn` at full 16 bits.
    pub fn built(&self) -> TicCommand {
        self.built
    }

    /// The four bytes written to the demo.
    pub fn demo(&self) -> [u8; 4] {
        self.demo
    }

    /// The executed command: `angle_turn` is `256 * ⌊(built + 128) / 256⌋` wrapped into
    /// −32768..=32512; `forward`, `side` and `buttons` equal the built ones.
    pub fn executed(&self) -> TicCommand {
        self.executed
    }

    /// A forged value for the recorder's own refusal tests.
    #[cfg(test)]
    pub(crate) fn forged(built: TicCommand, demo: [u8; 4], executed: TicCommand) -> Self {
        Self {
            built,
            demo,
            executed,
        }
    }
}
