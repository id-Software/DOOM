//! r_bsp.c: the solid-segment clip list (`R_ClearClipSegs`, `R_ClipSolidWallSegment`,
//! `R_ClipPassWallSegment`), `R_AddLine`, `R_CheckBBox`, `R_Subsector` and `R_RenderBSPNode`,
//! with the map geometry as `P_LoadVertexes`, `P_LoadSegs`, `P_LoadSubsectors` and
//! `P_LoadNodes` (p_setup.c) build it.
use crate::r_data::ResourceError;
use crate::r_main::{ANG90, ANG180, ANGLETOFINESHIFT, point_on_side, point_to_angle};
use crate::r_segs::DrawSeg;
use crate::{FrameContext, RenderRefusal};

/// `MAXSEGS` (r_bsp.c:88): solid-segment list capacity; the genuine list is accessed past it
/// unchecked, so the first access past it is the `SolidSegOverflow` refusal (decision R3,
/// accepted by the owner 2026-11-15 at the first out-of-bounds read).
pub const MAXSEGS: usize = 32;
/// `ML_TWOSIDED` (doomdata.h:121).
const ML_TWOSIDED: u16 = 4;
/// `NF_SUBSECTOR` (doomdata.h:192).
const NF_SUBSECTOR: u16 = 0x8000;

/// A seg as `P_LoadSegs` (p_setup.c:158-197) builds it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LevelSeg {
    /// `v1->x, v1->y` (16.16).
    pub v1: [i32; 2],
    /// `v2->x, v2->y` (16.16).
    pub v2: [i32; 2],
    /// `angle` (`SHORT(ml->angle) << 16`, p_setup.c:181).
    pub angle: u32,
    /// `offset` (`SHORT(ml->offset) << 16`, p_setup.c:182).
    pub offset: i32,
    /// `linedef`.
    pub linedef: usize,
    /// `sidedef` (`sides[ldef->sidenum[side]]`).
    pub sidedef: usize,
    /// `frontsector`.
    pub frontsector: usize,
    /// `backsector` (only for `ML_TWOSIDED`, p_setup.c:189-192).
    pub backsector: Option<usize>,
}

/// A subsector (`P_LoadSubsectors`, with `sector` from `P_GroupLines`, p_setup.c:517).
#[derive(Clone, Copy, Debug)]
pub(crate) struct LevelSubsector {
    pub firstline: usize,
    pub numlines: usize,
    pub sector: usize,
}

/// A node (`P_LoadNodes`): partition `(x, y, dx, dy)` and boxes in 16.16, raw children.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LevelNode {
    pub partition: [i32; 4],
    /// `bbox[2][4]` in BOXTOP, BOXBOTTOM, BOXLEFT, BOXRIGHT order.
    pub bbox: [[i32; 4]; 2],
    /// `children[2]` as the unsigned short the C stores (`NF_SUBSECTOR` marks a leaf).
    pub children: [u16; 2],
}

/// The level geometry the renderer traverses.
#[derive(Clone, Debug, Default)]
pub(crate) struct Geometry {
    pub segs: Vec<LevelSeg>,
    pub subsectors: Vec<LevelSubsector>,
    pub nodes: Vec<LevelNode>,
    /// Per linedef `flags`.
    pub line_flags: Vec<u16>,
    /// Per sidedef `rowoffset` (16.16).
    pub rowoffset: Vec<i32>,
}

impl Geometry {
    /// Builds the tables from a validated map.
    pub(crate) fn load(map: &doom_map::Map) -> Result<Self, ResourceError> {
        let vtx = |i: u16| -> Result<[i32; 2], ResourceError> {
            let v = map
                .vertices()
                .get(usize::from(i))
                .ok_or_else(|| ResourceError::Map(format!("vertex {i}")))?;
            Ok([i32::from(v.x) << 16, i32::from(v.y) << 16])
        };
        let side_sector = |s: Option<u16>| -> Result<usize, ResourceError> {
            let s = s.ok_or_else(|| ResourceError::Map("seg side without a sidedef".into()))?;
            map.sidedefs()
                .get(usize::from(s))
                .map(|d| usize::from(d.sector))
                .ok_or_else(|| ResourceError::Map(format!("sidedef {s}")))
        };
        let mut segs = Vec::with_capacity(map.segs().len());
        for s in map.segs() {
            let line = map
                .linedefs()
                .get(usize::from(s.linedef))
                .ok_or_else(|| ResourceError::Map(format!("linedef {}", s.linedef)))?;
            let side = usize::from(s.side & 1);
            let sidedef = line.sides[side]
                .ok_or_else(|| ResourceError::Map("seg side without a sidedef".into()))?;
            segs.push(LevelSeg {
                v1: vtx(s.vertices[0])?,
                v2: vtx(s.vertices[1])?,
                angle: u32::from(s.angle) << 16,
                offset: i32::from(s.offset) << 16,
                linedef: usize::from(s.linedef),
                sidedef: usize::from(sidedef),
                frontsector: side_sector(line.sides[side])?,
                // `sides[ldef->sidenum[side^1]]` (p_setup.c:189-190): a two-sided line without
                // that side reads `sides[-1]`, outside the array. Only a seg reads it, so a
                // two-sided line without a back side and without segs loads.
                backsector: if line.flags & ML_TWOSIDED != 0 {
                    let back = line.sides[side ^ 1].ok_or_else(|| {
                        ResourceError::Map(format!(
                            "seg of two-sided line {} reads sides[-1] (p_setup.c:189-190)",
                            s.linedef
                        ))
                    })?;
                    Some(side_sector(Some(back))?)
                } else {
                    None
                },
            });
        }
        let mut subsectors = Vec::with_capacity(map.subsectors().len());
        for ss in map.subsectors() {
            let first = usize::from(ss.first_seg);
            let seg = segs
                .get(first)
                .ok_or_else(|| ResourceError::Map(format!("subsector seg {first}")))?;
            subsectors.push(LevelSubsector {
                firstline: first,
                numlines: usize::from(ss.seg_count),
                // `ss->sector = seg->sidedef->sector` (p_setup.c:517).
                sector: usize::from(map.sidedefs()[seg.sidedef].sector),
            });
        }
        let nodes = map
            .nodes()
            .iter()
            .map(|n| {
                let f = |v: i16| i32::from(v) << 16;
                let child = |c: doom_map::BspChild| match c {
                    doom_map::BspChild::Node(i) => i,
                    doom_map::BspChild::Subsector(i) => i | NF_SUBSECTOR,
                };
                LevelNode {
                    partition: [f(n.origin.x), f(n.origin.y), f(n.delta.x), f(n.delta.y)],
                    bbox: n.bounds.map(|b| b.map(f)),
                    children: n.children.map(child),
                }
            })
            .collect();
        Ok(Self {
            segs,
            subsectors,
            nodes,
            line_flags: map.linedefs().iter().map(|l| l.flags).collect(),
            rowoffset: map
                .sidedefs()
                .iter()
                .map(|s| i32::from(s.offsets[1]) << 16)
                .collect(),
        })
    }
}

/// `cliprange_t` (r_bsp.c:80-85).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ClipRange {
    first: i32,
    last: i32,
}

/// The r_bsp.c globals of one frame.
#[derive(Clone, Debug)]
pub(crate) struct BspState {
    /// `solidsegs[MAXSEGS]` (r_bsp.c:92). Entries past the live ones are copies the crunch loop
    /// leaves behind (r_bsp.c:178-182); their values are never examined (every search stops at
    /// the `0x7fffffff` sentinel), only their count.
    solidsegs: [ClipRange; MAXSEGS],
    /// `newend - solidsegs` (r_bsp.c:91).
    newend: usize,
    /// The genuine runner's peak of `newend - solidsegs` (at every `R_StoreWallRange` call and
    /// the frame end).
    pub solid_peak: usize,
    /// `drawsegs[]` up to `ds_p` (r_bsp.c:53-54).
    pub drawsegs: Vec<DrawSeg>,
    /// `R_StoreWallRange` calls dropped at `MAXDRAWSEGS`.
    pub drawseg_drops: u32,
    /// `curline` (a seg index).
    pub curline: usize,
    /// `frontsector` (the subsector's sector, r_bsp.c:512).
    pub frontsector: usize,
    /// `backsector` (`line->backsector`, r_bsp.c:320).
    pub backsector: Option<usize>,
}

impl Default for BspState {
    fn default() -> Self {
        Self {
            solidsegs: [ClipRange::default(); MAXSEGS],
            newend: 0,
            solid_peak: 0,
            drawsegs: Vec::with_capacity(crate::r_segs::MAXDRAWSEGS),
            drawseg_drops: 0,
            curline: 0,
            frontsector: 0,
            backsector: None,
        }
    }
}

/// `checkcoord[12][4]` (r_bsp.c:365-378); rows 3, 7 and 11 are zero.
const CHECKCOORD: [[usize; 4]; 12] = [
    [3, 0, 2, 1],
    [3, 0, 2, 0],
    [3, 1, 2, 0],
    [0, 0, 0, 0],
    [2, 0, 2, 1],
    [0, 0, 0, 0],
    [3, 1, 3, 0],
    [0, 0, 0, 0],
    [2, 0, 3, 1],
    [2, 1, 3, 1],
    [2, 1, 3, 0],
    [0, 0, 0, 0],
];

impl FrameContext<'_> {
    /// `solidsegs[i]`; an index past the array is the first out-of-bounds access.
    fn solid(&self, i: usize) -> Result<ClipRange, RenderRefusal> {
        self.bsp
            .solidsegs
            .get(i)
            .copied()
            .ok_or(RenderRefusal::SolidSegOverflow)
    }

    /// `R_ClearClipSegs` (r_bsp.c:245-252); `R_ClearDrawSegs` (r_bsp.c:68-71).
    pub(crate) fn clear_clip_segs(&mut self) {
        self.bsp.solidsegs[0] = ClipRange {
            first: -0x7fff_ffff,
            last: -1,
        };
        self.bsp.solidsegs[1] = ClipRange {
            first: self.view.viewwidth,
            last: 0x7fff_ffff,
        };
        self.bsp.newend = 2;
        self.bsp.drawsegs.clear();
    }

    /// `R_ClipSolidWallSegment` (r_bsp.c:103-185).
    fn clip_solid_wall_segment(&mut self, first: i32, last: i32) -> Result<(), RenderRefusal> {
        // Find the first range that touches the range (r_bsp.c:112-114).
        let mut start = 0;
        while self.solid(start)?.last < first.wrapping_sub(1) {
            start += 1;
        }
        let s = self.solid(start)?;
        if first < s.first {
            if last < s.first.wrapping_sub(1) {
                // Post is entirely visible: insert a new clippost (r_bsp.c:120-133).
                self.store_wall_range(first, last)?;
                let mut next = self.bsp.newend;
                // `*next = ...` (or `next->first = first`) writes solidsegs[newend]: past the
                // array when newend is MAXSEGS.
                if next >= MAXSEGS {
                    return Err(RenderRefusal::SolidSegOverflow);
                }
                self.bsp.newend += 1;
                while next != start {
                    self.bsp.solidsegs[next] = self.bsp.solidsegs[next - 1];
                    next -= 1;
                }
                self.bsp.solidsegs[next] = ClipRange { first, last };
                return Ok(());
            }
            // There is a fragment above *start (r_bsp.c:136-140).
            self.store_wall_range(first, s.first.wrapping_sub(1))?;
            self.bsp.solidsegs[start].first = first;
        }
        // Bottom contained in start? (r_bsp.c:143-145)
        if last <= self.solid(start)?.last {
            return Ok(());
        }
        let mut next = start;
        let mut crunch = false;
        while last >= self.solid(next + 1)?.first.wrapping_sub(1) {
            // There is a fragment between two posts (r_bsp.c:147-161).
            let (a, b) = (self.solid(next)?, self.solid(next + 1)?);
            self.store_wall_range(a.last.wrapping_add(1), b.first.wrapping_sub(1))?;
            next += 1;
            if last <= self.solid(next)?.last {
                self.bsp.solidsegs[start].last = self.solid(next)?.last;
                crunch = true;
                break;
            }
        }
        if !crunch {
            // There is a fragment after *next (r_bsp.c:163-166).
            let a = self.solid(next)?;
            self.store_wall_range(a.last.wrapping_add(1), last)?;
            self.bsp.solidsegs[start].last = last;
        }
        // crunch (r_bsp.c:170-184).
        if next == start {
            return Ok(());
        }
        // `while (next++ != newend) *++start = *next;`: the last iteration reads
        // solidsegs[newend], past the array when newend is MAXSEGS.
        loop {
            let was = next;
            next += 1;
            if was == self.bsp.newend {
                break;
            }
            let v = self.solid(next)?;
            start += 1;
            self.bsp.solidsegs[start] = v;
        }
        self.bsp.newend = start + 1;
        Ok(())
    }

    /// `R_ClipPassWallSegment` (r_bsp.c:196-238).
    fn clip_pass_wall_segment(&mut self, first: i32, last: i32) -> Result<(), RenderRefusal> {
        let mut start = 0;
        while self.solid(start)?.last < first.wrapping_sub(1) {
            start += 1;
        }
        let s = self.solid(start)?;
        if first < s.first {
            if last < s.first.wrapping_sub(1) {
                // Post is entirely visible (r_bsp.c:211-216).
                return self.store_wall_range(first, last);
            }
            // There is a fragment above *start (r_bsp.c:218-219).
            self.store_wall_range(first, s.first.wrapping_sub(1))?;
        }
        if last <= self.solid(start)?.last {
            return Ok(());
        }
        while last >= self.solid(start + 1)?.first.wrapping_sub(1) {
            let (a, b) = (self.solid(start)?, self.solid(start + 1)?);
            self.store_wall_range(a.last.wrapping_add(1), b.first.wrapping_sub(1))?;
            start += 1;
            if last <= self.solid(start)?.last {
                return Ok(());
            }
        }
        let a = self.solid(start)?;
        self.store_wall_range(a.last.wrapping_add(1), last)
    }

    /// Records the genuine runner's `newend - solidsegs` peak (taken at every
    /// `R_StoreWallRange` call and at the frame end).
    pub(crate) fn note_solid_peak(&mut self) {
        self.bsp.solid_peak = self.bsp.solid_peak.max(self.bsp.newend);
    }

    /// `viewangletox[(angle + ANG90) >> ANGLETOFINESHIFT]`.
    fn angle_to_x(&self, angle: u32) -> Result<i32, RenderRefusal> {
        self.view
            .viewangletox
            .get((angle.wrapping_add(ANG90) >> ANGLETOFINESHIFT) as usize)
            .copied()
            .ok_or(RenderRefusal::TableRead("viewangletox"))
    }

    /// `R_AddLine` (r_bsp.c:259-356).
    fn add_line(&mut self, line: usize) -> Result<(), RenderRefusal> {
        let level = self.level;
        let seg = &level.geometry.segs[line];
        let f = self.frame;
        self.bsp.curline = line;
        let mut angle1 = point_to_angle(seg.v1[0], seg.v1[1], f.viewx, f.viewy);
        let mut angle2 = point_to_angle(seg.v2[0], seg.v2[1], f.viewx, f.viewy);
        // Clip to view edges (r_bsp.c:276-280): back side.
        let span = angle1.wrapping_sub(angle2);
        if span >= ANG180 {
            return Ok(());
        }
        // Global angle needed by segcalc (r_bsp.c:283).
        self.segs.rw_angle1 = angle1 as i32;
        angle1 = angle1.wrapping_sub(f.viewangle);
        angle2 = angle2.wrapping_sub(f.viewangle);
        let clipangle = self.view.clipangle;
        let mut tspan = angle1.wrapping_add(clipangle);
        if tspan > clipangle.wrapping_mul(2) {
            tspan = tspan.wrapping_sub(clipangle.wrapping_mul(2));
            if tspan >= span {
                return Ok(());
            }
            angle1 = clipangle;
        }
        tspan = clipangle.wrapping_sub(angle2);
        if tspan > clipangle.wrapping_mul(2) {
            tspan = tspan.wrapping_sub(clipangle.wrapping_mul(2));
            if tspan >= span {
                return Ok(());
            }
            angle2 = clipangle.wrapping_neg();
        }
        // r_bsp.c:311-318
        let x1 = self.angle_to_x(angle1)?;
        let x2 = self.angle_to_x(angle2)?;
        if x1 == x2 {
            return Ok(());
        }
        self.bsp.backsector = seg.backsector;
        let front = self.sectors[self.bsp.frontsector];
        let solid = match seg.backsector {
            // Single sided line (r_bsp.c:323-324).
            None => true,
            Some(b) => {
                let back = self.sectors[b];
                if back.ceilingheight <= front.floorheight
                    || back.floorheight >= front.ceilingheight
                {
                    // Closed door (r_bsp.c:327-329).
                    true
                } else if back.ceilingheight != front.ceilingheight
                    || back.floorheight != front.floorheight
                {
                    // Window (r_bsp.c:332-334).
                    false
                } else if back.ceilingpic == front.ceilingpic
                    && back.floorpic == front.floorpic
                    && back.lightlevel == front.lightlevel
                    && self.sides[seg.sidedef][1] == 0
                {
                    // Reject empty lines used for triggers (r_bsp.c:341-347).
                    return Ok(());
                } else {
                    false
                }
            }
        };
        if solid {
            self.clip_solid_wall_segment(x1, x2.wrapping_sub(1))
        } else {
            self.clip_pass_wall_segment(x1, x2.wrapping_sub(1))
        }
    }

    /// `R_CheckBBox` (r_bsp.c:381-487).
    fn check_bbox(&self, bspcoord: [i32; 4]) -> Result<bool, RenderRefusal> {
        let f = self.frame;
        // BOXTOP 0, BOXBOTTOM 1, BOXLEFT 2, BOXRIGHT 3 (m_bbox.h).
        let boxx = if f.viewx <= bspcoord[2] {
            0
        } else if f.viewx < bspcoord[3] {
            1
        } else {
            2
        };
        let boxy = if f.viewy >= bspcoord[0] {
            0
        } else if f.viewy > bspcoord[1] {
            1
        } else {
            2
        };
        let boxpos = (boxy << 2) + boxx;
        if boxpos == 5 {
            return Ok(true);
        }
        let cc = CHECKCOORD[boxpos];
        let (x1, y1, x2, y2) = (
            bspcoord[cc[0]],
            bspcoord[cc[1]],
            bspcoord[cc[2]],
            bspcoord[cc[3]],
        );
        // Check clip list for an open space (r_bsp.c:428-459).
        let mut angle1 = point_to_angle(x1, y1, f.viewx, f.viewy).wrapping_sub(f.viewangle);
        let mut angle2 = point_to_angle(x2, y2, f.viewx, f.viewy).wrapping_sub(f.viewangle);
        let span = angle1.wrapping_sub(angle2);
        // Sitting on a line?
        if span >= ANG180 {
            return Ok(true);
        }
        let clipangle = self.view.clipangle;
        let mut tspan = angle1.wrapping_add(clipangle);
        if tspan > clipangle.wrapping_mul(2) {
            tspan = tspan.wrapping_sub(clipangle.wrapping_mul(2));
            if tspan >= span {
                return Ok(false);
            }
            angle1 = clipangle;
        }
        tspan = clipangle.wrapping_sub(angle2);
        if tspan > clipangle.wrapping_mul(2) {
            tspan = tspan.wrapping_sub(clipangle.wrapping_mul(2));
            if tspan >= span {
                return Ok(false);
            }
            angle2 = clipangle.wrapping_neg();
        }
        // r_bsp.c:465-486
        let sx1 = self.angle_to_x(angle1)?;
        let mut sx2 = self.angle_to_x(angle2)?;
        if sx1 == sx2 {
            return Ok(false);
        }
        sx2 = sx2.wrapping_sub(1);
        let mut start = 0;
        while self.solid(start)?.last < sx2 {
            start += 1;
        }
        let s = self.solid(start)?;
        Ok(!(sx1 >= s.first && sx2 <= s.last))
    }

    /// `R_Subsector` (r_bsp.c:497-542).
    fn subsector(&mut self, num: usize) -> Result<(), RenderRefusal> {
        let level = self.level;
        let sub = *level
            .geometry
            .subsectors
            .get(num)
            .ok_or(RenderRefusal::RangeCheck("R_Subsector"))?;
        self.bsp.frontsector = sub.sector;
        let front = self.sectors[sub.sector];
        let viewz = self.frame.viewz;
        // r_bsp.c:516-523
        self.planes.floorplane = if front.floorheight < viewz {
            Some(self.find_plane(front.floorheight, front.floorpic, front.lightlevel)?)
        } else {
            None
        };
        // r_bsp.c:525-533
        self.planes.ceilingplane =
            if front.ceilingheight > viewz || self.res.skyflatnum == Some(front.ceilingpic) {
                Some(self.find_plane(front.ceilingheight, front.ceilingpic, front.lightlevel)?)
            } else {
                None
            };
        self.add_sprites(sub.sector)?;
        for line in sub.firstline..sub.firstline + sub.numlines {
            if line >= level.geometry.segs.len() {
                return Err(RenderRefusal::RangeCheck("R_Subsector seg"));
            }
            self.add_line(line)?;
        }
        Ok(())
    }

    /// `R_RenderBSPNode (numnodes-1)` (r_bsp.c:552-578). The genuine recursion (front child,
    /// then `R_CheckBBox` of the back box after the front subtree, then the back child) runs
    /// on an explicit stack in the same order.
    pub(crate) fn render_bsp(&mut self) -> Result<(), RenderRefusal> {
        enum Work {
            Visit(i32),
            Back(usize, usize),
        }
        let level = self.level;
        let nodes = &level.geometry.nodes;
        let mut stack = vec![Work::Visit(nodes.len() as i32 - 1)];
        while let Some(w) = stack.pop() {
            match w {
                Work::Visit(bspnum) => {
                    if bspnum == -1 {
                        // `bspnum & NF_SUBSECTOR` with `bspnum == -1` (r_bsp.c:558-561).
                        self.subsector(0)?;
                        continue;
                    }
                    let raw = bspnum as u16;
                    if raw & NF_SUBSECTOR != 0 {
                        self.subsector(usize::from(raw & !NF_SUBSECTOR))?;
                        continue;
                    }
                    let n = nodes
                        .get(usize::from(raw))
                        .ok_or(RenderRefusal::RangeCheck("R_RenderBSPNode"))?;
                    let side = point_on_side(self.frame.viewx, self.frame.viewy, n.partition);
                    stack.push(Work::Back(usize::from(raw), side ^ 1));
                    stack.push(Work::Visit(i32::from(n.children[side])));
                }
                Work::Back(node, back) => {
                    let n = &nodes[node];
                    if self.check_bbox(n.bbox[back])? {
                        stack.push(Work::Visit(i32::from(n.children[back])));
                    }
                }
            }
        }
        self.note_solid_peak();
        Ok(())
    }
}
