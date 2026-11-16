#![no_main]
//! Arbitrary bytes into the renderer's data parsers (`doom_render::r_data`, `RenderState::decode`)
//! without any simulation: PNAMES, TEXTURE1/TEXTURE2, patch and sprite lumps, COLORMAP, PLAYPAL,
//! flats and markers, the switch and animation tables and the sprite definitions
//! (`Resources::load`), the column and masked-post accessors, and the `DRST` decoder.
//!
//! The base is the committed synthetic graphics WAD (`tests/reference/m5a_gfx.wad`, no id art);
//! the input edits its lumps in memory and the edited WAD goes through `doom_wad::Wad::parse`,
//! `LumpDirectory::add_wad` and `Resources::load`, so the parsers see structured but hostile
//! lumps instead of random noise that the first length check rejects.
//!
//! Input: byte 0 the game mode (`% 3`), byte 1 the number of edit records, then six-byte edit
//! records `lump: u16 LE` (`% numlumps`), `offset: u16 LE`, `kind: u8`, `value: u8`, then the rest
//! of the input is raw bytes for the standalone parsers. Kinds (`kind % 8`): 0 set the byte at
//! `offset` to `value`; 1 truncate the lump to `offset` bytes; 2 delete the lump; 3 write the
//! `i32` `value as i8` at `offset`; 4 write `i32::MAX - value`; 5 write `i32::MIN + value`;
//! 6 write the `u16` `value * 257`; 7 grow the lump to `offset` bytes of `value`. An edit outside
//! the lump is skipped. At most 64 records; `-max_len=2048`.
//!
//! Properties: no panic and no abort on any input; a load that succeeds yields tables every
//! accessor can walk (every column of every texture through `get_column` and `masked_posts`,
//! every flat and sprite lump, every name lookup); an `Err` is a typed `ResourceError`; a
//! `DRST` decode of the raw tail either fails typed or yields a state that re-encodes to the
//! bytes it came from only when it is canonical (not asserted), and never panics; the patch
//! header and column-offset readers never panic. Allocation is bounded by the input (at most
//! 64 edits, 64 KiB growth each) plus the fixed base; the genuine 64 KiB composite limit
//! (`R_GenerateLookup`) bounds every texture, and libFuzzer's RSS limit is the guard.
use doom_render::{GameMode, LumpDirectory, RenderState, Resources, r_data};
use libfuzzer_sys::fuzz_target;

const GFX: &[u8] = include_bytes!("../../tests/reference/m5a_gfx.wad");
const MAX_EDITS: usize = 64;

fn base() -> &'static Vec<([u8; 8], Vec<u8>)> {
    static BASE: std::sync::OnceLock<Vec<([u8; 8], Vec<u8>)>> = std::sync::OnceLock::new();
    BASE.get_or_init(|| {
        let wad = doom_wad::Wad::parse(GFX, doom_wad::Limits::default()).unwrap();
        (0..wad.lumps().len())
            .map(|i| {
                (
                    *wad.lumps()[i].name().as_bytes(),
                    wad.data(i).unwrap().to_vec(),
                )
            })
            .collect()
    })
}

/// A PWAD holding `lumps` in order (header, lump data, directory).
fn pwad(lumps: &[([u8; 8], Vec<u8>)]) -> Vec<u8> {
    let mut out = b"PWAD".to_vec();
    out.extend((lumps.len() as u32).to_le_bytes());
    out.extend(0u32.to_le_bytes());
    let mut dir = Vec::new();
    for (name, data) in lumps {
        dir.extend((out.len() as u32).to_le_bytes());
        dir.extend((data.len() as u32).to_le_bytes());
        dir.extend(name);
        out.extend(data);
    }
    let at = out.len() as u32;
    out[8..12].copy_from_slice(&at.to_le_bytes());
    out.extend(dir);
    out
}

fn walk(res: &Resources) {
    for (tex, t) in res.textures.iter().enumerate() {
        for col in 0..i32::from(t.width) {
            let src = res.get_column(tex, col);
            for i in [0usize, 1, 127] {
                let _ = src.texel(i);
            }
            let _ = res.masked_posts(tex, col);
        }
        let _ = res.texture_num_for_name(&t.name);
    }
    for f in 0..res.numflats {
        let _ = res.flat(f).len();
    }
    for l in 0..res.sprite_lumps.len() {
        let _ = res.lump(res.firstspritelump + l).len();
    }
    for s in &res.sprites {
        for fr in &s.frames {
            let _ = (fr.rotate, fr.lump, fr.flip);
        }
    }
    let _ = (
        res.skytexture,
        res.skyflatnum,
        res.switchlist.len(),
        res.anims.len(),
    );
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let mode =
        [GameMode::Shareware, GameMode::Registered, GameMode::Retail][usize::from(data[0] % 3)];
    let edits = usize::from(data[1]).min(MAX_EDITS);
    let body = &data[2..];
    let split = (edits * 6).min(body.len());
    let (records, tail) = body.split_at(split);
    let mut lumps = base().clone();
    for r in records.as_chunks::<6>().0 {
        if lumps.is_empty() {
            break;
        }
        let n = usize::from(u16::from_le_bytes([r[0], r[1]])) % lumps.len();
        let off = usize::from(u16::from_le_bytes([r[2], r[3]]));
        let (kind, value) = (r[4] % 8, r[5]);
        match kind {
            2 => {
                lumps.remove(n);
            }
            1 => lumps[n].1.truncate(off),
            7 => lumps[n].1.resize(off, value),
            _ => {
                let bytes: Vec<u8> = match kind {
                    0 => vec![value],
                    3 => i32::from(value as i8).to_le_bytes().to_vec(),
                    4 => (i32::MAX - i32::from(value)).to_le_bytes().to_vec(),
                    5 => (i32::MIN + i32::from(value)).to_le_bytes().to_vec(),
                    _ => (u16::from(value) * 257).to_le_bytes().to_vec(),
                };
                if let Some(dst) = lumps[n].1.get_mut(off..off + bytes.len()) {
                    dst.copy_from_slice(&bytes);
                }
            }
        }
    }
    let bytes = pwad(&lumps);
    if let Ok(wad) = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()) {
        let mut dir = LumpDirectory::new();
        if dir.add_wad(&wad).is_ok() {
            if let Ok(res) = Resources::load(dir, mode) {
                walk(&res);
            }
        }
    }
    // The standalone parsers on the raw tail.
    let _ = RenderState::decode(tail);
    let _ = r_data::patch_header(tail);
    for col in [-1, 0, 1, 7, 255, i32::MAX, i32::MIN] {
        let _ = r_data::patch_column_offset(tail, col);
    }
});
