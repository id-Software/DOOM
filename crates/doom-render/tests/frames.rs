//! M5a: the pixel comparison harness. Every item (the 23 M5a rooms and the 22 M4b synthetic
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

#[test]
fn harness_runs_every_item_and_configuration() {
    let mut total = Counts::default();
    let mut ends = Vec::new();
    let mut failures = Vec::new();
    for item in items() {
        let configs: &[u8] = &[11, 10, 9];
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
    // The committed goldens hold 13,191 + 13,191 + 13,203 frames (docs/validation/M5A.md).
    assert_eq!(total.frames, 13_191 + 13_191 + 13_203, "{total:?}");
    // Every frame matches except M5SWTCH's parity-unknown frames (35 per configuration).
    assert_eq!(total.non_comparable, 3 * 35, "{total:?}");
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
    // VPLANES, OPENINGS and SOLIDSEGS at three configurations.
    assert_eq!(ends.len(), 9, "{ends:?}");
}

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
