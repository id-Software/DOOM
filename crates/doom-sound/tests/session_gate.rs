//! THE GATE of M5c step 4b: the rows of the 64 committed genuine sound-logic streams
//! (tests/reference/m5c_sound/<ITEM>.sound) and of the 64 committed genuine mixer session goldens
//! (tests/reference/m5c_mixer/sessions/goldens/<ITEM>.golden: start echoes and buffer digests)
//! reproduced end to end from the Rust wire-25 simulation through `doom_sound::session::Sound`
//! (tests/common/gate.rs), with these exceptions, which are not compared:
//!
//! - in the streams, the `#` header rows, the W rows (the wipe's draws and melt steps, one per
//!   item; the wipe's 320 draws are still checked through the next R row's index) and the Z rows
//!   (the totals); every S, T, K, C, L and R row is compared;
//! - in an S row of a start with no placement (a NULL origin or the listener's own body), the
//!   three listener columns (x, y, angle): a wire-25 record carries the listener only for a
//!   positional start, and the C does not read it for these starts (s_sound.c:304-324). The run
//!   prints how many S rows this applies to (381 of 785 at the time of writing);
//! - in the mixer session goldens, the fake device's `d` and `q` rows (`genuine_golden` in
//!   tests/common/gate.rs).
//!
//! Four items run only to the length of their genuine stream, which is shorter than their demo
//! (the run prints them): BOSS1 ends at the level exit, and the next tic is checked to be refused
//! as `LevelExited`; M5VPLAN, M5OPEN and M5SSEG end where the genuine process ends at a renderer
//! `I_Error`, and nothing after that is checked here.
//!
//! The reference is linuxdoom-1.10 built with its in-process sound code (one define removed);
//! the goldens come from the genuine C only. The R rows' status-bar draw and the wipe's 320 draws
//! are applied by the declared rule of tests/common/gate.rs.
//!
//! Equivalence class: source equivalence against genuine traces, end to end.

mod common;

use common::gate::{
    Carry, GoldenWriter, RowCounts, compare_stream, drain, genuine_golden, genuine_stream,
    load_item, run, session_set,
};
use common::{first_difference, wire25};
use doom_sound::session::Sound;

#[test]
fn every_genuine_row_of_the_64_items_is_reproduced_end_to_end() {
    let set = session_set();
    let mut counts = RowCounts::default();
    let (mut tics, mut starts, mut buffers, mut golden_rows) = (0u64, 0u64, 0u64, 0usize);
    let (mut firings, mut labelled) = (0u64, Vec::new());
    let mut short = Vec::new();
    for i in 0..wire25::ITEMS.len() {
        let item = load_item(i);
        let genuine = genuine_stream(&item.name);
        let sound = Sound::new(set.clone(), item.config).expect("session");
        let mut golden = GoldenWriter::new(&set);
        let out = run(
            &item.commands,
            item.sim,
            sound,
            Carry::default(),
            genuine.len(),
            Some(&mut golden),
            |_, _, _, _| {},
        );
        if let Some((tic, e)) = &out.refused {
            panic!("{}: the simulation refused tic {tic}: {e}", item.name);
        }
        if let Err(e) = compare_stream(&item.name, &out.stream, &genuine, &mut counts) {
            panic!("{e}");
        }
        drain(&out.sound, &mut golden);
        starts += golden.starts;
        buffers += golden.buffers();
        let rows = golden.finish();
        let want = genuine_golden(&item.name);
        if let Some((k, w, g)) = first_difference(&want, &rows) {
            panic!(
                "{}: mixer golden row {k}:\n  genuine: {w}\n  rust:    {g}",
                item.name
            );
        }
        golden_rows += want.len();
        // BOSS1's stream ends where the level is exited (the E1M8 boss special, decision of
        // M4b): the simulation refuses the next tic. M5OPEN, M5SSEG and M5VPLAN end at the
        // renderer's I_Error (the M5a overflow witnesses), outside sound.
        if item.name == "BOSS1" {
            let mut sim = out.sim.clone();
            let c = item.commands[genuine.len()];
            let next = sim.advance([
                c,
                Default::default(),
                Default::default(),
                Default::default(),
            ]);
            assert!(
                matches!(next, Err(doom_sim::SimError::LevelExited)),
                "BOSS1 after its stream: {next:?}"
            );
        }
        if item.commands.len() != genuine.len() {
            short.push(format!(
                "{}:{}/{}",
                item.name,
                genuine.len(),
                item.commands.len()
            ));
        }
        tics += out.stream.len() as u64;
        let d = out.sound.detector();
        firings += d.firings();
        if let Some(t) = d.layout_dependent_from() {
            labelled.push(format!("{}@{t}", item.name));
        }
    }
    eprintln!(
        "session gate: 64 items, {tics} tics; stream rows S {} ({} unplaced without the listener \
         columns), T {}, K {}, C {}, L {}, R {}; \
         mixer golden rows {golden_rows} ({starts} starts, {buffers} buffers); detector firings \
         {firings}, layout-dependent sessions [{}]; streams shorter than the demo [{}]",
        counts.s,
        counts.s_listener_unchecked,
        counts.t,
        counts.k,
        counts.c,
        counts.l,
        counts.r,
        labelled.join(" "),
        short.join(" ")
    );
    // tests/reference/m5c_mixer_sessions_results.json: 64 items, 741 starts, 22770 tics,
    // 14043 buffers.
    assert_eq!((tics, starts, buffers), (22_770, 741, 14_043));
    assert_eq!(firings, 0, "the T12 detector fired on a synthetic item");
}
