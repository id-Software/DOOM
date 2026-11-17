//! M5a: the renderer's switch tracker (decision R7) against the genuine switch textures of
//! the M5SWTCH room (`tools/reference/m5a_wad.py` `switch`), tic for tic.
//!
//! The expected middle textures are the genuine engine's: the `L` rows (`s0mid`) and `B` rows
//! of the strict M5a runner's game dumps of M5SWTCH (`tools/reference/m5a_driver.c` `dump`;
//! recorded in docs/validation/M5A.md, "Rooms added in step 2a"). Line 12 is an SR switch
//! (special 70) pressed at tic 2 and again at tic 12 while its button runs (p_switch.c:592-595,
//! `P_ChangeSwitchTexture` flips the texture back without a new button, p_switch.c:155-170,
//! 224-262); line 16 an SR switch pressed once at tic 59, restored at tic 93 (p_spec.c
//! `P_UpdateSpecials` button restore); line 20 an S1 switch (special 103, p_switch.c:455-459)
//! pressed at tic 107; line 24 a GR switch (special 46, p_spec.c:988-991) shot at tic 123 and
//! again at tic 146 while its button runs. Where the parity is derivable the tracker's side
//! textures must equal the genuine ones; after each re-press the genuine texture is the
//! opposite of a single flip, and the tracker must report the parity unknown from that tic
//! until the button expires (tics 12-35 and 146-156), and at no other tic.
mod common;

use common::{advance, items, load};
use doom_render::{RenderState, p_spec};

/// The genuine `s0mid` of lines 12, 16, 20 and 24 after dumped tic `t`.
fn genuine(t: i64) -> [usize; 4] {
    [
        if (2..=11).contains(&t) { 20 } else { 19 },
        if (59..=92).contains(&t) { 12 } else { 11 },
        if t >= 107 { 38 } else { 37 },
        if (123..=145).contains(&t) { 10 } else { 9 },
    ]
}

#[test]
fn switch_tracker_follows_the_genuine_switch_textures() {
    let item = items()
        .into_iter()
        .find(|i| i.lump == "M5SWTCH")
        .expect("M5SWTCH");
    let mut run = load(&item);
    let mut state = RenderState::level_start(&run.sim, &run.level).expect("level start");
    let level = run.level.clone();
    let lines = level.map.linedefs();
    let fronts: Vec<usize> = [12usize, 16, 20, 24]
        .iter()
        .map(|&l| usize::from(lines[l].sides[0].expect("front side")))
        .collect();
    let (mut compared, mut unknown) = (0usize, Vec::new());
    let mut tic: i64 = -1;
    loop {
        if state.switches.parity_unknown() {
            unknown.push(tic);
        } else {
            let sides =
                p_spec::side_textures(&run.res, &level.side_textures, lines, &state.switches);
            let got: Vec<usize> = fronts.iter().map(|&s| sides[s][1]).collect();
            assert_eq!(got, genuine(tic), "tic {tic}");
            compared += 1;
        }
        let index = (tic + 1) as usize;
        if index >= run.commands.len() {
            break;
        }
        advance(&mut run, index).expect("advance");
        state = state
            .next(&run.sim, &run.level)
            .unwrap_or_else(|e| panic!("tic {}: observe: {e:?}", tic + 1));
        tic += 1;
    }
    let want: Vec<i64> = (12..=35).chain(146..=156).collect();
    assert_eq!(unknown, want);
    assert_eq!(compared + unknown.len(), 214);
}
