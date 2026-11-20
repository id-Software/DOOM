//! The audio device (milestone M5c, owner ruling T7 of 2026-11-18; the device's place is
//! described in `docs/M5C_REFERENCE_INVESTIGATION.md` §4.7.2 and the library in §4.14).
//!
//! The library is maudio (miniaudio), pinned to an exact version. A device is opened only from
//! an explicit list of real backends for the operating system, tried one at a time in the
//! listed order, never from miniaudio's default list, which ends with the Null backend: a
//! timer-driven device that discards every sample (§4.14, "The Null backend"). When no listed
//! backend opens, [`open_playback`] returns an [`OpenError`] naming every backend tried and
//! why it failed, so the failure is loud.
//!
//! [`DeviceSink`] is the game's sink (§4.7.2): the game loop pushes each mixed buffer into a
//! bounded [`Queue`] and never waits for the device to play anything; it does take the queue's
//! lock, which the device's callback, on the library's own thread, holds only while it copies
//! one period out (and the loop only while it copies one buffer in), so the loop can wait for
//! at most one such copy. Underrun gives silence, overrun drops the oldest frames, and
//! the player's volume and mute are a gain applied as frames leave the queue. All of this is
//! presentation (decision T8): it receives the finished, proven stream and nothing flows back
//! from it into the game. When no device opens, or with `--no-sound`, [`Output::Discard`]
//! takes the stream and drops it, and the game runs the same.
//!
//! The `--audio-probe` diagnostic plays digital silence (all-zero samples) only.

use doom_game::app::AudioSink;
use maudio::audio::sample_rate::SampleRate;
use maudio::backend::Backend;
use maudio::context::ContextBuilder;
use maudio::device::Device;
use maudio::device::DeviceOps as _;
use maudio::device::device_builder::{DeviceBuilder, DeviceBuilderOps as _};
use maudio::device::device_type::DeviceType;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

/// The output rate the mixer produces (11,025 Hz, §4.7.2); miniaudio converts when the
/// device runs at another rate.
pub const RATE: SampleRate = SampleRate::Sr11025;

/// The device period asked for, in frames: one 512-frame mixer buffer (§4.7.2). It is a hint:
/// the backend may change it.
pub const PERIOD_FRAMES: u32 = 512;

/// The real backends tried on this operating system, by name, in order. Never `Null`, never
/// `Custom`, never empty where audio is supported.
#[cfg(target_os = "windows")]
pub const BACKENDS: &[(Backend, &str)] = &[
    (Backend::Wasapi, "WASAPI"),
    (Backend::DSound, "DirectSound"),
    (Backend::WinMm, "WinMM"),
];

/// The real backends tried on this operating system, by name, in order. Never `Null`, never
/// `Custom`, never empty where audio is supported.
#[cfg(target_os = "macos")]
pub const BACKENDS: &[(Backend, &str)] = &[(Backend::CoreAudio, "Core Audio")];

/// The real backends tried on this operating system, by name, in order. Never `Null`, never
/// `Custom`, never empty where audio is supported. JACK is not listed (§4.14,
/// "Recommendation").
#[cfg(target_os = "linux")]
pub const BACKENDS: &[(Backend, &str)] =
    &[(Backend::PulseAudio, "PulseAudio"), (Backend::Alsa, "ALSA")];

/// No backend is listed for other operating systems: opening always fails.
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub const BACKENDS: &[(Backend, &str)] = &[];

/// Why no playback device was opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenError {
    /// Each backend tried, in order, with the library's error for it. Empty when this
    /// operating system has no listed backend.
    pub tried: Vec<(&'static str, String)>,
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.tried.is_empty() {
            return write!(
                f,
                "no audio device: no audio backend is listed for this system"
            );
        }
        write!(f, "no audio device opened; backends tried:")?;
        for (i, (name, why)) in self.tried.iter().enumerate() {
            let sep = if i == 0 { " " } else { ", " };
            write!(f, "{sep}{name} ({why})")?;
        }
        Ok(())
    }
}

impl std::error::Error for OpenError {}

/// An open and started playback device and the backend it came from.
pub struct Opened {
    /// The device: signed 16-bit stereo at [`RATE`] as seen by the callback.
    pub device: Device<i16>,
    /// The name of the backend that opened it (an entry of [`BACKENDS`]).
    pub backend: &'static str,
}

/// Opens and starts a playback device (signed 16-bit, 2 channels, [`RATE`], period hint
/// [`PERIOD_FRAMES`]) from [`BACKENDS`] only, trying each backend alone in order; a backend
/// whose device opens but does not start counts as failed and the next one is tried.
/// `callback` is called for each backend tried and must fill its slice (interleaved
/// stereo frames).
pub fn open_playback<C>(mut callback: impl FnMut() -> C) -> Result<Opened, OpenError>
where
    C: FnMut(&mut [i16]) + Send + 'static,
{
    let mut tried = Vec::with_capacity(BACKENDS.len());
    for &(backend, name) in BACKENDS {
        // One backend per context: the list is never empty (an empty list would make
        // miniaudio use its default list, Null included), and the opened backend is known.
        let context = match ContextBuilder::new().preferred_backends([backend]).build() {
            Ok(c) => c,
            Err(e) => {
                tried.push((name, e.to_string()));
                continue;
            }
        };
        let mut fill = callback();
        let device = DeviceBuilder::playback()
            .i16()
            .playback_channels(2)
            .sample_rate(RATE)
            .period_size_frames(PERIOD_FRAMES)
            .context(&context)
            .with_callback(move |_, out: &mut [i16]| fill(out));
        match device {
            Ok(mut device) => match device.device_start() {
                Ok(()) => {
                    return Ok(Opened {
                        device,
                        backend: name,
                    });
                }
                Err(e) => tried.push((name, format!("the device did not start: {e}"))),
            },
            Err(e) => tried.push((name, e.to_string())),
        }
    }
    Err(OpenError { tried })
}

/// The `--audio-probe` diagnostic: opens the device, plays digital silence (every sample
/// zero) for about one second, stops, and returns the line to print; or the error.
pub fn probe() -> Result<String, String> {
    let frames = Arc::new(AtomicU64::new(0));
    let counter = Arc::clone(&frames);
    let mut opened = open_playback(|| {
        let counter = Arc::clone(&counter);
        move |out: &mut [i16]| {
            out.fill(0); // digital silence only
            counter.fetch_add((out.len() / 2) as u64, Ordering::Relaxed);
        }
    })
    .map_err(|e| e.to_string())?;
    let name = opened
        .device
        .get_name(DeviceType::Playback)
        .unwrap_or_else(|e| format!("name unavailable: {e}"));
    std::thread::sleep(std::time::Duration::from_secs(1));
    let stopped = opened.device.device_stop();
    stopped.map_err(|e| format!("{}: the device did not stop: {e}", opened.backend))?;
    Ok(format!(
        "audio probe: backend {}, device {name:?}; {} frames of silence requested in about 1 s",
        opened.backend,
        frames.load(Ordering::Relaxed)
    ))
}

/// Frames in one mixed buffer (`SAMPLECOUNT`, i_sound.c:84; the seam's buffer).
pub const BUFFER_FRAMES: usize = 512;

/// The queue's cap: 4 buffers, 2,048 frames, 185.8 ms at 11,025 Hz (§4.7.2). Pushing beyond it
/// drops the oldest frames.
pub const CAP_FRAMES: usize = 4 * BUFFER_FRAMES;

/// The start threshold: the device's callback takes nothing from the queue until it holds two
/// whole buffers, 1,024 frames, 92.9 ms at 11,025 Hz, and again after every underrun and every
/// resume.
///
/// Why two (arithmetic, not measured): under the declared schedule (decision T5 part 2,
/// doom-sound's `schedule` module) buffer k is handed over after tic floor(512k/315), that is at
/// floor(512k/315)/35 s on the loop's clock, while a device running at exactly 11,025 Hz plays
/// buffer k from 512k/11,025 s = (512k/315)/35 s after it starts: the two rates are equal, so
/// the hand-over is never later than the device needs it but early only by the fraction
/// (512k mod 315)/315 of a tic, which falls to 0 at every k that is a multiple of 315 (every
/// 14.6 s). With one buffer as the threshold the margin after the start is therefore that
/// fraction plus the callback's phase, at worst about one frame, and any sleep jitter of the
/// loop or drift of the device's clock gives an underrun (a gap) early in every session and
/// after every pause. With two buffers the queue starts one tic later (buffer 1 arrives after
/// tic 1), so every later buffer arrives at least one tic, 315 frames (28.6 ms), before the
/// device needs it; the price is that the whole latency range moves up by those 315 frames (in
/// any one session by nothing or by one 512-frame callback, 46.4 ms, depending on the
/// callback's phase), with 92.9 ms queued at the start, before the device's own buffering. The
/// cap of four buffers is unchanged. This is presentation (decision T8): nothing here reaches
/// the game, and none of it has been measured on a device. (An earlier version of this comment
/// gave the margin and the price as one whole buffer, 46.4 ms; the latency arithmetic of
/// 2026-11-18 in docs/validation/M5C.md corrected both figures. The code did not change.)
pub const START_FRAMES: usize = 2 * BUFFER_FRAMES;

/// The player's volume, a percentage 0–100 applied to the finished stream at the device
/// (decision T9). It never reaches the sound logic, whose volume stays the original default.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Volume(u8);

impl Volume {
    /// 100 %: the stream as mixed.
    pub const FULL: Self = Self(100);

    /// The volume of `percent`, or `None` above 100.
    pub fn new(percent: u8) -> Option<Self> {
        (percent <= 100).then_some(Self(percent))
    }
}

/// `sample × num / den` in integer arithmetic, truncated toward zero and saturated to the
/// 16-bit range. `den` must not be 0 (it is 100 here).
pub fn scale(sample: i16, num: i32, den: i32) -> i16 {
    let v = i64::from(sample) * i64::from(num) / i64::from(den);
    v.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16
}

/// One sample after the player's volume and mute.
pub fn gain(sample: i16, volume: Volume, muted: bool) -> i16 {
    if muted {
        0
    } else {
        scale(sample, i32::from(volume.0), 100)
    }
}

/// What the queue counted (diagnostics only, printed with `--tic-stats`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counters {
    /// Pulls that found too few frames after output had started.
    pub underruns: u64,
    /// Frames of silence written by those pulls.
    pub underrun_frames: u64,
    /// Pushes that took the queue beyond [`CAP_FRAMES`].
    pub overruns: u64,
    /// The oldest frames those pushes dropped.
    pub dropped_frames: u64,
}

/// The bounded frame queue between the game loop (which pushes) and the device's callback
/// (which pulls), with the volume and mute applied as frames leave it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Queue {
    frames: VecDeque<[i16; 2]>,
    started: bool,
    paused: bool,
    muted: bool,
    volume: Volume,
    counters: Counters,
}

impl Queue {
    /// An empty queue, not started, not paused, not muted.
    pub fn new(volume: Volume) -> Self {
        Self {
            frames: VecDeque::with_capacity(CAP_FRAMES + BUFFER_FRAMES),
            started: false,
            paused: false,
            muted: false,
            volume,
            counters: Counters::default(),
        }
    }

    /// Appends one mixed buffer; beyond [`CAP_FRAMES`] the oldest frames are dropped and
    /// counted. Ignored while paused (no tic runs then, so none arrives).
    pub fn push(&mut self, buffer: &[[i16; 2]; BUFFER_FRAMES]) {
        if self.paused {
            return;
        }
        self.frames.extend(buffer.iter().copied());
        let over = self.frames.len().saturating_sub(CAP_FRAMES);
        if over > 0 {
            self.frames.drain(..over);
            self.counters.overruns += 1;
            self.counters.dropped_frames += over as u64;
        }
    }

    /// Fills `out` (interleaved stereo; an odd last sample is set to 0) with the oldest frames,
    /// volume and mute applied. Silence while paused or before the queue first holds
    /// [`START_FRAMES`]; when frames run out after the start, the rest is silence, the underrun
    /// is counted and the start threshold applies again.
    pub fn pull(&mut self, out: &mut [i16]) {
        out.fill(0);
        if self.paused {
            return;
        }
        if !self.started {
            if self.frames.len() < START_FRAMES {
                return;
            }
            self.started = true;
        }
        let (volume, muted) = (self.volume, self.muted);
        let mut missing = 0u64;
        for slot in out.chunks_exact_mut(2) {
            match self.frames.pop_front() {
                Some([l, r]) => {
                    slot[0] = gain(l, volume, muted);
                    slot[1] = gain(r, volume, muted);
                }
                None => missing += 1,
            }
        }
        if missing > 0 {
            self.counters.underruns += 1;
            self.counters.underrun_frames += missing;
            self.started = false;
        }
    }

    /// The app paused: queued frames are dropped and output is silent until [`Queue::resume`].
    pub fn pause(&mut self) {
        self.frames.clear();
        self.paused = true;
        self.started = false;
    }

    /// The app resumed: output waits for the start threshold again.
    pub fn resume(&mut self) {
        self.paused = false;
        self.started = false;
    }

    /// A new session: queued frames of the previous one are dropped.
    pub fn session_start(&mut self) {
        self.frames.clear();
        self.started = false;
    }

    /// Mutes or unmutes; returns whether it is now muted. Frames keep leaving the queue while
    /// muted, so unmuting never plays stale audio.
    pub fn toggle_mute(&mut self) -> bool {
        self.muted = !self.muted;
        self.muted
    }

    /// Frames queued now.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether no frame is queued.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// What the queue counted so far.
    pub fn counters(&self) -> Counters {
        self.counters
    }
}

/// Locks the queue. A poisoned lock (a panic while it was held) still gives the queue: its
/// state is plain data and each method leaves it consistent.
fn lock(queue: &Mutex<Queue>) -> MutexGuard<'_, Queue> {
    queue
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The sink of a session with a device: the game loop's side of the [`Queue`], and the device
/// whose callback pulls from it. The loop holds the lock only to copy one buffer in; the
/// callback holds it only to copy one period out.
pub struct DeviceSink {
    queue: Arc<Mutex<Queue>>,
    /// The device, held so that it keeps running; dropping the sink stops and closes it.
    /// `None` only in headless tests.
    _device: Option<Device<i16>>,
    backend: &'static str,
}

impl DeviceSink {
    /// Opens and starts a device from [`BACKENDS`] with its callback pulling from a new queue.
    pub fn open(volume: Volume) -> Result<Self, OpenError> {
        let queue = Arc::new(Mutex::new(Queue::new(volume)));
        let opened = open_playback(|| {
            let queue = Arc::clone(&queue);
            move |out: &mut [i16]| lock(&queue).pull(out)
        })?;
        Ok(Self {
            queue,
            _device: Some(opened.device),
            backend: opened.backend,
        })
    }

    /// A sink with no device, for headless tests: the caller pulls with [`DeviceSink::pull`].
    #[cfg(test)]
    pub fn headless(volume: Volume) -> Self {
        Self {
            queue: Arc::new(Mutex::new(Queue::new(volume))),
            _device: None,
            backend: "none (headless)",
        }
    }

    /// What the device's callback does, for headless tests.
    #[cfg(test)]
    pub fn pull(&self, out: &mut [i16]) {
        lock(&self.queue).pull(out);
    }

    /// The backend the device came from.
    pub fn backend(&self) -> &'static str {
        self.backend
    }

    /// Mutes or unmutes; returns whether it is now muted.
    pub fn toggle_mute(&mut self) -> bool {
        lock(&self.queue).toggle_mute()
    }

    /// What the queue counted so far.
    pub fn counters(&self) -> Counters {
        lock(&self.queue).counters()
    }
}

impl AudioSink for DeviceSink {
    fn buffer(&mut self, frames: &[[i16; 2]; BUFFER_FRAMES]) {
        lock(&self.queue).push(frames);
    }
    fn pause(&mut self) {
        lock(&self.queue).pause();
    }
    fn resume(&mut self) {
        lock(&self.queue).resume();
    }
    fn session_start(&mut self) {
        lock(&self.queue).session_start();
    }
}

/// The sink when no device is open: it takes every buffer and drops it, counting them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Discard {
    /// Buffers received and dropped.
    pub buffers: u64,
}

impl AudioSink for Discard {
    fn buffer(&mut self, _frames: &[[i16; 2]; BUFFER_FRAMES]) {
        self.buffers += 1;
    }
}

/// Where the session's mixed stream goes.
pub enum Output {
    /// A device opened.
    Device(DeviceSink),
    /// No device (none opened, or `--no-sound`): the stream is dropped.
    Discard(Discard),
}

impl Output {
    /// The output for the launch options: with `no_sound`, [`Output::Discard`] and no device is
    /// opened; otherwise the result of [`DeviceSink::open`], see [`Output::from_open`].
    pub fn for_launch(no_sound: bool, volume: Volume) -> (Self, Option<String>) {
        if no_sound {
            return (Self::Discard(Discard::default()), None);
        }
        Self::from_open(DeviceSink::open(volume))
    }

    /// The output for an attempt to open the device, and the single line to print on standard
    /// error when it failed (the game then runs the same, its stream dropped).
    pub fn from_open(opened: Result<DeviceSink, OpenError>) -> (Self, Option<String>) {
        match opened {
            Ok(sink) => (Self::Device(sink), None),
            Err(e) => (
                Self::Discard(Discard::default()),
                Some(format!(
                    "doom-play: {e}; the game runs without sound output"
                )),
            ),
        }
    }

    /// The mute key: mutes or unmutes a device; `None` when there is none.
    pub fn toggle_mute(&mut self) -> Option<bool> {
        match self {
            Self::Device(d) => Some(d.toggle_mute()),
            Self::Discard(_) => None,
        }
    }

    /// The device's backend and queue counters; `None` without a device.
    pub fn device_counters(&self) -> Option<(&'static str, Counters)> {
        match self {
            Self::Device(d) => Some((d.backend(), d.counters())),
            Self::Discard(_) => None,
        }
    }

    fn sink(&mut self) -> &mut dyn AudioSink {
        match self {
            Self::Device(d) => d,
            Self::Discard(d) => d,
        }
    }
}

impl AudioSink for Output {
    fn buffer(&mut self, frames: &[[i16; 2]; BUFFER_FRAMES]) {
        self.sink().buffer(frames);
    }
    fn pause(&mut self) {
        self.sink().pause();
    }
    fn resume(&mut self) {
        self.sink().resume();
    }
    fn session_start(&mut self) {
        self.sink().session_start();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_backend_list_names_only_real_backends() {
        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        assert!(!BACKENDS.is_empty());
        for &(backend, name) in BACKENDS {
            assert!(
                !matches!(backend, Backend::Null | Backend::Custom),
                "{name}"
            );
            assert!(!name.is_empty());
        }
        #[cfg(target_os = "windows")]
        assert_eq!(
            BACKENDS.iter().map(|b| b.1).collect::<Vec<_>>(),
            ["WASAPI", "DirectSound", "WinMM"]
        );
        #[cfg(target_os = "macos")]
        assert_eq!(BACKENDS, [(Backend::CoreAudio, "Core Audio")]);
        #[cfg(target_os = "linux")]
        assert_eq!(
            BACKENDS,
            [(Backend::PulseAudio, "PulseAudio"), (Backend::Alsa, "ALSA")]
        );
    }

    #[test]
    fn the_error_names_every_backend_tried() {
        let e = OpenError {
            tried: vec![
                ("PulseAudio", "MA_NO_BACKEND (-203)".into()),
                ("ALSA", "MA_NO_BACKEND (-203)".into()),
            ],
        };
        assert_eq!(
            e.to_string(),
            "no audio device opened; backends tried: PulseAudio (MA_NO_BACKEND (-203)), \
             ALSA (MA_NO_BACKEND (-203))"
        );
        let none = OpenError { tried: Vec::new() }.to_string();
        assert!(none.contains("no audio backend is listed"), "{none}");
    }

    /// A buffer whose frame i is (base + i, -(base + i)).
    fn ramp(base: i16) -> [[i16; 2]; BUFFER_FRAMES] {
        std::array::from_fn(|i| {
            let v = base + i as i16;
            [v, -v]
        })
    }

    /// The frames a pull of `n` frames gives.
    fn pull(q: &mut Queue, n: usize) -> Vec<[i16; 2]> {
        let mut out = vec![1i16; 2 * n];
        q.pull(&mut out);
        out.chunks_exact(2).map(|c| [c[0], c[1]]).collect()
    }

    #[test]
    fn the_numbers_are_those_of_the_specification() {
        // a start of 2 buffers (92.9 ms; review-app N5), a cap of 4 buffers (186 ms, §4.7.2)
        assert_eq!((BUFFER_FRAMES, START_FRAMES, CAP_FRAMES), (512, 1024, 2048));
        // latency by arithmetic (not measured): frames x 1000 / 11,025 in tenths of a ms
        assert_eq!(START_FRAMES * 10_000 / 11_025, 928);
        assert_eq!(BUFFER_FRAMES * 10_000 / 11_025, 464);
        // the declared schedule hands buffer k over after tic floor(512k/315), never after the
        // device needs it (512k/315 tics at 11,025 Hz), with no slack at k = 315: one buffer of
        // threshold leaves no margin there, two leave at least one tic, 315 frames (28.6 ms; see
        // START_FRAMES)
        for k in 0..=630u64 {
            let handed = (512 * k / 315) * 11_025; // in units of 1/(35 x 11,025) s
            let needed = 512 * k * 35;
            assert!(handed <= needed, "buffer {k}");
        }
        assert_eq!(
            (512 * 315 / 315) * 11_025,
            512 * 315 * 35,
            "k = 315: no slack"
        );
        assert_eq!(CAP_FRAMES * 10_000 / 11_025, 1857);
        assert_eq!(PERIOD_FRAMES as usize, BUFFER_FRAMES);
    }

    #[test]
    fn nothing_plays_before_the_start_threshold_and_underrun_is_silence_counted() {
        let mut q = Queue::new(Volume::FULL);
        assert!(pull(&mut q, 100).iter().all(|f| *f == [0, 0]), "empty");
        assert_eq!(
            q.counters(),
            Counters::default(),
            "not started: no underrun"
        );
        q.push(&ramp(1));
        assert_eq!(q.len(), 512);
        // one buffer is below the threshold of two: silence, no underrun
        assert!(pull(&mut q, 100).iter().all(|f| *f == [0, 0]), "one buffer");
        assert_eq!((q.len(), q.counters()), (512, Counters::default()));
        q.push(&ramp(513));
        let got = pull(&mut q, 300);
        assert_eq!(got[0], [1, -1]);
        assert_eq!(got[299], [300, -300]);
        // 724 frames left; a pull of 800 gives them and 76 frames of silence
        let got = pull(&mut q, 800);
        assert_eq!(got[723], [1024, -1024]);
        assert!(got[724..].iter().all(|f| *f == [0, 0]));
        assert_eq!(
            q.counters(),
            Counters {
                underruns: 1,
                underrun_frames: 76,
                ..Counters::default()
            }
        );
        // after an underrun the threshold applies again: silence, not another underrun
        assert!(q.is_empty());
        assert_eq!(pull(&mut q, 50), vec![[0, 0]; 50]);
        assert_eq!(q.counters().underruns, 1);
        let mut next = [[0i16; 2]; BUFFER_FRAMES];
        next[0] = [7, 7];
        q.push(&next);
        assert_eq!(
            pull(&mut q, 1),
            vec![[0, 0]],
            "one buffer does not restart it"
        );
        q.push(&ramp(1));
        q.pull(&mut []);
        assert_eq!(
            pull(&mut q, 1),
            vec![[7, 7]],
            "two whole buffers restart it"
        );
        // an odd slice: the last sample is silence
        let mut out = [5i16; 3];
        q.pull(&mut out);
        assert_eq!(out, [0, 0, 0]);
    }

    #[test]
    fn overrun_drops_the_oldest_frames_and_counts_them() {
        let mut q = Queue::new(Volume::FULL);
        for k in 0..4 {
            q.push(&ramp(k * 1000));
        }
        assert_eq!(
            (q.len(), q.counters().overruns),
            (CAP_FRAMES, 0),
            "at the cap"
        );
        q.push(&ramp(4000));
        assert_eq!(q.len(), CAP_FRAMES, "never above the cap");
        assert_eq!(
            q.counters(),
            Counters {
                overruns: 1,
                dropped_frames: 512,
                ..Counters::default()
            }
        );
        // the first buffer (the oldest) is gone; the second is now at the front
        assert_eq!(pull(&mut q, 1), vec![[1000, -1000]]);
        let rest = pull(&mut q, CAP_FRAMES - 1);
        assert_eq!(rest.last(), Some(&[4511, -4511]), "the newest frames kept");
        assert_eq!(q.counters().underruns, 0);
    }

    #[test]
    fn pause_and_session_start_clear_the_queue() {
        let mut q = Queue::new(Volume::FULL);
        q.push(&ramp(1));
        q.push(&ramp(1));
        q.pause();
        assert!(q.is_empty());
        q.push(&ramp(1));
        assert!(q.is_empty(), "nothing is queued while paused");
        assert!(pull(&mut q, 600).iter().all(|f| *f == [0, 0]), "silent");
        q.resume();
        q.push(&ramp(9));
        q.push(&ramp(9));
        assert_eq!(
            pull(&mut q, 1),
            vec![[9, -9]],
            "the pre-pause audio is never heard"
        );
        q.session_start();
        assert!(q.is_empty());
        assert!(pull(&mut q, 10).iter().all(|f| *f == [0, 0]));
        q.push(&ramp(3));
        q.push(&ramp(3));
        assert_eq!(pull(&mut q, 1), vec![[3, -3]]);
        assert_eq!(q.counters(), Counters::default(), "no underrun was counted");
    }

    #[test]
    fn the_gain_is_integer_and_saturating_and_mute_is_silence() {
        for s in [i16::MIN, -12_345, -1, 0, 1, 12_345, i16::MAX] {
            assert_eq!(gain(s, Volume::FULL, false), s, "100 % is the stream");
            assert_eq!(gain(s, Volume::new(0).unwrap(), false), 0, "0 % is silence");
            assert_eq!(gain(s, Volume::FULL, true), 0, "muted");
        }
        let half = Volume::new(50).unwrap();
        assert_eq!(gain(1001, half, false), 500, "truncated toward zero");
        assert_eq!(gain(-1001, half, false), -500);
        assert_eq!(gain(i16::MIN, half, false), -16_384);
        assert_eq!(gain(i16::MAX, Volume::new(33).unwrap(), false), 10_813);
        assert_eq!(scale(i16::MAX, 3, 2), i16::MAX, "saturates high");
        assert_eq!(scale(i16::MIN, 3, 2), i16::MIN, "saturates low");
        assert_eq!(scale(22_000, 3, 2), i16::MAX);
        assert_eq!(scale(20_000, 3, 2), 30_000, "below the limit: exact");
        assert_eq!(scale(-22_000, 3, 2), i16::MIN);
        assert_eq!(scale(i16::MIN, -1, 1), i16::MAX);
        assert_eq!(Volume::new(101), None);
        assert_eq!(Volume::new(100), Some(Volume::FULL));
        // the gain is applied as frames leave the queue
        let mut q = Queue::new(half);
        q.push(&ramp(100));
        q.push(&ramp(100));
        assert_eq!(pull(&mut q, 2), vec![[50, -50], [50, -50]]);
        assert!(q.toggle_mute());
        assert_eq!(pull(&mut q, 1), vec![[0, 0]], "muted frames still leave");
        assert!(!q.toggle_mute());
        assert_eq!(pull(&mut q, 1), vec![[51, -51]]);
    }

    #[test]
    fn the_device_sink_passes_the_stream_through_its_queue() {
        let mut d = DeviceSink::headless(Volume::FULL);
        assert!(d._device.is_none());
        d.session_start();
        d.buffer(&ramp(1));
        d.buffer(&ramp(1));
        let mut out = [0i16; 4];
        d.pull(&mut out);
        assert_eq!(out, [1, -1, 2, -2]);
        assert!(d.toggle_mute());
        d.pull(&mut out);
        assert_eq!(out, [0; 4]);
        d.pause();
        d.resume();
        d.pull(&mut out);
        assert_eq!(d.counters(), Counters::default());
    }

    #[test]
    fn with_no_device_the_stream_is_dropped_and_one_line_says_so() {
        let failed = Err(OpenError {
            tried: vec![
                ("PulseAudio", "MA_NO_BACKEND (-203)".into()),
                ("ALSA", "MA_NO_BACKEND (-203)".into()),
            ],
        });
        let (mut out, line) = Output::from_open(failed);
        let line = line.expect("a line");
        assert_eq!(
            line,
            "doom-play: no audio device opened; backends tried: PulseAudio (MA_NO_BACKEND \
             (-203)), ALSA (MA_NO_BACKEND (-203)); the game runs without sound output"
        );
        assert!(!line.contains('\n'), "one line");
        assert!(matches!(out, Output::Discard(Discard { buffers: 0 })));
        out.session_start();
        out.buffer(&ramp(1));
        out.pause();
        out.resume();
        out.buffer(&ramp(1));
        assert!(matches!(out, Output::Discard(Discard { buffers: 2 })));
        assert_eq!(out.toggle_mute(), None);
        assert!(out.device_counters().is_none());
        // --no-sound opens nothing and prints nothing
        let (out, line) = Output::for_launch(true, Volume::FULL);
        assert!(line.is_none());
        assert!(matches!(out, Output::Discard(_)));
        // a device: no line
        let (mut out, line) = Output::from_open(Ok(DeviceSink::headless(Volume::FULL)));
        assert!(line.is_none());
        assert_eq!(out.toggle_mute(), Some(true));
        assert_eq!(
            out.device_counters(),
            Some(("none (headless)", Counters::default()))
        );
    }
}
