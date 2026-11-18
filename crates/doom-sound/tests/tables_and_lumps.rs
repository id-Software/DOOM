//! The sound table, the mixer's tables and the lump loading, against the C facts they port.
//!
//! - `S_SFX` names against doom-sim's own source sound list (the enumerator order of sounds.h
//!   it already uses for wire 24's events), read from its source file because the list is
//!   crate-private there.
//! - `STEPTABLE` exactly: each entry v satisfies v^64 <= 2^(1024 + p - 128) < (v+1)^64
//!   (i_sound.c:414-415 with the multiplication by 65536 exact), plus the genuine digest.
//! - `VOL_LOOKUP` against the genuine digest (i_sound.c:421-423).
//! - `SoundSet::load` on the edge WAD: padding, the pistol fallback, the linked chaingun's zero
//!   length, short lumps (i_sound.c:185-251, 799-813).

mod common;

use common::{directory, sound_set};
use doom_sound::lumps::{SfxSource, SoundCaps, SoundLoadError, SoundSet};
use doom_sound::sounds::{NUMSFX, S_SFX, SFX_CHGUN, SFX_PISTOL, SfxId};
use doom_sound::tables::{STEPTABLE, VOL_LOOKUP, table_bytes};
use doom_test_evidence::sha256;

#[test]
fn sfx_names_agree_with_doom_sim_source_sounds() {
    let src = include_str!("../../doom-sim/src/source_actors.rs");
    let body = &src[src
        .find("pub(crate) const SOURCE_SOUNDS")
        .expect("SOURCE_SOUNDS")..];
    let body = &body[..body.find("];").expect("end of SOURCE_SOUNDS")];
    let names: Vec<&str> = body
        .lines()
        .filter_map(|l| l.trim().strip_prefix('"'))
        .map(|l| l.trim_end_matches(','))
        .map(|l| l.trim_end_matches('"'))
        .collect();
    assert_eq!(names.len(), NUMSFX);
    for (i, n) in names.iter().enumerate() {
        let want = n
            .strip_prefix("sfx_")
            .expect("sfx_ prefix")
            .to_ascii_lowercase();
        assert_eq!(S_SFX[i].name, want, "sfx {i}");
    }
}

#[test]
fn the_table_has_one_link_and_the_documented_priorities() {
    // sounds.c:204: { "chgun", false, 64, &S_sfx[sfx_pistol], 150, 0, 0 }.
    let linked: Vec<usize> = (0..NUMSFX).filter(|&i| S_SFX[i].link.is_some()).collect();
    assert_eq!(linked, [usize::from(SFX_CHGUN)]);
    let c = &S_SFX[usize::from(SFX_CHGUN)];
    assert_eq!(
        (c.link, c.link_pitch, c.link_volume, c.priority),
        (Some(1), 150, 0, 64)
    );
    // M5c note §1.1.3: 23 singular entries (ids 32, 33, 36-50, 75-80).
    let sing: Vec<usize> = (0..NUMSFX).filter(|&i| S_SFX[i].singularity).collect();
    let want: Vec<usize> = [32, 33].into_iter().chain(36..=50).chain(75..=80).collect();
    assert_eq!(sing, want);
    assert_eq!(SfxId::new(0), None);
    assert_eq!(SfxId::new(109), None);
    assert_eq!(SfxId::new(108).map(SfxId::get), Some(108));
}

/// Little-endian limbs; multiply by a small factor.
fn mul_small(a: &mut Vec<u64>, m: u64) {
    let mut carry = 0u128;
    for limb in a.iter_mut() {
        let v = u128::from(*limb) * u128::from(m) + carry;
        *limb = v as u64;
        carry = v >> 64;
    }
    if carry > 0 {
        a.push(carry as u64);
    }
}
fn pow64(v: u64) -> Vec<u64> {
    let mut a = vec![1u64];
    for _ in 0..64 {
        mul_small(&mut a, v);
    }
    a
}
fn pow2(e: u32) -> Vec<u64> {
    let mut a = vec![0u64; (e / 64) as usize + 1];
    a[(e / 64) as usize] = 1u64 << (e % 64);
    a
}
fn cmp(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    let n = a.len().max(b.len());
    for i in (0..n).rev() {
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return x.cmp(&y);
        }
    }
    std::cmp::Ordering::Equal
}

#[test]
fn steptable_is_the_exact_floor_of_the_genuine_expression() {
    for (p, &v) in STEPTABLE.iter().enumerate() {
        let target = pow2(1024 + p as u32 - 128);
        assert_ne!(
            cmp(&pow64(u64::from(v)), &target),
            std::cmp::Ordering::Greater,
            "p {p}"
        );
        assert_eq!(
            cmp(&pow64(u64::from(v) + 1), &target),
            std::cmp::Ordering::Greater,
            "p {p}"
        );
    }
    assert_eq!(
        (STEPTABLE[0], STEPTABLE[128], STEPTABLE[255]),
        (16384, 65536, 259320)
    );
}

#[test]
fn tables_equal_the_genuine_digests() {
    let (st, vl) = table_bytes();
    // tests/reference/m5c_mixer_results.json `tables`.
    assert_eq!(
        sha256(&st),
        "95f2209c952180e797cf3ef5a836db333d8ca88fc3b1c7893f136272f4e96dde"
    );
    assert_eq!(
        sha256(&vl),
        "f727268ad58a2714746072664e37d5c8425d1c21d83a70eabe9e2098836a0043"
    );
    // M5c note §1.3.2 item 13: row 15 gives -3870 at 0 and 3840 at 255; column 128 is 0.
    assert_eq!((VOL_LOOKUP[15][0], VOL_LOOKUP[15][255]), (-3870, 3840));
    assert!(VOL_LOOKUP.iter().all(|r| r[128] == 0));
    assert_eq!((VOL_LOOKUP[127][0], VOL_LOOKUP[127][255]), (-32768, 32512));
}

#[test]
fn edge_lumps_load_as_getsfx_pads_them() {
    let set = sound_set("m5c_sfx_edge.wad");
    let s = |n: i64| set.sfx(SfxId::new(n).unwrap());
    // Lumps of 0, 5 and 8 bytes (dbopn 5, dbcls 6, dbload 7): padded length 0, no byte.
    for (n, size) in [(5, 0), (6, 5), (7, 8)] {
        assert_eq!(
            (s(n).len, s(n).lump_size, s(n).has_first_byte()),
            (0, size, false),
            "sfx {n}"
        );
    }
    // 1 sample (plasma 8) pads to 512, all padding 128 (i_sound.c:240-241).
    assert_eq!(s(8).len, 512);
    assert!(s(8).data[1..].iter().all(|b| *b == 128));
    // pstart (18) is missing: a copy of dspistol (i_sound.c:213-214), and no lump number.
    assert_eq!(s(18).source, SfxSource::Fallback);
    assert_eq!(s(18).data, s(1).data);
    assert_eq!(set.lump_num(SfxId::new(18).unwrap()), None);
    // chgun: the pistol's data and length 0 (i_sound.c:810-811), though dschgun exists.
    let chgun = set.sfx(SfxId::new(i64::from(SFX_CHGUN)).unwrap());
    assert_eq!((chgun.len, chgun.source), (0, SfxSource::Linked));
    assert_eq!(
        chgun.data,
        set.sfx(SfxId::new(i64::from(SFX_PISTOL)).unwrap()).data
    );
    assert!(chgun.has_first_byte());
    assert!(
        set.lump_num(SfxId::new(i64::from(SFX_CHGUN)).unwrap())
            .is_some()
    );
}

#[test]
fn load_refuses_a_missing_pistol_and_oversized_lumps() {
    let dir = doom_render::LumpDirectory::new();
    assert_eq!(
        SoundSet::load(&dir, SoundCaps::default()).unwrap_err(),
        SoundLoadError::PistolMissing { sfx: 1 }
    );
    let dir = directory(&["m5c_sfx.wad"]);
    let caps = SoundCaps {
        max_lump: 600,
        max_total: 1 << 24,
    };
    assert!(matches!(
        SoundSet::load(&dir, caps),
        Err(SoundLoadError::TooLarge { sfx: 2, .. })
    ));
    let caps = SoundCaps {
        max_lump: 1 << 20,
        max_total: 2048,
    };
    assert!(matches!(
        SoundSet::load(&dir, caps),
        Err(SoundLoadError::TooLarge { .. })
    ));
}
