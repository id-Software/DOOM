use crate::Actor;
use doom_map::Map;
use doom_types::Fixed;
use std::{error::Error, fmt};

/// Geometry/profile validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryError {
    /// Bounded geometry storage could not be reserved.
    Allocation,
    /// Explicit prepared-reference work budget exceeded.
    WorkLimit,
    /// The selected simulation profile does not implement this feature.
    Unsupported(&'static str),
    /// A reference line delta or sector span is not representable as signed i32.
    CoordinateOverflow,
    /// Caller supplied out-of-profile dimensions or grounded state.
    InvalidActor,
    /// An invariant index does not address validated geometry.
    InvalidReference,
}
impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "simulation geometry: {self:?}")
    }
}
impl Error for GeometryError {}

/// Predecoded fixed-point line with original slope and bbox semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineGeometry {
    pub(crate) start: [Fixed; 2],
    pub(crate) delta: [Fixed; 2],
    pub(crate) bounds: [Fixed; 4],
    pub(crate) slope: u8,
}
pub(crate) fn bbox(position: [Fixed; 2], radius: Fixed) -> [Fixed; 4] {
    [
        position[1] + radius,
        position[1] - radius,
        position[0] - radius,
        position[0] + radius,
    ]
}
impl LineGeometry {
    /// Constructs a reference-compatible line, rejecting overflowing endpoint deltas.
    pub fn new(start: [Fixed; 2], end: [Fixed; 2]) -> Result<Self, GeometryError> {
        let dx = end[0]
            .bits()
            .checked_sub(start[0].bits())
            .ok_or(GeometryError::CoordinateOverflow)?;
        let dy = end[1]
            .bits()
            .checked_sub(start[1].bits())
            .ok_or(GeometryError::CoordinateOverflow)?;
        let delta = [Fixed::from_bits(dx), Fixed::from_bits(dy)];
        let slope = if dx == 0 {
            1
        } else if dy == 0 {
            0
        } else if (delta[1] / delta[0]).bits() > 0 {
            2
        } else {
            3
        };
        Ok(Self {
            start,
            delta,
            bounds: [
                start[1].max(end[1]),
                start[1].min(end[1]),
                start[0].min(end[0]),
                start[0].max(end[0]),
            ],
            slope,
        })
    }
    /// P_PointOnLineSide with exact axis ties and shifted FixedMul predicates.
    /// Subtraction wraps deliberately where the C operation would overflow.
    pub fn point_side(&self, point: [Fixed; 2]) -> u8 {
        let [dx, dy] = self.delta;
        if dx == Fixed::ZERO {
            return u8::from(if point[0] <= self.start[0] {
                dy > Fixed::ZERO
            } else {
                dy < Fixed::ZERO
            });
        }
        if dy == Fixed::ZERO {
            return u8::from(if point[1] <= self.start[1] {
                dx < Fixed::ZERO
            } else {
                dx > Fixed::ZERO
            });
        }
        let left = Fixed::from_bits(dy.bits() >> 16) * (point[0] - self.start[0]);
        let right = (point[1] - self.start[1]) * Fixed::from_bits(dx.bits() >> 16);
        u8::from(right >= left)
    }
    /// P_BoxOnLineSide: 0/1 for one side and -1 for crossing an infinite line.
    pub fn box_side(&self, position: [Fixed; 2], radius: Fixed) -> i8 {
        self.box_side_bounds(bbox(position, radius))
    }
    fn box_side_bounds(&self, b: [Fixed; 4]) -> i8 {
        let (a, c) = match self.slope {
            0 => (
                u8::from(b[0] > self.start[1]) ^ u8::from(self.delta[0] < Fixed::ZERO),
                u8::from(b[1] > self.start[1]) ^ u8::from(self.delta[0] < Fixed::ZERO),
            ),
            1 => (
                u8::from(b[3] < self.start[0]) ^ u8::from(self.delta[1] < Fixed::ZERO),
                u8::from(b[2] < self.start[0]) ^ u8::from(self.delta[1] < Fixed::ZERO),
            ),
            2 => (self.point_side([b[2], b[0]]), self.point_side([b[3], b[1]])),
            _ => (self.point_side([b[3], b[0]]), self.point_side([b[2], b[1]])),
        };
        if a == c { a as i8 } else { -1 }
    }
    pub(crate) fn crosses(&self, b: [Fixed; 4]) -> bool {
        !(b[3] <= self.bounds[2]
            || b[2] >= self.bounds[3]
            || b[0] <= self.bounds[1]
            || b[1] >= self.bounds[0])
            && self.box_side_bounds(b) == -1
    }
}

/// A single destination check, in exact first-encounter BLOCKMAP order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Collision {
    /// First crossed blocking linedef, if any.
    pub blocked_by: Option<u16>,
    /// Whether the actor fits the static floor/ceiling interval.
    pub fits: bool,
    /// Deduplicated visited linedefs, stopping at the first blocker.
    pub candidates: Vec<u16>,
}
impl Collision {
    /// Reference one-sector try-move result.
    pub const fn allowed(&self) -> bool {
        self.fits && self.blocked_by.is_none()
    }
}

/// One P_XYMovement substep observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MoveAttempt {
    /// Attempted XY destination.
    pub target: [Fixed; 2],
    /// Checked line/fit result.
    pub collision: Collision,
}

/// Compiled immutable static geometry. Standalone construction uses the M2
/// subset; Simulation builds additional validated M3a/M3b profiles.
#[derive(Clone, Debug)]
pub struct StaticWorld {
    pub(crate) map: Map,
    pub(crate) lines: Vec<LineGeometry>,
    pub(crate) collision_cells: Option<Vec<Vec<u16>>>,
    pub(crate) topology: Option<crate::multisector::Topology>,
    pub(crate) extent_sight_work: Option<usize>,
    /// `P_GroupLines` per-sector line lists (linedef order), prepared by the
    /// real-map policy for the noise flood; `None` scans every linedef.
    pub(crate) sector_lines: Option<Vec<Vec<u16>>>,
    /// Per-operation sight work budget of the checked policies: the historical
    /// 1,048,576, or `64 · (expanded_work + 5 · lines)` derived from the map by the
    /// real-map policy.
    pub(crate) sight_work_budget: usize,
    /// Under the specials policy, per sector, whether its ceiling flat is
    /// `F_SKY1` (`ceilingpic == skyflatnum`): the name up to its first NUL,
    /// compared without case, as `R_FlatNumForName`/`W_CheckNumForName` look it
    /// up. `None` under every other policy (sky is rejected at load there).
    pub(crate) sky_ceilings: Option<Vec<bool>>,
    /// Under the specials policy and its supersets, every sector's `blockbox`
    /// (`P_GroupLines`), for the `P_ChangeSector` of a moving sector; `None`
    /// otherwise. Derived from the immutable map; never canonical state.
    pub(crate) sector_block_boxes: Option<Vec<Option<crate::multisector::SectorBlockBox>>>,
    /// Under the Episode 1 policy, the per-map bounds counted from the THINGS lump
    /// (decision H4); `None` under every other policy, whose bounds are constants.
    /// Derived from the immutable map; never canonical state.
    pub(crate) episode1: Option<crate::episode1::Episode1Bounds>,
    /// Line special 97 and the teleport landing are admitted (wire 24 only,
    /// `OriginalExecutionProfile::has_teleport`; M4b step 4). The defence-in-depth
    /// test of the crossing dispatch reads this flag, never `episode1`, which wire 23
    /// also sets. Derived from the policy at load; never canonical state.
    pub(crate) teleport: bool,
    /// Under wire 24 alone, the map number of the Episode 1 slot (source
    /// `gamemap`, 1–9), read by `A_BossDeath` (p_enemy.c:1629–1634; M4b step 5).
    /// The same value as `Simulation::level_map`, which is the canonical copy
    /// (DSIM40 `level.map`); set once by the wire-24 constructor and constant for
    /// the run. `None` under every other policy.
    pub(crate) map_number: Option<u8>,
    /// Per-operation monster sight-query allowance: the historical 64, or
    /// `8·(K + 1)` under the Episode 1 policy (decision D8).
    pub(crate) sight_query_limit: usize,
    /// Under the Episode 1 policy, whether the intercept and height sums the
    /// arithmetic certificate's clauses (i), (ii) and (vii) bound for older
    /// policies are computed checked, refusing the operation where the source
    /// would overflow (decision H3). False under every other policy, whose
    /// certificate proves the wrapping arithmetic never wraps.
    pub(crate) checked_arithmetic: bool,
    floor: Fixed,
    ceiling: Fixed,
}
impl StaticWorld {
    /// Rejects unsupported geometry before any simulation mutation.
    pub fn new(map: Map) -> Result<Self, GeometryError> {
        Self::build(map, false, false)
    }
    pub(crate) fn new_multisector(map: Map, specials: bool) -> Result<Self, GeometryError> {
        Self::build(map, true, specials)
    }
    fn build(map: Map, multi: bool, specials: bool) -> Result<Self, GeometryError> {
        if !multi && map.sectors().len() != 1 {
            return Err(GeometryError::Unsupported("multiple sectors/portals"));
        }
        if !specials
            && (map.sectors().iter().any(|s| s.special != 0)
                || map.linedefs().iter().any(|l| l.special != 0))
        {
            return Err(GeometryError::Unsupported("map specials"));
        }
        if !multi
            && map
                .linedefs()
                .iter()
                .any(|l| l.sides[1].is_some() || l.flags & 4 != 0)
        {
            return Err(GeometryError::Unsupported("two-sided lines"));
        }
        let floor = Fixed::from_map_units(map.sectors()[0].heights[0]);
        let ceiling = Fixed::from_map_units(map.sectors()[0].heights[1]);
        if !multi
            && ceiling
                .bits()
                .checked_sub(floor.bits())
                .is_none_or(|span| span <= 0)
        {
            return Err(GeometryError::CoordinateOverflow);
        }
        let mut lines = crate::line_work::reserved(map.linedefs().len())?;
        for line in map.linedefs() {
            let a = map
                .vertices()
                .get(usize::from(line.vertices[0]))
                .ok_or(GeometryError::InvalidReference)?;
            let b = map
                .vertices()
                .get(usize::from(line.vertices[1]))
                .ok_or(GeometryError::InvalidReference)?;
            lines.push(LineGeometry::new(
                [Fixed::from_map_units(a.x), Fixed::from_map_units(a.y)],
                [Fixed::from_map_units(b.x), Fixed::from_map_units(b.y)],
            )?);
        }
        Ok(Self {
            map,
            lines,
            collision_cells: None,
            topology: None,
            extent_sight_work: None,
            sector_lines: None,
            sight_work_budget: 1_048_576,
            sky_ceilings: None,
            sector_block_boxes: None,
            episode1: None,
            teleport: false,
            map_number: None,
            sight_query_limit: 64,
            checked_arithmetic: false,
            floor,
            ceiling,
        })
    }
    /// Source BSP location in the explicit M3b profile, independent of rendering.
    pub fn locate(&self, position: [Fixed; 2]) -> Result<crate::Location, GeometryError> {
        if self.extent_sight_work.is_some()
            && position
                .iter()
                .any(|v| !crate::sight::within(*v, crate::map_extent::QUERY_UNITS))
        {
            return Err(GeometryError::CoordinateOverflow);
        }
        let topology = self.topology.as_ref().ok_or(GeometryError::Unsupported(
            "BSP lookup requires M3b profile",
        ))?;
        Ok(topology.locate(self, position))
    }
    pub(crate) fn locate_validated(&self, position: [Fixed; 2]) -> crate::Location {
        self.topology
            .as_ref()
            .expect("M3b topology initialized")
            .locate(self, position)
    }
    /// Bbox-cell omissions under M3b reference-list policy. This conservative
    /// diagnostic is not a proof of missing true line coverage.
    pub fn conservative_omissions(&self) -> Option<u32> {
        self.topology.as_ref().map(|t| t.conservative_omissions)
    }
    pub(crate) fn line_candidates(&self, x: usize, y: usize) -> impl Iterator<Item = u16> + '_ {
        let row = y * usize::from(self.map.blockmap().dimensions()[0]) + x;
        let words = if let Some(cells) = &self.collision_cells {
            cells[row].as_slice()
        } else {
            &self.map.blockmap().words()[self.map.blockmap().cell_ranges()[row].clone()]
        };
        words.iter().copied()
    }
    /// Immutable validated source map; no runtime state is stored in it.
    pub const fn map(&self) -> &Map {
        &self.map
    }
    /// First-sector floor height for the legacy flat profile. M3b callers use
    /// runtime sectors or the actor's current constraints.
    pub const fn floor(&self) -> Fixed {
        self.floor
    }
    /// First-sector ceiling height for the legacy flat profile.
    pub const fn ceiling(&self) -> Fixed {
        self.ceiling
    }
    /// Validates actor dimensions in this supported profile.
    pub fn validate_actor(&self, radius: Fixed, height: Fixed) -> Result<(), GeometryError> {
        if self.topology.is_some() {
            return Err(GeometryError::Unsupported(
                "M3b dimensions require spawn-sector context",
            ));
        }
        if radius < Fixed::ZERO
            || radius > Fixed::from_map_units(32)
            || height <= Fixed::ZERO
            || height > self.ceiling - self.floor
        {
            return Err(GeometryError::InvalidActor);
        }
        Ok(())
    }
    /// Exact static line collision foundation. This is destination-box collision,
    /// not a swept test and not actor/portal/step/dropoff/sliding simulation.
    pub fn check_position(
        &self,
        position: [Fixed; 2],
        radius: Fixed,
        height: Fixed,
    ) -> Result<Collision, GeometryError> {
        if self.topology.is_some() {
            return Err(GeometryError::Unsupported(
                "M3b collision requires actor/sector context",
            ));
        }
        if radius < Fixed::ZERO || radius > Fixed::from_map_units(32) || height <= Fixed::ZERO {
            return Err(GeometryError::InvalidActor);
        }
        Ok(self.check_validated_position(position, radius, height))
    }
    pub(crate) fn check_validated_position(
        &self,
        position: [Fixed; 2],
        radius: Fixed,
        height: Fixed,
    ) -> Collision {
        let b = bbox(position, radius);
        let origin = self.map.blockmap().origin();
        let ox = Fixed::from_map_units(origin.x);
        let oy = Fixed::from_map_units(origin.y);
        let xl = ((b[2] - ox).bits() >> 23).max(0);
        let xh = ((b[3] - ox).bits() >> 23).min(i32::from(self.map.blockmap().dimensions()[0]) - 1);
        let yl = ((b[1] - oy).bits() >> 23).max(0);
        let yh = ((b[0] - oy).bits() >> 23).min(i32::from(self.map.blockmap().dimensions()[1]) - 1);
        let mut result = Collision {
            blocked_by: None,
            fits: height <= self.ceiling - self.floor,
            candidates: Vec::new(),
        };
        let mut seen = vec![false; self.lines.len()];
        for x in xl..=xh {
            for y in yl..=yh {
                let list = self.line_candidates(x as usize, y as usize);
                for index in list {
                    let mark = seen
                        .get_mut(usize::from(index))
                        .expect("validated immutable linedef index");
                    if *mark {
                        continue;
                    }
                    *mark = true;
                    result.candidates.push(index);
                    if self
                        .lines
                        .get(usize::from(index))
                        .expect("validated immutable linedef geometry")
                        .crosses(b)
                    {
                        result.blocked_by = Some(index);
                        return result;
                    }
                }
            }
        }
        result
    }
    pub(crate) fn move_xy(&self, actor: &mut Actor) -> Vec<MoveAttempt> {
        let mut attempts = Vec::new();
        if actor.spec.momentum == [Fixed::ZERO; 2] {
            return attempts;
        }
        let max = 30 * 65536;
        for value in &mut actor.spec.momentum {
            *value = Fixed::from_bits(value.bits().clamp(-max, max));
        }
        let [mut xmove, mut ymove] = actor.spec.momentum.map(Fixed::bits);
        loop {
            let delta = if xmove > max / 2 || ymove > max / 2 {
                let d = [xmove / 2, ymove / 2];
                xmove >>= 1;
                ymove >>= 1;
                d
            } else {
                let d = [xmove, ymove];
                xmove = 0;
                ymove = 0;
                d
            };
            let target = [
                actor.spec.position[0] + Fixed::from_bits(delta[0]),
                actor.spec.position[1] + Fixed::from_bits(delta[1]),
            ];
            let collision =
                self.check_validated_position(target, actor.spec.radius, actor.spec.height);
            if collision.allowed() {
                actor.spec.position = target;
            } else {
                actor.spec.momentum = [Fixed::ZERO; 2];
            }
            attempts.push(MoveAttempt { target, collision });
            if xmove == 0 && ymove == 0 {
                break;
            }
        }
        if actor
            .spec
            .momentum
            .iter()
            .all(|m| m.bits() > -0x1000 && m.bits() < 0x1000)
        {
            actor.spec.momentum = [Fixed::ZERO; 2];
        } else {
            for m in &mut actor.spec.momentum {
                *m = *m * Fixed::from_bits(0xe800);
            }
        }
        attempts
    }
}
