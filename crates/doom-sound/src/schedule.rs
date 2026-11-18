//! The declared schedule of a session (M5c decision T5, confirmed by the owner on 2026-11-18;
//! M5c note §4.3.3 and §3.2.4 (3)).
//!
//! The original ties `S_UpdateSounds` and the mixer to passes of its main loop (d_main.c:389-405),
//! paced by the sound device, not to tics. The port declares instead, per tic `t` counted from
//! the session's first tic (0):
//!
//! 1. the tic's sound records run with `gametic == t` (the global `G_Ticker` sees, g_game.c:727-733,
//!    with `gametic++` after it, d_net.c:746 and d_main.c:384);
//! 2. `S_UpdateSounds` runs once after every tic with `gametic == t + 1` (d_main.c:392, the
//!    owner's ruling: after every tic);
//! 3. then every buffer `k` not yet mixed with `512 * k < 315 * (t + 1)` is mixed: buffer `k` of
//!    512 frames (`SAMPLECOUNT`, i_sound.c:93) after tic `floor(512 k / 315)`, none or one per tic
//!    (11,025 frames per second over 35 tics is 315 frames per tic).
//!
//! A session is a fresh process started straight on the map (decision T15): `gametic` 0 and no
//! buffer mixed before the first tic.

use crate::lumps::SAMPLECOUNT;
use crate::mixer::buffers_due;

/// Frames of audio per tic at 11,025 Hz and 35 tics per second (M5c note §1.3.6).
pub const FRAMES_PER_TIC: u64 = 315;

/// The last tic a session can run: `gametic` is a C `int` (g_game.c:123), and `S_UpdateSounds` after
/// tic `t` reads `gametic == t + 1`, which must fit.
pub const MAX_TIC: u64 = i32::MAX as u64 - 1;

/// Where a session stands in the declared schedule: the next tic to run and the buffers mixed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct Schedule {
    tic: u64,
    buffers: u64,
}

/// Why a [`Schedule`] could not be built or advanced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleError {
    /// The next tic would be past [`MAX_TIC`].
    TicLimit,
    /// The buffer count is not the one the rule gives for the tics run.
    Buffers {
        /// The count given.
        got: u64,
        /// The count the rule gives.
        want: u64,
    },
}

impl Schedule {
    /// A fresh session: tic 0 next, no buffer mixed.
    pub const fn new() -> Self {
        Self { tic: 0, buffers: 0 }
    }

    /// A schedule that has run `tic` tics, checked: `buffers` must be exactly
    /// [`Self::buffers_before`]`(tic)`.
    pub fn at(tic: u64, buffers: u64) -> Result<Self, ScheduleError> {
        if tic > MAX_TIC + 1 {
            return Err(ScheduleError::TicLimit);
        }
        let want = Self::buffers_before(tic);
        if buffers != want {
            return Err(ScheduleError::Buffers { got: buffers, want });
        }
        Ok(Self { tic, buffers })
    }

    /// The next tic to run (the number of tics run so far).
    pub const fn tic(&self) -> u64 {
        self.tic
    }

    /// The buffers mixed so far.
    pub const fn buffers(&self) -> u64 {
        self.buffers
    }

    /// The number of buffers the rule has mixed once tics `0..tic` have run:
    /// `ceil(315 * tic / 512)`.
    pub fn buffers_before(tic: u64) -> u64 {
        let frames = u128::from(FRAMES_PER_TIC) * u128::from(tic);
        frames.div_ceil(SAMPLECOUNT as u128) as u64
    }

    /// `gametic` while the next tic runs, and after it (for its `S_UpdateSounds`), or
    /// [`ScheduleError::TicLimit`].
    pub fn gametics(&self) -> Result<(i32, i32), ScheduleError> {
        if self.tic > MAX_TIC {
            return Err(ScheduleError::TicLimit);
        }
        // tic <= i32::MAX - 1, so both fit.
        Ok((self.tic as i32, self.tic as i32 + 1))
    }

    /// The number of buffers due after the next tic (0 or 1).
    pub fn due(&self) -> u64 {
        buffers_due(self.tic, self.buffers)
    }

    /// The schedule after the next tic, with its due buffers mixed.
    pub fn advanced(&self) -> Result<Self, ScheduleError> {
        self.gametics()?;
        Ok(Self {
            tic: self.tic + 1,
            buffers: self.buffers + self.due(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Buffer k after tic floor(512k/315): 0 or 1 per tic, 315 per 512 tics, buffer 0 after
    /// tic 0, and the iteration pattern 1,1,0,1,1,0,1,0 of M5c note §3.2.4.
    #[test]
    fn buffers_follow_the_declared_rule() {
        let mut s = Schedule::new();
        let mut pattern = Vec::new();
        for _ in 0..512 {
            let due = s.due();
            assert!(due <= 1);
            pattern.push(due);
            if due == 1 {
                // Buffer k is mixed after tic floor(512k/315).
                assert_eq!(s.tic(), 512 * s.buffers() / 315);
            }
            s = s.advanced().unwrap();
            assert_eq!(Schedule::at(s.tic(), s.buffers()), Ok(s));
        }
        assert_eq!(&pattern[..8], &[1, 1, 0, 1, 1, 0, 1, 0]);
        assert_eq!(s.buffers(), 315);
        assert_eq!(
            Schedule::at(3, 3),
            Err(ScheduleError::Buffers { got: 3, want: 2 })
        );
        let last = Schedule::at(MAX_TIC, Schedule::buffers_before(MAX_TIC)).unwrap();
        assert_eq!(last.gametics(), Ok((i32::MAX - 1, i32::MAX)));
        let end = last.advanced().unwrap();
        assert_eq!(end.gametics(), Err(ScheduleError::TicLimit));
        assert_eq!(end.advanced(), Err(ScheduleError::TicLimit));
    }
}
