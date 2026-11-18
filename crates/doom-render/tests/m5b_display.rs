//! M5b step 3: the composed display (status bar, 3D view, HUD message, palette choice) against
//! the genuine goldens of `tests/reference/m5b_frames` (DOOM-M5B-FRAMES-v1, written by the C only:
//! `tools/reference/m5b_reference.py`). Frames are identical to the genuine engine under the
//! declared sound-free, wipe-free contract (decision S15), with two exclusions whose counts this
//! test asserts: frames with tracker-flagged columns are compared on the masked digest only
//! (300), and frames whose switch parity the renderer reports unknown are not digest-compared
//! (70).
//!
//! Every item of the M5b reference set (the 26 M5a rooms and 22 M4b scripts with
//! `m5b_gfx.wad`, and the 11 rooms of `m5b_manifest.json`) runs once through the simulation; after
//! every committed `advance` one display per configuration (screenblocks 10 and 11) observes
//! the tic with the command fed and draws. Per frame: the whole-frame digest (the digest with the
//! genuine flagged columns zeroed where the tracker flagged columns, as M5a), the renderer
//! counters, `fuzzpos` and the newly mapped lines of the F and M rows, and the P row's palette,
//! palette set during the frame, face, `message_on` and `M_Random` index; per tic, the message
//! taken against the X rows (decision S16: compared separately from the game dump). Where the
//! genuine run ends at a renderer limit, the next frame must be refused with the matching
//! witness, and a frame the genuine run refuses as a 2D witness (`refused2d`, none so far) must
//! be refused by a 2D drawer. After every tic the display's state must pass the restore checks
//! (`DisplayState::check`). The bespoke scenario M5CHEAT (decision S1) applies its two cheats' effect on the
//! status bar inputs from their tics (cited below), since wire 24 has no cheats.
mod common;
mod m5b;

use doom_render::{Display, DisplayConfig, DisplayRefusal, Frame, RenderRefusal};
use m5b::{Stream, cheats, items, load, read_stream, results};

#[derive(Debug, Default)]
struct Counts {
    frames: usize,
    digests: usize,
    masked: usize,
    non_comparable: usize,
    refused2d: usize,
    fields: usize,
    messages: usize,
}

#[test]
fn every_item_matches_the_genuine_display() {
    let results = results();
    let mut total = Counts::default();
    let mut failures: Vec<String> = Vec::new();
    let mut ends = Vec::new();
    let all = items();
    assert_eq!(all.len(), 59);
    for item in &all {
        let configs = [10u8, 11];
        let streams: Vec<Stream> = configs
            .iter()
            .map(|&sb| read_stream(&item.lump, sb))
            .collect();
        let (res, level, mut sim, commands) = load(item);
        let mut displays: Vec<Display> = configs
            .iter()
            .map(|&screenblocks| {
                let mut d =
                    Display::new(res.clone(), level.clone(), DisplayConfig { screenblocks })
                        .expect("display");
                d.begin_level(&sim).expect("level start");
                d
            })
            .collect();
        let mut frame = Frame::new();
        let mut taken: Vec<Vec<(i64, i32, String)>> = vec![Vec::new(); 2];
        let mut done = [false; 2];
        for (tic, cmd) in commands.iter().enumerate() {
            let tic = tic as i64;
            if done.iter().all(|d| *d)
                || sim
                    .advance([
                        *cmd,
                        Default::default(),
                        Default::default(),
                        Default::default(),
                    ])
                    .is_err()
            {
                break;
            }
            for (k, d) in displays.iter_mut().enumerate() {
                if done[k] {
                    continue;
                }
                let ctx = format!("{} sb{} tic {tic}", item.lump, configs[k]);
                let observed = if item.lump == "M5CHEAT" {
                    d.observe_with(&sim, cmd, |i, m| cheats(tic, i, m))
                } else {
                    d.observe(&sim, cmd)
                };
                match observed {
                    Ok(Some(m)) => taken[k].push((tic, m.code, m.text.into())),
                    Ok(None) => {}
                    Err(e) => {
                        failures.push(format!("{ctx}: observe {e:?}"));
                        done[k] = true;
                        continue;
                    }
                }
                let result = d.draw(&sim, &mut frame);
                let Some(g) = streams[k].frames.get(&tic) else {
                    let end = results["items"][&item.lump]["views"][configs[k].to_string()]["end"]
                        .as_str()
                        .expect("end")
                        .to_string();
                    let want = match end.as_str() {
                        "error:R_FindPlane: no more visplanes" => {
                            Some(RenderRefusal::VisplaneOverflow)
                        }
                        "refused:OpeningOverflow" => Some(RenderRefusal::OpeningOverflow),
                        "refused:SolidSegOverflow" => Some(RenderRefusal::SolidSegOverflow),
                        _ => None,
                    };
                    if want.clone().map(DisplayRefusal::Render) != result.clone().err() {
                        failures.push(format!("{ctx}: genuine end {end}, Rust {result:?}"));
                    }
                    ends.push(format!("{ctx}: {end}"));
                    done[k] = true;
                    continue;
                };
                total.frames += 1;
                if let Err(e) = d.state().expect("state").check(&sim) {
                    failures.push(format!("{ctx}: the restore checks refuse the state: {e:?}"));
                }
                if g.sha256 == "refused2d" {
                    // the genuine frame read or wrote outside an allocation (decision S17)
                    total.refused2d += 1;
                    if !matches!(result, Err(DisplayRefusal::Draw(_))) {
                        failures.push(format!("{ctx}: genuine 2D witness, Rust {result:?}"));
                    }
                    done[k] = true;
                    continue;
                }
                let f = match result {
                    Ok(f) => f,
                    Err(e) => {
                        failures.push(format!("{ctx}: refused {e:?}"));
                        done[k] = true;
                        continue;
                    }
                };
                let px = frame.pixels();
                if g.whole || f.view.switch_parity_unknown {
                    total.non_comparable += 1;
                } else if g.columns.is_empty() {
                    total.digests += 1;
                    if doom_test_evidence::sha256(px) != g.sha256 {
                        failures.push(format!("{ctx}: frame differs"));
                    }
                } else {
                    total.masked += 1;
                    let m = common::masked(px, &g.columns);
                    if doom_test_evidence::sha256(&m) != g.masked
                        || f.view.flagged_columns != g.columns
                    {
                        failures.push(format!("{ctx}: masked frame or flagged columns differ"));
                    }
                }
                if f.view.counters != g.counters
                    || f.view.fuzzpos != g.fuzzpos
                    || f.view.newly_mapped != g.mapped
                {
                    failures.push(format!("{ctx}: counters"));
                }
                let set = f.palette_set.map_or(-1, i64::from);
                let got = [
                    i64::from(f.palette),
                    set,
                    i64::from(f.face),
                    i64::from(f.message_on),
                    i64::from(d.status().rndindex),
                ];
                total.fields += 5;
                if got != g.status {
                    failures.push(format!("{ctx}: P {got:?} genuine {:?}", g.status));
                }
            }
        }
        for (k, &sb) in configs.iter().enumerate() {
            let want = results["items"][&item.lump]["views"][sb.to_string()]["frames"]
                .as_u64()
                .expect("frames") as usize;
            assert_eq!(streams[k].frames.len(), want, "{} sb{sb}", item.lump);
            total.messages += streams[k].messages.len();
            if taken[k] != streams[k].messages {
                failures.push(format!(
                    "{} sb{sb}: messages {:?} genuine {:?}",
                    item.lump, taken[k], streams[k].messages
                ));
            }
        }
    }
    eprintln!("m5b display: {total:?}");
    for e in &ends {
        eprintln!("end: {e}");
    }
    failures.truncate(40);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(total.frames, 2 * 21_176, "{total:?}");
    // what is compared how: a change in what is skipped fails here
    assert_eq!(
        (
            total.digests,
            total.masked,
            total.non_comparable,
            total.refused2d
        ),
        (41_982, 300, 70, 0),
        "{total:?}"
    );
    assert_eq!(ends.len(), 6, "{ends:?}");
}
