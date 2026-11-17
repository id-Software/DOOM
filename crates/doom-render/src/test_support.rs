//! Test-only fixture: a [`FrameContext`] over the committed synthetic room `amp.wad` and the
//! committed synthetic graphics set (`tests/reference/m5a_gfx.wad`; no id art) at simulation
//! tic 0, so that the crate-private drawers can be called with arguments the public API never
//! produces (the unit tests of `r_draw`, `r_things` and `r_data` use it).
use crate::{
    Detail, FrameContext, GameMode, Level, LumpDirectory, RenderConfig, Resources, p_spec, r_bsp,
    r_main, r_plane, r_segs, r_things, sector_views,
};
use doom_sim::{MapContextOptions, PlayerSlot, Simulation, Skill, WeaponInventory};
use std::path::Path;

/// Runs `f` on a fresh frame context at `screenblocks` (high detail), the framebuffer
/// zeroed (the level-start state of decision R8).
pub(crate) fn with_ctx<R>(screenblocks: u8, f: impl FnOnce(&mut FrameContext<'_>) -> R) -> R {
    with_ctx_res(screenblocks, |_| {}, f)
}

/// [`with_ctx`] after `edit` has changed the loaded [`Resources`] (for example to make a
/// texture's columns undefined), before the level and the frame context are built.
pub(crate) fn with_ctx_res<R>(
    screenblocks: u8,
    edit: impl FnOnce(&mut Resources),
    f: impl FnOnce(&mut FrameContext<'_>) -> R,
) -> R {
    with_room("amp.wad", screenblocks, edit, f)
}

/// [`with_ctx_res`] in another committed synthetic room of `tests/reference/m5a_wads` (map
/// E1M1 of that WAD).
pub(crate) fn with_room<R>(
    room: &str,
    screenblocks: u8,
    edit: impl FnOnce(&mut Resources),
    f: impl FnOnce(&mut FrameContext<'_>) -> R,
) -> R {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
    let wad_bytes = std::fs::read(root.join("m5a_wads").join(room)).expect("room WAD");
    let gfx_bytes = std::fs::read(root.join("m5a_gfx.wad")).expect("graphics WAD");
    let wad = doom_wad::Wad::parse(&wad_bytes, doom_wad::Limits::default()).expect("room");
    let gfx = doom_wad::Wad::parse(&gfx_bytes, doom_wad::Limits::default()).expect("graphics");
    let mut dir = LumpDirectory::new();
    dir.add_wad(&wad).expect("room directory");
    dir.add_wad(&gfx).expect("graphics directory");
    let mut res = Resources::load(dir, GameMode::Registered).expect("resources");
    edit(&mut res);
    let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).expect("map");
    let level = Level::load(&res, &map, 1).expect("level");
    let mut sim = Simulation::original_player_episode1_v2_map(
        map,
        1,
        Skill::Medium,
        MapContextOptions::default(),
    )
    .expect("admission");
    sim.spawn_map_things().expect("THINGs");
    sim.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .expect("weapons");
    sim.spawn_level_specials().expect("specials");
    let view = r_main::ViewSize::new(RenderConfig {
        screenblocks,
        detail: Detail::High,
    })
    .expect("view size");
    let sectors = sector_views(&sim, &res, &level).expect("sectors");
    let (flattranslation, texturetranslation) = p_spec::translations(&res, sim.tic());
    let sides = p_spec::side_textures(
        &res,
        &level.side_textures,
        level.map.linedefs(),
        &crate::state::RenderState::level_start(&sim, &level)
            .expect("level start")
            .switches,
    );
    let textureoffset = p_spec::texture_offsets(&level.map, sim.tic());
    let mut screen = vec![0u8; crate::SCREENWIDTH * crate::SCREENHEIGHT];
    let mut taint = vec![0u32; screen.len()];
    let mut mapped = vec![false; level.map.linedefs().len()];
    let mut trace = vec![crate::Feature::Untouched; crate::SCREENWIDTH * crate::SCREENHEIGHT];
    let frame = r_main::FrameView::setup(0, 0, 0, 0, 0, 0);
    let nsectors = sectors.len();
    let mut ctx = FrameContext {
        res: &res,
        level: &level,
        view: &view,
        frame,
        sim: &sim,
        screen: &mut screen,
        taint: &mut taint,
        trace: Some(&mut trace),
        mapped: &mut mapped,
        newly_mapped: Vec::new(),
        fuzzpos: 0,
        sectors,
        sides,
        textureoffset,
        flattranslation,
        texturetranslation,
        bsp: r_bsp::BspState::default(),
        segs: r_segs::SegState::new(frame.fixedcolormap, crate::WallSteps::default()),
        planes: r_plane::PlaneState::default(),
        things: r_things::ThingState::new(nsectors),
    };
    f(&mut ctx)
}

/// The sprite lump (relative to the first sprite lump) of the fixture player's weapon psprite
/// frame 0 rotation 0, for tests that edit its width in the resources.
pub(crate) fn fixture_weapon_lump(res: &Resources) -> i16 {
    let sprite = {
        let mut found = None;
        with_ctx(11, |ctx| {
            let w = ctx
                .sim
                .original_player_weapons(PlayerSlot::One)
                .expect("player")
                .expect("weapon state");
            let state = w.psprites[0].state.expect("weapon psprite");
            found = Some(
                ctx.sim
                    .actor_registry()
                    .expect("registry")
                    .state(state)
                    .expect("state")
                    .sprite,
            );
        });
        found.expect("sprite")
    };
    res.sprites[usize::from(sprite)].frames[0].lump[0]
}
