#![no_main]
//! Arbitrary sound lumps into `doom_sound::lumps::SoundSet::load` (M5c).
//!
//! Input: byte 0 the caps (`% 4`: 0 default, 1 `max_lump` 700, 2 `max_total` 3000, 3 both small),
//! byte 1 the record count `n` (at most 128), then `n` four-byte records `sfx: u8` (`% 109`, so 0 is
//! a lump named `dspistol` and ids 1..=108 take their S_sfx name), `kind: u8` (`% 4`: 0 the lump is
//! present, 1 it is left out, 2 it is present with `len % 17` bytes, 3 it is present twice, the
//! second copy one byte shorter), `len: u16 LE`; the rest of the input is the content, handed out
//! cyclically to the lumps in record order (a lump's first bytes are its 8-byte header). The lumps
//! form a PWAD that goes through `doom_wad::Wad::parse` and `LumpDirectory::add_wad`. At most 128
//! lumps of at most 65,535 bytes; `-max_len=4096`.
//!
//! Invariants asserted against an independent model of `I_InitSound`'s load loop (i_sound.c:796-813)
//! and `getsfx` (i_sound.c:185-251): the result is `Ok` or a typed `SoundLoadError`, decided in
//! sound order 1..=108 exactly as the model decides it (a linked entry takes no lump, i_sound.c:
//! 808-811; a missing `ds` lump falls back to `dspistol`, i_sound.c:213-214, and `PistolMissing`
//! only when that is missing too; `TooLarge` when a lump exceeds `max_lump` or the running total of
//! padded bytes exceeds `max_total`). On `Ok`, for every unlinked sound: `len = ((size-8+511)/512)*512`
//! in C `int` arithmetic (i_sound.c:230; 0 for a lump of 8 bytes or fewer), `data.len() == len`,
//! the data are the lump's bytes after the 8-byte header then 128 up to `len` (i_sound.c:239-241),
//! `lump_size` is the size read, the source is `Own` or `Fallback`, and `has_first_byte` is
//! `len > 0`; for the linked sound: length 0, the target's data, source `Linked`; `lump_num` is
//! `Some` exactly when a `ds` lump of that name is in the directory and names the last such lump
//! (w_wad.c backward scan); the total of padded bytes is within `max_total`.
use doom_render::LumpDirectory;
use doom_sound::lumps::{SfxSource, SoundCaps, SoundLoadError, SoundSet};
use doom_sound::sounds::{NUMSFX, S_SFX, SfxId};
use libfuzzer_sys::fuzz_target;

const MAX_RECORDS: usize = 128;

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

fn lump_name(sfx: usize) -> [u8; 8] {
    let mut n = [0u8; 8];
    n[..2].copy_from_slice(b"DS");
    for (o, b) in n[2..].iter_mut().zip(S_SFX[sfx].name.bytes()) {
        *o = b.to_ascii_uppercase();
    }
    n
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let caps = {
        let d = SoundCaps::default();
        match data[0] % 4 {
            0 => d,
            1 => SoundCaps { max_lump: 700, ..d },
            2 => SoundCaps {
                max_total: 3000,
                ..d
            },
            _ => SoundCaps {
                max_lump: 700,
                max_total: 3000,
            },
        }
    };
    let n = usize::from(data[1]).min(MAX_RECORDS);
    let records_end = (2 + 4 * n).min(data.len());
    let tail = &data[records_end..];
    let mut off = 0usize;
    let mut take = |len: usize| -> Vec<u8> {
        let v = (0..len)
            .map(|i| {
                if tail.is_empty() {
                    (off + i) as u8
                } else {
                    tail[(off + i) % tail.len()]
                }
            })
            .collect();
        off += len;
        v
    };
    let mut lumps: Vec<([u8; 8], Vec<u8>)> = Vec::new();
    for r in data[2..records_end].chunks_exact(4) {
        let sfx = usize::from(r[0]) % NUMSFX;
        let len = usize::from(u16::from_le_bytes([r[2], r[3]]));
        let name = if sfx == 0 {
            lump_name(1)
        } else {
            lump_name(sfx)
        };
        match r[1] % 4 {
            0 => lumps.push((name, take(len))),
            1 => {}
            2 => lumps.push((name, take(len % 17))),
            _ => {
                lumps.push((name, take(len)));
                lumps.push((name, take(len.saturating_sub(1))));
            }
        }
    }
    let bytes = pwad(&lumps);
    let Ok(wad) = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()) else {
        return;
    };
    let mut dir = LumpDirectory::new();
    if dir.add_wad(&wad).is_err() {
        return;
    }
    let result = SoundSet::load(&dir, caps);

    // The model: the last lump of a name wins (w_wad.c backward scan).
    let find = |name: &[u8; 8]| lumps.iter().rposition(|(n, _)| n == name);
    let mut expect: Result<(), SoundLoadError> = Ok(());
    let mut total = 0usize;
    for i in 1..NUMSFX {
        if S_SFX[i].link.is_some() {
            continue;
        }
        let lump = match find(&lump_name(i)).or_else(|| find(&lump_name(1))) {
            Some(l) => l,
            None => {
                expect = Err(SoundLoadError::PistolMissing { sfx: i as u16 });
                break;
            }
        };
        let size = lumps[lump].1.len();
        if size > caps.max_lump {
            expect = Err(SoundLoadError::TooLarge {
                sfx: i as u16,
                size,
            });
            break;
        }
        // i_sound.c:230 in C int arithmetic (size >= 0, size-8+511 >= 503, so no negative result).
        let padded = ((size as i64 - 8 + 511) / 512 * 512) as usize;
        total += padded;
        if total > caps.max_total {
            expect = Err(SoundLoadError::TooLarge {
                sfx: i as u16,
                size,
            });
            break;
        }
    }
    let set = match (&result, &expect) {
        (Err(a), Err(b)) => {
            assert_eq!(a, b, "typed error differs from the model");
            return;
        }
        (Ok(s), Ok(())) => s,
        (a, b) => panic!("load {a:?} but model {b:?}"),
    };
    let mut grand = 0usize;
    for i in 1..NUMSFX {
        let id = SfxId::new(i as i64).unwrap();
        let d = set.sfx(id);
        let own = find(&lump_name(i));
        assert_eq!(
            set.lump_num(id).is_some(),
            own.is_some(),
            "lump_num presence, sfx {i}"
        );
        if let (Some(num), Some(own)) = (set.lump_num(id), own) {
            // The directory is the PWAD's lumps in order, so the lump numbers are the indices.
            assert_eq!(num, own, "lump_num is the last lump of the name, sfx {i}");
        }
        if let Some(t) = S_SFX[i].link {
            assert_eq!(d.source, SfxSource::Linked);
            assert_eq!(d.len, 0);
            assert_eq!(d.lump_size, 0);
            assert_eq!(
                &d.data[..],
                &set.sfx(SfxId::new(i64::from(t)).unwrap()).data[..]
            );
            continue;
        }
        let (lump, source) = match own {
            Some(l) => (l, SfxSource::Own),
            None => (find(&lump_name(1)).unwrap(), SfxSource::Fallback),
        };
        let bytes = &lumps[lump].1;
        let padded = ((bytes.len() as i64 - 8 + 511) / 512 * 512) as usize;
        assert_eq!(d.source, source, "source, sfx {i}");
        assert_eq!(d.lump_size, bytes.len());
        assert_eq!(d.len as usize, padded, "padded length, sfx {i}");
        assert_eq!(d.len % 512, 0);
        assert_eq!(d.data.len(), padded);
        assert_eq!(d.has_first_byte(), padded > 0);
        let body = bytes.get(8..).unwrap_or(&[]);
        assert_eq!(
            &d.data[..body.len().min(padded)],
            &body[..body.len().min(padded)]
        );
        assert!(d.data[body.len().min(padded)..].iter().all(|&b| b == 128));
        grand += padded;
    }
    assert!(grand <= caps.max_total);
});
