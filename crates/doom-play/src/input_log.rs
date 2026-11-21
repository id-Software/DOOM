//! `--input-log FILE`: a diagnostic record of every raw input doom-play receives from the
//! window library and every event the app posts to the game, one line per item, for the
//! stuck-fire report of 2026-11-18 (Windows: a left-button release that comes during mouse
//! motion seemed lost). It reads only: the app, the game, the demo and the frames are the same
//! with and without it (the test below runs one session both ways).
//!
//! Each line is `KIND PUMP TIC TEXT`. `PUMP` counts the window loop's `update` calls from 1:
//! the window library hands over the inputs that arrived since the previous call, then calls
//! `update`, which runs the tics that fell due (each tic first drains what the app's event
//! layer holds). A `raw` or `app` line written while inputs arrive carries the number of the
//! `update` call that follows it, so it usually shares its `PUMP` with the `game` lines of the
//! tics that drained it. The exceptions: `capture` lines (`capture again` too) and `cursor-left`
//! lines are written inside an `update` call, after its tics, so they carry that call's number;
//! a button release that decision S6 holds back reaches the game in a later tic, so its `game`
//! line carries a later `PUMP` than its `raw` and `app` lines; and inputs that arrive before an `update` call that runs no tic (the
//! game paused or ended, or no tic yet due) have no `game` line in their pump. `TIC` is the
//! app's tic count when the line is written; on a `game` or `tic` line it is the number of the
//! tic whose command was built after those events.
//!
//! - `session`: the session's demo file, named when the file is created (a restart begins a new
//!   session and a new file, and the tic count starts again from 0); an unrecorded session
//!   (`--unrecorded-full-turning`) creates no file and has no `session` line.
//! - `raw`: what the window library delivered: `button-down` / `button-up` (with the cursor
//!   position it reported; on Windows this is the position of the last cursor-motion message,
//!   not of the click itself, because miniquad 0.4.11 passes the last `WM_MOUSEMOVE` position
//!   with every button message), `motion` (raw device motion, `ignored` while the mouse is not
//!   captured), `cursor` (the window-relative cursor position, logged only, never forwarded;
//!   the library reports it only while the cursor is over the window's client area),
//!   `key-down` / `key-up`, `focus-lost` (minimised or unfocused), `restored` (restored from
//!   minimised, or focused), `resize`.
//! - `app`: what doom-play passed to the app or the window, or concluded itself: `mouse-motion`
//!   (the whole units given to `App::mouse_motion`), `mouse-button`, `capture on|off` (the cursor
//!   grab, with the window size). On Windows only (owner ruling 1 of the M5d Windows session,
//!   2026-11-20; `window::GRAB_AGAIN`) also: `cursor-left from x=X y=Y dx=DX dy=DY pumps=N`, the
//!   inference that the cursor left the client area while the grab is held (last cursor position,
//!   raw motion since and silent pumps; [`crate::cursor_leave`]), once until the next `cursor`
//!   line; and `capture again (REASONS)`, the grab asked for again while held, once per pump at
//!   most, with the reasons since the previous pump in the order `button-down`, `focus-gained`,
//!   `cursor-left`, separated by spaces. Both come after the pump's `capture` line, if any.
//! - `game`: an event the app posted before a tic's command was built: `keydown 0xNN`,
//!   `keyup 0xNN`, `mouse buttons=B dx=X dy=Y` (`ev_mouse`'s `data1`, `data2`, `data3`). Only
//!   committed and drawn tics report their events.
//! - `tic`: one per committed and drawn tic, after its `game` lines: the command the game
//!   executed (the demo's re-read copy) as `forward`, `side`, `turn` and `buttons`, and its
//!   fire bit (`BT_ATTACK`) as `fire=0` or `fire=1`, so a game that was told of a release and
//!   kept firing shows in the log itself.
//! - `pump`: the end of an `update` call in which something was logged or a tic ran: the tics
//!   run and dropped, the app's mode and whether the mouse is captured. The pump's lines are
//!   handed to the log's writer thread here (M5d, the owner's ruling of 2026-11-20 that no file
//!   write may block the tic loop), which writes and flushes each pump's lines in order; so a
//!   crash loses the current pump and the pumps the thread had not written yet. The lines are
//!   formatted on the loop's thread exactly as before, so the file's bytes are the same.
//! - `dropped`: `N lines of P pumps were not written: the log's writer was behind`. The pumps
//!   waiting for the thread are bounded ([`LOG_QUEUE_PUMPS`]; the loop never waits for the
//!   thread while tics run): when that many are waiting (a disk that has stopped answering), a
//!   pump's lines are dropped whole and counted, and this one line is written in their place,
//!   before the lines of the first pump the thread takes again, with that pump's `PUMP` and
//!   `TIC`. It never appears while the disk keeps up, so the bytes are then those of b374aaa.
//! - `summary`: written on quit: the raw downs and ups counted per button. From then on the loop
//!   waits for the thread rather than drop a pump (no tic runs after quit); the window ends the
//!   log ([`InputLog::finish`]) after the last pump's line, so, as in b374aaa, a `pump` line
//!   follows the summary.

use crate::FsHost;
use crate::keys::Mute;
use crate::write_stats::{Kind, Timed, WriteTimes};
use doom_game::app::{App, AudioSink, Host, Mode, TicReport, Timestamp};
use doom_game::input::Event;
use std::fmt;
use std::io::Write;
use std::path::Path;
use std::sync::mpsc::{SyncSender, TrySendError};
use std::time::Instant;

// The window and this module's test share the mouse forwarding, which lives in its own module.
#[cfg(test)]
use crate::mouse_forward::Forward;
#[cfg(test)]
use doom_game::input::MouseButton;

/// The log: off, or its output and the counters.
pub(crate) struct InputLog {
    out: Option<Out>,
    pump: u64,
    busy: bool,
    downs: [u64; 3],
    ups: [u64; 3],
}

/// Where the lines go: a writer written on the loop's thread (`InputLog::to_writer`, for
/// tests), or the log's writer thread ([`InputLog::create`]; M5d, the owner's ruling of 2026-11-20
/// that no file write may block the tic loop).
enum Out {
    #[cfg(test)]
    Direct(Box<dyn Write>),
    Thread(LogThread),
}

/// The most pumps' lines waiting for the log's writer thread (about 30 seconds of play at 35
/// pumps a second; a few kilobytes each even with a 1,000 Hz mouse). Past it, a pump's lines
/// are dropped whole and counted (the module documentation's `dropped` line).
pub(crate) const LOG_QUEUE_PUMPS: usize = 1_024;

/// The log's writer thread: the loop formats the lines into `buf` (the same bytes the direct
/// writer gets) and hands them over at the end of each pump; the thread writes and flushes each
/// hand-over in order and times it. A failure stops the thread's writing and is reported on the
/// loop's next line or hand-over, as the direct writer's failure is. At most a bound of
/// hand-overs wait ([`LOG_QUEUE_PUMPS`]); while play runs a full queue drops the pump's lines
/// (counted in `dropped`), after the summary the loop waits instead (`ending`).
struct LogThread {
    buf: Vec<u8>,
    tx: Option<SyncSender<(Vec<u8>, u64)>>,
    shared: std::sync::Arc<std::sync::Mutex<LogState>>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// Lines and pumps dropped since the thread last took a hand-over.
    dropped: (u64, u64),
    /// The summary was written: hand-overs wait for room instead of dropping.
    ending: bool,
}

#[derive(Default)]
struct LogState {
    error: Option<String>,
    times: WriteTimes,
}

impl LogThread {
    /// Starts the thread writing to `w`, with at most `bound` hand-overs waiting.
    fn spawn(mut w: Box<dyn Write + Send>, bound: usize) -> Result<Self, String> {
        let shared = std::sync::Arc::new(std::sync::Mutex::new(LogState::default()));
        let state = std::sync::Arc::clone(&shared);
        let (tx, rx) = std::sync::mpsc::sync_channel::<(Vec<u8>, u64)>(bound);
        let thread = std::thread::Builder::new()
            .name("input-log-writer".into())
            .spawn(move || {
                for (bytes, tic) in rx {
                    if lock(&state).error.is_some() {
                        continue;
                    }
                    let began = Instant::now();
                    let r = w.write_all(&bytes).and_then(|()| w.flush());
                    let mut st = lock(&state);
                    st.times.record(
                        Timed {
                            kind: Kind::Log,
                            tic,
                            took: began.elapsed(),
                            began,
                        },
                        r.is_ok(),
                    );
                    if let Err(e) = r {
                        st.error = Some(e.to_string());
                    }
                }
            })
            .map_err(|e| format!("the input log's writer could not start: {e}"))?;
        Ok(Self {
            buf: Vec::new(),
            tx: Some(tx),
            shared,
            thread: Some(thread),
            dropped: (0, 0),
            ending: false,
        })
    }

    /// The thread's failure, if any.
    fn error(&self) -> Option<String> {
        lock(&self.shared).error.clone()
    }

    /// Hands the lines formatted so far over to the thread (`pump` and `tic` the loop's, for the
    /// `dropped` line and the timing). While the queue is full and the summary not written, the
    /// lines are dropped whole and counted; the first hand-over the thread takes after a drop
    /// begins with the `dropped` line.
    fn hand_over(&mut self, pump: u64, tic: u64) {
        if self.buf.is_empty() {
            return;
        }
        let Some(tx) = &self.tx else {
            self.buf.clear();
            return;
        };
        let mut bytes = Vec::new();
        let (lines, pumps) = self.dropped;
        if lines > 0 {
            let _ = writeln!(
                bytes,
                "dropped {pump} {tic} {lines} lines of {pumps} pumps were not written: the log's \
                 writer was behind"
            );
        }
        bytes.append(&mut self.buf);
        let sent = if self.ending {
            tx.send((bytes, tic)).map_err(|_| ())
        } else {
            match tx.try_send((bytes, tic)) {
                Ok(()) => Ok(()),
                Err(TrySendError::Full((mut bytes, _))) => {
                    // keep the buffer's allocation for the next pump, without the notice
                    let notice = bytes.iter().position(|&b| b == b'\n').map_or(0, |i| i + 1);
                    let skip = if lines > 0 { notice } else { 0 };
                    let n = bytes[skip..].iter().filter(|&&b| b == b'\n').count() as u64;
                    self.dropped = (lines + n, pumps + 1);
                    bytes.clear();
                    self.buf = bytes;
                    return;
                }
                Err(TrySendError::Disconnected(_)) => Err(()),
            }
        };
        match sent {
            Ok(()) => self.dropped = (0, 0),
            Err(()) => lock(&self.shared).error = Some("the input log's writer stopped".into()),
        }
    }

    /// Hands the rest over, waits for the thread to write everything and returns its timing.
    fn close(&mut self, pump: u64, tic: u64) -> WriteTimes {
        self.ending = true;
        self.hand_over(pump, tic);
        self.tx = None;
        if let Some(t) = self.thread.take() {
            if t.join().is_err() {
                lock(&self.shared).error = Some("the input log's writer panicked".into());
            }
        }
        lock(&self.shared).times.clone()
    }
}

impl Drop for LogThread {
    /// Writes what is left (as a dropped buffered writer would) and joins the thread.
    fn drop(&mut self) {
        self.close(0, 0);
    }
}

fn lock(m: &std::sync::Mutex<LogState>) -> std::sync::MutexGuard<'_, LogState> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The header line's text (written as `# 1 0 HEADER`, pump 1 and tic 0). The kinds are those of
/// the module documentation; `dropped` (M5d review 3) appears only when the log's writer fell
/// [`LOG_QUEUE_PUMPS`] pumps behind, and says how many lines are missing at that point.
const HEADER: &str = "doom-play input log 1: KIND PUMP TIC TEXT";

/// `BT_ATTACK`, the fire bit of a command's buttons (d_event.h:75), for the `tic` lines.
const BT_ATTACK: u8 = 1;

/// The button names of the log, by the game's button index (left, middle, right).
pub(crate) const BUTTONS: [&str; 3] = ["Left", "Middle", "Right"];

impl InputLog {
    /// No log: every call does nothing.
    pub(crate) fn off() -> Self {
        Self {
            out: None,
            pump: 1,
            busy: false,
            downs: [0; 3],
            ups: [0; 3],
        }
    }

    /// Creates (or truncates) `path` and writes the header line (before play, on the caller's
    /// thread, so the file holds it at once); every later line goes through the log's writer
    /// thread.
    pub(crate) fn create(path: &Path) -> Result<Self, String> {
        let header_failed = |e: std::io::Error| {
            format!("{}: the header could not be written ({e})", path.display())
        };
        let mut file =
            std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        writeln!(file, "# 1 0 {HEADER}")
            .and_then(|()| file.flush())
            .map_err(header_failed)?;
        Self::threaded(Box::new(file))
    }

    /// A log whose lines go to `w` through the log's writer thread, with no header line.
    pub(crate) fn threaded(w: Box<dyn Write + Send>) -> Result<Self, String> {
        Self::threaded_bounded(w, LOG_QUEUE_PUMPS)
    }

    /// [`InputLog::threaded`] with at most `bound` pumps waiting for the thread.
    pub(crate) fn threaded_bounded(w: Box<dyn Write + Send>, bound: usize) -> Result<Self, String> {
        let mut log = Self::off();
        log.out = Some(Out::Thread(LogThread::spawn(w, bound)?));
        Ok(log)
    }

    /// A log written to `w` on the caller's thread, starting with the header line (tests).
    #[cfg(test)]
    pub(crate) fn to_writer(w: Box<dyn Write>) -> Self {
        let mut log = Self::off();
        log.out = Some(Out::Direct(w));
        log.write("#", 0, format_args!("{HEADER}"));
        log
    }

    /// Whether lines are written.
    pub(crate) fn is_on(&self) -> bool {
        self.out.is_some()
    }

    /// Ends the log: the writer thread writes everything handed over and stops. Returns its
    /// timing (`None` for a log that is off or written directly). A failure found at the end is
    /// printed as during play.
    pub(crate) fn finish(&mut self) -> Option<WriteTimes> {
        let Some(Out::Thread(t)) = self.out.as_mut() else {
            return None;
        };
        let times = t.close(self.pump, 0);
        if let Some(e) = t.error() {
            eprintln!("doom-play: the input log stopped: {e}");
        }
        self.out = None;
        Some(times)
    }

    fn stop(&mut self, e: impl fmt::Display) {
        eprintln!("doom-play: the input log stopped: {e}");
        self.out = None;
    }

    fn write(&mut self, kind: &str, tic: u64, text: fmt::Arguments<'_>) {
        let pump = self.pump;
        let failed = match self.out.as_mut() {
            None => None,
            #[cfg(test)]
            Some(Out::Direct(w)) => writeln!(w, "{kind} {pump} {tic} {text}")
                .err()
                .map(|e| e.to_string()),
            Some(Out::Thread(t)) => t.error().or_else(|| {
                writeln!(t.buf, "{kind} {pump} {tic} {text}")
                    .err()
                    .map(|e| e.to_string())
            }),
        };
        if let Some(e) = failed {
            self.stop(e);
        }
    }

    /// One line of `kind` (`raw` or `app`) in the current pump.
    pub(crate) fn line(&mut self, kind: &str, tic: u64, text: fmt::Arguments<'_>) {
        if self.is_on() {
            self.busy = true;
            self.write(kind, tic, text);
        }
    }

    /// A raw button change, counted.
    pub(crate) fn raw_button(&mut self, tic: u64, index: usize, down: bool, x: f32, y: f32) {
        let Some(name) = BUTTONS.get(index) else {
            return;
        };
        if down {
            self.downs[index] += 1;
        } else {
            self.ups[index] += 1;
        }
        let what = if down { "button-down" } else { "button-up" };
        self.line("raw", tic, format_args!("{what} {name} x={x:.1} y={y:.1}"));
    }

    /// The events a committed tic's command was built after.
    pub(crate) fn game_events(&mut self, report: &TicReport<'_>) {
        if !self.is_on() {
            return;
        }
        self.busy = true;
        for e in report.events {
            match *e {
                Event::KeyDown(k) => {
                    self.write("game", report.tic, format_args!("keydown 0x{k:02x}"))
                }
                Event::KeyUp(k) => self.write("game", report.tic, format_args!("keyup 0x{k:02x}")),
                Event::Mouse { buttons, dx, dy } => self.write(
                    "game",
                    report.tic,
                    format_args!("mouse buttons={buttons} dx={dx} dy={dy}"),
                ),
            }
        }
    }

    /// The command a committed tic executed and its fire bit.
    pub(crate) fn executed(&mut self, report: &TicReport<'_>) {
        if !self.is_on() {
            return;
        }
        self.busy = true;
        let c = report.executed;
        let fire = u8::from(c.buttons & BT_ATTACK != 0);
        self.write(
            "tic",
            report.tic,
            format_args!(
                "cmd forward={} side={} turn={} buttons={} fire={fire}",
                c.forward, c.side, c.angle_turn, c.buttons
            ),
        );
    }

    /// The end of an `update` call: a `pump` line when anything happened, a flush, and the
    /// next pump number.
    pub(crate) fn pump_end(&mut self, app: &App, ran: u64, dropped: u64, captured: bool) {
        if self.is_on() && (self.busy || ran > 0) {
            let mode = match app.mode() {
                Mode::Playing => "playing",
                Mode::Paused => "paused",
                Mode::Ended(_) => "ended",
            };
            let capture = if captured { "on" } else { "off" };
            self.write(
                "pump",
                app.tic(),
                format_args!("ran={ran} dropped={dropped} mode={mode} capture={capture}"),
            );
            self.flush(app.tic());
        }
        self.busy = false;
        self.pump += 1;
    }

    /// The `summary` line and a flush. From here on the log's thread is waited for rather than a
    /// pump dropped (no tic runs after quit).
    pub(crate) fn summary(&mut self, tic: u64) {
        if let Some(Out::Thread(t)) = self.out.as_mut() {
            t.ending = true;
        }
        let [d, u] = [self.downs, self.ups];
        self.line(
            "summary",
            tic,
            format_args!(
                "raw downs/ups: Left {}/{} Middle {}/{} Right {}/{}",
                d[0], u[0], d[1], u[1], d[2], u[2]
            ),
        );
        self.flush(tic);
    }

    /// Flushes the direct writer, or hands the pump's lines over to the writer thread (`tic`
    /// is the app's tic, for the timing).
    fn flush(&mut self, tic: u64) {
        let pump = self.pump;
        let failed = match self.out.as_mut() {
            None => None,
            #[cfg(test)]
            Some(Out::Direct(w)) => w.flush().err().map(|e| e.to_string()),
            Some(Out::Thread(t)) => t.error().or_else(|| {
                t.hand_over(pump, tic);
                None
            }),
        };
        if let Some(e) = failed {
            self.stop(e);
        }
    }
}

/// The host of the window: the host proper (the window runs
/// [`crate::session_writer::WriterHost`]; tests use [`FsHost`]) and the log, which hears every
/// committed tic's events and changes nothing else.
pub(crate) struct LoggedHost<H = FsHost> {
    /// The host proper.
    pub(crate) host: H,
    /// The input log.
    pub(crate) log: InputLog,
}

impl<H: Host> Host for LoggedHost<H> {
    fn timestamp(&mut self) -> Timestamp {
        self.host.timestamp()
    }
    fn exists(&mut self, name: &str) -> bool {
        self.host.exists(name)
    }
    fn claim(&mut self, name: &str, bytes: &[u8]) -> Result<bool, String> {
        let claimed = self.host.claim(name, bytes);
        if matches!(claimed, Ok(true)) {
            self.log.line("session", 0, format_args!("{name}"));
        }
        claimed
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.host.save(name, bytes)
    }
    fn appends(&self) -> bool {
        self.host.appends()
    }
    fn append(&mut self, name: &str, at: u64, tail: &[u8; 5]) -> Result<bool, String> {
        self.host.append(name, at, tail)
    }
    fn tic(&mut self, report: &TicReport<'_>) {
        self.log.game_events(report);
        self.log.executed(report);
        self.host.tic(report);
    }
    fn audio(&mut self) -> Option<&mut dyn AudioSink> {
        self.host.audio()
    }
}

impl<H: Mute> Mute for LoggedHost<H> {
    fn toggle_mute(&mut self) {
        self.host.toggle_mute();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{Discard, Output};
    use crate::keys::key_event;
    use doom_game::app::{GameData, SessionConfig};
    use doom_game::input::Preset;
    use doom_render::GameMode;
    use miniquad::KeyCode;

    fn load() -> GameData {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
        let files: Vec<Vec<u8>> = ["m5a_wads/amp.wad", "m5b_gfx.wad", "m5c_sfx.wad"]
            .iter()
            .map(|w| std::fs::read(dir.join(w)).expect("WAD"))
            .collect();
        let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
        GameData::load_with_sound(&refs, GameMode::Registered, 1, 2).expect("game data")
    }

    fn fnv(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3)
        })
    }

    fn at(k: u64) -> u64 {
        (k * 1_000_000).div_ceil(35)
    }

    /// One raw input of the script.
    #[derive(Clone, Copy, Debug)]
    enum R {
        Down(Option<MouseButton>),
        Up(Option<MouseButton>),
        Motion(f32, f32),
        Cursor(f32, f32),
        Key(KeyCode, bool),
    }

    /// Pumps of raw inputs: a drag with the left button held, a release during motion, clicks
    /// during motion, a key fire and an unknown button.
    fn script() -> Vec<Vec<R>> {
        use MouseButton::{Left, Right};
        use R::*;
        let mut s: Vec<Vec<R>> = vec![vec![]; 3];
        s.push(vec![Motion(2.5, 0.25), Down(Some(Left)), Motion(3.0, 0.0)]);
        s.extend(vec![vec![Motion(4.25, -0.5), Cursor(500.0, 360.0)]; 5]);
        s.push(vec![Motion(4.0, 0.0), Up(Some(Left)), Motion(4.0, 0.0)]);
        s.push(vec![Motion(-3.0, 0.0)]);
        for _ in 0..4 {
            s.push(vec![
                Motion(1.0, 0.0),
                Down(Some(Left)),
                Motion(1.0, 0.0),
                Up(Some(Left)),
            ]);
            s.push(vec![Motion(1.0, 0.0)]);
        }
        s.push(vec![Key(KeyCode::RightControl, true), Motion(2.0, 1.0)]);
        s.push(vec![
            Key(KeyCode::RightControl, false),
            Down(Some(Right)),
            Down(None),
        ]);
        s.push(vec![Up(Some(Right)), Up(None)]);
        s.extend(vec![vec![]; 3]);
        s
    }

    /// The pump structure of the window loop: inputs, `update`, capture, `pump_end`.
    fn run(log: InputLog, dir: &Path) -> (Vec<u64>, Vec<u8>, InputLog) {
        let host = FsHost {
            dir: dir.to_path_buf(),
            audio: Output::Discard(Discard::default()),
        };
        let mut h = LoggedHost { host, log };
        let cfg = SessionConfig {
            preset: Preset::Modern,
            ..SessionConfig::default()
        };
        let mut app = App::new(load(), cfg, &mut h);
        let mut fwd = Forward::default();
        let mut frames = Vec::new();
        let u = app.update(0, &mut h);
        fwd.sync_capture(&app, &mut h.log, || (960.0, 720.0));
        h.log.pump_end(&app, u.ran, u.dropped, fwd.captured());
        for (k, pump) in script().iter().enumerate() {
            for r in pump {
                let tic = app.tic();
                match *r {
                    R::Down(b) => fwd.button(&mut app, &mut h.log, b, true, (480.0, 360.0)),
                    R::Up(b) => fwd.button(&mut app, &mut h.log, b, false, (480.0, 360.0)),
                    R::Motion(x, y) => fwd.raw_motion(&mut app, &mut h.log, x, y),
                    R::Cursor(x, y) => {
                        h.log
                            .line("raw", tic, format_args!("cursor x={x:.1} y={y:.1}"))
                    }
                    R::Key(key, down) => {
                        let what = if down { "key-down" } else { "key-up" };
                        h.log.line("raw", tic, format_args!("{what} {key:?}"));
                        key_event(&mut app, &mut h, key, down, false);
                    }
                }
            }
            let u = app.update(at(k as u64 + 1), &mut h);
            assert_eq!(u.ran, 1);
            fwd.sync_capture(&app, &mut h.log, || (960.0, 720.0));
            h.log.pump_end(&app, u.ran, u.dropped, fwd.captured());
            frames.push(fnv(app.frame().expect("a frame").0));
        }
        app.quit(&mut h);
        h.log.summary(app.tic());
        let demo = std::fs::read(dir.join(app.session_name())).expect("the demo");
        (frames, demo, h.log)
    }

    /// A writer the test can read back.
    #[derive(Clone, Default)]
    struct Shared(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

    impl Write for Shared {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn the_log_changes_no_frame_or_demo_byte_and_records_every_input_and_event() {
        let base = std::env::temp_dir().join(format!("doom-play-input-log-{}", std::process::id()));
        let (off_dir, on_dir) = (base.join("off"), base.join("on"));
        let (frames_off, demo_off, log_off) = run(InputLog::off(), &off_dir);
        assert!(!log_off.is_on());
        let text = Shared::default();
        let (frames_on, demo_on, log_on) =
            run(InputLog::to_writer(Box::new(text.clone())), &on_dir);
        assert!(log_on.is_on());
        assert_eq!(frames_on, frames_off, "the same frames");
        assert_eq!(demo_on, demo_off, "the same demo bytes");
        assert_eq!(frames_on.len(), script().len());
        std::fs::remove_dir_all(&base).unwrap();

        let text = String::from_utf8(text.0.borrow().clone()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "# 1 0 doom-play input log 1: KIND PUMP TIC TEXT");
        let count = |kind: &str, what: &str| {
            lines
                .iter()
                .filter(|l| l.starts_with(&format!("{kind} ")) && l.contains(what))
                .count()
        };
        // five left downs and five left ups, raw and forwarded; the unknown button is logged
        // and not forwarded
        assert_eq!(count("raw", "button-down Left"), 5);
        assert_eq!(count("raw", "button-up Left"), 5);
        assert_eq!(count("app", "mouse-button Left down"), 5);
        assert_eq!(count("app", "mouse-button Left up"), 5);
        assert_eq!(count("raw", "button-down Unknown"), 1);
        assert_eq!(count("app", "mouse-button Right"), 2);
        assert_eq!(count("raw", "key-down RightControl"), 1);
        assert_eq!(count("game", "keydown 0x9d"), 1);
        assert_eq!(count("raw", "cursor x=500.0"), 5);
        assert_eq!(count("app", "capture on window 960x720"), 1);
        assert!(lines.last().unwrap().starts_with("summary "));
        assert!(
            lines
                .last()
                .unwrap()
                .ends_with("Left 5/5 Middle 0/0 Right 1/1")
        );
        // every raw and app line, and every game line, carries a pump; the pump of the press
        // during motion (script pump 3, update 5) also carries the game's mouse event with the
        // button set and the pump's whole motion: 2.5 + 3.0 = 5 whole units, << 2
        let fields = |l: &str| -> (String, u64, u64) {
            let mut w = l.split(' ');
            let kind = w.next().unwrap().to_string();
            (
                kind,
                w.next().unwrap().parse().unwrap(),
                w.next().unwrap().parse().unwrap(),
            )
        };
        for l in &lines {
            fields(l);
        }
        assert!(lines.contains(&"raw 5 3 button-down Left x=480.0 y=360.0"));
        assert!(lines.contains(&"game 5 3 mouse buttons=1 dx=20 dy=0"));
        // the release during motion (script pump 9, update 11) reaches the game in the same
        // pump as a mouse event with no button and that pump's motion
        assert!(lines.contains(&"raw 11 9 button-up Left x=480.0 y=360.0"));
        let release = lines
            .iter()
            .find(|l| l.starts_with("game 11 9 mouse"))
            .expect("the release's event");
        assert!(
            release.starts_with("game 11 9 mouse buttons=0 dx="),
            "{release}"
        );
        let pumps = lines.iter().filter(|l| l.starts_with("pump ")).count();
        assert_eq!(
            pumps,
            script().len() + 1,
            "one pump line per update that did anything"
        );
    }

    /// Review finding M1 (2026-11-18): the log names the session's file and gives every
    /// committed tic's executed command with its fire bit, in the same order as the demo.
    #[test]
    fn the_log_names_the_session_and_gives_each_tic_command_and_fire_bit() {
        let dir =
            std::env::temp_dir().join(format!("doom-play-input-log-m1-{}", std::process::id()));
        let text = Shared::default();
        let (frames, demo, _) = run(InputLog::to_writer(Box::new(text.clone())), &dir);
        let text = String::from_utf8(text.0.borrow().clone()).unwrap();
        let sessions: Vec<&str> = text.lines().filter(|l| l.starts_with("session ")).collect();
        assert_eq!(sessions.len(), 1, "{sessions:?}");
        let name = sessions[0].rsplit(' ').next().unwrap();
        assert!(dir.join(name).is_file(), "{name}");
        std::fs::remove_dir_all(&dir).unwrap();
        let tics: Vec<&str> = text.lines().filter(|l| l.starts_with("tic ")).collect();
        assert_eq!(tics.len(), frames.len());
        // the demo's command bytes: forward, side, turn (high byte), buttons
        for (k, (line, cmd)) in tics
            .iter()
            .zip(demo[13..demo.len() - 1].chunks_exact(4))
            .enumerate()
        {
            let (f, sd, t, b) = (
                cmd[0] as i8,
                cmd[1] as i8,
                i16::from(cmd[2] as i8) << 8,
                cmd[3],
            );
            let want = format!(
                "cmd forward={f} side={sd} turn={t} buttons={b} fire={}",
                b & 1
            );
            assert_eq!(
                line.split(' ').nth(2),
                Some(k.to_string().as_str()),
                "{line}"
            );
            assert!(line.ends_with(&want), "{line} vs {want}");
        }
        // the drag's press sets the fire bit and its release clears it
        assert!(tics.iter().any(|l| l.ends_with("fire=1")));
        assert!(tics.iter().any(|l| l.ends_with("fire=0")));
    }

    #[test]
    fn the_option_is_off_by_default_and_listed_in_the_help() {
        let args = |s: &str| s.split_whitespace().map(String::from).collect::<Vec<_>>();
        let o = crate::parse(args("--iwad a.wad")).unwrap().unwrap();
        assert_eq!(o.input_log, None, "off by default");
        let o = crate::parse(args("--iwad a.wad --input-log in.txt --bindings modern"))
            .unwrap()
            .unwrap();
        assert_eq!(o.input_log, Some(std::path::PathBuf::from("in.txt")));
        assert_eq!(o.preset, Preset::Modern);
        assert!(!o.unrecorded, "the log implies nothing else");
        assert!(crate::parse(args("--iwad a.wad --input-log")).is_err());
        assert!(crate::HELP.contains("--input-log FILE"));
        assert!(crate::HELP.contains("no effect on the game, the demo or the picture"));
        // a log file is created and holds its header at once
        let path = std::env::temp_dir().join(format!("doom-play-log-{}.txt", std::process::id()));
        let log = InputLog::create(&path).unwrap();
        assert!(log.is_on());
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text, "# 1 0 doom-play input log 1: KIND PUMP TIC TEXT\n");
        drop(log);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_failing_writer_stops_the_log_and_nothing_else() {
        struct Full;
        impl Write for Full {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk full"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let log = InputLog::to_writer(Box::new(Full));
        assert!(!log.is_on(), "the header write failed, so the log is off");
        assert!(InputLog::create(Path::new("/nonexistent-dir/x/log.txt")).is_err());
    }

    /// A writer the log's thread can own, which the test reads back; each write call can be
    /// made slow.
    #[derive(Clone, Default)]
    struct SharedSend {
        bytes: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
        delay: std::time::Duration,
    }

    impl Write for SharedSend {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            std::thread::sleep(self.delay);
            self.bytes.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// M5d (ruling of 2026-11-20): the log written by its own thread holds, after `finish`,
    /// exactly the bytes the direct writer gets for the same session (the header is written by
    /// `create` before the thread starts, so it is the one difference), and it is timed.
    #[test]
    fn the_threaded_log_writes_the_same_bytes() {
        let base =
            std::env::temp_dir().join(format!("doom-play-input-log-thread-{}", std::process::id()));
        let direct = Shared::default();
        let (frames_a, demo_a, _) = run(
            InputLog::to_writer(Box::new(direct.clone())),
            &base.join("a"),
        );
        let threaded = SharedSend::default();
        let (frames_b, demo_b, mut log) = run(
            InputLog::threaded(Box::new(threaded.clone())).unwrap(),
            &base.join("b"),
        );
        assert_eq!((frames_a, demo_a), (frames_b, demo_b));
        let times = log.finish().expect("a threaded log");
        assert!(!log.is_on(), "finished");
        assert!(times.count > 20 && times.failed == 0, "{times:?}");
        let direct = String::from_utf8(direct.0.borrow().clone()).unwrap();
        let threaded = String::from_utf8(threaded.bytes.lock().unwrap().clone()).unwrap();
        let header = format!("# 1 0 {HEADER}\n");
        // the session's file name holds the second it began, which may differ between the runs
        let unnamed = |t: &str| -> String {
            t.lines()
                .map(|l| {
                    if l.starts_with("session ") {
                        "session"
                    } else {
                        l
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        let direct = direct.strip_prefix(&header).expect("the header first");
        assert_eq!(unnamed(direct), unnamed(&threaded));
        assert_eq!(threaded.matches("session 1 0 E1M1-skill2-").count(), 1);
        assert!(threaded.ends_with("Left 5/5 Middle 0/0 Right 1/1\n"));
        std::fs::remove_dir_all(&base).unwrap();
    }

    /// The ruling that no write may block the loop, for the log: with every write 200 ms slow,
    /// each line and each pump's end returns at once; `finish` waits for the writes, and a
    /// failure of the thread's writer stops the log at the next line, as the direct writer's does.
    #[test]
    fn a_slow_log_file_never_holds_up_the_pump() {
        let slow = SharedSend {
            delay: std::time::Duration::from_millis(200),
            ..SharedSend::default()
        };
        let mut log = InputLog::threaded(Box::new(slow.clone())).unwrap();
        let app = App::new(
            load(),
            SessionConfig::default(),
            &mut LoggedHost {
                host: FsHost {
                    dir: std::env::temp_dir()
                        .join(format!("doom-play-input-log-slow-{}", std::process::id())),
                    audio: Output::Discard(Discard::default()),
                },
                log: InputLog::off(),
            },
        );
        let mut longest = std::time::Duration::ZERO;
        for k in 0..5 {
            let t0 = std::time::Instant::now();
            log.line("raw", k, format_args!("line {k}"));
            log.pump_end(&app, 1, 0, false);
            longest = longest.max(t0.elapsed());
        }
        assert!(
            longest < std::time::Duration::from_millis(50),
            "a pump took {longest:?}"
        );
        let times = log.finish().unwrap();
        assert_eq!(times.count, 5);
        assert!(times.longest.unwrap().took >= std::time::Duration::from_millis(200));
        let text = String::from_utf8(slow.bytes.lock().unwrap().clone()).unwrap();
        assert_eq!(text.lines().count(), 10, "{text}");
        assert!(text.starts_with("raw 1 0 line 0\npump 1 0 ran=1 dropped=0"));
        let _ = std::fs::remove_dir_all(
            std::env::temp_dir().join(format!("doom-play-input-log-slow-{}", std::process::id())),
        );
        // a failing file: the thread's error stops the log at the next line
        struct Full;
        impl Write for Full {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk full"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut log = InputLog::threaded(Box::new(Full)).unwrap();
        log.line("raw", 0, format_args!("x"));
        log.pump_end(&app, 1, 0, false);
        for _ in 0..200 {
            if !log.is_on() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
            log.line("raw", 0, format_args!("y"));
        }
        assert!(!log.is_on(), "the failure stopped the log");
        assert_eq!(log.finish().map(|t| t.count), None, "already off");
    }
}

/// Review survivors of M5d: [`LoggedHost`] must pass `append` to the host proper.
#[cfg(test)]
mod logged_host_tests {
    use super::*;
    use crate::audio::{Discard, Output};

    /// Kills `<LoggedHost as Host>::append` returning `Ok(false)`: through the wrapper, a
    /// five-byte append at the marker of a claimed file is written (`Ok(true)`), exactly as the
    /// host proper writes it. The public API cannot reach it: the window's whole-file fallback
    /// would hide a decline (the file is the same after the next whole write).
    #[test]
    fn the_wrapper_delegates_append_to_the_host() {
        let dir =
            std::env::temp_dir().join(format!("doom-play-logged-append-{}", std::process::id()));
        let host = FsHost {
            dir: dir.clone(),
            audio: Output::Discard(Discard::default()),
        };
        let mut h = LoggedHost {
            host,
            log: InputLog::off(),
        };
        std::fs::create_dir_all(&dir).unwrap();
        let mut file = vec![7u8; 13];
        file.push(0x80);
        assert_eq!(h.claim("a.lmp", &file), Ok(true));
        assert!(h.appends());
        let tail = [1, 2, 3, 4, 0x80];
        assert_eq!(h.append("a.lmp", 13, &tail), Ok(true));
        let mut want = vec![7u8; 13];
        want.extend_from_slice(&tail);
        assert_eq!(std::fs::read(dir.join("a.lmp")).unwrap(), want);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// Mutation-survivor tests of M5d (mut6).
#[cfg(test)]
mod mut6_tests {
    use super::*;
    use doom_game::app::Timestamp;
    use std::sync::{Arc, Mutex};

    /// A writer the log's thread can own, which the test reads back.
    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);

    impl Write for Sink {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// The thread's drop "writes what is left (as a dropped buffered writer would) and joins the
    /// thread": a line not yet handed over (no pump ended) is in the file once the log is dropped.
    #[test]
    fn dropping_a_threaded_log_writes_what_is_left() {
        let sink = Sink::default();
        let mut log = InputLog::threaded(Box::new(sink.clone())).unwrap();
        log.line("raw", 0, format_args!("last words"));
        drop(log);
        let text = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
        assert_eq!(text, "raw 1 0 last words\n");
    }

    /// A host that keeps the defaults of [`Host`]: its files are not kept current tic by tic.
    struct Plain;

    impl Host for Plain {
        fn timestamp(&mut self) -> Timestamp {
            Timestamp {
                year: 2026,
                month: 11,
                day: 20,
                hour: 0,
                minute: 0,
                second: 0,
            }
        }
        fn exists(&mut self, _: &str) -> bool {
            false
        }
        fn save(&mut self, _: &str, _: &[u8]) -> Result<(), String> {
            Ok(())
        }
    }

    /// [`Host::appends`] says whether the host keeps the file current tic by tic; the logging
    /// wrapper must say what the host proper says (here: it does not), or the app would offer
    /// tics to a host that declines them.
    #[test]
    fn the_wrapper_says_what_the_host_says_about_appends() {
        let h = LoggedHost {
            host: Plain,
            log: InputLog::off(),
        };
        assert!(!h.appends());
    }
}
