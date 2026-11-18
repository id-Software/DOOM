//! Tests written for the three doom-game mutants the first M5c mutation run missed (cargo-mutants
//! 27.1.0 at 216f976; docs/validation/M5C.md "Step 10"). Each names what it pins and the mutant
//! it kills. All go through the public API of `doom_game::app`.

use doom_game::app::{AudioSink, GameData, PresentationRestoreError, RecordingSink};
use doom_render::GameMode;
use doom_sound::mixer::Buffer;
use doom_sound::state::StateError;
use std::path::{Path, PathBuf};

fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name)
}

/// `I_SubmitSound` writes each buffer as 512 frames of two little-endian signed 16-bit samples,
/// left first (i_sound.c:665-670, the device format `AFMT_S16_LE`, i_sound.c:788-789): the
/// recording sink's stream is the buffers' bytes concatenated in order, 2,048 bytes each.
/// Kills app/audio.rs `RecordingSink::stream_bytes` -> `vec![1]`.
#[test]
fn the_recording_sinks_stream_is_the_buffers_as_little_endian_pairs_in_order() {
    let mut first: Buffer = [[0; 2]; 512];
    first[0] = [1, -2];
    first[1] = [0x1234, -0x1234];
    first[511] = [i16::MIN, i16::MAX];
    let mut second: Buffer = [[0; 2]; 512];
    second[0] = [i16::MAX, i16::MIN];
    let mut sink = RecordingSink::default();
    assert!(sink.stream_bytes().is_empty());
    sink.buffer(&first);
    sink.buffer(&second);
    let bytes = sink.stream_bytes();
    assert_eq!(bytes.len(), 2 * 2048);
    assert_eq!(bytes[..4], [0x01, 0x00, 0xfe, 0xff]);
    assert_eq!(bytes[4..8], [0x34, 0x12, 0xcc, 0xed]);
    assert_eq!(bytes[2044..2048], [0x00, 0x80, 0xff, 0x7f]);
    assert_eq!(bytes[2048..2052], [0xff, 0x7f, 0x00, 0x80]);
    assert!(bytes[2052..].iter().all(|b| *b == 0));
}

/// The message of a refused restore is plain text that names the part that did not fit: the
/// documented `Display` of the error (a Rust-only pin of its form, "presentation restore: "
/// followed by the variant).
/// Kills app/frames.rs `<impl Display for PresentationRestoreError>::fmt` -> `Ok(Default::default())`
/// (an empty message).
#[test]
fn a_refused_restore_has_a_message_that_names_what_did_not_fit() {
    assert_eq!(
        PresentationRestoreError::Mismatch.to_string(),
        "presentation restore: Mismatch"
    );
    assert_eq!(
        PresentationRestoreError::Sound(StateError::Magic).to_string(),
        "presentation restore: Sound(Magic)"
    );
}

/// `GameData::load` has no sound data and `load_with_sound` has it, built for the map it was
/// loaded for (decision T15; a fresh process's configuration: map number, volume 8, 3 channels).
/// Kills app/mod.rs `GameData::sound` -> `None`.
#[test]
fn game_data_holds_sound_assets_only_when_loaded_with_sound() {
    let wads: Vec<Vec<u8>> = ["m5a_wads/amp.wad", "m5b_gfx.wad", "m5c_sfx.wad"]
        .iter()
        .map(|w| std::fs::read(reference(w)).expect("WAD"))
        .collect();
    let refs: Vec<&[u8]> = wads.iter().map(Vec::as_slice).collect();
    let with = GameData::load_with_sound(&refs, GameMode::Registered, 1, 2).unwrap();
    let c = with.sound().expect("sound assets").config();
    assert_eq!(
        (c.logic.gamemap, c.logic.sfx_volume, c.logic.channels),
        (1, 8, 3)
    );
    let without = GameData::load(&refs[..2], GameMode::Registered, 1, 2).unwrap();
    assert!(without.sound().is_none());
}
