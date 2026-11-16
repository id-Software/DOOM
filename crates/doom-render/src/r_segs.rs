//! r_segs.c: `R_StoreWallRange`, `R_RenderSegLoop`, `R_RenderMaskedSegRange`, drawsegs and
//! openings (step 2b).

/// `MAXDRAWSEGS` (r_defs.h): further `R_StoreWallRange` calls are dropped silently
/// (r_segs.c:386-387), as the genuine engine does (decision R3).
pub const MAXDRAWSEGS: usize = 256;
