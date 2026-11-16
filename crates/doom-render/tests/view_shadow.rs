//! M5a step 2a: the renderer's view inputs, tic for tic, against the genuine view records
//! (`tests/reference/m5a_views/<LUMP>.view`, DOOM-M5A-VIEW-v1, written by the M5a runner
//! after every dumped tic: `tools/reference/m5a_driver.c` `write_view`) for all 42 items (the
//! 20 rooms and the 22 M4b synthetic scripts).
//!
//! The view shadow (decision R6) re-derives `viewz`, `viewheight`, `deltaviewheight`,
//! `onground` and `fixedcolormap`, which the simulation does not keep; the other fields
//! (`viewx`, `viewy`, `viewangle`, `bob`, `extralight`, the invisibility power, both
//! psprites, the player's sector and its light level) are read from the simulation's public
//! API as the renderer reads them, and the flat and texture translations (A rows) are derived
//! from the tic (`p_spec::translations`). Every field of every record must match exactly. The
//! record of the load (tic -1) is compared with the state at simulation tic 0, and the record
//! of dumped tic t with the state after the (t+1)-th `advance`.
mod common;

use common::{VIEW_FIELDS, ViewTic, advance, items, load, read_view};
use doom_render::{RenderState, p_spec};
use doom_sim::{PlayerSlot, Simulation};

/// The Rust side of one view record.
fn rust_view(
    sim: &Simulation,
    state: &RenderState,
    res: &doom_render::Resources,
    level: &doom_render::Level,
) -> ViewTic {
    let p = sim.original_player(PlayerSlot::One).expect("player");
    let mo = sim.actor(p.mo.expect("mo")).expect("actor");
    let w = sim
        .original_player_weapons(PlayerSlot::One)
        .expect("weapons")
        .expect("weapon state");
    let powers = sim
        .original_player_powers(PlayerSlot::One)
        .map(|p| p.powers)
        .unwrap_or([0; 6]);
    let sector = mo.location().expect("location").sector;
    let light = sim
        .sector_lights_and_specials()
        .and_then(|mut l| l.nth(usize::from(sector)))
        .map_or(level.map.sectors()[usize::from(sector)].light, |(l, _)| l);
    let ps = |i: usize| {
        let s = &w.psprites[i];
        [
            s.state.map_or(-1, |id| i64::from(id.0)),
            i64::from(s.tics),
            i64::from(s.position[0].bits()),
            i64::from(s.position[1].bits()),
        ]
    };
    let v = &state.view;
    let mut values = vec![
        i64::from(mo.position()[0].bits()),
        i64::from(mo.position()[1].bits()),
        i64::from(v.viewz),
        i64::from(mo.angle()),
        i64::from(v.viewheight),
        i64::from(v.deltaviewheight),
        i64::from(w.bob.bits()),
        i64::from(v.onground),
        i64::from(w.extra_light),
        i64::from(v.fixedcolormap),
        i64::from(powers[2]),
    ];
    values.extend(ps(0));
    values.extend(ps(1));
    values.push(i64::from(sector));
    values.push(i64::from(light));
    let (flats, textures) = p_spec::translations(res, sim.tic());
    let mut anims = Vec::new();
    for (kind, table) in [("flat", &flats), ("texture", &textures)] {
        for (i, t) in table.iter().enumerate() {
            if *t != i {
                anims.push((kind.to_string(), i, *t));
            }
        }
    }
    ViewTic { values, anims }
}

#[test]
fn view_shadow_matches_every_genuine_view_record() {
    let mut matches = vec![0usize; VIEW_FIELDS.len() + 1];
    let mut compared = 0usize;
    let mut failures = Vec::new();
    let mut uncovered = Vec::new();
    for item in items() {
        let record = read_view(&item.lump);
        let mut run = load(&item);
        let mut state = RenderState::level_start(&run.sim, &run.level).expect("level start");
        let mut tic: i64 = -1;
        while let Some(want) = record.get(&tic) {
            let got = rust_view(&run.sim, &state, &run.res, &run.level);
            compared += 1;
            for (k, name) in VIEW_FIELDS.iter().enumerate() {
                if got.values[k] == want.values[k] {
                    matches[k] += 1;
                } else if failures.len() < 40 {
                    failures.push(format!(
                        "{} tic {tic}: {name} rust {} genuine {}",
                        item.lump, got.values[k], want.values[k]
                    ));
                }
            }
            if got.anims == want.anims {
                matches[VIEW_FIELDS.len()] += 1;
            } else if failures.len() < 40 {
                failures.push(format!("{} tic {tic}: translations differ", item.lump));
            }
            let index = (tic + 1) as usize;
            if index >= run.commands.len() || advance(&mut run, index).is_err() {
                break;
            }
            state = state
                .next(&run.sim, &run.level)
                .unwrap_or_else(|e| panic!("{} tic {}: observe: {e:?}", item.lump, tic + 1));
            tic += 1;
        }
        let last = *record.keys().last().expect("records");
        if tic < last {
            uncovered.push(format!(
                "{} ({} genuine tics after {tic})",
                item.lump,
                last - tic
            ));
        }
    }
    eprintln!("view records compared: {compared}");
    for (k, name) in VIEW_FIELDS
        .iter()
        .chain(["translations"].iter())
        .enumerate()
    {
        eprintln!("  {name}: {} / {compared}", matches[k]);
    }
    eprintln!("items whose Rust run ended before the record: {uncovered:?}");
    assert!(
        failures.is_empty(),
        "view mismatches:\n{}",
        failures.join("\n")
    );
    assert!(compared > 0);
}
