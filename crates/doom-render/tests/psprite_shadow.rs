//! M5a: the weapon sprite's partial-invisibility shadow rule on the committed fuzz room, at the
//! power values around its thresholds. The rule is `R_DrawPSprite`'s (r_things.c:716-736):
//! `powers[pw_invisibility] > 4*32 || powers[pw_invisibility] & 8` draws the weapon as fuzz
//! (the shadow colour map), anything else draws it with its light. The committed fuzz demo ends
//! long before the power counts down to these values, so the run continues with
//! empty commands (the player stands still), observing every tic as the renderer requires.
mod common;

use common::{advance, items, load};
use doom_render::{Detail, Feature, Frame, RenderConfig, Renderer};
use doom_sim::{PlayerSlot, TicCommand};

#[test]
fn the_weapon_is_fuzz_above_128_or_with_bit_3_and_lit_otherwise() {
    let item = items()
        .into_iter()
        .find(|i| i.lump == "M5FUZZ")
        .expect("M5FUZZ");
    let mut run = load(&item);
    let cfg = RenderConfig {
        screenblocks: 11,
        detail: Detail::High,
    };
    let mut r = Renderer::new(run.res.clone(), run.level.clone(), cfg).unwrap();
    r.set_trace(true);
    r.begin_level(&run.sim).unwrap();
    for i in 0..run.commands.len() {
        advance(&mut run, i).unwrap();
        r.observe(&run.sim).unwrap();
    }
    let power = |run: &common::Run| {
        run.sim
            .original_player_powers(PlayerSlot::One)
            .unwrap()
            .powers[2]
    };
    assert!(
        power(&run) > 1000,
        "the sphere is still active at the demo's end"
    );
    // The values the thresholds separate: 4*32 itself (visible: 128 has no bit 3), one above
    // (fuzz), a value with bit 3 below it (fuzz), and values without bit 3 below it (visible).
    let want: [(i32, bool); 8] = [
        (130, true),
        (129, true),
        (128, false),
        (127, true),
        (120, true),
        (112, false),
        (100, false),
        (96, false),
    ];
    let mut frame = Frame::new();
    let mut checked = 0;
    while power(&run) > 0 {
        let p = power(&run);
        if let Some((_, shadow)) = want.iter().find(|(v, _)| *v == p) {
            r.render(&run.sim, &mut frame).unwrap();
            let t = r.trace().unwrap();
            let fuzz = t
                .iter()
                .any(|f| matches!(f, Feature::Fuzz { psprite: true, .. }));
            let lit = t.iter().any(|f| matches!(f, Feature::PSprite { .. }));
            assert_eq!((fuzz, lit), (*shadow, !*shadow), "invisibility {p}");
            checked += 1;
        }
        run.sim
            .advance([TicCommand::default(); 4])
            .expect("the player stands still");
        r.observe(&run.sim).unwrap();
    }
    assert_eq!(checked, want.len());
}
