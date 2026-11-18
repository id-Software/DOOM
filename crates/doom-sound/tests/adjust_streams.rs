//! `S_AdjustSoundParams` (s_sound.c:752-818) as a pure function against every positional S row of
//! the 64 committed genuine sound-logic streams (tests/reference/m5c_sound/*.sound, schema
//! DOOM-M5C-SOUND-v1, M5c note §3.1.4), written by the genuine linuxdoom-1.10 sound logic built
//! with its in-process sound code (one define removed).
//!
//! A positional row is an S row whose origin is neither NULL nor the listener (s_sound.c:304), the
//! rows carrying ox, oy. From ox, oy, lx, ly, langle and the map number of the stream's `#level`
//! line, at `snd_SfxVolume` 8 (the stream's `#config`):
//!
//! - `inaudible`: the function returns 0, and the row's vol, sep are the probe's: 8 and 128 when
//!   clipped (the runner's initial values, `*vol` and `*sep` unwritten, s_sound.c:773-777), else
//!   the written values (tools/reference/m5c_driver.c, `__wrap_S_StartSound`);
//! - `started`: audible, vol equal, sep equal (or 128 when samexy, s_sound.c:312-316);
//! - `no_channel`: audible;
//! - the samexy column equals (ox, oy) == (lx, ly).

mod common;

use common::reference;
use doom_sound::logic::{Adjusted, adjust_sound_params};

fn streams() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = std::fs::read_dir(reference("m5c_sound"))
        .expect("m5c_sound")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "sound"))
        .map(|p| {
            let name = p.file_stem().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&p).expect("stream"))
        })
        .collect();
    v.sort();
    v
}

fn map_of(text: &str) -> i32 {
    let l = text
        .lines()
        .find(|l| l.starts_with("#level,"))
        .expect("#level");
    let f: Vec<&str> = l.split(',').collect();
    assert_eq!(f[3], "map");
    f[4].parse().expect("map")
}

#[test]
fn every_positional_start_row_is_reproduced() {
    let all = streams();
    assert_eq!(all.len(), 64, "streams");
    let (mut rows, mut inaudible, mut clipped, mut map8) = (0, 0, 0, 0);
    for (name, text) in &all {
        assert!(
            text.contains("#config,schedule,tic,sound,on,handle,integrated,wipe,genuine,sfx_volume,8,channels,3,"),
            "{name}: configuration"
        );
        let map = map_of(text);
        for line in text.lines().filter(|l| l.starts_with("S,")) {
            let f: Vec<&str> = line.split(',').collect();
            assert_eq!(f.len(), 22, "{name}: {line}");
            if f[17] == "-" {
                continue;
            }
            let n = |i: usize| -> i32 { f[i].parse().unwrap_or_else(|_| panic!("{name}: {line}")) };
            let (o, l) = ([n(17), n(18)], [n(19), n(20)]);
            let angle: u32 = f[21].parse().expect("langle");
            let a = adjust_sound_params(l, angle, o, map, 8).unwrap();
            let samexy = o == l;
            assert_eq!(f[15] == "1", samexy, "{name}: samexy in {line}");
            match f[6] {
                "inaudible" => {
                    assert!(!a.audible(), "{name}: {line}");
                    let (v, s) = match a {
                        Adjusted::Clipped => {
                            clipped += 1;
                            (8, 128)
                        }
                        Adjusted::Computed { vol, sep } => (vol, sep),
                    };
                    assert_eq!((n(9), n(10)), (v, s), "{name}: {line}");
                    inaudible += 1;
                }
                "started" => {
                    let Adjusted::Computed { vol, sep } = a else {
                        panic!("{name}: clipped {line}")
                    };
                    assert!(vol > 0, "{name}: {line}");
                    assert_eq!(
                        (n(9), n(10)),
                        (vol, if samexy { 128 } else { sep }),
                        "{name}: {line}"
                    );
                }
                "no_channel" => assert!(a.audible(), "{name}: {line}"),
                other => panic!("{name}: outcome {other} in {line}"),
            }
            rows += 1;
            map8 += usize::from(map == 8);
        }
    }
    eprintln!(
        "S_AdjustSoundParams: {rows} positional S rows in 64 streams reproduced \
         ({inaudible} inaudible, {clipped} of them clipped; {map8} on map 8)"
    );
    assert_eq!(rows, 404, "positional S rows");
}
