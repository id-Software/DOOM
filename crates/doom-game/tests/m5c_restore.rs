//! M5c restore and continue (M5c note §4.8.3): the saved display state `DDSP` version 2 and the
//! sound session's `DSND` version 1, taken from an uninterrupted run of the app core's
//! `FrameSource` (with sound), decoded and restored into a fresh `FrameSource` next to a clone of
//! the simulation at that tic (doom-sim has no restore from its snapshot; `Simulation` is `Clone`,
//! as in M5a and M5b), then continued to the end of the item. Every frame, palette number, palette
//! set, face, `message_on`, message taken, `M_Random` index, the tic's sound report (every start
//! echo, stop, the update, the draws and the buffer), the channel table and the mixer, and both
//! encodings at the end must equal the uninterrupted run's; and each frame is compared with its
//! genuine golden of `tests/reference/m5c_frames` (the whole-frame and face-zeroed digests and
//! the status row), except the 70 frames of M5SWTCH (35 per screen size) whose switch parity the
//! renderer reports unknown, for which only the status row is compared. None of the seven items
//! has a masked (tracker-flagged) frame.
//!
//! Forged or mismatched saves are refused and nothing is built; a version-1 display state of the
//! M5b configuration (`GameData::load`) still restores as the M5b contract.
mod common {
    use std::path::{Path, PathBuf};
    pub fn reference(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/reference")
            .join(name)
    }
}
#[path = "../../doom-render/tests/m5c/mod.rs"]
mod m5c;

use doom_game::app::{
    FrameSource, GameData, PresentationRestoreError, Presented, SavedPresentation,
};
use doom_render::{Frame, GameMode};
use doom_sim::{Simulation, TicCommand};
use doom_sound::state::StateError;
use doom_test_evidence::sha256;
use m5c::{noface, read_stream, wire25};

fn load(lump: &str, with_sound: bool) -> (GameData, Vec<TicCommand>) {
    let (_, file, lmp) = *wire25::ITEMS.iter().find(|i| i.0 == lump).expect("item");
    let files: Vec<Vec<u8>> = [file, "m5b_gfx.wad", "m5c_sfx.wad", "m5c_music.wad"]
        .iter()
        .map(|w| std::fs::read(common::reference(w)).expect("WAD"))
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let demo = std::fs::read(common::reference(lmp)).expect("demo");
    let (map, skill) = (demo[3], demo[1]);
    let (_, parsed) = doom_sim::demo::parse_demo_episode1_v2(&demo, map).expect("demo");
    let data = if with_sound {
        GameData::load_with_sound(&refs, GameMode::Registered, map, skill)
    } else {
        GameData::load(&refs, GameMode::Registered, map, skill)
    }
    .expect("game data");
    (data, parsed.commands)
}

fn advance(sim: &mut Simulation, cmd: TicCommand) -> bool {
    sim.advance([
        cmd,
        Default::default(),
        Default::default(),
        Default::default(),
    ])
    .is_ok()
}

/// Everything one tic shows, compared between the runs.
#[derive(Debug, PartialEq)]
struct Shown {
    frame: String,
    noface: String,
    p: [i64; 5],
    message: Option<(i32, String)>,
    sound: Option<doom_sound::session::TicSound>,
    channels: Vec<Option<doom_sound::logic::ChannelEntry>>,
    mixer: Option<doom_sound::mixer::Mixer>,
}

fn show(fs: &FrameSource, p: &Presented, frame: &Frame, rect: [usize; 4]) -> Shown {
    Shown {
        frame: sha256(frame.pixels()),
        noface: sha256(&noface(frame.pixels(), rect)),
        p: [
            i64::from(p.display.palette),
            p.display.palette_set.map_or(-1, i64::from),
            i64::from(p.display.face),
            i64::from(p.display.message_on),
            i64::from(fs.rndindex()),
        ],
        message: p.message.map(|m| (m.code, m.text.to_string())),
        sound: p.sound.clone(),
        channels: fs
            .sound()
            .map_or(Vec::new(), |s| s.logic().channels().to_vec()),
        mixer: fs.sound().map(|s| s.mixer().clone()),
    }
}

/// (lump, why): items with sound, as note §4.8.3 asks.
const CASES: &[(&str, &str)] = &[
    (
        "SNDBUSY",
        "every channel busy: evictions and a refusal for want of a channel",
    ),
    (
        "SNDMONST",
        "a monster fight: sight, attack, pain and death sounds",
    ),
    (
        "SNDBTN2",
        "the switch press and the release from a busy button slot",
    ),
    ("TELE1", "a teleport's fog sounds"),
    ("BINF1", "monsters fighting each other"),
    ("SLIDEB", "a sliding door's sector origin"),
    ("M5SWTCH", "switches and a door"),
];

#[test]
fn ddsp_v2_and_dsnd_restore_and_continue_on_seven_items() {
    let mut saves = 0;
    let mut busy_saves = 0;
    let mut continued = 0;
    for &(lump, why) in CASES {
        for sb in [10u8, 11] {
            let (data, commands) = load(lump, true);
            let golden = read_stream(lump, sb);
            let rect = golden.frames[&0].face_rect;
            let mut sim = data.start().clone();
            let mut fs = FrameSource::start(&data, sb, &sim).expect("start");
            let mut frame = Frame::new();
            let mut shown = Vec::new();
            let mut sims = Vec::new();
            let mut saved = Vec::new();
            let mut first_busy = None;
            let mut comparable_tics = Vec::new();
            for (t, cmd) in commands.iter().enumerate() {
                if !advance(&mut sim, *cmd) {
                    break;
                }
                let p = fs.tic(&data, &sim, cmd, &mut frame).expect("tic");
                let g = &golden.frames[&(t as i64)];
                // the frames whose switch parity the renderer reports unknown are not compared
                // with the golden (as in the frame gate)
                let comparable = !p.display.view.switch_parity_unknown && g.columns.is_empty();
                comparable_tics.push(comparable);
                if comparable {
                    assert_eq!(sha256(frame.pixels()), g.sha256, "{lump} sb{sb} tic {t}");
                    assert_eq!(sha256(&noface(frame.pixels(), rect)), g.noface);
                }
                assert_eq!(i64::from(fs.rndindex()), g.status[4]);
                shown.push(show(&fs, &p, &frame, rect));
                let busy = fs.sound().expect("sound").mixer().mask() != 0;
                if busy && first_busy.is_none() {
                    first_busy = Some(t);
                }
                sims.push(sim.clone());
                saved.push(fs.save().expect("save"));
            }
            let n = shown.len();
            let mut points = vec![n / 4, n / 2, 3 * n / 4];
            points.extend(first_busy);
            points.sort_unstable();
            points.dedup();
            for at in points {
                let save = &saved[at];
                assert_eq!(&save.display[4..9], &[2, 0, 0, 0, 1], "DDSP version 2");
                let sound_bytes = save.sound.as_ref().expect("DSND");
                assert_eq!(&sound_bytes[..4], b"DSND");
                let mut sim = sims[at].clone();
                let mut fresh = FrameSource::restore(&data, &sim, save).expect("restore");
                assert_eq!(
                    fresh.save().as_ref(),
                    Some(save),
                    "re-encodes to the same bytes"
                );
                saves += 1;
                busy_saves += usize::from(fresh.sound().expect("sound").mixer().mask() != 0);
                for t in at + 1..n {
                    assert!(advance(&mut sim, commands[t]));
                    let p = fresh
                        .tic(&data, &sim, &commands[t], &mut frame)
                        .expect("tic");
                    let s = show(&fresh, &p, &frame, rect);
                    assert_eq!(s, shown[t], "{lump} sb{sb} saved at {at}, tic {t} ({why})");
                    let g = &golden.frames[&(t as i64)];
                    if comparable_tics[t] {
                        assert_eq!(s.frame, g.sha256);
                        assert_eq!(s.noface, g.noface);
                    }
                    assert_eq!(s.p, g.status);
                    continued += 1;
                }
                assert_eq!(
                    fresh.save().as_ref(),
                    saved.last(),
                    "{lump}: final encodings"
                );
            }
        }
    }
    eprintln!(
        "restore and continue: {saves} saves ({busy_saves} with a busy mixer channel), {continued} tics continued"
    );
    assert_eq!((saves, busy_saves, continued), (56, 16, 7580));
}

/// Runs `n` tics of `lump` with sound at screenblocks 10, returning the data, the commands, the
/// simulations after each tic, the saves after each tic and what each tic showed.
#[allow(clippy::type_complexity, reason = "test plumbing")]
fn run(
    lump: &str,
    with_sound: bool,
    n: usize,
) -> (
    GameData,
    Vec<TicCommand>,
    Vec<Simulation>,
    Vec<SavedPresentation>,
    Vec<Shown>,
) {
    let (data, commands) = load(lump, with_sound);
    let mut sim = data.start().clone();
    let mut fs = FrameSource::start(&data, 10, &sim).expect("start");
    let mut frame = Frame::new();
    let (mut sims, mut saves, mut shown) = (Vec::new(), Vec::new(), Vec::new());
    for cmd in &commands[..n] {
        assert!(advance(&mut sim, *cmd));
        let p = fs.tic(&data, &sim, cmd, &mut frame).expect("tic");
        shown.push(show(&fs, &p, &frame, [142, 168, 168, 199]));
        sims.push(sim.clone());
        saves.push(fs.save().expect("save"));
    }
    (data, commands, sims, saves, shown)
}

/// Continues a restored source from tic `at + 1` to `n - 1`; true when every tic shows what the
/// uninterrupted run showed.
fn continues_equal(
    data: &GameData,
    commands: &[TicCommand],
    sim: &Simulation,
    mut fs: FrameSource,
    at: usize,
    shown: &[Shown],
) -> bool {
    let mut sim = sim.clone();
    let mut frame = Frame::new();
    (at + 1..shown.len()).all(|t| {
        assert!(advance(&mut sim, commands[t]));
        let p = fs.tic(data, &sim, &commands[t], &mut frame).expect("tic");
        show(&fs, &p, &frame, [142, 168, 168, 199]) == shown[t]
    })
}

/// The comparison can fail: a session restored without one busy mixer channel, or a display with
/// the `M_Random` index one higher, restores (each is a valid state) but does not continue as the
/// uninterrupted run.
#[test]
fn the_restore_comparison_can_fail() {
    let (data, commands, sims, saves, shown) = run("SNDBUSY", true, 120);
    let at = (0..saves.len())
        .find(|&t| {
            let s = doom_sound::state::SoundState::decode(saves[t].sound.as_ref().expect("DSND"))
                .expect("decode");
            s.mixer.iter().flatten().count() >= 2
        })
        .expect("two busy mixer channels");
    let good = FrameSource::restore(&data, &sims[at], &saves[at]).expect("restore");
    assert!(continues_equal(
        &data, &commands, &sims[at], good, at, &shown
    ));
    let mut state = doom_sound::state::SoundState::decode(saves[at].sound.as_ref().expect("DSND"))
        .expect("decode");
    let k = state.mixer.iter().position(Option::is_some).expect("busy");
    state.mixer[k] = None;
    let dropped = SavedPresentation {
        display: saves[at].display.clone(),
        sound: Some(state.encode()),
    };
    let fs = FrameSource::restore(&data, &sims[at], &dropped).expect("a valid state");
    assert!(!continues_equal(
        &data, &commands, &sims[at], fs, at, &shown
    ));
    let mut display = doom_render::DisplayState::decode_any(&saves[at].display).expect("decode");
    display.status.rndindex = display.status.rndindex.wrapping_add(1);
    let shifted = SavedPresentation {
        display: display.encode(),
        sound: saves[at].sound.clone(),
    };
    let fs = FrameSource::restore(&data, &sims[at], &shifted).expect("a valid state");
    assert!(!continues_equal(
        &data, &commands, &sims[at], fs, at, &shown
    ));
}

#[test]
fn forged_and_mismatched_saves_are_refused() {
    let (data, _, sims, saves, _) = run("SNDBUSY", true, 60);
    let (m5b, _, m5b_sims, m5b_saves, _) = run("SNDBUSY", false, 60);
    let at = 40;
    let good = &saves[at];
    let refused = |data: &GameData, sim: &Simulation, s: &SavedPresentation| {
        FrameSource::restore(data, sim, s).expect_err("refused")
    };
    // the M5b contract's version-1 display state with sound data, and the reverse
    let v1 = SavedPresentation {
        display: m5b_saves[at].display.clone(),
        sound: good.sound.clone(),
    };
    assert_eq!(
        refused(&data, &sims[at], &v1),
        PresentationRestoreError::Mismatch
    );
    let no_sound = SavedPresentation {
        display: good.display.clone(),
        sound: None,
    };
    assert_eq!(
        refused(&data, &sims[at], &no_sound),
        PresentationRestoreError::Mismatch
    );
    assert_eq!(
        refused(&m5b, &m5b_sims[at], good),
        PresentationRestoreError::Mismatch
    );
    // decoders
    let mut b = good.clone();
    b.display[0] = b'X';
    assert_eq!(
        refused(&data, &sims[at], &b),
        PresentationRestoreError::DisplayDecode(doom_render::StateDecodeError::Magic)
    );
    let mut b = good.clone();
    b.display.truncate(b.display.len() - 1);
    assert_eq!(
        refused(&data, &sims[at], &b),
        PresentationRestoreError::DisplayDecode(doom_render::StateDecodeError::Truncated)
    );
    let mut b = good.clone();
    b.display[8] = 0;
    assert_eq!(
        refused(&data, &sims[at], &b),
        PresentationRestoreError::DisplayDecode(doom_render::StateDecodeError::Invalid("contract"))
    );
    let mut b = good.clone();
    b.sound.as_mut().expect("DSND")[4] = 2;
    assert_eq!(
        refused(&data, &sims[at], &b),
        PresentationRestoreError::Sound(StateError::Version(2))
    );
    let mut b = good.clone();
    b.sound.as_mut().expect("DSND").truncate(10);
    assert!(matches!(
        refused(&data, &sims[at], &b),
        PresentationRestoreError::Sound(StateError::Truncated)
    ));
    // review-sound M2: the DSND bytes never choose the session's configuration; a state saved on
    // map 8 (the map number at offset 24) is refused against the data's own map
    let mut b = good.clone();
    b.sound.as_mut().expect("DSND")[24..28].copy_from_slice(&8i32.to_le_bytes());
    assert!(matches!(
        refused(&data, &sims[at], &b),
        PresentationRestoreError::Sound(StateError::ConfigMismatch { .. })
    ));
    // a simulation of another tic: the sound session's check comes first, then the display's
    assert!(matches!(
        refused(&data, &sims[at + 1], good),
        PresentationRestoreError::Sound(StateError::TicMismatch { .. })
    ));
    let mixed = SavedPresentation {
        display: saves[at + 1].display.clone(),
        sound: good.sound.clone(),
    };
    assert!(matches!(
        refused(&data, &sims[at], &mixed),
        PresentationRestoreError::Display(doom_render::d_display::DisplayRestoreError::Tic { .. })
    ));
    // the uninterrupted source's saves are untouched by every refusal
    assert_eq!(&saves[at], good);
}

/// A version-1 display state of the M5b configuration still restores as the M5b contract, and
/// continues as its uninterrupted run.
#[test]
fn an_m5b_save_is_version_1_and_restores_as_the_m5b_contract() {
    let (data, commands, sims, saves, shown) = run("SNDBUSY", false, 80);
    assert!(data.sound().is_none());
    let at = 30;
    assert_eq!(&saves[at].display[4..8], &1u32.to_le_bytes());
    assert_eq!(saves[at].sound, None);
    let state = doom_render::DisplayState::decode(&saves[at].display).expect("M5b's decoder");
    assert_eq!(
        state.contract,
        doom_render::RandomContract::SoundFreeWipeFree
    );
    let fs = FrameSource::restore(&data, &sims[at], &saves[at]).expect("restore");
    assert_eq!(
        fs.display().contract(),
        doom_render::RandomContract::SoundFreeWipeFree
    );
    assert!(continues_equal(&data, &commands, &sims[at], fs, at, &shown));
    // the M5b index is the tic count (only ST_Ticker draws)
    assert_eq!(shown[at].p[4], (at as i64 + 1) & 0xff);
}
