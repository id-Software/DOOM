//! The render state (owner ruling on R5, R6 and R7, 2026-11-15): everything the renderer
//! carries from one tic or frame to the next, self-contained, so that a saved render state
//! restored next to a simulation snapshot of the same tic renders the following tics exactly
//! as an uninterrupted run. Its byte encoding (`DRST` version 2) is documented in
//! `docs/STATE_FORMAT.md` ("Render state"), separate from the frozen simulation schemas.
use crate::p_spec::{ButtonSlot, SwitchInputs, SwitchTracker};
use crate::view::{ShadowError, ViewShadow};
use crate::{Level, ObserveError, SCREENHEIGHT, SCREENWIDTH};
use doom_sim::{ActorStateEvent, Event, MoverEvent, Simulation};
use std::fmt;

/// Encoding magic.
pub const MAGIC: &[u8; 4] = b"DRST";
/// Encoding version (2: version 1, which never left the unmerged M5a branch, lacked
/// [`RenderState::wall_steps`]).
pub const VERSION: u32 = 2;
const PIXELS: usize = SCREENWIDTH * SCREENHEIGHT;
/// Bound on the decoded sector and line counts (doom-map's own bounds are lower).
const MAX_ITEMS: u32 = 1 << 20;

/// The r_segs.c globals `pixhigh`, `pixhighstep`, `pixlow` and `pixlowstep` (r_segs.c:81-84):
/// file-scope, never reset, so a two-sided wall whose top or bottom texture was chosen on the
/// unshifted heights (r_segs.c:570, 589) but whose shifted test fails (r_segs.c:693-704)
/// draws with the values an earlier wall left, in this frame or an earlier one. Zero in a
/// freshly started process (decision R8).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WallSteps {
    /// `pixhigh`.
    pub pixhigh: i32,
    /// `pixhighstep`.
    pub pixhighstep: i32,
    /// `pixlow`.
    pub pixlow: i32,
    /// `pixlowstep`.
    pub pixlowstep: i32,
}

/// The renderer's own history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderState {
    /// The last observed simulation tic.
    pub tic: u64,
    /// The view shadow (decision R6).
    pub view: ViewShadow,
    /// The switch tracker (decision R7).
    pub switches: SwitchTracker,
    /// The lines the renderer has marked mapped (`ML_MAPPED`, r_segs.c:398; decision R5).
    pub mapped: Vec<bool>,
    /// `fuzzpos` (r_draw.c:274; decision R8), 0..50.
    pub fuzzpos: u32,
    /// The r_segs.c wall-step globals after the last frame.
    pub wall_steps: WallSteps,
    /// The persistent framebuffer (`screens[0]`, never cleared; decision R8).
    pub screen: Vec<u8>,
    /// Per framebuffer pixel, the origin of an undefined source read that produced it (0:
    /// defined). Persists with the framebuffer, as the genuine tracker's taint does (R4).
    pub taint: Vec<u32>,
}

/// A malformed render-state encoding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateDecodeError {
    /// Wrong magic.
    Magic,
    /// Unsupported version.
    Version(u32),
    /// The bytes end early.
    Truncated,
    /// A count, flag or value is out of range.
    Invalid(&'static str),
    /// Bytes follow the encoding.
    Trailing,
}
impl fmt::Display for StateDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "render state: {self:?}")
    }
}
impl std::error::Error for StateDecodeError {}

/// A render state that does not fit the renderer's level, or holds a value the simulation
/// and renderer never produce ([`RenderState::check_level`]). The renderer keeps its state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestoreError {
    /// A per-line or per-sector table, or the framebuffer, has the wrong length.
    Count {
        /// The table.
        field: &'static str,
        /// The level's (or the screen's) count.
        expected: usize,
        /// The state's count.
        got: usize,
    },
    /// A line index is outside the level's lines (or not a tracked switch line).
    Line {
        /// The table.
        field: &'static str,
        /// The index.
        line: u16,
    },
    /// A value outside the range the simulation and the renderer produce.
    Value(&'static str),
}
impl fmt::Display for RestoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "render state does not fit the level: {self:?}")
    }
}
impl std::error::Error for RestoreError {}

/// Every value the decoder and [`RenderState::check_level`] bound outside the level: the
/// `player->powers` are counters the simulation keeps nonnegative (p_user.c:339-353 counts
/// them down to 0, `pw_strength` up from 1), so `P_PlayerThink`'s countdown never underflows.
fn value_error(s: &RenderState) -> Option<&'static str> {
    if !(0..=crate::view::INVERSECOLORMAP).contains(&s.view.fixedcolormap) {
        return Some("fixedcolormap");
    }
    if s.view.powers.iter().any(|p| *p < 0) {
        return Some("powers");
    }
    if s.switches
        .buttons
        .iter()
        .any(|b| !(0..=35).contains(&b.timer))
    {
        return Some("btimer");
    }
    for list in [&s.switches.cleared, &s.switches.unknown] {
        if list.windows(2).any(|w| w[0] >= w[1]) {
            return Some("line list order");
        }
    }
    if s.fuzzpos as usize >= crate::r_draw::FUZZTABLE {
        return Some("fuzzpos");
    }
    None
}

/// The canonical switch facts after a tic: the button slots and the cleared switch lines from
/// the snapshot (`button.{k}.*`, `line.{i}.special`), and the tic's `sfx_swtchn` requests.
fn switch_inputs(sim: &Simulation, level: &Level) -> Result<SwitchInputs, ObserveError> {
    let snap = sim.snapshot();
    let mut inputs = SwitchInputs::default();
    let bad = |p: &str| ObserveError::Snapshot(p.to_string());
    for f in snap.fields() {
        let path = f.path();
        if let Some(rest) = path.strip_prefix("button.") {
            let (k, field) = rest.split_once('.').ok_or_else(|| bad(path))?;
            let k: usize = k.parse().map_err(|_| bad(path))?;
            let slot = inputs.buttons.get_mut(k).ok_or_else(|| bad(path))?;
            match field {
                "btimer" => {
                    slot.timer = i32::from_le_bytes(f.value().try_into().map_err(|_| bad(path))?)
                }
                "line" => {
                    slot.line = u16::from_le_bytes(f.value().try_into().map_err(|_| bad(path))?)
                }
                _ => {}
            }
        } else if let Some(rest) = path.strip_prefix("line.") {
            let Some(i) = rest.strip_suffix(".special") else {
                continue;
            };
            let i: u16 = i.parse().map_err(|_| bad(path))?;
            let special = i16::from_le_bytes(f.value().try_into().map_err(|_| bad(path))?);
            if special == 0 && level.switch_lines.binary_search(&i).is_ok() {
                inputs.cleared.push(i);
            }
        }
    }
    inputs.cleared.sort_unstable();
    inputs.sounds = sim
        .events()
        .iter()
        .filter(|e| match e {
            Event::Mover(MoverEvent::SectorSound { sound, .. }) => {
                *sound == crate::p_spec::SFX_SWTCHN
            }
            Event::ActorState(ActorStateEvent::Sound { sound, .. }) => {
                *sound == crate::p_spec::SFX_SWTCHN
            }
            _ => false,
        })
        .count() as u32;
    Ok(inputs)
}

impl RenderState {
    /// The state at level load (tic 0, decision R8).
    pub fn level_start(sim: &Simulation, level: &Level) -> Result<Self, ObserveError> {
        let view = ViewShadow::level_start(sim).map_err(ObserveError::Shadow)?;
        let inputs = switch_inputs(sim, level)?;
        Ok(Self {
            tic: 0,
            view,
            switches: SwitchTracker {
                buttons: inputs.buttons,
                cleared: inputs.cleared,
                unknown: Vec::new(),
            },
            mapped: vec![false; level.map.linedefs().len()],
            fuzzpos: 0,
            wall_steps: WallSteps::default(),
            screen: vec![0; PIXELS],
            taint: vec![0; PIXELS],
        })
    }

    /// The state after observing the simulation's next tic.
    pub fn next(&self, sim: &Simulation, level: &Level) -> Result<Self, ObserveError> {
        let expected = self.tic.checked_add(1);
        if Some(sim.tic()) != expected {
            return Err(ObserveError::Shadow(ShadowError::Gap {
                expected: expected.unwrap_or(u64::MAX),
                got: sim.tic(),
            }));
        }
        let view = self.view.next(sim).map_err(ObserveError::Shadow)?;
        let switches = self
            .switches
            .next(&switch_inputs(sim, level)?)
            .map_err(|_| ObserveError::SwitchInconsistency)?;
        Ok(Self {
            tic: sim.tic(),
            view,
            switches,
            ..self.clone()
        })
    }

    /// Checks the state against `level` before a restore: the mapped-line set and the sector
    /// floors have the level's counts, every line a running button, a cleared switch or an
    /// unknown parity names is one of the level's lines (cleared ones among its tracked switch
    /// lines), the framebuffer and taint cover the screen, and every bounded value is in
    /// range (the bounds the decoder applies, and nonnegative powers).
    pub fn check_level(&self, level: &Level) -> Result<(), RestoreError> {
        let count = |field, expected: usize, got: usize| {
            if expected == got {
                Ok(())
            } else {
                Err(RestoreError::Count {
                    field,
                    expected,
                    got,
                })
            }
        };
        let lines = level.map.linedefs().len();
        count("mapped", lines, self.mapped.len())?;
        count(
            "sector_floors",
            level.map.sectors().len(),
            self.view.sector_floors.len(),
        )?;
        count("screen", PIXELS, self.screen.len())?;
        count("taint", PIXELS, self.taint.len())?;
        if let Some(v) = value_error(self) {
            return Err(RestoreError::Value(v));
        }
        let running = self.switches.buttons.iter().filter(|b| b.timer != 0);
        for (field, line) in running
            .map(|b| ("button", b.line))
            .chain(self.switches.unknown.iter().map(|l| ("unknown", *l)))
        {
            if usize::from(line) >= lines {
                return Err(RestoreError::Line { field, line });
            }
        }
        for line in &self.switches.cleared {
            if level.switch_lines.binary_search(line).is_err() {
                return Err(RestoreError::Line {
                    field: "cleared",
                    line: *line,
                });
            }
        }
        Ok(())
    }

    /// The `DRST` version 2 encoding (docs/STATE_FORMAT.md, "Render state").
    pub fn encode(&self) -> Vec<u8> {
        let mut o = Vec::with_capacity(PIXELS + 4096);
        o.extend_from_slice(MAGIC);
        o.extend(VERSION.to_le_bytes());
        o.extend(self.tic.to_le_bytes());
        let v = &self.view;
        for x in [v.viewheight, v.deltaviewheight, v.viewz] {
            o.extend(x.to_le_bytes());
        }
        o.push(u8::from(v.onground));
        for x in [v.fixedcolormap, v.z, v.floorz, v.ceilingz, v.reactiontime] {
            o.extend(x.to_le_bytes());
        }
        o.push(u8::from(v.dead));
        for x in v.powers {
            o.extend(x.to_le_bytes());
        }
        o.extend((v.sector_floors.len() as u32).to_le_bytes());
        for x in &v.sector_floors {
            o.extend(x.to_le_bytes());
        }
        for b in &self.switches.buttons {
            o.extend(b.timer.to_le_bytes());
            o.extend(b.line.to_le_bytes());
        }
        for list in [&self.switches.cleared, &self.switches.unknown] {
            o.extend((list.len() as u32).to_le_bytes());
            for l in list {
                o.extend(l.to_le_bytes());
            }
        }
        o.extend((self.mapped.len() as u32).to_le_bytes());
        for chunk in self.mapped.chunks(8) {
            o.push(
                chunk
                    .iter()
                    .enumerate()
                    .fold(0u8, |acc, (i, m)| acc | (u8::from(*m) << i)),
            );
        }
        o.extend(self.fuzzpos.to_le_bytes());
        let w = &self.wall_steps;
        for x in [w.pixhigh, w.pixhighstep, w.pixlow, w.pixlowstep] {
            o.extend(x.to_le_bytes());
        }
        o.extend_from_slice(&self.screen);
        let mut runs: Vec<(u32, u32)> = Vec::new();
        for t in &self.taint {
            match runs.last_mut() {
                Some((n, v)) if v == t => *n += 1,
                _ => runs.push((1, *t)),
            }
        }
        o.extend((runs.len() as u32).to_le_bytes());
        for (n, v) in runs {
            o.extend(n.to_le_bytes());
            o.extend(v.to_le_bytes());
        }
        o
    }

    /// Decodes a `DRST` version 2 encoding, validating every count and value that does not
    /// depend on the level ([`Self::check_level`] checks the rest before a restore).
    pub fn decode(bytes: &[u8]) -> Result<Self, StateDecodeError> {
        let mut r = Reader { b: bytes, at: 0 };
        if r.take(4)? != MAGIC {
            return Err(StateDecodeError::Magic);
        }
        let version = r.u32()?;
        if version != VERSION {
            return Err(StateDecodeError::Version(version));
        }
        let tic = r.u64()?;
        let viewheight = r.i32()?;
        let deltaviewheight = r.i32()?;
        let viewz = r.i32()?;
        let onground = r.flag()?;
        let fixedcolormap = r.i32()?;
        if !(0..=crate::view::INVERSECOLORMAP).contains(&fixedcolormap) {
            return Err(StateDecodeError::Invalid("fixedcolormap"));
        }
        let z = r.i32()?;
        let floorz = r.i32()?;
        let ceilingz = r.i32()?;
        let reactiontime = r.i32()?;
        let dead = r.flag()?;
        let mut powers = [0; 6];
        for p in &mut powers {
            *p = r.i32()?;
            if *p < 0 {
                return Err(StateDecodeError::Invalid("powers"));
            }
        }
        let n = r.count()?;
        let sector_floors = (0..n).map(|_| r.i32()).collect::<Result<_, _>>()?;
        let mut buttons = [ButtonSlot::default(); 16];
        for b in &mut buttons {
            b.timer = r.i32()?;
            b.line = r.u16()?;
            if !(0..=35).contains(&b.timer) {
                return Err(StateDecodeError::Invalid("btimer"));
            }
        }
        let mut lists = [Vec::new(), Vec::new()];
        for list in &mut lists {
            let n = r.count()?;
            for _ in 0..n {
                list.push(r.u16()?);
            }
            if list.windows(2).any(|w| w[0] >= w[1]) {
                return Err(StateDecodeError::Invalid("line list order"));
            }
        }
        let [cleared, unknown] = lists;
        let lines = r.count()? as usize;
        let bits = r.take(lines.div_ceil(8))?;
        let mapped: Vec<bool> = (0..lines)
            .map(|i| bits[i / 8] >> (i % 8) & 1 == 1)
            .collect();
        if lines % 8 != 0 && bits[lines / 8] >> (lines % 8) != 0 {
            return Err(StateDecodeError::Invalid("mapped padding"));
        }
        let fuzzpos = r.u32()?;
        if fuzzpos as usize >= crate::r_draw::FUZZTABLE {
            return Err(StateDecodeError::Invalid("fuzzpos"));
        }
        let wall_steps = WallSteps {
            pixhigh: r.i32()?,
            pixhighstep: r.i32()?,
            pixlow: r.i32()?,
            pixlowstep: r.i32()?,
        };
        let screen = r.take(PIXELS)?.to_vec();
        let runs = r.count()?;
        let mut taint = Vec::with_capacity(PIXELS);
        for _ in 0..runs {
            let n = r.u32()? as usize;
            let v = r.u32()?;
            if n == 0 || taint.len() + n > PIXELS {
                return Err(StateDecodeError::Invalid("taint runs"));
            }
            taint.resize(taint.len() + n, v);
        }
        if taint.len() != PIXELS {
            return Err(StateDecodeError::Invalid("taint runs"));
        }
        if r.at != bytes.len() {
            return Err(StateDecodeError::Trailing);
        }
        Ok(Self {
            tic,
            view: ViewShadow {
                viewheight,
                deltaviewheight,
                viewz,
                onground,
                fixedcolormap,
                z,
                floorz,
                ceilingz,
                reactiontime,
                dead,
                powers,
                sector_floors,
            },
            switches: SwitchTracker {
                buttons,
                cleared,
                unknown,
            },
            mapped,
            fuzzpos,
            wall_steps,
            screen,
            taint,
        })
    }
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], StateDecodeError> {
        let end = self.at.checked_add(n).ok_or(StateDecodeError::Truncated)?;
        let s = self
            .b
            .get(self.at..end)
            .ok_or(StateDecodeError::Truncated)?;
        self.at = end;
        Ok(s)
    }
    fn u16(&mut self) -> Result<u16, StateDecodeError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("2 bytes"),
        ))
    }
    fn u32(&mut self) -> Result<u32, StateDecodeError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("4 bytes"),
        ))
    }
    fn i32(&mut self) -> Result<i32, StateDecodeError> {
        Ok(i32::from_le_bytes(
            self.take(4)?.try_into().expect("4 bytes"),
        ))
    }
    fn u64(&mut self) -> Result<u64, StateDecodeError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("8 bytes"),
        ))
    }
    fn flag(&mut self) -> Result<bool, StateDecodeError> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(StateDecodeError::Invalid("flag")),
        }
    }
    fn count(&mut self) -> Result<u32, StateDecodeError> {
        let n = self.u32()?;
        if n > MAX_ITEMS {
            return Err(StateDecodeError::Invalid("count"));
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> RenderState {
        let mut taint = vec![0; PIXELS];
        taint[5] = 7;
        taint[6] = 7;
        let mut screen = vec![0; PIXELS];
        screen[100] = 42;
        RenderState {
            tic: 9,
            view: ViewShadow {
                viewheight: 41 << 16,
                deltaviewheight: -3,
                viewz: 12345,
                onground: true,
                fixedcolormap: 1,
                z: 1,
                floorz: 2,
                ceilingz: 3,
                reactiontime: 4,
                dead: false,
                powers: [0, 1, 2, 3, 4, 5],
                sector_floors: vec![0, -65536],
            },
            switches: SwitchTracker {
                buttons: [ButtonSlot { timer: 3, line: 2 }; 16],
                cleared: vec![1, 5],
                unknown: vec![2],
            },
            mapped: vec![true, false, true, true, false, false, false, false, true],
            fuzzpos: 49,
            wall_steps: WallSteps {
                pixhigh: 1,
                pixhighstep: -2,
                pixlow: 3,
                pixlowstep: i32::MIN,
            },
            screen,
            taint,
        }
    }

    #[test]
    fn round_trip_and_rejections() {
        let s = sample();
        let e = s.encode();
        assert_eq!(RenderState::decode(&e), Ok(s));
        assert_eq!(
            RenderState::decode(&e[..e.len() - 1]),
            Err(StateDecodeError::Truncated)
        );
        let mut t = e.clone();
        t.push(0);
        assert_eq!(RenderState::decode(&t), Err(StateDecodeError::Trailing));
        let mut m = e.clone();
        m[0] = b'X';
        assert_eq!(RenderState::decode(&m), Err(StateDecodeError::Magic));
        let mut v = e;
        v[4] = 1;
        assert_eq!(RenderState::decode(&v), Err(StateDecodeError::Version(1)));
    }

    // ---- DRST decoder rejections (docs/STATE_FORMAT.md, "Render state"). Each test pins one
    // validated field of `RenderState::decode`; the public decoder reaches all of them.

    /// The offset of the first byte at which two encodings differ.
    fn first_diff(a: &[u8], b: &[u8]) -> usize {
        a.iter()
            .zip(b)
            .position(|(x, y)| x != y)
            .expect("encodings differ")
    }

    #[test]
    fn every_proper_prefix_is_truncated_or_invalid_never_a_panic() {
        let e = sample().encode();
        // Every prefix up to the end of the fixed header region, then a stride through the
        // framebuffer and taint runs, then the last 20 bytes.
        let cuts = (0..400)
            .chain((400..e.len()).step_by(997))
            .chain(e.len() - 20..e.len());
        for n in cuts {
            assert!(RenderState::decode(&e[..n]).is_err(), "prefix {n}");
        }
        assert!(RenderState::decode(&[]).is_err());
    }

    #[test]
    fn flags_must_be_zero_or_one() {
        let mut s = sample();
        let on = s.encode();
        s.view.onground = false;
        let at = first_diff(&on, &s.encode());
        let mut bad = on.clone();
        bad[at] = 2;
        assert_eq!(
            RenderState::decode(&bad),
            Err(StateDecodeError::Invalid("flag"))
        );
        let mut s = sample();
        s.view.dead = true;
        let at = first_diff(&on, &s.encode());
        let mut bad = on;
        bad[at] = 0xff;
        assert_eq!(
            RenderState::decode(&bad),
            Err(StateDecodeError::Invalid("flag"))
        );
    }

    #[test]
    fn fixedcolormap_is_within_the_colour_maps_and_the_inverse_map() {
        // STATE_FORMAT, "Render state": `player->fixedcolormap`, 0..=32 (INVERSECOLORMAP,
        // p_user.c:41, is 32).
        for bad in [-1, crate::view::INVERSECOLORMAP + 1, i32::MAX, i32::MIN] {
            let mut s = sample();
            s.view.fixedcolormap = bad;
            assert_eq!(
                RenderState::decode(&s.encode()),
                Err(StateDecodeError::Invalid("fixedcolormap")),
                "{bad}"
            );
        }
        for good in [0, crate::view::INVERSECOLORMAP] {
            let mut s = sample();
            s.view.fixedcolormap = good;
            assert_eq!(RenderState::decode(&s.encode()), Ok(s));
        }
    }

    #[test]
    fn button_timers_are_zero_to_buttontime() {
        // STATE_FORMAT, "Render state": `btimer` 0..=35.
        for bad in [-1, 36, i32::MAX] {
            let mut s = sample();
            s.switches.buttons[3].timer = bad;
            assert_eq!(
                RenderState::decode(&s.encode()),
                Err(StateDecodeError::Invalid("btimer")),
                "{bad}"
            );
        }
        let mut s = sample();
        s.switches.buttons[15].timer = 35;
        assert_eq!(RenderState::decode(&s.encode()), Ok(s));
    }

    #[test]
    fn line_lists_are_strictly_ascending() {
        for (cleared, unknown) in [
            (vec![5, 1], vec![]),
            (vec![3, 3], vec![]),
            (vec![], vec![9, 2]),
            (vec![], vec![4, 4]),
        ] {
            let mut s = sample();
            s.switches.cleared = cleared;
            s.switches.unknown = unknown;
            assert_eq!(
                RenderState::decode(&s.encode()),
                Err(StateDecodeError::Invalid("line list order"))
            );
        }
        let mut s = sample();
        s.switches.cleared = vec![];
        s.switches.unknown = vec![0, 65535];
        assert_eq!(RenderState::decode(&s.encode()), Ok(s));
    }

    #[test]
    fn fuzzpos_is_below_fuzztable() {
        for bad in [50, 51, u32::MAX] {
            let mut s = sample();
            s.fuzzpos = bad;
            assert_eq!(
                RenderState::decode(&s.encode()),
                Err(StateDecodeError::Invalid("fuzzpos")),
                "{bad}"
            );
        }
    }

    #[test]
    fn counts_are_bounded_before_any_allocation() {
        // The sector-floor count follows the 94-byte-or-so fixed prefix; find it as the first
        // byte that differs between two states with 2 and 3 sector floors, then claim 2^20 + 1
        // entries (the decoder must refuse the count itself, not run out of bytes first).
        let mut a = sample();
        let mut b = sample();
        a.view.sector_floors = vec![0; 2];
        b.view.sector_floors = vec![0; 3];
        let at = first_diff(&a.encode(), &b.encode());
        let mut bad = a.encode();
        bad[at..at + 4].copy_from_slice(&((1u32 << 20) + 1).to_le_bytes());
        assert_eq!(
            RenderState::decode(&bad),
            Err(StateDecodeError::Invalid("count"))
        );
        // The largest admitted count is then merely truncated.
        bad[at..at + 4].copy_from_slice(&(1u32 << 20).to_le_bytes());
        assert_eq!(RenderState::decode(&bad), Err(StateDecodeError::Truncated));
        bad[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            RenderState::decode(&bad),
            Err(StateDecodeError::Invalid("count"))
        );
    }

    #[test]
    fn mapped_set_padding_bits_must_be_zero() {
        // 9 mapped lines occupy two bytes; bits 1-7 of the second are padding.
        let mut a = sample();
        let mut b = sample();
        a.mapped[8] = false;
        b.mapped[8] = true;
        let ea = a.encode();
        let at = first_diff(&ea, &b.encode());
        for pad in [0x02, 0x80, 0xfe] {
            let mut bad = ea.clone();
            bad[at] |= pad;
            assert_eq!(
                RenderState::decode(&bad),
                Err(StateDecodeError::Invalid("mapped padding")),
                "{pad:#x}"
            );
        }
    }

    #[test]
    fn taint_runs_must_tile_the_framebuffer_exactly() {
        // The encoding ends `run count, (length, value)*`. The sample's taint has runs
        // (5 x 0), (2 x 7), (63993 x 0): the last 28 bytes.
        let e = sample().encode();
        let tail = e.len() - 28;
        assert_eq!(u32::from_le_bytes(e[tail..tail + 4].try_into().unwrap()), 3);
        let run = |i: usize| tail + 4 + 8 * i;
        let with = |i: usize, n: u32| {
            let mut bad = e.clone();
            bad[run(i)..run(i) + 4].copy_from_slice(&n.to_le_bytes());
            bad
        };
        let invalid = Err(StateDecodeError::Invalid("taint runs"));
        // A zero-length run, a run past the end, and runs that stop short.
        assert_eq!(RenderState::decode(&with(1, 0)), invalid);
        assert_eq!(RenderState::decode(&with(2, 63_994)), invalid);
        assert_eq!(RenderState::decode(&with(2, 63_992)), invalid);
        assert_eq!(RenderState::decode(&with(0, u32::MAX)), invalid);
        // A run count past the bound.
        let mut bad = e.clone();
        bad[tail..tail + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            RenderState::decode(&bad),
            Err(StateDecodeError::Invalid("count"))
        );
        // A run count of zero leaves the framebuffer untiled.
        let mut bad = e[..tail + 4].to_vec();
        bad[tail..].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(RenderState::decode(&bad), invalid);
    }

    #[test]
    fn a_zero_length_taint_run_is_refused_even_when_the_runs_still_tile() {
        // Pins the `n == 0` half of the run check: a (0, value) run inserted before valid runs
        // leaves the framebuffer exactly tiled, so only the zero length itself can refuse it.
        let e = sample().encode();
        let tail = e.len() - 28;
        let mut bad = e[..tail].to_vec();
        bad.extend(4u32.to_le_bytes());
        bad.extend(0u32.to_le_bytes());
        bad.extend(9u32.to_le_bytes());
        bad.extend_from_slice(&e[tail + 4..]);
        assert_eq!(
            RenderState::decode(&bad),
            Err(StateDecodeError::Invalid("taint runs"))
        );
    }

    #[test]
    fn value_error_bounds_fuzzpos_below_fuzztable() {
        // `fuzzpos` indexes `fuzzoffset[FUZZTABLE]` (r_draw.c:259-263, FUZZTABLE 50): 49 is
        // the last admitted value. Pins the check `check_level` shares with the decoder.
        let mut s = sample();
        assert_eq!(value_error(&s), None);
        for bad in [50, 51, u32::MAX] {
            s.fuzzpos = bad;
            assert_eq!(value_error(&s), Some("fuzzpos"), "{bad}");
        }
        s.fuzzpos = 0;
        assert_eq!(value_error(&s), None);
    }

    #[test]
    fn decode_errors_display_their_variant() {
        assert_eq!(
            StateDecodeError::Trailing.to_string(),
            "render state: Trailing"
        );
        assert_eq!(
            RestoreError::Value("powers").to_string(),
            "render state does not fit the level: Value(\"powers\")"
        );
    }
}
