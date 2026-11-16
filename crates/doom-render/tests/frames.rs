//! M5a: the pixel comparison harness. Every item (the 23 M5a rooms and the 22 M4b synthetic
//! scripts) runs through the simulation once, tic by tic; after every committed `advance`
//! three renderers (screenblocks 11, 10, 9) observe the tic and render, and every frame is
//! compared with the genuine per-frame SHA-256 of `tests/reference/m5a_frames/` (decisions R2,
//! R8). Frames the genuine read-provenance tracker flagged are compared on the digest with the
//! flagged columns zeroed (the masked digest the genuine runner wrote; owner ruling on R4).
//!
//! Step 2b draws the world (BSP, walls, planes, sky, lighting, masked midtextures) but not the
//! sprites and weapon sprites (step 2c), so a frame can match its digest only where nothing of
//! step 2c is on screen. What step 2b asserts on every frame:
//!
//! - the frame's renderer counters equal the genuine F row's (mapped lines, visplanes,
//!   drawsegs, vissprites, openings, the solid-segment peak, dropped drawsegs, the vissprite
//!   overflow) and its newly mapped lines equal the M row (decision R5);
//! - no frame is refused, and a frame with no pending step-2c column matches its digest;
//! - where the genuine run ends at a renderer limit (decision R3), the Rust renderer refuses
//!   the next tic with the matching witness: VPLANES (`R_FindPlane`'s I_Error), OPENINGS (the
//!   first write past `openings[]`) and SOLIDSEGS (the first access past `solidsegs[]`, a read
//!   one frame before the genuine write at screenblocks 9).
//!
//! The world pixels of frames with pending columns are compared locally against the genuine
//! raw frames by the `m5a_compare` example (docs/TESTING.md).
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
        for (sb, c) in harness(&item, configs, false, |s| {
            let pending = s
                .result
                .as_ref()
                .is_ok_and(|r| !r.pending_columns.is_empty());
            if matches!(s.outcome, Outcome::Mismatch) && !pending {
                failures.push(format!(
                    "{} sb{} tic {}: mismatch without pending columns",
                    s.item.lump, s.screenblocks, s.tic
                ));
            }
            if let Outcome::Refused(e) = s.outcome {
                failures.push(format!(
                    "{} sb{} tic {}: refused {e:?}",
                    s.item.lump, s.screenblocks, s.tic
                ));
            }
        }) {
            assert_eq!(c.not_reached, 0, "{} sb{sb}: frames not reached", item.lump);
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
            total.pending_frames += c.pending_frames;
        }
    }
    eprintln!("harness: {total:?}");
    for e in &ends {
        eprintln!("limit: {e}");
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    // The committed goldens hold 13,191 + 13,191 + 13,203 frames (docs/validation/M5A.md).
    assert_eq!(total.frames, 13_191 + 13_191 + 13_203, "{total:?}");
    assert_eq!(
        total.frames,
        total.matched + total.mismatched + total.non_comparable + total.refused
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
    let refused = Err(RenderRefusal::NotImplemented("test"));
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
