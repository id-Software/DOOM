//! r_bsp.c: the solid-segment clip list, `R_AddLine`, `R_CheckBBox`, `R_Subsector` and
//! `R_RenderBSPNode`. Step 2b implements them; until then the traversal refuses the frame
//! with a typed "not yet implemented" refusal (never a silent no-op).
use crate::RenderRefusal;

/// `MAXSEGS` (r_bsp.c:88): solid-segment list capacity; the genuine list writes past it
/// unchecked, so the first access past it is the `SolidSegOverflow` refusal (decision R3,
/// accepted by the owner 2026-11-15 at the first out-of-bounds read).
pub const MAXSEGS: usize = 32;

/// `R_RenderBSPNode (numnodes-1)` (r_bsp.c:540-570).
pub(crate) fn render_bsp_node(_frame: &mut crate::FrameContext<'_>) -> Result<(), RenderRefusal> {
    Err(RenderRefusal::NotImplemented("R_RenderBSPNode"))
}
