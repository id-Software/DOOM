//! The audio seam (M5c note §4.7.1; decisions T7, T8, T9): the proven stream leaves the app core
//! here and nothing else does. doom-game opens no device and starts no thread; a platform host
//! hands the app an [`AudioSink`] through [`super::Host::audio`]. Gain, mute, resampling, queueing,
//! underrun and overrun belong to the sink (presentation, outside every claim, T8); the app never
//! blocks on it and a sink can change no game outcome.

use doom_sound::mixer::{Buffer, buffer_bytes};

/// Receives the mixed stream of a session with sound, buffer by buffer, and the app's pause,
/// resume and session start. Every method has a default that does nothing, so a host without
/// audio needs no code.
pub trait AudioSink {
    /// One mixed buffer of 512 stereo frames at 11,025 Hz, in order (`I_UpdateSound`'s
    /// `mixbuffer`, i_sound.c:112, 539-654; the declared schedule of decision T5 makes 0 or 1 after
    /// each tic). Unscaled: the player's volume is applied after this point (T9).
    fn buffer(&mut self, _frames: &Buffer) {}
    /// The app paused (decision S12): stop or mute output and drop queued audio.
    fn pause(&mut self) {}
    /// The app resumed after [`AudioSink::pause`].
    fn resume(&mut self) {}
    /// A new session started (the first session, or a restart, decision S11/T15): drop queued
    /// audio from the previous session. A restart from the pause is preceded by
    /// [`AudioSink::resume`].
    fn session_start(&mut self) {}
}

/// One notification a [`RecordingSink`] received.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SinkCall {
    /// A buffer: its position in [`RecordingSink::buffers`].
    Buffer(usize),
    /// [`AudioSink::pause`].
    Pause,
    /// [`AudioSink::resume`].
    Resume,
    /// [`AudioSink::session_start`].
    SessionStart,
}

/// A sink that keeps everything it receives, for headless tests: the buffers and the order of
/// every call.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecordingSink {
    /// Every buffer received, in order.
    pub buffers: Vec<Buffer>,
    /// Every call, in order.
    pub calls: Vec<SinkCall>,
}

impl RecordingSink {
    /// The little-endian bytes of every buffer received, concatenated (each buffer as
    /// `doom_sound::mixer::buffer_bytes` gives it).
    pub fn stream_bytes(&self) -> Vec<u8> {
        self.buffers.iter().flat_map(buffer_bytes).collect()
    }
}

impl AudioSink for RecordingSink {
    fn buffer(&mut self, frames: &Buffer) {
        self.calls.push(SinkCall::Buffer(self.buffers.len()));
        self.buffers.push(*frames);
    }
    fn pause(&mut self) {
        self.calls.push(SinkCall::Pause);
    }
    fn resume(&mut self) {
        self.calls.push(SinkCall::Resume);
    }
    fn session_start(&mut self) {
        self.calls.push(SinkCall::SessionStart);
    }
}
