//! The render state (owner ruling on R5, R6 and R7, 2026-11-15): everything the renderer
//! carries from one tic or frame to the next, self-contained, so that a saved render state
//! restored next to a simulation snapshot of the same tic renders the following tics exactly
//! as an uninterrupted run. Its byte encoding (`DRST` version 1) is documented in
//! `docs/STATE_FORMAT.md` ("Render state"), separate from the frozen simulation schemas.
use crate::p_spec::{ButtonSlot, SwitchInputs, SwitchTracker};
use crate::view::{ShadowError, ViewShadow};
use crate::{Level, ObserveError, SCREENHEIGHT, SCREENWIDTH};
use doom_sim::{ActorStateEvent, Event, MoverEvent, Simulation};
use std::fmt;

/// Encoding magic.
pub const MAGIC: &[u8; 4] = b"DRST";
/// Encoding version.
pub const VERSION: u32 = 1;
const PIXELS: usize = SCREENWIDTH * SCREENHEIGHT;
/// Bound on the decoded sector and line counts (doom-map's own bounds are lower).
const MAX_ITEMS: u32 = 1 << 20;

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
            screen: vec![0; PIXELS],
            taint: vec![0; PIXELS],
        })
    }

    /// The state after observing the simulation's next tic.
    pub fn next(&self, sim: &Simulation, level: &Level) -> Result<Self, ObserveError> {
        if sim.tic() != self.tic + 1 {
            return Err(ObserveError::Shadow(ShadowError::Gap {
                expected: self.tic + 1,
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

    /// The `DRST` version 1 encoding (docs/STATE_FORMAT.md, "Render state").
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

    /// Decodes a `DRST` version 1 encoding, validating every count and value.
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
        v[4] = 2;
        assert_eq!(RenderState::decode(&v), Err(StateDecodeError::Version(2)));
    }
}
