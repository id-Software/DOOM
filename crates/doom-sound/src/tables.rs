//! The mixer's two lookup tables, built by `I_SetChannels` (i_sound.c:396-424), with no floating
//! point at run time (M5c note §4.6.2).
//!
//! - `steptable` is the only table the C builds with floating point:
//!   `steptablemid[i] = (int)(pow(2.0, (i/64.0))*65536.0)` for i in -128..127 with
//!   `steptablemid = steptable + 128` (i_sound.c:404, 414-415). It is committed as the constant
//!   [`STEPTABLE`], generated exactly (integer 64th root) by `tools/reference/m5c_sounds_table.py`
//!   and checked against the genuine table's SHA-256 (M5c note §3.2.6; the nearest entry is 0.0125
//!   from an integer, so no libm can disagree).
//! - `vol_lookup[i*256+j] = (i*(j-128)*256)/127` for i 0..127, j 0..255 (i_sound.c:421-423) is
//!   integer arithmetic; Rust's `/` truncates toward zero as gcc does (M5c note §1.3.8 item 5).
//!   [`VOL_LOOKUP`] is evaluated at compile time.

include!("steptable.rs");

/// `vol_lookup[128*256]` (i_sound.c:147) as rows: `VOL_LOOKUP[v][s]` is
/// `vol_lookup[v*256 + s]`, the signed contribution of unsigned sample `s` at volume row `v`
/// (i_sound.c:421-423, read at 596-597 through `channelleftvol_lookup` and
/// `channelrightvol_lookup`, which point at row `leftvol`/`rightvol`, i_sound.c:372-373).
pub static VOL_LOOKUP: [[i32; 256]; 128] = vol_lookup();

const fn vol_lookup() -> [[i32; 256]; 128] {
    let mut t = [[0i32; 256]; 128];
    let mut i = 0;
    while i < 128 {
        let mut j = 0;
        while j < 256 {
            // (i*(j-128)*256)/127, i_sound.c:423: at most 4,161,536 in magnitude, no overflow.
            t[i][j] = (i as i32 * (j as i32 - 128) * 256) / 127;
            j += 1;
        }
        i += 1;
    }
    t
}

/// The two tables as the genuine harness digests them: `steptable` then `vol_lookup`, each as
/// little-endian `int32` (tests/reference/m5c_mixer goldens, `T` rows).
pub fn table_bytes() -> (Vec<u8>, Vec<u8>) {
    let step = STEPTABLE.iter().flat_map(|v| v.to_le_bytes()).collect();
    let vol = VOL_LOOKUP
        .iter()
        .flat_map(|row| row.iter().flat_map(|v| v.to_le_bytes()))
        .collect();
    (step, vol)
}
