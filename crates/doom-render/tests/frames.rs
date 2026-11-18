//! M5a: the pixel comparison harness. Every item (the 26 M5a rooms and the 22 M4b synthetic
//! scripts) runs through the simulation once, tic by tic; after every committed `advance`
//! three renderers (screenblocks 11, 10, 9) observe the tic and render, and every frame is
//! compared with the genuine per-frame SHA-256 of `tests/reference/m5a_frames/` (decisions R2,
//! R8). Frames the genuine read-provenance tracker flagged are compared on the digest with the
//! flagged columns zeroed (the masked digest the genuine runner wrote; owner ruling on R4).
//!
//! The renderer is complete for the admitted configurations (step 2c), so every frame must
//! match. What is asserted on every frame of every item and configuration:
//!
//! - the full digest matches (the masked digest where the genuine tracker flagged columns), and
//!   the Rust renderer's flagged columns equal the genuine U rows;
//! - the frame's renderer counters equal the genuine F row's (mapped lines, visplanes,
//!   drawsegs, vissprites, openings, the solid-segment peak, dropped drawsegs, the vissprite
//!   overflow), as do `fuzzpos` after the frame and the newly mapped lines (M row, decision
//!   R5);
//! - no frame is refused, and the only non-comparable frames are M5SWTCH's 35 parity-unknown
//!   frames per configuration (decision R7);
//! - where the genuine run ends at a renderer limit (decision R3), the Rust renderer refuses
//!   the next tic with the matching witness: VPLANES (`R_FindPlane`'s I_Error), OPENINGS (the
//!   first write past `openings[]`) and SOLIDSEGS (the first access past `solidsegs[]`, a read
//!   one frame before the genuine write at screenblocks 9). DSEGS and VSPRITES reproduce the
//!   silent `MAXDRAWSEGS` and `MAXVISSPRITES` drops and match on every frame.
mod common;

use common::{Counts, FrameRow, Outcome, compare, harness, items, masked};
use doom_render::{Frame, FrameReport, RenderRefusal};

/// The work units, split into `PARTS` groups of near-equal work (golden frames, longest first
/// onto the lightest group; ties by position, so the split is deterministic), so the groups run
/// on parallel test threads. A unit is an item with its three configurations (screenblocks 11,
/// 10, 9: the item runs through the simulation once), except an item of more than
/// `SPLIT_FRAMES` golden frames (M5AMPFL alone: 12,690), which is one unit per configuration.
/// Each group is its items with the configurations it owns. Every (item, configuration) is in
/// exactly one group; `PART_COUNTS` and `the_parts_sum_to_the_totals` make a skipped one fail.
fn part_units(part: usize) -> Vec<(common::Item, Vec<u8>)> {
    const SPLIT_FRAMES: usize = 4500;
    let all = items();
    let frames = |k: usize, sbs: &[u8]| -> usize {
        sbs.iter()
            .map(|&sb| common::genuine_frames(&all[k].lump, sb))
            .sum()
    };
    let mut units: Vec<(usize, Vec<u8>)> = Vec::new();
    for k in 0..all.len() {
        if frames(k, &[11, 10, 9]) > SPLIT_FRAMES {
            units.extend([11u8, 10, 9].map(|sb| (k, vec![sb])));
        } else {
            units.push((k, vec![11, 10, 9]));
        }
    }
    let work: Vec<usize> = units.iter().map(|(k, sbs)| frames(*k, sbs)).collect();
    let mut order: Vec<usize> = (0..units.len()).collect();
    order.sort_by_key(|&u| (std::cmp::Reverse(work[u]), u));
    let mut load = [0usize; PARTS];
    let mut owner = vec![0usize; units.len()];
    for u in order {
        let lightest = (0..PARTS).min_by_key(|&p| (load[p], p)).expect("parts");
        load[lightest] += work[u];
        owner[u] = lightest;
    }
    units
        .into_iter()
        .enumerate()
        .filter(|(u, _)| owner[*u] == part)
        .map(|(_, (k, sbs))| (all[k].clone(), sbs))
        .collect()
}

const PARTS: usize = 8;

/// The counts of each part: (frames, not comparable, limit endings). They sum to the totals the
/// original single test asserted: 13,391 + 13,391 + 13,403 = 40,185 frames, 105 (3 x 35)
/// not comparable and 9 limit endings (VPLANES, OPENINGS and SOLIDSEGS at three configurations).
const PART_COUNTS: [(usize, usize, usize); PARTS] = [
    (5001, 0, 3),
    (5007, 0, 3),
    (4992, 0, 0),
    (5052, 0, 0),
    (5040, 0, 0),
    (5073, 0, 0),
    (5010, 105, 0),
    (5010, 0, 3),
];

#[test]
fn the_parts_sum_to_the_totals() {
    let sum = PART_COUNTS
        .iter()
        .fold([0; 3], |a, p| [a[0] + p.0, a[1] + p.1, a[2] + p.2]);
    assert_eq!(sum, [13_391 + 13_391 + 13_403, 3 * 35, 9]);
}

fn run_part(part: usize) {
    let mut total = Counts::default();
    let mut ends = Vec::new();
    let mut failures = Vec::new();
    for (item, configs) in part_units(part) {
        let configs: &[u8] = &configs;
        // Per item and configuration: the golden stream has a row for every tic from 0 and
        // the frame count `m5a_results.json` records, so a gap or a short golden cannot pass.
        let expected: Vec<usize> = configs
            .iter()
            .map(|&sb| {
                let want = common::genuine_frames(&item.lump, sb);
                let stream = common::read_frames(&item.lump, sb);
                assert_eq!(
                    common::stream_gaps(&stream, want),
                    None,
                    "{} sb{sb}",
                    item.lump
                );
                want
            })
            .collect();
        for ((sb, c), want) in harness(&item, configs, false, |s| match &s.outcome {
            Outcome::Match { flags_agree: true } => {}
            Outcome::NonComparable if s.item.lump == "M5SWTCH" => {}
            other => failures.push(format!(
                "{} sb{} tic {}: {other:?}",
                s.item.lump, s.screenblocks, s.tic
            )),
        })
        .into_iter()
        .zip(expected)
        {
            assert_eq!(c.not_reached, 0, "{} sb{sb}: frames not reached", item.lump);
            assert_eq!(
                c.missing, 0,
                "{} sb{sb}: tics without a golden row",
                item.lump
            );
            assert_eq!(c.frames, want, "{} sb{sb}: frames compared", item.lump);
            if c.counter_mismatches != 0 {
                failures.push(format!(
                    "{} sb{sb}: {} frames with differing counters or mapped lines",
                    item.lump, c.counter_mismatches
                ));
            }
            if let Some((tic, end, result)) = &c.end {
                let want = common::expected_refusal(end).expect("a limit ending");
                if result.as_ref().err() != Some(&want) {
                    failures.push(format!(
                        "{} sb{sb} tic {tic}: genuine {end}, Rust {result:?}",
                        item.lump
                    ));
                }
                ends.push(format!(
                    "{} sb{sb} tic {tic}: {end} -> {result:?}",
                    item.lump
                ));
            }
            total.frames += c.frames;
            total.matched += c.matched;
            total.mismatched += c.mismatched;
            total.non_comparable += c.non_comparable;
            total.refused += c.refused;
            total.flag_disagreements += c.flag_disagreements;
            total.counter_mismatches += c.counter_mismatches;
        }
    }
    eprintln!("harness: {total:?}");
    for e in &ends {
        eprintln!("limit: {e}");
    }
    failures.truncate(60);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    // The committed goldens hold 13,391 + 13,391 + 13,403 frames (docs/validation/M5A.md); this
    // part's share is fixed in `PART_COUNTS`, which sums to that.
    let (frames, non_comparable, limit_ends) = PART_COUNTS[part];
    assert_eq!(total.frames, frames, "{total:?}");
    // Every frame matches except M5SWTCH's parity-unknown frames (35 per configuration).
    assert_eq!(total.non_comparable, non_comparable, "{total:?}");
    assert_eq!(
        total.matched,
        total.frames - total.non_comparable,
        "{total:?}"
    );
    assert_eq!(
        (total.mismatched, total.refused, total.flag_disagreements),
        (0, 0, 0),
        "{total:?}"
    );
    assert_eq!(ends.len(), limit_ends, "{ends:?}");
}

macro_rules! parts {
    ($($name:ident: $part:expr),*) => {
        $(#[test] fn $name() { run_part($part); })*
    };
}

parts!(
    harness_part_0: 0,
    harness_part_1: 1,
    harness_part_2: 2,
    harness_part_3: 3,
    harness_part_4: 4,
    harness_part_5: 5,
    harness_part_6: 6,
    harness_part_7: 7
);

/// The comparison can fail: a planted pixel change is a mismatch, a change inside a column
/// the genuine tracker flagged is not, and a refusal is never a match.
#[test]
fn comparison_detects_a_changed_pixel() {
    let zero = Frame::new();
    let ok: Result<FrameReport, RenderRefusal> = Ok(FrameReport::default());
    let full = FrameRow {
        sha256: doom_test_evidence::sha256(zero.pixels()),
        masked_sha256: "-".into(),
        ..FrameRow::default()
    };
    assert_eq!(
        compare(&full, &ok, &zero),
        Outcome::Match { flags_agree: true }
    );
    let mut changed = Frame::new();
    changed.pixels_mut()[77 * 320 + 123] = 1;
    assert_eq!(compare(&full, &ok, &changed), Outcome::Mismatch);
    let flagged = FrameRow {
        sha256: "unused".into(),
        masked_sha256: doom_test_evidence::sha256(&masked(zero.pixels(), &[123])),
        columns: vec![123],
        ..FrameRow::default()
    };
    assert_eq!(
        compare(&flagged, &ok, &changed),
        Outcome::Match { flags_agree: false }
    );
    changed.pixels_mut()[77 * 320 + 124] = 1;
    assert_eq!(compare(&flagged, &ok, &changed), Outcome::Mismatch);
    let refused = Err(RenderRefusal::PlayerTranslation);
    assert!(matches!(
        compare(&full, &refused, &zero),
        Outcome::Refused(_)
    ));
    let whole = FrameRow {
        whole: true,
        ..full
    };
    assert_eq!(compare(&whole, &ok, &changed), Outcome::NonComparable);
}

/// The per-item gap check can fail: a stream missing a tic, or holding fewer rows than the
/// results record, is reported.
#[test]
fn a_gap_in_the_goldens_is_detected() {
    let mut s = common::FrameStream::default();
    for tic in [0, 1, 3] {
        s.frames.insert(tic, FrameRow::default());
    }
    assert_eq!(
        common::stream_gaps(&s, 3),
        Some("no F row for tic 2".into())
    );
    assert_eq!(
        common::stream_gaps(&s, 4),
        Some("3 F rows, expected 4".into())
    );
    s.frames.insert(2, FrameRow::default());
    assert_eq!(common::stream_gaps(&s, 4), None);
}
