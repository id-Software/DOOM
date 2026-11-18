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
//! Nothing here is part of the game loop yet; the only user is the `--audio-probe` diagnostic,
//! which plays digital silence (all-zero samples) only.

use maudio::audio::sample_rate::SampleRate;
use maudio::backend::Backend;
use maudio::context::ContextBuilder;
use maudio::device::Device;
use maudio::device::DeviceOps as _;
use maudio::device::device_builder::{DeviceBuilder, DeviceBuilderOps as _};
use maudio::device::device_type::DeviceType;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

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

/// An open (not yet started) playback device and the backend it came from.
pub struct Opened {
    /// The device: signed 16-bit stereo at [`RATE`] as seen by the callback.
    pub device: Device<i16>,
    /// The name of the backend that opened it (an entry of [`BACKENDS`]).
    pub backend: &'static str,
}

/// Opens a playback device (signed 16-bit, 2 channels, [`RATE`], period hint
/// [`PERIOD_FRAMES`]) from [`BACKENDS`] only, trying each backend alone in order.
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
            Ok(device) => {
                return Ok(Opened {
                    device,
                    backend: name,
                });
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
    let started = opened.device.device_start();
    if started.is_ok() {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    let stopped = opened.device.device_stop();
    started.map_err(|e| format!("{}: the device did not start: {e}", opened.backend))?;
    stopped.map_err(|e| format!("{}: the device did not stop: {e}", opened.backend))?;
    Ok(format!(
        "audio probe: backend {}, device {name:?}; {} frames of silence requested in about 1 s",
        opened.backend,
        frames.load(Ordering::Relaxed)
    ))
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
}
