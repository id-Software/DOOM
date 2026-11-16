//! M5a: the pixel comparison harness. Every item (the 20 rooms and the 22 M4b synthetic
//! scripts) runs through the simulation once, tic by tic; after every committed `advance`
//! three renderers (screenblocks 11, 10, 9) observe the tic and render, and every frame is
//! compared with the genuine per-frame SHA-256 of `tests/reference/m5a_frames/` (decisions R2,
//! R8). Frames the genuine read-provenance tracker flagged are compared on the digest with the
//! flagged columns zeroed (the masked digest the genuine runner wrote; owner ruling on R4).
//!
//! Step 2a: the BSP traversal and the drawers are not implemented, so every frame is refused
//! with `RenderRefusal::NotImplemented`; the test checks the harness runs end to end and
//! prints the counts. Steps 2b and 2c turn the counts into assertions.
mod common;

use common::{FrameRow, Outcome, compare, harness, items, masked};
use doom_render::{Frame, FrameReport, RenderRefusal};

#[test]
fn harness_runs_every_item_and_configuration() {
    let mut total = common::Counts::default();
    for item in items() {
        let configs: &[u8] = &[11, 10, 9];
        for (sb, c) in harness(&item, configs, false, |s| {
            if let Outcome::Refused(e) = s.outcome {
                assert!(
                    matches!(e, RenderRefusal::NotImplemented(_)),
                    "{} sb{} tic {}: {e:?}",
                    s.item.lump,
                    s.screenblocks,
                    s.tic
                );
            }
        }) {
            assert_eq!(c.not_reached, 0, "{} sb{sb}: frames not reached", item.lump);
            total.frames += c.frames;
            total.matched += c.matched;
            total.mismatched += c.mismatched;
            total.non_comparable += c.non_comparable;
            total.refused += c.refused;
            total.flag_disagreements += c.flag_disagreements;
        }
    }
    eprintln!("harness: {total:?}");
    // The committed goldens hold 8,180 + 8,180 + 8,192 frames (docs/validation/M5A.md),
    // less the frames past each run's limit witness (none is rendered there).
    assert!(total.frames > 24_000, "{total:?}");
    assert_eq!(
        total.frames,
        total.matched + total.mismatched + total.non_comparable + total.refused
    );
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
