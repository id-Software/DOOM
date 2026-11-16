//! r_plane.c: visplanes, openings, `R_FindPlane`, `R_CheckPlane`, `R_MakeSpans`,
//! `R_MapPlane` and `R_DrawPlanes` (step 2b).

/// `MAXVISPLANES` (r_plane.c:52); exceeding it is the `VisplaneOverflow` refusal (decision R3).
pub const MAXVISPLANES: usize = 128;
/// `MAXOPENINGS` (r_plane.c:59); a write past it is the `OpeningOverflow` refusal (decision R3).
pub const MAXOPENINGS: usize = 320 * 64;
