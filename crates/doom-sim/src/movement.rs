//! Bounded, source-derived grounded player and solid-actor XY profile.
use crate::{
    Actor, ActorId, Collision, GeometryError, LineGeometry, StaticWorld, StoreError, TicCommand,
};
use doom_types::Fixed;
include!("tan_to_angle.rs");

/// Collision participation supported independently of player input binding.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Solidity {
    /// Flags zero: does not block others, but still collides with solid actors.
    #[default]
    NonSolid,
    /// MF_SOLID only; no pickup, missile, shootable or Z-overlap mechanics.
    Solid,
}

/// One grounded movement attempt, including things-before-lines observations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorMoveAttempt {
    /// Requested destination.
    pub target: [Fixed; 2],
    /// Static fit and line candidates; empty if an actor blocked first.
    pub lines: Collision,
    /// Cell traversal and cell-head order, including non-solid candidates and self.
    pub actors: Vec<ActorId>,
    /// First strictly overlapping solid candidate, excluding self.
    pub blocked_by_actor: Option<ActorId>,
}
impl ActorMoveAttempt {
    /// Whether the entire destination check succeeded.
    pub const fn allowed(&self) -> bool {
        self.blocked_by_actor.is_none() && self.lines.allowed()
    }
}

/// Most-recent-link order. Filtering by center cell equals per-cell head lists.
#[derive(Clone, Debug, Default)]
pub(crate) struct SpatialIndex {
    pub order: Vec<ActorId>,
}
impl SpatialIndex {
    pub fn reserve_spawn(&mut self) -> Result<(), StoreError> {
        self.order.try_reserve(1).map_err(|_| StoreError::Capacity)
    }
    pub fn unlink(&mut self, id: ActorId) {
        self.order.retain(|entry| *entry != id);
    }
    pub fn link(&mut self, id: ActorId) {
        self.unlink(id);
        // Spawn reserves before state changes; relinking never grows active count.
        self.order.insert(0, id);
    }
}

pub(crate) fn validate_profile(world: &mut StaticWorld) -> Result<(), GeometryError> {
    prepare_cells(world, true, 128, 128 * 4096, 4096)
}
pub(crate) fn prepare_reference_profile(world: &mut StaticWorld) -> Result<(), GeometryError> {
    prepare_cells(world, false, 128, 128 * 4096, 4096)
}
pub(crate) fn prepare_map_scale(
    world: &mut StaticWorld,
    budget: usize,
) -> Result<(), GeometryError> {
    prepare_cells(world, false, crate::line_work::MAX_LINES, budget, 4096)
}
/// Expanded-geometry preparation with explicit line and cell capacities: the M3f2
/// constants for the historical profiles, the format's own under the real-map
/// policy.
pub(crate) fn prepare_extent_with(
    world: &mut StaticWorld,
    max_lines: usize,
    budget: usize,
    max_cells: usize,
) -> Result<(), GeometryError> {
    prepare_cells(world, false, max_lines, budget, max_cells)
}
fn prepare_cells(
    world: &mut StaticWorld,
    conservative: bool,
    max_lines: usize,
    budget: usize,
    max_cells: usize,
) -> Result<(), GeometryError> {
    let map = world.map();
    let [w, h] = map.blockmap().dimensions().map(usize::from);
    if world.lines.len() > max_lines || w * h > max_cells || map.blockmap().words().len() > 65536 {
        return Err(GeometryError::Unsupported(if conservative {
            "M3a geometry work limit"
        } else {
            "M3b geometry work limit"
        }));
    }
    let origin = map.blockmap().origin();
    if conservative {
        for v in map.vertices() {
            if i64::from(v.x) < i64::from(origin.x)
                || i64::from(v.y) < i64::from(origin.y)
                || i64::from(v.x) >= i64::from(origin.x) + w as i64 * 128
                || i64::from(v.y) >= i64::from(origin.y) + h as i64 * 128
            {
                return Err(GeometryError::Unsupported("M3a vertices outside BLOCKMAP"));
            }
        }
    }
    // Compile first-occurrence order for each requested suffix in bounded work.
    // Shared/overlapping long lists must not multiply scans of raw words.
    let mut starts = crate::line_work::reserved(w * h)?;
    for (cell, range) in map.blockmap().cell_ranges().iter().enumerate() {
        starts.push((range.start, cell));
    }
    starts.sort_unstable(); // Preparation lookup only; never candidate traversal.
    let mut cells = crate::line_work::reserved(w * h)?;
    cells.resize_with(w * h, Vec::new);
    let mut next = crate::line_work::reserved(world.lines.len())?;
    next.resize(world.lines.len(), usize::MAX);
    let mut references = 0_usize;
    for (position, word) in map.blockmap().words().iter().copied().enumerate().rev() {
        if word == u16::MAX {
            next.fill(usize::MAX);
        } else if usize::from(word) < world.lines.len() {
            next[usize::from(word)] = position;
        }
        let first = starts.partition_point(|&(start, _)| start < position);
        let end = starts.partition_point(|&(start, _)| start <= position);
        if first == end {
            continue;
        }
        let mut order = crate::line_work::reserved(next.len())?;
        for (line, &word_position) in next.iter().enumerate() {
            if word_position != usize::MAX {
                order.push((word_position, line as u16));
            }
        }
        order.sort_unstable(); // Original word positions, never linedef IDs.
        for &(_, row) in &starts[first..end] {
            references = references
                .checked_add(order.len())
                .filter(|n| *n <= budget)
                .ok_or(GeometryError::WorkLimit)?;
            let mut list = crate::line_work::reserved(order.len())?;
            list.extend(order.iter().map(|&(_, line)| line));
            cells[row] = list;
        }
    }
    let masks = crate::line_work::CellMembership::new(&cells, world.lines.len())?;
    for (i, line) in world.lines.iter().enumerate() {
        if line.delta == [Fixed::ZERO; 2] {
            return Err(GeometryError::Unsupported(if conservative {
                "M3a degenerate line"
            } else {
                "M3b degenerate line"
            }));
        }
        if !conservative {
            continue;
        }
        let xl = ((i64::from(line.bounds[2].bits()) - (i64::from(origin.x) << 16)) >> 23) as usize;
        let xh = ((i64::from(line.bounds[3].bits()) - (i64::from(origin.x) << 16)) >> 23) as usize;
        let yl = ((i64::from(line.bounds[1].bits()) - (i64::from(origin.y) << 16)) >> 23) as usize;
        let yh = ((i64::from(line.bounds[0].bits()) - (i64::from(origin.y) << 16)) >> 23) as usize;
        for x in xl..=xh {
            for y in yl..=yh {
                if !masks.contains(y * w + x, i) {
                    return Err(GeometryError::Unsupported(
                        "M3a incomplete collision BLOCKMAP",
                    ));
                }
            }
        }
    }
    world.collision_cells = Some(cells);
    Ok(())
}

pub(crate) fn cell(world: &StaticWorld, position: [Fixed; 2]) -> [i32; 2] {
    let o = world.map().blockmap().origin();
    [
        (position[0] - Fixed::from_map_units(o.x)).bits() >> 23,
        (position[1] - Fixed::from_map_units(o.y)).bits() >> 23,
    ]
}

pub(crate) fn check(
    world: &StaticWorld,
    id: ActorId,
    actor: &Actor,
    target: [Fixed; 2],
    spatial: &SpatialIndex,
    get: &impl Fn(ActorId) -> Option<Actor>,
) -> ActorMoveAttempt {
    let mut result = check_things(world, id, actor, target, spatial, get);
    if result.blocked_by_actor.is_none() {
        result.lines = world.check_validated_position(target, actor.spec.radius, actor.spec.height);
    }
    result
}

pub(crate) fn check_things(
    world: &StaticWorld,
    id: ActorId,
    actor: &Actor,
    target: [Fixed; 2],
    spatial: &SpatialIndex,
    get: &impl Fn(ActorId) -> Option<Actor>,
) -> ActorMoveAttempt {
    let mut result = ActorMoveAttempt {
        target,
        lines: Collision {
            blocked_by: None,
            fits: true,
            candidates: Vec::new(),
        },
        actors: Vec::new(),
        blocked_by_actor: None,
    };
    for_things(
        world,
        id,
        actor,
        target,
        spatial,
        get,
        |candidate, other| {
            result.actors.push(candidate);
            if other.solidity == Solidity::NonSolid || candidate == id {
                return true;
            }
            let distance =
                i64::from(actor.spec.radius.bits()) + i64::from(other.spec.radius.bits());
            if (i64::from(other.spec.position[0].bits()) - i64::from(target[0].bits())).abs()
                < distance
                && (i64::from(other.spec.position[1].bits()) - i64::from(target[1].bits())).abs()
                    < distance
            {
                result.blocked_by_actor = Some(candidate);
                return false;
            }
            true
        },
    );
    result
}
pub(crate) fn for_things(
    world: &StaticWorld,
    id: ActorId,
    actor: &Actor,
    target: [Fixed; 2],
    spatial: &SpatialIndex,
    get: &impl Fn(ActorId) -> Option<Actor>,
    mut visit: impl FnMut(ActorId, Actor) -> bool,
) {
    let margin = actor.spec.radius + Fixed::from_map_units(32);
    let [xl, yl] = cell(world, [target[0] - margin, target[1] - margin]);
    let [xh, yh] = cell(world, [target[0] + margin, target[1] + margin]);
    let [w, h] = world.map().blockmap().dimensions().map(i32::from);
    for x in xl.max(0)..=xh.min(w - 1) {
        for y in yl.max(0)..=yh.min(h - 1) {
            for candidate in &spatial.order {
                let other = if *candidate == id {
                    *actor
                } else {
                    get(*candidate).expect("valid spatial handle")
                };
                if !crate::lifecycle::blocks_cell_index(&other)
                    || cell(world, other.spec.position) != [x, y]
                {
                    continue;
                }
                if !visit(*candidate, other) {
                    return;
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Divline {
    start: [Fixed; 2],
    delta: [Fixed; 2],
}
fn shifted(value: Fixed, bits: u32) -> Fixed {
    Fixed::from_bits(value.bits() >> bits)
}
fn div_side(line: Divline, point: [Fixed; 2]) -> u8 {
    let [dx, dy] = line.delta;
    if dx == Fixed::ZERO {
        return u8::from(if point[0] <= line.start[0] {
            dy > Fixed::ZERO
        } else {
            dy < Fixed::ZERO
        });
    }
    if dy == Fixed::ZERO {
        return u8::from(if point[1] <= line.start[1] {
            dx < Fixed::ZERO
        } else {
            dx > Fixed::ZERO
        });
    }
    let x = point[0] - line.start[0];
    let y = point[1] - line.start[1];
    if (dy.bits() ^ dx.bits() ^ x.bits() ^ y.bits()) < 0 {
        return u8::from((dy.bits() ^ x.bits()) < 0);
    }
    u8::from(shifted(y, 8) * shifted(dx, 8) >= shifted(dy, 8) * shifted(x, 8))
}
fn intercept(trace: Divline, line: &LineGeometry) -> Fixed {
    let [dx, dy] = line.delta;
    let den = shifted(dy, 8) * trace.delta[0] - shifted(dx, 8) * trace.delta[1];
    if den == Fixed::ZERO {
        return Fixed::ZERO;
    }
    let num = shifted(line.start[0] - trace.start[0], 8) * dy
        + shifted(trace.start[1] - line.start[1], 8) * dx;
    num / den
}

/// Why a slide or use trace was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PathError {
    /// The source's 129th intercept of one trace (p_local.h `MAXINTERCEPTS`),
    /// which it writes past `intercepts[]`; refused by the real-map policy.
    Overflow,
    /// Under the Episode 1 policy's checked arithmetic (`checked` set), a
    /// `P_InterceptVector` the source would overflow (decision H3).
    Arithmetic,
}
/// P_PathTraverse with PT_ADDLINES for a slide trace: every line listed in a
/// visited cell whose endpoints straddle the infinite trace line with a
/// non-negative fraction, in fraction order. `limit` is the policy's intercept
/// witness (`Some(128)` under the real-map policy): the intercept that would be
/// the `limit + 1`th is an overflow. Historical policies pass `None` and collect
/// without bound, exactly as before.
pub(crate) fn path(
    world: &StaticWorld,
    mut start: [Fixed; 2],
    end: [Fixed; 2],
    limit: Option<usize>,
    checked: bool,
) -> Result<Vec<(Fixed, usize)>, PathError> {
    let o = world.map().blockmap().origin();
    let origin = [Fixed::from_map_units(o.x), Fixed::from_map_units(o.y)];
    for axis in 0..2 {
        if (start[axis] - origin[axis]).bits() & ((128 << 16) - 1) == 0 {
            start[axis] = start[axis] + Fixed::ONE;
        }
    }
    let trace = Divline {
        start,
        delta: [end[0] - start[0], end[1] - start[1]],
    };
    let a = [start[0] - origin[0], start[1] - origin[1]];
    let b = [end[0] - origin[0], end[1] - origin[1]];
    let first = a.map(|p| p.bits() >> 23);
    let last = b.map(|p| p.bits() >> 23);
    let mut map = first;
    let mut steps = [0; 2];
    let mut increments = [Fixed::ZERO; 2];
    let mut crossings = [Fixed::ZERO; 2];
    for axis in 0..2 {
        let other = 1 - axis;
        let fraction = Fixed::from_bits((a[axis].bits() >> 7) & 0xffff);
        let partial;
        if last[axis] > first[axis] {
            steps[axis] = 1;
            partial = Fixed::ONE - fraction;
            increments[axis] =
                (b[other] - a[other]) / Fixed::from_bits((b[axis] - a[axis]).bits().wrapping_abs());
        } else if last[axis] < first[axis] {
            steps[axis] = -1;
            partial = fraction;
            increments[axis] =
                (b[other] - a[other]) / Fixed::from_bits((b[axis] - a[axis]).bits().wrapping_abs());
        } else {
            partial = Fixed::ONE;
            increments[axis] = Fixed::from_bits(256 << 16);
        }
        crossings[axis] = shifted(a[other], 7) + partial * increments[axis];
    }
    let [w, h] = world.map().blockmap().dimensions().map(i32::from);
    let mut seen = crate::line_work::SeenLines::for_lines(world.lines.len());
    let mut hits = Vec::new();
    for _ in 0..64 {
        if map[0] >= 0 && map[1] >= 0 && map[0] < w && map[1] < h {
            for n in world.line_candidates(map[0] as usize, map[1] as usize) {
                let i = usize::from(n);
                if !seen.insert(i) {
                    continue;
                }
                let line = &world.lines[i];
                let sides = if trace
                    .delta
                    .iter()
                    .any(|d| d.bits() > 16 * 65536 || d.bits() < -16 * 65536)
                {
                    [
                        div_side(trace, line.start),
                        div_side(
                            trace,
                            [line.start[0] + line.delta[0], line.start[1] + line.delta[1]],
                        ),
                    ]
                } else {
                    [line.point_side(start), line.point_side(end)]
                };
                if sides[0] != sides[1] {
                    let frac = if checked {
                        crate::sight::checked_intercept(
                            trace.start,
                            trace.delta,
                            line.start,
                            line.delta,
                        )
                        .ok_or(PathError::Arithmetic)?
                    } else {
                        intercept(trace, line)
                    };
                    if frac >= Fixed::ZERO {
                        if limit.is_some_and(|limit| hits.len() == limit) {
                            crate::level_stats::observe_intercepts(hits.len() + 1);
                            return Err(PathError::Overflow);
                        }
                        hits.push((frac, i));
                    }
                }
            }
        }
        if map == last {
            break;
        }
        if crossings[0].bits() >> 16 == map[1] {
            crossings[0] = crossings[0] + increments[0];
            map[0] += steps[0];
        } else if crossings[1].bits() >> 16 == map[0] {
            crossings[1] = crossings[1] + increments[1];
            map[1] += steps[1];
        }
    }
    // Stable sort reproduces the C repeated strict-minimum scan on ties.
    hits.sort_by_key(|hit| hit.0);
    crate::level_stats::observe_intercepts(hits.len());
    Ok(hits)
}

fn slope_div(num: u32, den: u32) -> usize {
    if den < 512 {
        return 2048;
    }
    ((num.wrapping_shl(3) / (den >> 8)).min(2048)) as usize
}
pub(crate) fn point_angle(delta: [Fixed; 2]) -> u32 {
    let [x, y] = delta.map(Fixed::bits);
    if x == 0 && y == 0 {
        return 0;
    }
    let ax = x.unsigned_abs();
    let ay = y.unsigned_abs();
    let value = if ax > ay {
        TAN_TO_ANGLE[slope_div(ay, ax)]
    } else {
        TAN_TO_ANGLE[slope_div(ax, ay)]
    };
    match (x >= 0, y >= 0, ax > ay) {
        (true, true, true) => value,
        (true, true, false) => 0x3fffffff_u32.wrapping_sub(value),
        (true, false, true) => value.wrapping_neg(),
        (true, false, false) => 0xc0000000_u32.wrapping_add(value),
        (false, true, true) => 0x7fffffff_u32.wrapping_sub(value),
        (false, true, false) => 0x40000000_u32.wrapping_add(value),
        (false, false, true) => 0x80000000_u32.wrapping_add(value),
        (false, false, false) => 0xbfffffff_u32.wrapping_sub(value),
    }
}
fn project(line: &LineGeometry, position: [Fixed; 2], motion: [Fixed; 2]) -> [Fixed; 2] {
    if line.slope == 0 {
        return [motion[0], Fixed::ZERO];
    }
    if line.slope == 1 {
        return [Fixed::ZERO, motion[1]];
    }
    let mut angle = point_angle(line.delta);
    if line.point_side(position) == 1 {
        angle = angle.wrapping_add(0x80000000);
    }
    let mut delta = point_angle(motion).wrapping_sub(angle);
    if delta > 0x80000000 {
        delta = delta.wrapping_add(0x80000000);
    }
    let [x, y] = motion.map(|m| m.bits().unsigned_abs());
    let length = Fixed::from_bits(x.wrapping_add(y).wrapping_sub(x.min(y) >> 1) as i32);
    let new_length = length * Fixed::from_bits(crate::FINE_SINE[(delta >> 19) as usize + 2048]);
    let index = (angle >> 19) as usize;
    [
        new_length * Fixed::from_bits(crate::FINE_SINE[index + 2048]),
        new_length * Fixed::from_bits(crate::FINE_SINE[index]),
    ]
}

pub(crate) enum MotionEvent {
    Contact(crate::world_pickups::ContactEvent),
    Attempt(ActorMoveAttempt),
    Slide(u16),
    Sector(crate::SectorMoveAttempt),
    /// Under the movers policy, a line the preceding accepted attempt crossed
    /// (the `P_TryMove` crossing loop, in call order); the caller runs
    /// `P_CrossSpecialLine` for it with the staged tables.
    Cross(crate::line_triggers::Crossing),
    /// Under the movers policy, the preceding attempt would have stored a ninth
    /// `spechit` entry: the caller refuses the tic (`MapScaleError::SpecialCross`).
    /// No attempt follows it in the same motion.
    SpecialCross,
    /// Under the teleport policy (wire 24), an event of a crossing that already ran
    /// inside the move ([`MotionAccess::cross`]), in its place in the motion.
    Outer(crate::Event),
    /// Under the teleport policy, a crossing inside the move changed the line's
    /// runtime special to `after` (a W1 clear): the caller stages it after the
    /// motion. Later attempts of the motion already read it through `cleared`.
    Stage {
        /// The line.
        line: u16,
        /// Its special after the crossing.
        after: i16,
    },
}
pub(crate) trait MotionAccess {
    type Error;
    fn get(&self, id: ActorId) -> Option<Actor>;
    fn contacts(
        &mut self,
        _id: ActorId,
        _actor: &mut Actor,
        _attempt: &ActorMoveAttempt,
        _spatial: &mut SpatialIndex,
        _events: &mut Vec<MotionEvent>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    /// Called once a projected slide move (`P_HitSlideLine` then `P_TryMove`) has
    /// been accepted, with the actor at its new position: the diagonal policy's
    /// escape witness (M3h5 note §4.5). No-op elsewhere.
    fn after_slide(&mut self, _actor: &Actor) -> Result<(), Self::Error> {
        Ok(())
    }
    /// Called after every accepted `P_TryMove` attempt (each halved step, flush,
    /// projected slide or stairstep), with the actor at its new position: the
    /// barrel escape witness of the barrels policy
    /// ([`crate::barrels::BarrelMotion`]). No-op elsewhere.
    fn after_move(&mut self, _actor: &Actor) -> Result<(), Self::Error> {
        Ok(())
    }
    /// Whether the crossings of an accepted attempt run inside the move, in
    /// `P_TryMove`'s loop (p_map.c:500–514), through [`Self::cross`]: under the
    /// teleport policy (wire 24), where `EV_Teleport` moves the thing mid-motion. Every
    /// other access returns the crossings as [`MotionEvent::Cross`] for the caller to
    /// run after the motion, as before.
    fn crosses_in_move(&self) -> bool {
        false
    }
    /// `P_CrossSpecialLine` for one crossing of the loop, inside the move, with
    /// `actor` the mover's current record (updated in place by a teleport). The
    /// crossing's events are appended to `events` as [`MotionEvent::Outer`] (and a
    /// changed special as [`MotionEvent::Stage`]). Returns whether the loop ends
    /// (`P_TeleportMove` was reached, p_map.c:152). Called only when
    /// [`Self::crosses_in_move`] is true.
    #[expect(
        clippy::too_many_arguments,
        reason = "the move's state, borrowed apart"
    )]
    fn cross(
        &mut self,
        _id: ActorId,
        _actor: &mut Actor,
        _crosser: crate::line_triggers::Crosser,
        _crossing: crate::line_triggers::Crossing,
        _sectors: crate::staging::Sectors<'_>,
        _spatial: &mut SpatialIndex,
        _events: &mut Vec<MotionEvent>,
    ) -> Result<bool, Self::Error> {
        unreachable!("no crossing inside the move without the teleport policy")
    }
    /// The slide trace's intercept witness: `Some(MAX_INTERCEPTS)` under the
    /// real-map policy, `None` (unbounded, the historical behavior) elsewhere.
    fn intercept_limit(&self) -> Option<usize> {
        None
    }
    /// The error of a slide trace that reached the witness; called only when
    /// `intercept_limit` is `Some`.
    fn intercept_overflow(&self) -> Self::Error {
        unreachable!("no intercept limit without a policy error")
    }
    /// The error of a slide trace whose checked intercept overflowed; called only
    /// when `intercept_limit` is `Some` under the Episode 1 policy.
    fn intercept_arithmetic(&self) -> Self::Error {
        unreachable!("no checked arithmetic without a policy error")
    }
}
impl<F: Fn(ActorId) -> Option<Actor>> MotionAccess for F {
    type Error = std::convert::Infallible;
    fn get(&self, id: ActorId) -> Option<Actor> {
        self(id)
    }
}
fn infallible<T>(result: Result<T, std::convert::Infallible>) -> T {
    match result {
        Ok(v) => v,
        Err(never) => match never {},
    }
}
struct Mover<'a, F> {
    world: &'a StaticWorld,
    id: ActorId,
    actor: &'a mut Actor,
    spatial: &'a mut SpatialIndex,
    access: F,
    events: Vec<MotionEvent>,
    profile: MotionProfile<'a>,
    /// Lines cleared by crossings of this motion, which the caller stages after
    /// it: later attempts read them as 0 (`P_CrossSpecialLine` clears a W1 line
    /// for a player whatever its `EV_*` did, so the clear is known here).
    cleared: Vec<u16>,
    /// A ninth `spechit` entry was refused: the tic is discarded, nothing more is
    /// attempted.
    overflow: bool,
}
impl<F: MotionAccess> Mover<'_, F> {
    fn attempt(&mut self, delta: [Fixed; 2]) -> Result<bool, F::Error> {
        if self.overflow {
            return Ok(false);
        }
        let target = [
            self.actor.spec.position[0] + delta[0],
            self.actor.spec.position[1] + delta[1],
        ];
        let mut things = check_things(
            self.world,
            self.id,
            self.actor,
            target,
            self.spatial,
            &|id| self.access.get(id),
        );
        self.access
            .contacts(self.id, self.actor, &things, self.spatial, &mut self.events)?;
        if let Some(sectors) = self.profile.sectors {
            let attempt = crate::multisector::check_move_from_things(
                (self.world, sectors),
                self.actor,
                target,
                things,
                self.profile.command.is_some(),
                false,
            );
            let Ok(hit) = crate::line_triggers::spechit(sectors, &attempt, &self.cleared) else {
                self.overflow = true;
                self.events.push(MotionEvent::Sector(attempt));
                self.events.push(MotionEvent::SpecialCross);
                return Ok(false);
            };
            let old = self.actor.spec.position;
            let allowed = attempt.allowed();
            if allowed {
                self.actor.spec.position = target;
                self.actor.floor = attempt.floor;
                self.actor.ceiling = attempt.ceiling;
                self.actor.location = Some(attempt.location);
                self.spatial.link(self.id);
            }
            self.events.push(MotionEvent::Sector(attempt));
            if allowed && !hit.is_empty() {
                let crosser = if self.profile.command.is_some() {
                    crate::line_triggers::Crosser::Player
                } else {
                    crate::line_triggers::Crosser::Monster
                };
                // The entries are distinct lines, so a clear by one crossing of this
                // loop never changes another entry's test.
                let found = crate::line_triggers::crossings(
                    self.world,
                    sectors,
                    &hit,
                    old,
                    target,
                    &self.cleared,
                )
                .collect::<Vec<_>>();
                let inside = self.access.crosses_in_move();
                for crossing in found {
                    if crate::line_triggers::clears(crosser, crossing.special) {
                        self.cleared.push(crossing.line);
                    }
                    if !inside {
                        self.events.push(MotionEvent::Cross(crossing));
                        continue;
                    }
                    // Wire 24: the crossing runs now. A side computed before an
                    // earlier crossing of this loop stays exact: only a teleport
                    // moves the thing, and it ends the loop.
                    if self.access.cross(
                        self.id,
                        self.actor,
                        crosser,
                        crossing,
                        sectors,
                        self.spatial,
                        &mut self.events,
                    )? {
                        break;
                    }
                }
            }
            if allowed {
                self.access.after_move(self.actor)?;
            }
            return Ok(allowed);
        }
        if things.blocked_by_actor.is_none() {
            things.lines = self.world.check_validated_position(
                target,
                self.actor.spec.radius,
                self.actor.spec.height,
            );
        }
        let attempt = things;
        let allowed = attempt.allowed();
        self.events.push(MotionEvent::Attempt(attempt));
        if allowed {
            self.actor.spec.position = target;
            self.spatial.link(self.id);
            self.access.after_move(self.actor)?;
        }
        Ok(allowed)
    }
    fn stairstep(&mut self) -> Result<(), F::Error> {
        if !self.attempt([Fixed::ZERO, self.actor.spec.momentum[1]])? {
            self.attempt([self.actor.spec.momentum[0], Fixed::ZERO])?;
        }
        Ok(())
    }
    fn slide(&mut self) -> Result<(), F::Error> {
        for _ in 0..2 {
            let pos = self.actor.spec.position;
            let radius = self.actor.spec.radius;
            let mom = self.actor.spec.momentum;
            let lead = [
                if mom[0] > Fixed::ZERO {
                    pos[0] + radius
                } else {
                    pos[0] - radius
                },
                if mom[1] > Fixed::ZERO {
                    pos[1] + radius
                } else {
                    pos[1] - radius
                },
            ];
            let trail = [
                if mom[0] > Fixed::ZERO {
                    pos[0] - radius
                } else {
                    pos[0] + radius
                },
                if mom[1] > Fixed::ZERO {
                    pos[1] - radius
                } else {
                    pos[1] + radius
                },
            ];
            let mut best = Fixed::from_bits(65537);
            let mut blocker = None;
            for start in [lead, [trail[0], lead[1]], [lead[0], trail[1]]] {
                let end = [start[0] + mom[0], start[1] + mom[1]];
                // Only an access with an intercept limit (the player of the real-map
                // policies) can carry the Episode 1 policy's checked arithmetic.
                let limit = self.access.intercept_limit();
                let checked = self.world.checked_arithmetic && limit.is_some();
                let hits = match path(self.world, start, end, limit, checked) {
                    Ok(hits) => hits,
                    Err(PathError::Overflow) => return Err(self.access.intercept_overflow()),
                    Err(PathError::Arithmetic) => return Err(self.access.intercept_arithmetic()),
                };
                for (frac, i) in hits {
                    if frac > Fixed::ONE {
                        break;
                    }
                    let blocks = if let Some(sectors) = self.profile.sectors {
                        crate::multisector::slide_blocks(self.world, sectors, i, self.actor)
                    } else {
                        self.world.lines[i].point_side(pos) == 0
                    };
                    if !blocks {
                        continue;
                    }
                    if frac < best {
                        best = frac;
                        blocker = Some(i);
                    }
                    break;
                }
            }
            let Some(i) = blocker else {
                self.stairstep()?;
                return Ok(());
            };
            let flush = best - Fixed::from_bits(0x800);
            if flush > Fixed::ZERO && !self.attempt([mom[0] * flush, mom[1] * flush])? {
                self.stairstep()?;
                return Ok(());
            }
            let remainder = (Fixed::ONE - best).min(Fixed::ONE);
            if remainder <= Fixed::ZERO {
                return Ok(());
            }
            self.events.push(MotionEvent::Slide(i as u16));
            // p_map.c:776–777 read `mo->momx/momy` after the move up to the wall,
            // which a teleport inside it zeroed (wire 24); no other attempt changes
            // the momentum, so `mom` is that value for every older policy.
            let mom = self.actor.spec.momentum;
            let motion = project(
                &self.world.lines[i],
                self.actor.spec.position,
                [mom[0] * remainder, mom[1] * remainder],
            );
            self.actor.spec.momentum = motion;
            if self.attempt(motion)? {
                self.access.after_slide(self.actor)?;
                return Ok(());
            }
        }
        self.stairstep()
    }
}

pub(crate) fn move_xy(
    world: &StaticWorld,
    id: ActorId,
    actor: &mut Actor,
    spatial: &mut SpatialIndex,
    get: impl Fn(ActorId) -> Option<Actor>,
    command: Option<TicCommand>,
) -> Vec<MotionEvent> {
    infallible(move_xy_profile(
        world,
        id,
        actor,
        spatial,
        get,
        MotionProfile {
            command,
            sectors: None,
        },
    ))
}
pub(crate) fn move_xy_multisector(
    world: &StaticWorld,
    id: ActorId,
    actor: &mut Actor,
    spatial: &mut SpatialIndex,
    get: impl Fn(ActorId) -> Option<Actor>,
    command: Option<TicCommand>,
    sectors: crate::staging::Sectors<'_>,
) -> Vec<MotionEvent> {
    infallible(move_xy_profile(
        world,
        id,
        actor,
        spatial,
        get,
        MotionProfile {
            command,
            sectors: Some(sectors),
        },
    ))
}
struct MotionProfile<'a> {
    command: Option<TicCommand>,
    sectors: Option<crate::staging::Sectors<'a>>,
}
fn move_xy_profile<F: MotionAccess>(
    world: &StaticWorld,
    id: ActorId,
    actor: &mut Actor,
    spatial: &mut SpatialIndex,
    access: F,
    profile: MotionProfile<'_>,
) -> Result<Vec<MotionEvent>, F::Error> {
    if actor.spec.momentum == [Fixed::ZERO; 2] {
        return Ok(Vec::new());
    }
    let command = profile.command;
    let mut mover = Mover {
        world,
        id,
        actor,
        spatial,
        access,
        events: Vec::new(),
        profile,
        cleared: Vec::new(),
        overflow: false,
    };
    for m in &mut mover.actor.spec.momentum {
        *m = Fixed::from_bits(m.bits().clamp(-30 * 65536, 30 * 65536));
    }
    let [mut x, mut y] = mover.actor.spec.momentum.map(Fixed::bits);
    loop {
        let delta = if x > 15 * 65536 || y > 15 * 65536 {
            let d = [x / 2, y / 2];
            x >>= 1;
            y >>= 1;
            d
        } else {
            let d = [x, y];
            x = 0;
            y = 0;
            d
        };
        if !mover.attempt(delta.map(Fixed::from_bits))? {
            if command.is_some() {
                mover.slide()?;
            } else {
                mover.actor.spec.momentum = [Fixed::ZERO; 2];
            }
        }
        if x == 0 && y == 0 {
            break;
        }
    }
    if mover.profile.sectors.is_some() && mover.actor.z > mover.actor.floor {
        return Ok(mover.events);
    }
    if mover
        .actor
        .life
        .is_some_and(|life| life.flags & 1048576 != 0)
        && mover
            .actor
            .spec
            .momentum
            .iter()
            .any(|m| m.bits() > 16384 || m.bits() < -16384)
    {
        if let Some(sectors) = mover.profile.sectors {
            if mover.actor.floor != sectors[usize::from(mover.actor.location.unwrap().sector)].floor
            {
                return Ok(mover.events);
            }
        }
    }
    if mover
        .actor
        .spec
        .momentum
        .iter()
        .all(|m| m.bits() > -0x1000 && m.bits() < 0x1000)
        && command.is_none_or(|c| c.forward == 0 && c.side == 0)
    {
        mover.actor.spec.momentum = [Fixed::ZERO; 2];
    } else {
        for m in &mut mover.actor.spec.momentum {
            *m = *m * Fixed::from_bits(0xe800);
        }
    }
    Ok(mover.events)
}

pub(crate) fn move_xy_contacts<F: MotionAccess>(
    world: &StaticWorld,
    id: ActorId,
    actor: &mut Actor,
    spatial: &mut SpatialIndex,
    access: F,
    command: Option<TicCommand>,
    sectors: crate::staging::Sectors<'_>,
) -> Result<Vec<MotionEvent>, F::Error> {
    move_xy_profile(
        world,
        id,
        actor,
        spatial,
        access,
        MotionProfile {
            command,
            sectors: Some(sectors),
        },
    )
}
/// Original P_AproxDistance within the admitted signed coordinate bounds.
pub(crate) fn approx_distance(delta: [Fixed; 2]) -> Fixed {
    let x = delta[0].bits().abs();
    let y = delta[1].bits().abs();
    Fixed::from_bits(x + y - (x.min(y) >> 1))
}
