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
use maudio::device::device_info::DeviceInfo;
use maudio::device::device_type::DeviceType;
use maudio::logging::{LogLevel, LogOps as _};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

/// The output rate the mixer produces (11,025 Hz, §4.7.2); miniaudio converts when the
/// device runs at another rate.
pub const RATE: SampleRate = SampleRate::Sr11025;

/// The device period asked for, in frames: 128, a quarter of one 512-frame mixer buffer
/// ([`BUFFER_FRAMES`]), 11.6 ms at 11,025 Hz. It is a hint: the backend may change it. The
/// option `--experimental-audio-period-frames` ([`Trial`]) can ask for 256 or 512 instead.
///
/// Until 2026-11-21 the default was 512, one whole mixer buffer (§4.7.2, the M5c
/// specification). The owner chose 128 with the two-buffer start kept, by ear, on run d of his
/// Windows retest of 2026-11-20 (docs/validation/M5D.md, which also records his later runs at
/// 128): the sound is taken from the queue in smaller blocks, so it can leave sooner. It is
/// presentation only (decision T8); [`BUFFER_FRAMES`], [`START_FRAMES`] and [`CAP_FRAMES`] are
/// unchanged.
pub const PERIOD_FRAMES: u32 = 128;

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

/// An open and started playback device, the backend it came from and what it reported.
pub struct Opened {
    /// The device: signed 16-bit stereo at [`RATE`] as seen by the callback.
    pub device: Device<i16>,
    /// The name of the backend that opened it (an entry of [`BACKENDS`]).
    pub backend: &'static str,
    /// What the device reported before it was started.
    pub facts: DeviceFacts,
}

/// What a playback device reported about itself when it was opened (milestone M5d, read
/// through maudio 0.1.15's safe API only: `DeviceOps::get_name` and `DeviceOps::get_info`,
/// `src/device.rs:242-256`). maudio exposes no accessor for the granted internal sample
/// rate, period or period count; miniaudio prints those itself in its device report, which
/// [`open_playback`] turns on (see [`DeviceFacts::info_report`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceFacts {
    /// The device's name, or why it could not be read.
    pub name: Result<String, String>,
    /// The sample rates of the device's native formats, in Hz, sorted without repeats, and the
    /// number of reported formats whose rate could not be read; or why none could be read.
    pub native_rates: Result<(Vec<u32>, usize), String>,
    /// Whether miniaudio's own INFO messages are printed on standard error (its device
    /// report at open, `miniaudio.h:43887-43937`, gives the granted period and buffer): on, off
    /// (the default; `--audio-report` turns it on, the owner's ruling 7 of 2026-11-20), or why
    /// it could not be turned on.
    pub info_report: InfoReport,
}

/// Whether miniaudio's INFO report is printed (see [`DeviceFacts::info_report`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InfoReport {
    /// Printed (`--audio-report`).
    On,
    /// Not asked for (the default).
    Off,
    /// Asked for, but miniaudio refused: why.
    Unavailable(String),
}

impl DeviceFacts {
    /// The facts of a device read with `name` and `info`, the results of maudio's calls.
    pub fn read(
        name: Result<String, String>,
        info: Result<DeviceInfo, String>,
        info_report: InfoReport,
    ) -> Self {
        Self {
            name,
            native_rates: info.map(|i| native_rates(&i)),
            info_report,
        }
    }
}

/// The sample rate, in Hz, in the `Debug` text of one of maudio 0.1.15's `DeviceFormat`s:
/// its fields are private and `Debug` is its only view (`src/device/device_info.rs:262-282`),
/// written by `derive(Debug)` with the field `sample_rate` holding a `SampleRate`, whose
/// variants are `Sr<rate>` or `Custom(<rate>)` (`src/audio/sample_rate.rs:19-37`). `None`
/// when the text does not have that shape. maudio is pinned to that exact version.
pub fn rate_in_format_debug(debug: &str) -> Option<u32> {
    let rest = debug.split_once("sample_rate: ")?.1;
    let digits = rest
        .strip_prefix("Sr")
        .or_else(|| rest.strip_prefix("Custom("))?;
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    digits[..end].parse().ok()
}

/// The sample rates of `info`'s native formats, in Hz, sorted without repeats, and how many
/// reported formats gave no rate (maudio skips entries it cannot represent, such as a rate of
/// 0, `src/device/device_info.rs:77-88`, `src/audio/sample_rate.rs:108-111`).
pub fn native_rates(info: &DeviceInfo) -> (Vec<u32>, usize) {
    let formats = info.device_formats();
    let mut rates: Vec<u32> = formats
        .iter()
        .filter_map(|f| rate_in_format_debug(&format!("{f:?}")))
        .collect();
    let unreadable = info.format_count().saturating_sub(rates.len());
    rates.sort_unstable();
    rates.dedup();
    (rates, unreadable)
}

/// Opens and starts a playback device (signed 16-bit, 2 channels, [`RATE`], period hint
/// `period_frames`, normally [`PERIOD_FRAMES`]) from [`BACKENDS`] only, trying each backend
/// alone in order; a backend whose device opens but does not start counts as failed and the
/// next one is tried. `callback` is called for each backend tried and must fill its slice
/// (interleaved stereo frames).
///
/// With miniaudio's fixed-size callback (maudio's default: `noFixedSizedCallback` stays 0,
/// `src/device/device_builder.rs:958-970`), the callback is always asked for exactly
/// `period_frames` frames at [`RATE`]: miniaudio keeps an intermediary buffer of that many
/// client frames (`miniaudio.h:43798-43836`) and fires the callback only to refill it
/// (`miniaudio.h:20271-20302`).
///
/// With `report` (`--audio-report`; off by default, the owner's ruling 7 of 2026-11-20), before
/// each device is built, miniaudio's INFO messages are printed on standard error
/// (`LogOps::print_level`, maudio `src/logging.rs:151-188`, registered on the context's log,
/// which the device uses: `miniaudio.h:43129-43138`, `44119-44122`), so the device report
/// miniaudio posts at the end of a successful init (`miniaudio.h:43887-43937`: the backend,
/// the device's name, `Sample Rate: <stream> -> <device>` and `Buffer Size: <period>*<periods>
/// (<buffer>)`, the granted internal values) appears once per open.
pub fn open_playback<C>(
    period_frames: u32,
    report: bool,
    mut callback: impl FnMut() -> C,
) -> Result<Opened, OpenError>
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
        let info_report = if report {
            match context.log().print_level(LogLevel::Info) {
                Ok(()) => InfoReport::On,
                Err(e) => InfoReport::Unavailable(e.to_string()),
            }
        } else {
            InfoReport::Off
        };
        let mut fill = callback();
        let device = DeviceBuilder::playback()
            .i16()
            .playback_channels(2)
            .sample_rate(RATE)
            .period_size_frames(period_frames)
            .context(&context)
            .with_callback(move |_, out: &mut [i16]| fill(out));
        match device {
            Ok(mut device) => {
                // read before the start, while the callback does not run yet
                let facts = DeviceFacts::read(
                    device
                        .get_name(DeviceType::Playback)
                        .map_err(|e| e.to_string()),
                    device
                        .get_info(DeviceType::Playback)
                        .map_err(|e| e.to_string()),
                    info_report,
                );
                match device.device_start() {
                    Ok(()) => {
                        return Ok(Opened {
                            device,
                            backend: name,
                            facts,
                        });
                    }
                    Err(e) => tried.push((name, format!("the device did not start: {e}"))),
                }
            }
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
    let pulls = Arc::new(Mutex::new(PullStats::default()));
    let seen = Arc::clone(&pulls);
    let mut opened = open_playback(PERIOD_FRAMES, false, || {
        let counter = Arc::clone(&counter);
        let seen = Arc::clone(&seen);
        move |out: &mut [i16]| {
            out.fill(0); // digital silence only
            counter.fetch_add((out.len() / 2) as u64, Ordering::Relaxed);
            seen.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .record((out.len() / 2) as u64, None);
        }
    })
    .map_err(|e| e.to_string())?;
    let name = opened
        .facts
        .name
        .clone()
        .unwrap_or_else(|e| format!("name unavailable: {e}"));
    std::thread::sleep(std::time::Duration::from_secs(1));
    let stopped = opened.device.device_stop();
    stopped.map_err(|e| format!("{}: the device did not stop: {e}", opened.backend))?;
    let p = *pulls
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(format!(
        "audio probe: backend {}, device {name:?}; {} frames of silence requested in about 1 s; \
         {} callbacks of {} to {} frames; {}",
        opened.backend,
        frames.load(Ordering::Relaxed),
        p.pulls,
        p.min_frames,
        p.max_frames,
        rates_text(&opened.facts.native_rates)
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
/// callback's phase; that sentence was written for the 512-frame period that was the default
/// until 2026-11-21, and with [`PERIOD_FRAMES`] at 128 a callback takes 128 frames, 11.6 ms),
/// with 92.9 ms queued at the start, before the device's own buffering. The
/// cap of four buffers is unchanged. This is presentation (decision T8): nothing here reaches
/// the game, and none of it has been measured on a device. (An earlier version of this comment
/// gave the margin and the price as one whole buffer, 46.4 ms; the latency arithmetic of
/// 2026-11-18 in docs/validation/M5C.md corrected both figures. The code did not change.)
///
/// Milestone M5d: the experimental option `--experimental-audio-start-buffers 1` ([`Trial`])
/// sets a one-buffer threshold for the owner's latency trials; this default is unchanged (also
/// by the change of the default period to 128 frames on 2026-11-21).
pub const START_FRAMES: usize = 2 * BUFFER_FRAMES;

/// The start thresholds the experimental trial option accepts, in whole buffers.
pub const TRIAL_START_BUFFERS: [usize; 2] = [1, 2];

/// The device periods the experimental trial option accepts, in frames at [`RATE`].
pub const TRIAL_PERIOD_FRAMES: [u32; 3] = [128, 256, 512];

/// The experimental device settings of the owner's M5d latency trials (presentation only,
/// decision T8: they change how the finished stream reaches the device, never the stream, the
/// game or the demo). [`Trial::DEFAULT`] is a start threshold of [`START_FRAMES`] (as in M5c)
/// and a period of [`PERIOD_FRAMES`] (128 frames since 2026-11-21; 512 in M5c).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Trial {
    start_buffers: usize,
    period_frames: u32,
}

impl Trial {
    /// The defaults: two buffers and 128 frames.
    pub const DEFAULT: Self = Self {
        start_buffers: START_FRAMES / BUFFER_FRAMES,
        period_frames: PERIOD_FRAMES,
    };

    /// These settings with a start threshold of `buffers` whole buffers; `None` unless it is
    /// one of [`TRIAL_START_BUFFERS`].
    pub fn with_start_buffers(self, buffers: usize) -> Option<Self> {
        TRIAL_START_BUFFERS.contains(&buffers).then_some(Self {
            start_buffers: buffers,
            ..self
        })
    }

    /// These settings with a device period of `frames`; `None` unless it is one of
    /// [`TRIAL_PERIOD_FRAMES`].
    pub fn with_period_frames(self, frames: u32) -> Option<Self> {
        TRIAL_PERIOD_FRAMES.contains(&frames).then_some(Self {
            period_frames: frames,
            ..self
        })
    }

    /// The start threshold in frames.
    pub fn start_frames(self) -> usize {
        self.start_buffers * BUFFER_FRAMES
    }

    /// The device period asked for, in frames.
    pub fn period_frames(self) -> u32 {
        self.period_frames
    }

    /// The settings as the exit line gives them; a setting away from its default says so.
    pub fn describe(self) -> String {
        let period = self.period_frames;
        let period = if period == PERIOD_FRAMES {
            format!("period asked {period} frames")
        } else {
            format!("period asked {period} frames (EXPERIMENTAL; default {PERIOD_FRAMES})")
        };
        let (frames, buffers) = (self.start_frames(), self.start_buffers);
        let plural = if buffers == 1 { "" } else { "s" };
        let start = if frames == START_FRAMES {
            format!("start threshold {frames} frames ({buffers} buffer{plural})")
        } else {
            format!(
                "start threshold {frames} frames ({buffers} buffer{plural}; EXPERIMENTAL; \
                 default {START_FRAMES} frames)"
            )
        };
        format!("{period}; {start}")
    }
}

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

/// What the queue counted (diagnostics only, printed in the exit lines whenever a device is
/// open; before milestone M5d only with `--tic-stats`).
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

/// How many underruns the exit lines list one by one ([`underrun_line`]); the rest are only
/// counted. The list's storage is reserved when the queue is made, so the device's callback
/// never allocates for it.
pub const UNDERRUNS_LISTED: usize = 32;

/// Why the queue last began to fill from empty towards its start threshold.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fill {
    /// A session started ([`Queue::session_start`]), or the queue is new.
    SessionStart,
    /// The app resumed from a pause ([`Queue::resume`]).
    Resume,
    /// An underrun before this one.
    Underrun,
}

/// Where the game was at one underrun (diagnostics only, printed in the exit lines; added
/// 2026-11-21 so that a run answers whether its underruns are the start-up fill).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Underrun {
    /// The session of the launch, counted from 1 ([`Queue::session_start`] calls so far; 0
    /// only for a queue no session was started on).
    pub session: u64,
    /// The tics of that session committed when the loop last said so before the callback came
    /// ([`Queue::note_tic`], after every update of the app; 0 before the first). It can be
    /// behind by the tics of an update in progress.
    pub tic: u64,
    /// Whether the app was playing when the loop last said so. `false` is a session that had
    /// ended: the app sends no more buffers then and does not pause the sink, so the queue
    /// runs dry once and that is counted as an underrun, though it is no gap in play.
    pub playing: bool,
    /// The frames of silence this one callback wrote ([`Counters::underrun_frames`] is their
    /// sum). They are what one callback missed, not the length of the gap heard.
    pub frames: u64,
    /// Why the queue had last begun to fill.
    pub fill: Fill,
    /// The mixer buffers pushed since it began to fill. While playing, a small number after
    /// [`Fill::SessionStart`] or [`Fill::Resume`] is the start-up fill running dry.
    pub buffers: u64,
}

/// What the device's callback saw, pull by pull (diagnostics only, printed in the exit lines
/// whenever a device is open; milestone M5d).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PullStats {
    /// Pulls (calls of the device's callback), paused or not.
    pub pulls: u64,
    /// The fewest frames one pull asked for (0 before the first pull).
    pub min_frames: u64,
    /// The most frames one pull asked for.
    pub max_frames: u64,
    /// Pulls that sampled the queue's depth: every pull while not paused (paused, the queue
    /// is empty and nothing arrives).
    pub depth_samples: u64,
    /// The fewest frames queued when a sampling pull arrived, before it took any.
    pub depth_min: u64,
    /// The most frames queued when a sampling pull arrived, before it took any.
    pub depth_max: u64,
    /// The sum of the sampled depths, in frames, for the mean.
    pub depth_sum: u64,
}

impl PullStats {
    fn record(&mut self, frames: u64, depth: Option<u64>) {
        let first = self.pulls == 0;
        self.pulls += 1;
        self.min_frames = if first {
            frames
        } else {
            self.min_frames.min(frames)
        };
        self.max_frames = self.max_frames.max(frames);
        if let Some(depth) = depth {
            let first = self.depth_samples == 0;
            self.depth_samples += 1;
            self.depth_min = if first {
                depth
            } else {
                self.depth_min.min(depth)
            };
            self.depth_max = self.depth_max.max(depth);
            self.depth_sum += depth;
        }
    }

    /// The mean sampled depth in frames, rounded to the nearest tenth (halves up), as text;
    /// `None` without a sample.
    pub fn depth_mean(&self) -> Option<String> {
        let n = self.depth_samples;
        (n > 0).then(|| {
            let tenths = (20 * self.depth_sum + n) / (2 * n);
            format!("{}.{}", tenths / 10, tenths % 10)
        })
    }
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
    start_frames: usize,
    stats: PullStats,
    /// Sessions started so far ([`Underrun::session`]).
    session: u64,
    /// The loop's last [`Queue::note_tic`] of this session ([`Underrun::tic`],
    /// [`Underrun::playing`]).
    tic: u64,
    playing: bool,
    /// Why the queue last began to fill, and the buffers pushed since.
    fill: Fill,
    fill_buffers: u64,
    /// The first [`UNDERRUNS_LISTED`] underruns, in order.
    underruns: Vec<Underrun>,
}

impl Queue {
    /// An empty queue, not started, not paused, not muted, with the start threshold
    /// [`START_FRAMES`].
    pub fn new(volume: Volume) -> Self {
        Self {
            frames: VecDeque::with_capacity(CAP_FRAMES + BUFFER_FRAMES),
            started: false,
            paused: false,
            muted: false,
            volume,
            counters: Counters::default(),
            start_frames: START_FRAMES,
            stats: PullStats::default(),
            session: 0,
            tic: 0,
            playing: true,
            fill: Fill::SessionStart,
            fill_buffers: 0,
            underruns: Vec::with_capacity(UNDERRUNS_LISTED),
        }
    }

    /// The queue with the start threshold `frames` instead (milestone M5d's experimental
    /// trial, [`Trial::start_frames`]).
    pub fn with_start_frames(self, frames: usize) -> Self {
        Self {
            start_frames: frames,
            ..self
        }
    }

    /// Appends one mixed buffer; beyond [`CAP_FRAMES`] the oldest frames are dropped and
    /// counted. Ignored while paused (no tic runs then, so none arrives).
    pub fn push(&mut self, buffer: &[[i16; 2]; BUFFER_FRAMES]) {
        if self.paused {
            return;
        }
        self.frames.extend(buffer.iter().copied());
        self.fill_buffers += 1;
        let over = self.frames.len().saturating_sub(CAP_FRAMES);
        if over > 0 {
            self.frames.drain(..over);
            self.counters.overruns += 1;
            self.counters.dropped_frames += over as u64;
        }
    }

    /// Fills `out` (interleaved stereo; an odd last sample is set to 0) with the oldest frames,
    /// volume and mute applied. Silence while paused or before the queue first holds its start
    /// threshold ([`START_FRAMES`] unless [`Queue::with_start_frames`] set another); when
    /// frames run out after the start, the rest is silence, the underrun is counted and the
    /// start threshold applies again. Every pull is counted in [`Queue::pull_stats`].
    pub fn pull(&mut self, out: &mut [i16]) {
        out.fill(0);
        let depth = (!self.paused).then_some(self.frames.len() as u64);
        self.stats.record((out.len() / 2) as u64, depth);
        if self.paused {
            return;
        }
        if !self.started {
            if self.frames.len() < self.start_frames {
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
            if self.underruns.len() < UNDERRUNS_LISTED {
                self.underruns.push(Underrun {
                    session: self.session,
                    tic: self.tic,
                    playing: self.playing,
                    frames: missing,
                    fill: self.fill,
                    buffers: self.fill_buffers,
                });
            }
            (self.fill, self.fill_buffers) = (Fill::Underrun, 0);
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
        (self.fill, self.fill_buffers) = (Fill::Resume, 0);
    }

    /// A new session: queued frames of the previous one are dropped.
    pub fn session_start(&mut self) {
        self.frames.clear();
        self.started = false;
        self.session += 1;
        self.tic = 0;
        self.playing = true;
        (self.fill, self.fill_buffers) = (Fill::SessionStart, 0);
    }

    /// The loop says how many tics of the session are committed and whether the app is playing
    /// (after every update of the app): an underrun is recorded with the last such values
    /// ([`Underrun::tic`], [`Underrun::playing`]). Diagnostics only: nothing but the underrun
    /// list reads them.
    pub fn note_tic(&mut self, tic: u64, playing: bool) {
        self.tic = tic;
        self.playing = playing;
    }

    /// The first [`UNDERRUNS_LISTED`] underruns, in order.
    pub fn underruns(&self) -> &[Underrun] {
        &self.underruns
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

    /// What the pulls saw so far.
    pub fn pull_stats(&self) -> PullStats {
        self.stats
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
    facts: DeviceFacts,
    trial: Trial,
}

impl DeviceSink {
    /// Opens and starts a device from [`BACKENDS`] with its callback pulling from a new queue,
    /// with the settings of `trial` ([`Trial::DEFAULT`] unless the experimental options say
    /// otherwise) and miniaudio's INFO report printed when `report` ([`open_playback`]).
    pub fn open(volume: Volume, trial: Trial, report: bool) -> Result<Self, OpenError> {
        let queue = Queue::new(volume).with_start_frames(trial.start_frames());
        let queue = Arc::new(Mutex::new(queue));
        let opened = open_playback(trial.period_frames(), report, || {
            let queue = Arc::clone(&queue);
            move |out: &mut [i16]| lock(&queue).pull(out)
        })?;
        Ok(Self {
            queue,
            _device: Some(opened.device),
            backend: opened.backend,
            facts: opened.facts,
            trial,
        })
    }

    /// A sink with no device, for headless tests: the caller pulls with [`DeviceSink::pull`].
    #[cfg(test)]
    pub fn headless(volume: Volume) -> Self {
        Self::headless_with(volume, Trial::DEFAULT)
    }

    /// A headless sink with the settings of `trial` (its period is only reported: the test
    /// chooses the pull sizes).
    #[cfg(test)]
    pub fn headless_with(volume: Volume, trial: Trial) -> Self {
        let no_device = "no device (headless)".to_string();
        Self {
            queue: Arc::new(Mutex::new(
                Queue::new(volume).with_start_frames(trial.start_frames()),
            )),
            _device: None,
            backend: "none (headless)",
            facts: DeviceFacts {
                name: Err(no_device.clone()),
                native_rates: Err(no_device.clone()),
                info_report: InfoReport::Unavailable(no_device),
            },
            trial,
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
    #[cfg(test)]
    pub fn counters(&self) -> Counters {
        self.snapshot().1
    }

    /// The pulls' statistics, the counters and the listed underruns, read under one lock so
    /// that they are of the same moment (the device's callback may run between two locks).
    fn snapshot(&self) -> (PullStats, Counters, Vec<Underrun>) {
        let q = lock(&self.queue);
        (q.pull_stats(), q.counters(), q.underruns().to_vec())
    }

    /// What the device's pulls saw so far.
    #[cfg(test)]
    pub fn pull_stats(&self) -> PullStats {
        self.snapshot().0
    }

    /// The session's committed tics and whether the app is playing, from the loop
    /// ([`Queue::note_tic`]).
    pub fn note_tic(&self, tic: u64, playing: bool) {
        lock(&self.queue).note_tic(tic, playing);
    }

    /// The first [`UNDERRUNS_LISTED`] underruns, in order.
    #[cfg(test)]
    pub fn underruns(&self) -> Vec<Underrun> {
        self.snapshot().2
    }

    /// The device's exit lines (milestone M5d), each to be printed after `doom-play: `: what
    /// the device reported and the settings asked for, what its pulls saw, and what the queue
    /// counted. One fact per field, units named.
    ///
    /// When at least one underrun was counted, one more line follows ([`underrun_line`]): the
    /// session and tic of each. Without an underrun there is no such line.
    pub fn exit_lines(&self) -> Vec<String> {
        let (pulls, counters, underruns) = self.snapshot();
        let mut lines = device_lines(self.backend(), &self.facts, self.trial, &pulls, &counters);
        lines.extend(underrun_line(&underruns, counters.underruns));
        lines
    }
}

/// The exit line that says where the game was at each underrun, `None` when `total` (the
/// counted underruns) is 0. `listed` are the first of them ([`UNDERRUNS_LISTED`] at most); the
/// line says how many more were counted and not listed.
pub fn underrun_line(listed: &[Underrun], total: u64) -> Option<String> {
    if total == 0 {
        return None;
    }
    let each: Vec<String> = listed
        .iter()
        .map(|u| {
            let why = match u.fill {
                Fill::SessionStart => "the session's start",
                Fill::Resume => "a resume",
                Fill::Underrun => "the underrun before",
            };
            format!(
                "session {} tic {}, {} frames, {} buffer{} after {why}{}",
                u.session,
                u.tic,
                u.frames,
                u.buffers,
                if u.buffers == 1 { "" } else { "s" },
                if u.playing {
                    ""
                } else {
                    " (the session had ended: its last sound ran out, no gap in play)"
                }
            )
        })
        .collect();
    let more = match total.saturating_sub(listed.len() as u64) {
        0 => String::new(),
        n => format!("; {n} more not listed"),
    };
    Some(format!(
        "audio underruns, each: session of this launch (from 1) and its tics committed at the \
         loop's last update before the callback, frames of silence in that callback, mixer \
         buffers queued since the queue last began to fill and why: {}{more}",
        each.join("; ")
    ))
}

/// The native sample rates as the exit lines and the probe give them.
fn rates_text(rates: &Result<(Vec<u32>, usize), String>) -> String {
    match rates {
        Ok((rates, unreadable)) => {
            let listed = if rates.is_empty() {
                "none reported".to_string()
            } else {
                let hz: Vec<String> = rates.iter().map(|r| format!("{r} Hz")).collect();
                hz.join(", ")
            };
            if *unreadable == 0 {
                format!("native sample rates {listed}")
            } else {
                format!("native sample rates {listed} ({unreadable} reported formats unreadable)")
            }
        }
        Err(e) => format!("native sample rates unavailable ({e})"),
    }
}

/// The lines of [`DeviceSink::exit_lines`] from their parts.
pub fn device_lines(
    backend: &str,
    facts: &DeviceFacts,
    trial: Trial,
    pulls: &PullStats,
    c: &Counters,
) -> Vec<String> {
    let name = match &facts.name {
        Ok(name) => format!("name {name:?}"),
        Err(e) => format!("name unavailable ({e})"),
    };
    let rates = rates_text(&facts.native_rates);
    let report = match &facts.info_report {
        InfoReport::On => {
            "granted period and buffer in miniaudio's [INFO] report at open".to_string()
        }
        InfoReport::Off => "granted period and buffer not shown (miniaudio's [INFO] report is \
                            off; --audio-report turns it on)"
            .to_string(),
        InfoReport::Unavailable(e) => format!("miniaudio's [INFO] report unavailable ({e})"),
    };
    let stream: u32 = RATE.into();
    let callbacks = if pulls.pulls == 0 {
        "audio callbacks: 0 calls".to_string()
    } else {
        format!(
            "audio callbacks: {} calls; frames per call min {}, max {}",
            pulls.pulls, pulls.min_frames, pulls.max_frames
        )
    };
    let depth = match pulls.depth_mean() {
        Some(mean) => format!(
            "depth in frames when a callback arrived (not paused) min {}, mean {mean}, max {} \
             ({} samples)",
            pulls.depth_min, pulls.depth_max, pulls.depth_samples
        ),
        None => "depth in frames when a callback arrived (not paused): no sample".to_string(),
    };
    vec![
        format!("audio device: backend {backend}; {name}; {rates}; stream {stream} Hz"),
        format!("audio settings: {}; {report}", trial.describe()),
        callbacks,
        format!(
            "audio queue: {depth}; underruns {} ({} frames of silence); overruns {} ({} frames \
             dropped)",
            c.underruns, c.underrun_frames, c.overruns, c.dropped_frames
        ),
    ]
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
    /// [`Output::for_launch_reporting`] with the default device settings ([`Trial::DEFAULT`])
    /// and no report.
    #[cfg(test)]
    pub fn for_launch(no_sound: bool, volume: Volume) -> (Self, Option<String>) {
        Self::for_launch_with(no_sound, volume, Trial::DEFAULT)
    }

    /// [`Output::for_launch_reporting`] without miniaudio's INFO report.
    #[cfg(test)]
    pub fn for_launch_with(no_sound: bool, volume: Volume, trial: Trial) -> (Self, Option<String>) {
        Self::for_launch_reporting(no_sound, volume, trial, false)
    }

    /// The output for the launch options: with `no_sound`, [`Output::Discard`] and no device is
    /// opened (`trial` and `report` are then unused); otherwise the result of
    /// [`DeviceSink::open`] (miniaudio's INFO report printed at open when `report`,
    /// `--audio-report`), see [`Output::from_open`].
    pub fn for_launch_reporting(
        no_sound: bool,
        volume: Volume,
        trial: Trial,
        report: bool,
    ) -> (Self, Option<String>) {
        if no_sound {
            return (Self::Discard(Discard::default()), None);
        }
        Self::from_open(DeviceSink::open(volume, trial, report))
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
    #[cfg(test)]
    pub fn device_counters(&self) -> Option<(&'static str, Counters)> {
        match self {
            Self::Device(d) => Some((d.backend(), d.counters())),
            Self::Discard(_) => None,
        }
    }

    /// The session's committed tics and whether the app is playing, from the loop, for the
    /// underrun list ([`Queue::note_tic`]); nothing without a device.
    pub fn note_tic(&self, tic: u64, playing: bool) {
        if let Self::Device(d) = self {
            d.note_tic(tic, playing);
        }
    }

    /// The device's exit lines ([`DeviceSink::exit_lines`]); none without a device.
    pub fn device_exit_lines(&self) -> Vec<String> {
        match self {
            Self::Device(d) => d.exit_lines(),
            Self::Discard(_) => Vec::new(),
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
        // The device period. Until 2026-11-21 this line was
        // `assert_eq!(PERIOD_FRAMES as usize, BUFFER_FRAMES)`: the specification's period of one
        // whole 512-frame buffer. The owner changed the default to 128 frames (his ruling of
        // 2026-11-20, docs/validation/M5D.md), so the line was edited with it: 128 frames, 11.6 ms,
        // four callbacks to one mixer buffer. The buffer, start and cap figures above are as before.
        assert_eq!(PERIOD_FRAMES, 128);
        assert_eq!(PERIOD_FRAMES as usize * 10_000 / 11_025, 116);
        assert_eq!(BUFFER_FRAMES, 4 * PERIOD_FRAMES as usize);
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

    // Milestone M5d step A2: the trial settings, the one-buffer threshold, the pull statistics,
    // the native rates and the device's exit lines (Rust-only checks; no device is opened).

    #[test]
    fn the_trial_settings_accept_only_the_listed_values_and_default_to_two_buffers_and_128() {
        let d = Trial::DEFAULT;
        assert_eq!((d.start_frames(), d.period_frames()), (1024, 128));
        assert_eq!(
            (d.start_frames(), d.period_frames()),
            (START_FRAMES, PERIOD_FRAMES)
        );
        assert_eq!(d.with_start_buffers(2), Some(d), "2 is the default");
        assert_eq!(d.with_period_frames(128), Some(d), "128 is the default");
        let one = d.with_start_buffers(1).expect("1 is accepted");
        assert_eq!((one.start_frames(), one.period_frames()), (512, 128));
        for bad in [0, 3, 4, 512, usize::MAX] {
            assert_eq!(d.with_start_buffers(bad), None, "{bad}");
        }
        for (frames, ok) in [(128, true), (256, true), (512, true)] {
            let t = one.with_period_frames(frames).expect("accepted");
            assert_eq!((t.start_frames(), t.period_frames()), (512, frames), "{ok}");
        }
        for bad in [0, 1, 64, 127, 129, 255, 384, 511, 513, 1024, u32::MAX] {
            assert_eq!(d.with_period_frames(bad), None, "{bad}");
        }
        assert_eq!(
            d.describe(),
            "period asked 128 frames; start threshold 1024 frames (2 buffers)"
        );
        assert_eq!(
            one.with_period_frames(512).unwrap().describe(),
            "period asked 512 frames (EXPERIMENTAL; default 128); start threshold 512 frames \
             (1 buffer; EXPERIMENTAL; default 1024 frames)"
        );
        assert_eq!(
            d.with_period_frames(256).unwrap().describe(),
            "period asked 256 frames (EXPERIMENTAL; default 128); start threshold 1024 frames \
             (2 buffers)"
        );
    }

    #[test]
    fn a_one_buffer_threshold_starts_at_512_frames_and_restarts_after_an_underrun() {
        let start = Trial::DEFAULT.with_start_buffers(1).unwrap().start_frames();
        let mut q = Queue::new(Volume::FULL).with_start_frames(start);
        assert_eq!(pull(&mut q, 64), vec![[0, 0]; 64], "empty: silence");
        assert_eq!(
            q.counters(),
            Counters::default(),
            "not started: no underrun"
        );
        // while not started the queue holds whole buffers only, so one buffer is the least
        // that can start it; the two-buffer queue stays silent with the same buffer
        let mut two = Queue::new(Volume::FULL);
        q.push(&ramp(1));
        two.push(&ramp(1));
        assert_eq!(
            pull(&mut two, 1),
            vec![[0, 0]],
            "two buffers: one is not enough"
        );
        assert_eq!(
            pull(&mut q, 1),
            vec![[1, -1]],
            "one buffer: 512 frames start it"
        );
        assert_eq!(q.counters(), Counters::default());
        // 511 left; a pull of 600 gives them and 89 frames of silence, one underrun
        let got = pull(&mut q, 600);
        assert_eq!(got[510], [512, -512]);
        assert!(got[511..].iter().all(|f| *f == [0, 0]));
        assert_eq!(
            q.counters(),
            Counters {
                underruns: 1,
                underrun_frames: 89,
                ..Counters::default()
            }
        );
        // the threshold applies again: nothing until 512 frames are queued, then output
        assert_eq!(pull(&mut q, 10), vec![[0, 0]; 10]);
        assert_eq!(q.counters().underruns, 1, "waiting is not an underrun");
        q.push(&ramp(1));
        assert_eq!(
            pull(&mut q, 2),
            vec![[1, -1], [2, -2]],
            "restarted at 512 frames"
        );
        // pause and resume: the threshold applies again too
        q.pause();
        q.resume();
        q.push(&ramp(100));
        assert_eq!(pull(&mut q, 1), vec![[100, -100]]);
        // the default queue keeps the M5c threshold
        assert_eq!(
            Queue::new(Volume::FULL),
            Queue::new(Volume::FULL).with_start_frames(1024)
        );
    }

    #[test]
    fn the_depth_statistics_sample_each_unpaused_pull_before_it_takes_frames() {
        let mut q = Queue::new(Volume::FULL);
        assert_eq!(q.pull_stats(), PullStats::default());
        assert_eq!(q.pull_stats().depth_mean(), None);
        q.pull(&mut [0; 2 * 100]); // depth 0
        q.push(&ramp(1));
        q.pull(&mut [0; 2 * 100]); // depth 512, below the threshold: nothing taken
        q.push(&ramp(1));
        q.pull(&mut [0; 2 * 300]); // depth 1024, takes 300
        q.push(&ramp(1));
        q.push(&ramp(1));
        q.pull(&mut [0; 2 * 512]); // depth 724 + 1024 = 1748, takes 512
        q.pause();
        q.pull(&mut [0; 2 * 512]); // paused: counted as a pull, no depth sample
        q.resume();
        q.push(&ramp(1));
        q.pull(&mut [0; 2 * 7]); // depth 512
        let s = q.pull_stats();
        assert_eq!(
            s,
            PullStats {
                pulls: 6,
                min_frames: 7,
                max_frames: 512,
                depth_samples: 5,
                depth_min: 0,
                depth_max: 1748,
                depth_sum: 512 + 1024 + 1748 + 512,
            }
        );
        // (0 + 512 + 1024 + 1748 + 512) / 5 = 759.2
        assert_eq!(s.depth_mean().as_deref(), Some("759.2"));
        // the mean rounds to the nearest tenth, halves up
        let mean = |sum, n| {
            PullStats {
                depth_samples: n,
                depth_sum: sum,
                ..PullStats::default()
            }
            .depth_mean()
        };
        assert_eq!(mean(1, 3).as_deref(), Some("0.3"), "0.333");
        assert_eq!(mean(2, 3).as_deref(), Some("0.7"), "0.667");
        assert_eq!(mean(1, 20).as_deref(), Some("0.1"), "0.05 rounds up");
        assert_eq!(mean(1, 21).as_deref(), Some("0.0"), "0.0476");
        assert_eq!(mean(2048, 1).as_deref(), Some("2048.0"));
    }

    #[test]
    fn the_callback_sizes_are_counted_for_every_pull_of_any_length() {
        let mut q = Queue::new(Volume::FULL);
        for frames in [512, 128, 300, 1, 777, 256] {
            let mut out = vec![0i16; 2 * frames];
            q.pull(&mut out);
        }
        let s = q.pull_stats();
        assert_eq!((s.pulls, s.min_frames, s.max_frames), (6, 1, 777));
        // an empty slice is a pull of 0 frames; an odd slice counts its whole frames only
        q.pull(&mut []);
        let mut odd = [0i16; 5];
        q.pull(&mut odd);
        let s = q.pull_stats();
        assert_eq!((s.pulls, s.min_frames, s.max_frames), (8, 0, 777));
        // the first pull sets the minimum (it does not stay at the default 0)
        let mut q = Queue::new(Volume::FULL);
        q.pull(&mut [0; 2 * 128]);
        q.pull(&mut [0; 2 * 256]);
        let s = q.pull_stats();
        assert_eq!((s.pulls, s.min_frames, s.max_frames), (2, 128, 256));
        // statistics never change the counters or what is pulled
        assert_eq!(q.counters(), Counters::default());
    }

    #[test]
    fn the_native_rates_are_read_from_the_reported_formats() {
        use maudio::audio::formats::Format;
        use maudio::device::device_info::DeviceInfoBuilder;
        let info = DeviceInfoBuilder::from_index(0, "test".into())
            .add_data_formats(Format::F32, 2, SampleRate::Sr48000, false)
            .add_data_formats(Format::S16, 2, SampleRate::Sr44100, true)
            .add_data_formats(Format::S16, 1, SampleRate::Sr48000, false)
            .add_data_formats(Format::S16, 2, SampleRate::Custom(12_345), false)
            .add_data_formats(Format::S16, 2, SampleRate::Sr11025, false)
            .build();
        assert_eq!(
            native_rates(&info),
            (vec![11_025, 12_345, 44_100, 48_000], 0)
        );
        // a rate of 0 is skipped by maudio and counted as unreadable here
        let info = DeviceInfoBuilder::from_index(1, "zero".into())
            .add_data_formats(Format::S16, 2, SampleRate::Custom(0), false)
            .add_data_formats(Format::S16, 2, SampleRate::Sr44100, false)
            .build();
        assert_eq!(native_rates(&info), (vec![44_100], 1));
        let none = DeviceInfoBuilder::from_index(2, "none".into()).build();
        assert_eq!(native_rates(&none), (vec![], 0));
        // the Debug text the parser reads, pinned
        assert_eq!(
            format!("{:?}", info.device_formats()[0]),
            "DeviceFormat { format: S16, channels: 2, sample_rate: Sr44100, exclusive: false }"
        );
        for (text, want) in [
            (
                "DeviceFormat { sample_rate: Sr48000, exclusive: false }",
                Some(48_000),
            ),
            (
                "DeviceFormat { sample_rate: Custom(7), exclusive: false }",
                Some(7),
            ),
            ("DeviceFormat { sample_rate: Sr8000 }", Some(8_000)),
            ("DeviceFormat { rate: 48000 }", None),
            ("DeviceFormat { sample_rate: Other(1) }", None),
            ("DeviceFormat { sample_rate: Sr }", None),
        ] {
            assert_eq!(rate_in_format_debug(text), want, "{text}");
        }
    }

    /// Ruling 7: with the report off (the default) the settings line does not point at a report
    /// that was never printed.
    #[test]
    fn the_settings_line_says_when_the_info_report_is_off() {
        let facts = DeviceFacts {
            name: Ok("x".into()),
            native_rates: Ok((vec![], 0)),
            info_report: InfoReport::Off,
        };
        let lines = device_lines(
            "ALSA",
            &facts,
            Trial::DEFAULT,
            &PullStats::default(),
            &Counters::default(),
        );
        assert_eq!(
            lines[1],
            "audio settings: period asked 128 frames; start threshold 1024 frames (2 buffers); \
             granted period and buffer not shown (miniaudio's [INFO] report is off; \
             --audio-report turns it on)"
        );
        assert!(!lines[1].contains("report at open"));
    }

    #[test]
    fn the_device_lines_give_one_fact_per_field_with_units() {
        let facts = DeviceFacts {
            name: Ok("Speakers (Test)".into()),
            native_rates: Ok((vec![44_100, 48_000], 0)),
            info_report: InfoReport::On,
        };
        let pulls = PullStats {
            pulls: 40,
            min_frames: 441,
            max_frames: 512,
            depth_samples: 38,
            depth_min: 0,
            depth_max: 1536,
            depth_sum: 31_357,
        };
        let c = Counters {
            underruns: 2,
            underrun_frames: 99,
            overruns: 1,
            dropped_frames: 512,
        };
        assert_eq!(
            device_lines("WASAPI", &facts, Trial::DEFAULT, &pulls, &c),
            [
                "audio device: backend WASAPI; name \"Speakers (Test)\"; native sample rates \
                 44100 Hz, 48000 Hz; stream 11025 Hz",
                "audio settings: period asked 128 frames; start threshold 1024 frames (2 \
                 buffers); granted period and buffer in miniaudio's [INFO] report at open",
                "audio callbacks: 40 calls; frames per call min 441, max 512",
                "audio queue: depth in frames when a callback arrived (not paused) min 0, mean \
                 825.2, max 1536 (38 samples); underruns 2 (99 frames of silence); overruns 1 \
                 (512 frames dropped)",
            ]
        );
        // what could not be read says so; a trial setting says it is experimental
        let facts = DeviceFacts {
            name: Err("MA_ERROR".into()),
            native_rates: Ok((vec![], 2)),
            info_report: InfoReport::Unavailable("MA_INVALID_ARGS".into()),
        };
        let trial = Trial::DEFAULT.with_start_buffers(1).unwrap();
        let lines = device_lines("ALSA", &facts, trial, &PullStats::default(), &c);
        assert_eq!(
            lines[0],
            "audio device: backend ALSA; name unavailable (MA_ERROR); native sample rates none \
             reported (2 reported formats unreadable); stream 11025 Hz"
        );
        assert_eq!(
            lines[1],
            "audio settings: period asked 128 frames; start threshold 512 frames (1 buffer; \
             EXPERIMENTAL; default 1024 frames); miniaudio's [INFO] report unavailable \
             (MA_INVALID_ARGS)"
        );
        assert_eq!(lines[2], "audio callbacks: 0 calls");
        assert!(
            lines[3].starts_with(
                "audio queue: depth in frames when a callback arrived (not paused): no sample; \
                 underruns 2"
            ),
            "{}",
            lines[3]
        );
        assert!(lines.iter().all(|l| !l.contains('\n')), "one line each");
        // a device sink's lines are these, from its own facts and statistics; no device, none
        let d = DeviceSink::headless_with(Volume::FULL, trial);
        d.pull(&mut [0; 2 * 64]);
        let lines = d.exit_lines();
        assert_eq!(lines.len(), 4);
        assert_eq!(
            lines[0],
            "audio device: backend none (headless); name unavailable (no device (headless)); \
             native sample rates unavailable (no device (headless)); stream 11025 Hz"
        );
        assert_eq!(
            lines[2],
            "audio callbacks: 1 calls; frames per call min 64, max 64"
        );
        let out = Output::Device(d);
        assert_eq!(out.device_exit_lines(), lines);
        assert!(
            Output::Discard(Discard::default())
                .device_exit_lines()
                .is_empty()
        );
        let (out, line) = Output::for_launch_with(true, Volume::FULL, trial);
        assert!(
            line.is_none() && out.device_exit_lines().is_empty(),
            "--no-sound"
        );
    }
}

/// The underrun list (2026-11-21): where the game was at each underrun. Rust-only checks of a
/// diagnostic; nothing here is evidence about what a person hears.
#[cfg(test)]
mod underrun_tests {
    use super::*;

    const BUF: [[i16; 2]; BUFFER_FRAMES] = [[1, 1]; BUFFER_FRAMES];

    /// Pushes `buffers` buffers and pulls `frames` frames in one callback.
    fn run(q: &mut Queue, buffers: usize, frames: usize) {
        for _ in 0..buffers {
            q.push(&BUF);
        }
        q.pull(&mut vec![0; 2 * frames]);
    }

    fn at(session: u64, tic: u64, frames: u64, fill: Fill, buffers: u64) -> Underrun {
        Underrun {
            session,
            tic,
            playing: true,
            frames,
            fill,
            buffers,
        }
    }

    /// Each underrun is recorded with the session, the loop's last tic count, the frames that
    /// one callback missed, and the buffers pushed since the queue last began to fill and why.
    #[test]
    fn an_underrun_is_recorded_with_its_session_tic_frames_and_fill() {
        let mut q = Queue::new(Volume::FULL);
        assert!(q.underruns().is_empty());
        q.session_start();
        q.note_tic(3, true);
        run(&mut q, 2, 1100); // 1,024 queued, 1,100 asked: the start-up fill runs dry
        let first = at(1, 3, 76, Fill::SessionStart, 2);
        assert_eq!(q.underruns(), [first]);
        // waiting for the threshold is no underrun and is not listed
        run(&mut q, 1, 100);
        assert_eq!(q.underruns().len(), 1);
        // the next one is counted from the underrun before: 3 buffers (1,536) since then
        q.note_tic(40, true);
        run(&mut q, 2, 1546);
        let second = at(1, 40, 10, Fill::Underrun, 3);
        assert_eq!(q.underruns()[1], second);
        // one buffer before a pause, and pushes while paused, are not counted after the
        // resume: the fill begins again there
        q.push(&BUF);
        q.pause();
        run(&mut q, 5, 4000);
        q.resume();
        q.note_tic(41, true);
        run(&mut q, 2, 1030);
        let third = at(1, 41, 6, Fill::Resume, 2);
        assert_eq!(q.underruns()[2], third);
        // a new session: its number goes up, its tic count is 0 until the loop says more,
        // and the buffer queued before it is dropped and not counted
        q.push(&BUF);
        q.note_tic(999, false);
        q.session_start();
        run(&mut q, 2, 1025);
        let fourth = at(2, 0, 1, Fill::SessionStart, 2);
        assert_eq!(q.underruns(), [first, second, third, fourth]);
        let c = q.counters();
        assert_eq!((c.underruns, c.underrun_frames), (4, 76 + 10 + 6 + 1));
    }

    /// A session that ended sends no more buffers and does not pause the sink, so its queue
    /// runs dry once. That is counted as an underrun and is listed as what it is: the loop
    /// had said the app was not playing.
    #[test]
    fn the_queue_running_dry_after_a_session_ended_is_marked() {
        let mut q = Queue::new(Volume::FULL);
        q.session_start();
        q.note_tic(50, true);
        run(&mut q, 4, 1024);
        q.note_tic(51, false); // the session ended in this update: no more buffers
        q.pull(&mut vec![0; 2 * 1024]);
        assert!(q.underruns().is_empty(), "the queue held what was asked");
        q.pull(&mut [0; 2 * 128]);
        let ended = Underrun {
            playing: false,
            ..at(1, 51, 128, Fill::SessionStart, 4)
        };
        assert_eq!(q.underruns(), [ended]);
        let line = underrun_line(q.underruns(), 1).unwrap();
        assert!(
            line.ends_with(
                ": session 1 tic 51, 128 frames, 4 buffers after the session's start (the \
                 session had ended: its last sound ran out, no gap in play)"
            ),
            "{line}"
        );
    }

    /// The list holds the first [`UNDERRUNS_LISTED`]; later ones are counted only, and the list
    /// never grows beyond the storage reserved for it.
    #[test]
    fn the_list_stops_at_its_limit_and_the_count_goes_on() {
        let mut q = Queue::new(Volume::FULL);
        let reserved = q.underruns.capacity();
        assert!(reserved >= UNDERRUNS_LISTED);
        q.session_start();
        for tic in 0..UNDERRUNS_LISTED as u64 + 5 {
            q.note_tic(tic, true);
            run(&mut q, 2, 1025);
        }
        assert_eq!(q.counters().underruns, UNDERRUNS_LISTED as u64 + 5);
        assert_eq!(q.underruns().len(), UNDERRUNS_LISTED);
        assert_eq!(q.underruns()[UNDERRUNS_LISTED - 1].tic, 31);
        assert_eq!(
            q.underruns.capacity(),
            reserved,
            "the callback never grew it"
        );
        let line = underrun_line(q.underruns(), q.counters().underruns).unwrap();
        assert!(line.ends_with("; 5 more not listed"), "{line}");
        assert_eq!(line.matches("session 1 tic ").count(), UNDERRUNS_LISTED);
    }

    /// The line's exact text; no line without an underrun.
    #[test]
    fn the_underrun_line_names_each_and_there_is_none_without_one() {
        assert_eq!(underrun_line(&[], 0), None);
        let listed = [
            at(1, 3, 76, Fill::SessionStart, 2),
            at(1, 2051, 128, Fill::Underrun, 1),
            at(2, 0, 64, Fill::Resume, 0),
        ];
        assert_eq!(
            underrun_line(&listed, 3).unwrap(),
            "audio underruns, each: session of this launch (from 1) and its tics committed at \
             the loop's last update before the callback, frames of silence in that callback, \
             mixer buffers queued since the queue last began to fill and why: session 1 tic 3, \
             76 frames, 2 buffers after the session's start; session 1 tic 2051, 128 frames, 1 \
             buffer after the underrun before; session 2 tic 0, 64 frames, 0 buffers after a \
             resume"
        );
    }

    /// A device sink's exit lines: the four device lines alone without an underrun, and the
    /// underrun line after them with one, with the count of those not listed. Without a device
    /// there is nothing to note.
    #[test]
    fn the_sinks_exit_lines_gain_the_underrun_line_only_after_an_underrun() {
        let mut d = DeviceSink::headless(Volume::FULL);
        d.session_start();
        d.note_tic(7, true);
        d.buffer(&BUF);
        d.buffer(&BUF);
        d.pull(&mut [0; 2 * 1024]);
        assert_eq!(d.exit_lines().len(), 4);
        d.pull(&mut [0; 2 * 128]);
        let lines = d.exit_lines();
        assert_eq!(lines.len(), 5);
        assert!(lines[3].contains("underruns 1 (128 frames of silence)"));
        assert!(
            lines[4]
                .ends_with(": session 1 tic 7, 128 frames, 2 buffers after the session's start"),
            "{}",
            lines[4]
        );
        let out = Output::Device(d);
        out.note_tic(8, true);
        assert_eq!(
            out.device_exit_lines(),
            lines,
            "a note changes no past line"
        );
        Output::Discard(Discard::default()).note_tic(1, true);
        // more underruns than the list holds: the sink's line says how many are not listed
        let Output::Device(mut d) = out else {
            unreachable!()
        };
        for _ in 0..UNDERRUNS_LISTED + 2 {
            d.buffer(&BUF);
            d.buffer(&BUF);
            d.pull(&mut [0; 2 * 1025]);
        }
        let lines = d.exit_lines();
        assert!(
            lines[3].contains(&format!("underruns {} (", UNDERRUNS_LISTED + 3)),
            "{}",
            lines[3]
        );
        assert!(lines[4].ends_with("; 3 more not listed"), "{}", lines[4]);
    }
}
