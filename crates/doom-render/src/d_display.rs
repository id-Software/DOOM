//! `D_Display` (d_main.c:191-355) for a single-player level at screenblocks 10 and 11
//! (decision S14): the composed frame of the status bar, the 3D view and the heads-up message,
//! the palette number, and the saveable display state (decision S18).
//!
//! Per displayed frame the genuine order is `HU_Erase` (d_main.c:231-232), `ST_Drawer` with
//! `redrawsbar` (d_main.c:237-247), `R_RenderPlayerView` over the persistent `screens[0]`
//! (d_main.c:267-268), then `HU_Drawer` (d_main.c:270-271); at these sizes the border steps
//! return at once (r_draw.c:746-747, d_main.c:285). Per tic, `ST_Ticker` and `HU_Ticker` run after
//! the simulation (g_game.c:730-733).
//!
//! Frames, palette numbers, faces and messages are identical to the genuine engine under the
//! declared sound-free, wipe-free contract (decision S15): the `M_Random` index is 0 at a new
//! game and only the status bar ticker advances it. A vanilla session also draws it for sound
//! and for the level-start melt (once per screen column, f_wipe.c:159-162), so the idle face's
//! glance can differ from one; nothing else on screen depends on it. The evidence has two
//! exclusions: 300 frames are compared only on a masked digest and 70 are not compared (counts
//! asserted in tests/m5b_display.rs).
//!
//! Use: [`Display::new`], [`Display::begin_level`] at tic 0, then after every committed
//! `Simulation::advance` [`Display::observe`] with the command fed, and [`Display::draw`] for a
//! frame and its palette number. [`Display::state`] and [`Display::restore`] save and restore.
//!
//! M5c adds a second, explicit contract value, [`RandomContract::WithSound`] (M5c note §4.3.4,
//! decisions T6 and T16), chosen with [`Display::set_contract`] before the first tic; the default
//! stays the M5b contract above, so every M5b caller, frame and saved state is unchanged. Under it
//! the one `M_Random` index (m_random.c:63-67) follows the C order of a tic: the caller's sound
//! starts first (`S_StartSoundAtVolume` inside `P_Ticker`, g_game.c:730; their number is passed to
//! [`Display::observe_after`]), then `ST_Ticker`'s own draw (g_game.c:731, st_stuff.c:992);
//! `S_UpdateSounds` after the tic draws nothing (d_main.c:392); and the frame of tic 0 is followed by
//! the level-start wipe's 320 draws ([`WIPE_DRAWS`]: `D_Display` wipes because `gamestate !=
//! wipegamestate`, d_main.c:223, and after composing the frame calls `wipe_ScreenWipe`,
//! d_main.c:327-340, whose first call runs `wipe_initMelt`, f_wipe.c:280-286, drawing once for
//! column 0 and once for each of columns 1..319, f_wipe.c:159-166). The melt's pictures are not
//! drawn (decisions T5 and T6: no melt); the frame shown is the composed frame, which is the
//! melt's final picture. Its saved state is `DDSP` version 2 ([`DisplayState::encode`]).
use crate::hu_stuff::{self, HudState, Message};
use crate::st_stuff::{self, Attacker, StatusBarState, StatusGraphics, StatusInputs};
use crate::v_video::{BG_ROWS, Canvas, DrawError, Patch};
use crate::{
    ConfigError, Detail, Frame, FrameReport, Level, ObserveError, RenderConfig, RenderRefusal,
    RenderState, Renderer, ResourceError, Resources, RestoreError, SCREENWIDTH, StateDecodeError,
};
use doom_sim::{PickupMessage, PlayerSlot, Simulation, TicCommand};
use std::fmt;
use std::sync::Arc;

/// How the display accounts for the `M_Random` index (M5c note §4.3.4; decisions S15, T6, T16).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RandomContract {
    /// M5b's declared contract (decision S15): the index is 0 at a new game and only `ST_Ticker`
    /// advances it; no sound, no wipe. Saved as `DDSP` version 1. The default.
    #[default]
    SoundFreeWipeFree,
    /// M5c: per tic the caller's sound draws ([`Display::observe_after`]) before `ST_Ticker`'s
    /// draw, and the wipe's [`WIPE_DRAWS`] after the frame of tic 0 (see the module
    /// documentation). Saved as `DDSP` version 2.
    WithSound,
}

/// The level-start wipe's `M_Random` draws under [`RandomContract::WithSound`]: `wipe_initMelt`
/// draws once for `y[0]` (f_wipe.c:159) and once for each `y[i]`, i = 1..width-1 (f_wipe.c:160-166),
/// with `width` = `SCREENWIDTH` 320 (d_main.c:339-340, f_wipe.c:285); only 160 values are used.
pub const WIPE_DRAWS: u16 = 320;

/// The index after `draws` calls of `M_Random` (m_random.c:63-67: `rndindex = (rndindex+1)&0xff`
/// per call).
fn advanced(index: u8, draws: u16) -> u8 {
    index.wrapping_add((draws & 0xff) as u8)
}

/// The display configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayConfig {
    /// `screenblocks` 10 (with the status bar) or 11 (full screen) (decision S14).
    pub screenblocks: u8,
}

/// Why a tic could not be observed. The display state is unchanged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplayObserveError {
    /// The renderer could not observe the tic.
    Render(ObserveError),
    /// The simulation has no Episode 1 player whose status bar fields can be read.
    NoPlayer,
    /// The tic's message assignment could not be derived from its events and command, or does
    /// not match the simulation's stored code (decision S17, §1.6 item 5); the stored code after
    /// the tic is named.
    MessageUnderivable(PickupMessage),
    /// A message would stay pending in `player->message` (`showMessages` 0 or a protected
    /// message, hu_stuff.c:518-523), which the observer cannot carry to a later tic.
    MessageNotTaken,
    /// The face would read the position of an attacker whose thing has been freed
    /// (st_stuff.c:814-817; decision S17).
    AttackerRemoved,
    /// The call does not fit the display's [`RandomContract`]: [`Display::observe`] or
    /// [`Display::observe_with`] under [`RandomContract::WithSound`] (sound draws would be
    /// skipped silently), [`Display::observe_after`] with draws under
    /// [`RandomContract::SoundFreeWipeFree`], or [`Display::set_contract`] after a tic was observed.
    Contract,
    /// [`Display::observe_with`]'s adjustment gave a status bar input no genuine player state
    /// holds (health or armour outside 0..=200, ammunition outside 0..=its maximum, a maximum
    /// other than the table or its double, a weapon number of 9 or more, a damage count outside
    /// 0..=100, a negative bonus count or power); the input is named.
    AdjustedInputOutOfRange(&'static str),
}

/// A refused frame. The display state is unchanged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplayRefusal {
    /// The 3D view refused the frame.
    Render(RenderRefusal),
    /// A 2D drawer refused (decision S17).
    Draw(DrawError),
    /// No tic has been observed since the level start: `D_Display` draws neither the status
    /// bar, the view nor the HUD at `gametic` 0 (d_main.c:237-238, 267, 270).
    BeforeFirstTic,
}
impl From<RenderRefusal> for DisplayRefusal {
    fn from(e: RenderRefusal) -> Self {
        Self::Render(e)
    }
}
impl From<DrawError> for DisplayRefusal {
    fn from(e: DrawError) -> Self {
        Self::Draw(e)
    }
}

/// A display state that cannot be restored. The display keeps its state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplayRestoreError {
    /// The embedded render state does not fit the level.
    Render(RestoreError),
    /// The state is not of the simulation's tic.
    Tic {
        /// The state's tic.
        state: u64,
        /// The simulation's tic.
        simulation: u64,
    },
    /// A value outside the range the status bar and HUD produce.
    Value(&'static str),
    /// Values that are each in range but that no run of one level from a fresh process gives
    /// together, or that disagree with the simulation (see [`DisplayState::check`]).
    Inconsistent(&'static str),
    /// The simulation's player cannot be read.
    Observe(DisplayObserveError),
    /// The state's screenblocks is not admitted.
    Config(ConfigError),
}

macro_rules! display_error {
    ($($t:ty),*) => {$(
        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "display: {self:?}")
            }
        }
        impl std::error::Error for $t {}
    )*};
}
display_error!(DisplayObserveError, DisplayRefusal, DisplayRestoreError);

/// One displayed frame beside its pixels.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayFrame {
    /// The PLAYPAL index in effect (`st_palette`, 0..=13; st_stuff.c:1045-1050).
    pub palette: u8,
    /// The palette `I_SetPalette` received during this frame, if any.
    pub palette_set: Option<u8>,
    /// The face index of the frame (`st_faceindex`).
    pub face: i32,
    /// Whether the message line was drawn (`message_on`).
    pub message_on: bool,
    /// The 3D view's report; its flagged columns cover the composed frame.
    pub view: FrameReport,
}

/// Everything the display carries between tics and frames (decision S18): the renderer's
/// state (whose `screen` here is the composed `screens[0]`), the status bar, the HUD and the
/// `D_Display` statics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayState {
    /// `screenblocks`.
    pub screenblocks: u8,
    /// The render state (DRST version 2), screen composed.
    pub render: RenderState,
    /// The status bar.
    pub status: StatusBarState,
    /// The HUD.
    pub hud: HudState,
    /// `D_Display`'s `fullscreen` (d_main.c:198, 247).
    pub fullscreen: bool,
    /// A wipe is due: `gamestate != wipegamestate` (d_main.c:223; `G_DoLoadLevel` forces one,
    /// g_game.c:472-473). Only its `redrawsbar` effect is modelled, and under
    /// [`RandomContract::WithSound`] its [`WIPE_DRAWS`]; the melt is not drawn.
    pub wipe: bool,
    /// The random contract ([`RandomContract::SoundFreeWipeFree`] encodes as `DDSP` version 1,
    /// [`RandomContract::WithSound`] as version 2).
    pub contract: RandomContract,
}

/// The composed display of one level.
#[derive(Clone, Debug)]
pub struct Display {
    res: Arc<Resources>,
    renderer: Renderer,
    gfx: StatusGraphics,
    font: Vec<Patch>,
    playpal: Vec<u8>,
    status: StatusBarState,
    hud: HudState,
    fullscreen: bool,
    wipe: bool,
    contract: RandomContract,
    inputs: Option<StatusInputs>,
    bg: Vec<u8>,
    bg_defined: Vec<bool>,
}

/// `maxammo` (p_inter.c:58), doubled by the backpack (p_inter.c:589-595).
const MAXAMMO: [i32; 4] = [200, 50, 300, 50];

/// The status bar's inputs of the console player after the simulation's last tic.
pub fn status_inputs(sim: &Simulation) -> Result<StatusInputs, DisplayObserveError> {
    let e = |_| DisplayObserveError::NoPlayer;
    let s = PlayerSlot::One;
    let player = sim.original_player(s).map_err(e)?;
    let mo_id = player.mo.ok_or(DisplayObserveError::NoPlayer)?;
    let mo = sim.actor(mo_id).ok_or(DisplayObserveError::NoPlayer)?;
    let w = sim
        .original_player_weapons(s)
        .map_err(e)?
        .ok_or(DisplayObserveError::NoPlayer)?;
    let powers = sim.original_player_powers(s).map_err(e)?;
    let attacker = match player.attacker {
        None => Attacker::None,
        Some(a) if a == mo_id => Attacker::Player,
        Some(a) => sim.actor(a).map_or(Attacker::Removed, |t| {
            Attacker::At(t.position().map(|f| f.bits()))
        }),
    };
    Ok(StatusInputs {
        health: player.health,
        armor: sim.original_player_armor(s).map_err(e)?.points,
        ammo: [w.bullets, w.shells, w.cells, w.rockets].map(i32::from),
        maxammo: MAXAMMO.map(|m| m << u8::from(w.backpack)),
        weaponowned: std::array::from_fn(|i| w.owned >> i & 1 == 1),
        readyweapon: w.ready as u8,
        cards: powers.cards,
        damagecount: player.damage_count,
        bonuscount: sim.original_player_pickups(s).map_err(e)?.bonus_count,
        powers: powers.powers,
        godmode: false,
        attackdown: w.attack_down,
        attacker,
        position: mo.position().map(|f| f.bits()),
        angle: mo.angle(),
    })
}

fn stored_message(sim: &Simulation) -> Result<PickupMessage, DisplayObserveError> {
    sim.original_player_pickups(PlayerSlot::One)
        .map(|p| p.message)
        .map_err(|_| DisplayObserveError::NoPlayer)
}

/// PLAYPAL holds 14 palettes of 768 bytes (st_stuff.c:71-76, 1048).
const PLAYPAL_BYTES: usize = 14 * 768;

impl Display {
    /// A display over loaded resources and a level: loads the status bar graphics, the HUD
    /// font and the 14 palettes from the global lump list.
    pub fn new(
        res: Arc<Resources>,
        level: Arc<Level>,
        cfg: DisplayConfig,
    ) -> Result<Self, ResourceError> {
        let renderer = Renderer::new(res.clone(), level, render_config(cfg.screenblocks)?)
            .map_err(ResourceError::Config)?;
        let gfx = StatusGraphics::load(&res)?;
        let font = hu_stuff::load_font(&res)?;
        let pal = res.lump(res.directory().get_num_for_name(b"PLAYPAL")?);
        let playpal = pal
            .get(..PLAYPAL_BYTES)
            .ok_or(ResourceError::LumpTooShort {
                name: "PLAYPAL".into(),
                needed: PLAYPAL_BYTES,
                length: pal.len(),
            })?
            .to_vec();
        Ok(Self {
            status: StatusBarState::level_start([false; 9]),
            hud: HudState::level_start(PickupMessage::None),
            res,
            renderer,
            gfx,
            font,
            playpal,
            fullscreen: false,
            wipe: true,
            contract: RandomContract::SoundFreeWipeFree,
            inputs: None,
            bg: vec![0; SCREENWIDTH * BG_ROWS],
            bg_defined: vec![false; SCREENWIDTH * BG_ROWS],
        })
    }

    /// The 14 palettes (`palette * 768` is the start of a frame's palette).
    pub fn playpal(&self) -> &[u8] {
        &self.playpal
    }

    /// The configuration.
    pub fn config(&self) -> DisplayConfig {
        DisplayConfig {
            screenblocks: self.renderer.config().screenblocks,
        }
    }

    /// Starts a new game on the level at tic 0, with fresh-process status bar and HUD history
    /// (M5b's restart is a new game, §1.6 item 8): `M_ClearRandom` (g_game.c:1414), `ST_Start`
    /// and `HU_Start` (p_mobj.c:693-698), a forced wipe (g_game.c:472-473) and the renderer's
    /// level start (decision R8).
    pub fn begin_level(&mut self, sim: &Simulation) -> Result<(), DisplayObserveError> {
        let inputs = status_inputs(sim)?;
        let stored = stored_message(sim)?;
        self.renderer
            .begin_level(sim)
            .map_err(DisplayObserveError::Render)?;
        self.status = StatusBarState::level_start(inputs.weaponowned);
        self.hud = HudState::level_start(stored);
        self.fullscreen = false;
        self.wipe = true;
        self.inputs = Some(inputs);
        self.bg.fill(0);
        self.bg_defined.fill(false);
        Ok(())
    }

    /// The random contract.
    pub fn contract(&self) -> RandomContract {
        self.contract
    }

    /// Chooses the random contract (the default is [`RandomContract::SoundFreeWipeFree`]). Only
    /// before the first observed tic of a level (before [`Self::begin_level`] or right after it);
    /// later it is refused with [`DisplayObserveError::Contract`] and nothing changes, since a
    /// change mid-level would mix two accountings of one index.
    pub fn set_contract(&mut self, contract: RandomContract) -> Result<(), DisplayObserveError> {
        if self.renderer.state().is_some_and(|s| s.tic != 0) {
            return Err(DisplayObserveError::Contract);
        }
        self.contract = contract;
        Ok(())
    }

    /// Observes the simulation's next committed tic (exactly once per tic, in order), with the
    /// command fed to `advance`: `ST_Ticker` and `HU_Ticker` with the derived message. Returns
    /// the message `HU_Ticker` took in this tic, if any. Refused under
    /// [`RandomContract::WithSound`], whose caller must pass the tic's sound draws
    /// ([`Self::observe_after`]).
    pub fn observe(
        &mut self,
        sim: &Simulation,
        fed: &TicCommand,
    ) -> Result<Option<Message>, DisplayObserveError> {
        self.observe_with(sim, fed, |_, _| {})
    }

    /// [`Self::observe`] under [`RandomContract::WithSound`]: `draws` is the number of `M_Random`
    /// calls the tic's sound starts made inside `P_Ticker` (g_game.c:730), which come before
    /// `ST_Ticker`'s own draw (g_game.c:731, st_stuff.c:992); the index advances by them first
    /// (m_random.c:63-67). Under [`RandomContract::SoundFreeWipeFree`] only `draws == 0` is
    /// accepted (then it is exactly [`Self::observe`]); anything else is refused with
    /// [`DisplayObserveError::Contract`]. On an error nothing changes.
    pub fn observe_after(
        &mut self,
        sim: &Simulation,
        fed: &TicCommand,
        draws: u16,
    ) -> Result<Option<Message>, DisplayObserveError> {
        self.observe_after_with(sim, fed, draws, |_, _| {})
    }

    /// [`Self::observe_after`] with [`Self::observe_with`]'s adjustment, for the bespoke cheat
    /// scenario of decision S1 under the M5c contract (its frame test) only; the app never calls
    /// it.
    pub fn observe_after_with(
        &mut self,
        sim: &Simulation,
        fed: &TicCommand,
        draws: u16,
        adjust: impl FnOnce(&mut StatusInputs, &mut Option<Message>),
    ) -> Result<Option<Message>, DisplayObserveError> {
        if self.contract == RandomContract::SoundFreeWipeFree && draws != 0 {
            return Err(DisplayObserveError::Contract);
        }
        self.observe_inner(sim, fed, draws, adjust)
    }

    /// [`Self::observe`] with the derived inputs adjusted by `adjust` before the tickers. For
    /// the bespoke cheat scenario of decision S1 only (its golden test applies the cheats'
    /// effect on the status bar inputs, `powers` and `godmode`, and the message, since cheats
    /// are outside wire 24), the display's own refusal tests and the fuzz target; the app never
    /// calls it. Adjusted inputs that no genuine player state holds are refused with
    /// [`DisplayObserveError::AdjustedInputOutOfRange`] before any state changes.
    /// Refused under [`RandomContract::WithSound`] (use [`Self::observe_after_with`]).
    pub fn observe_with(
        &mut self,
        sim: &Simulation,
        fed: &TicCommand,
        adjust: impl FnOnce(&mut StatusInputs, &mut Option<Message>),
    ) -> Result<Option<Message>, DisplayObserveError> {
        if self.contract == RandomContract::WithSound {
            return Err(DisplayObserveError::Contract);
        }
        self.observe_inner(sim, fed, 0, adjust)
    }

    fn observe_inner(
        &mut self,
        sim: &Simulation,
        fed: &TicCommand,
        draws: u16,
        adjust: impl FnOnce(&mut StatusInputs, &mut Option<Message>),
    ) -> Result<Option<Message>, DisplayObserveError> {
        let mut inputs = status_inputs(sim)?;
        let stored = stored_message(sim)?;
        let gametic = sim.tic().wrapping_sub(1);
        let mut message =
            hu_stuff::derive_message(sim.events(), gametic, fed.forward, self.hud.stored, stored)
                .map_err(DisplayObserveError::MessageUnderivable)?
                .map(|m| {
                    hu_stuff::message_text(m).ok_or(DisplayObserveError::MessageUnderivable(m))
                })
                .transpose()?;
        let derived = inputs;
        adjust(&mut inputs, &mut message);
        if inputs != derived {
            if let Some(name) = adjusted_input_error(&inputs) {
                return Err(DisplayObserveError::AdjustedInputOutOfRange(name));
            }
        }
        let mut status = self.status;
        // The tic's sound draws inside P_Ticker come before ST_Ticker's (g_game.c:730-731); 0
        // under the M5b contract.
        status.rndindex = advanced(status.rndindex, draws);
        status
            .tick(&inputs)
            .map_err(|_| DisplayObserveError::AttackerRemoved)?;
        let mut hud = self.hud.clone();
        if !hud.tick(message) && message.is_some() {
            return Err(DisplayObserveError::MessageNotTaken);
        }
        hud.stored = stored;
        self.renderer
            .observe(sim)
            .map_err(DisplayObserveError::Render)?;
        self.status = status;
        self.hud = hud;
        self.inputs = Some(inputs);
        Ok(message)
    }

    /// The status bar's history.
    pub fn status(&self) -> &StatusBarState {
        &self.status
    }

    /// The HUD's history.
    pub fn hud(&self) -> &HudState {
        &self.hud
    }

    /// Draws the simulation's current (observed) tic into `out` in `D_Display`'s order and
    /// returns the palette number and the frame's report. On refusal the display state is
    /// unchanged and `out` is unspecified. Under [`RandomContract::WithSound`] the frame that
    /// wipes (the first of the level) is followed by the wipe's [`WIPE_DRAWS`] (d_main.c:327-340,
    /// f_wipe.c:159-166); the frame itself does not depend on them.
    pub fn draw(
        &mut self,
        sim: &Simulation,
        out: &mut Frame,
    ) -> Result<DisplayFrame, DisplayRefusal> {
        let Some(inputs) = self.inputs.filter(|_| sim.tic() != 0) else {
            return Err(DisplayRefusal::BeforeFirstTic);
        };
        let full = self.renderer.config().screenblocks == 11;
        let redrawsbar = self.wipe || (!full && self.fullscreen);
        let mut status = self.status;
        let mut hud = self.hud.clone();
        hud.erase();
        let (mut bg, mut bg_defined) = (self.bg.clone(), self.bg_defined.clone());
        let mut st = None;
        let (res, gfx, font) = (&*self.res, &self.gfx, &self.font);
        let view = self.renderer.render_with(
            sim,
            out,
            |fg, taint| {
                let mut c = Canvas {
                    res,
                    fg,
                    taint,
                    bg: &mut bg,
                    bg_defined: &mut bg_defined,
                };
                st = Some(st_stuff::draw(
                    &mut status,
                    gfx,
                    &inputs,
                    &mut c,
                    full,
                    redrawsbar,
                )?);
                Ok::<_, DisplayRefusal>(())
            },
            |fg, taint| {
                let (mut b, mut d) = (Vec::new(), Vec::new());
                let mut c = Canvas {
                    res,
                    fg,
                    taint,
                    bg: &mut b,
                    bg_defined: &mut d,
                };
                Ok(hud.draw(&mut c, font)?)
            },
        )?;
        let st = st.expect("drawn");
        if self.wipe && self.contract == RandomContract::WithSound {
            // wipe_EndScreen and the melt loop after the composed frame (d_main.c:327-340): the
            // first wipe_ScreenWipe call runs wipe_initMelt's draws (f_wipe.c:280-286, 159-166).
            status.rndindex = advanced(status.rndindex, WIPE_DRAWS);
        }
        self.status = status;
        self.hud = hud;
        self.bg = bg;
        self.bg_defined = bg_defined;
        self.fullscreen = full;
        self.wipe = false;
        Ok(DisplayFrame {
            palette: st.palette,
            palette_set: st.palette_set,
            face: st.face,
            message_on: self.hud.message_on,
            view,
        })
    }

    /// Switches between screenblocks 10 and 11 (`R_SetViewSize`); the next frame follows
    /// `D_Display`'s `fullscreen` rule (d_main.c:242-243).
    pub fn set_screenblocks(&mut self, screenblocks: u8) -> Result<(), ConfigError> {
        let mut r = Renderer::new(
            self.res.clone(),
            self.renderer_level(),
            render_config(screenblocks).map_err(|_| ConfigError::ScreenBlocks(screenblocks))?,
        )?;
        if let Some(s) = self.renderer.state() {
            r.restore(s.clone()).expect("same level");
        }
        self.renderer = r;
        Ok(())
    }

    fn renderer_level(&self) -> Arc<Level> {
        self.renderer.level.clone()
    }

    /// The saveable state (None before [`Self::begin_level`]).
    pub fn state(&self) -> Option<DisplayState> {
        Some(DisplayState {
            screenblocks: self.renderer.config().screenblocks,
            render: self.renderer.state()?.clone(),
            status: self.status,
            hud: self.hud.clone(),
            fullscreen: self.fullscreen,
            wipe: self.wipe,
            contract: self.contract,
        })
    }

    /// Restores a saved state next to the simulation at the same tic (its status bar inputs are
    /// read from `sim`), after [`DisplayState::check`]. On an error the display keeps its state.
    pub fn restore(
        &mut self,
        state: DisplayState,
        sim: &Simulation,
    ) -> Result<(), DisplayRestoreError> {
        let inputs = state.check(sim)?;
        let mut next = self.clone();
        next.set_screenblocks(state.screenblocks)
            .map_err(DisplayRestoreError::Config)?;
        next.renderer
            .restore(state.render)
            .map_err(DisplayRestoreError::Render)?;
        next.bg.fill(0);
        next.bg_defined.fill(false);
        if state.status.bg_drawn {
            let (mut fg, mut taint) = (vec![0; 64000], vec![0; 64000]);
            let mut c = Canvas {
                res: &next.res,
                fg: &mut fg,
                taint: &mut taint,
                bg: &mut next.bg,
                bg_defined: &mut next.bg_defined,
            };
            c.patch(0, 0, crate::v_video::Screen::Bg, &next.gfx.sbar)
                .map_err(|_| DisplayRestoreError::Value("bg_drawn"))?;
        }
        next.status = state.status;
        next.hud = state.hud;
        next.fullscreen = state.fullscreen;
        next.wipe = state.wipe;
        next.contract = state.contract;
        next.inputs = Some(inputs);
        *self = next;
        Ok(())
    }
}

fn render_config(screenblocks: u8) -> Result<RenderConfig, ResourceError> {
    if !(10..=11).contains(&screenblocks) {
        return Err(ResourceError::Config(ConfigError::ScreenBlocks(
            screenblocks,
        )));
    }
    Ok(RenderConfig {
        screenblocks,
        detail: Detail::High,
    })
}

/// The first status bar input outside the values a genuine player state holds, by the writers
/// of linuxdoom-1.10: `health` 0..=200 (clamped at 0, p_inter.c:871-872; at most 200,
/// p_inter.c:385-386, 402-403, 412), `armorpoints` 0..=200 (p_inter.c:393-394; the armour
/// pickups give at most 200), `maxammo` the table or its double (p_inter.c:58, 593), each `ammo`
/// 0..=its `maxammo` (p_inter.c:107-108), `readyweapon` below `NUMWEAPONS` (doomdef.h:192),
/// `damagecount` 0..=100 (p_inter.c:877-878), `bonuscount` and every power counter
/// nonnegative. `bonuscount` has no cap in the C (`+= BONUSADD`, p_inter.c:184, 658); its bound
/// `i32::MAX - 7` only keeps `ST_doPaletteStuff`'s `bonuscount + 7` (st_stuff.c:1032) in range.
fn adjusted_input_error(i: &StatusInputs) -> Option<&'static str> {
    let checks: [(bool, &'static str); 7] = [
        ((0..=200).contains(&i.health), "health"),
        ((0..=200).contains(&i.armor), "armor"),
        (
            (0..4).all(|k| [MAXAMMO[k], 2 * MAXAMMO[k]].contains(&i.maxammo[k])),
            "maxammo",
        ),
        (
            (0..4).all(|k| (0..=i.maxammo[k]).contains(&i.ammo[k])),
            "ammo",
        ),
        (
            usize::from(i.readyweapon) < i.weaponowned.len(),
            "readyweapon",
        ),
        ((0..=100).contains(&i.damagecount), "damagecount"),
        (
            (0..=i32::MAX - 7).contains(&i.bonuscount) && i.powers.iter().all(|p| *p >= 0),
            "bonuscount or powers",
        ),
    ];
    checks.iter().find(|(ok, _)| !ok).map(|(_, name)| *name)
}

/// The bounds of every status bar and HUD value (the ranges st_stuff.c and hu_stuff.c produce).
fn value_error(s: &StatusBarState, h: &HudState) -> Option<&'static str> {
    let face = -1..=st_stuff::DEADFACE;
    let checks: [(bool, &'static str); 14] = [
        (s.oldhealth >= -1, "oldhealth"),
        ((0..=70).contains(&s.facecount), "facecount"),
        ((0..=st_stuff::DEADFACE).contains(&s.faceindex), "faceindex"),
        ([0, 4, 5, 6, 7, 8, 9].contains(&s.priority), "priority"),
        ((-1..=70).contains(&s.lastattackdown), "lastattackdown"),
        (
            (0..3).all(|i| [-1, i, i + 3].contains(&s.keyboxes[i as usize])),
            "keyboxes",
        ),
        (s.ready_ammo.is_none_or(|a| a < 4), "ready_ammo"),
        ((-1..=13).contains(&s.palette), "palette"),
        (s.arms_old.iter().all(|v| (-1..=1).contains(v)), "arms_old"),
        (face.contains(&s.face_old), "face_old"),
        (s.keys_old.iter().all(|v| (-1..=5).contains(v)), "keys_old"),
        (
            (0..=hu_stuff::MSGTIMEOUT).contains(&h.message_counter),
            "message_counter",
        ),
        (
            h.line.text.len() <= crate::hu_lib::MAXLINELENGTH && !h.line.text.contains(&0),
            "line",
        ),
        ((0..=4).contains(&h.line.needsupdate), "needsupdate"),
    ];
    checks.iter().find(|(ok, _)| !ok).map(|(_, name)| *name)
}

impl DisplayState {
    /// The checks [`Display::restore`] makes, returning the simulation's status bar inputs: the
    /// state is of `sim`'s tic, each value is in its range ([`DisplayRestoreError::Value`], as
    /// [`DisplayState::decode`] checks), and the values agree with each other and with `sim` as
    /// the C's writers keep them within one level of a fresh process (the only states the
    /// display produces; [`DisplayRestoreError::Inconsistent`]):
    ///
    /// - `message_on` exactly while `message_counter` is nonzero: `HU_Ticker` sets both
    ///   together and clears `message_on` when the counter runs out (hu_stuff.c:512-516,
    ///   527-528; the chat and Enter-refresh writers, hu_stuff.c:562-564 and 661-662, also set
    ///   both and are not reached), and both start cleared (hu_stuff.c:429 with a fresh
    ///   process's counter 0);
    /// - `must_show` and `protected` clear: only the menu's message toggle sets
    ///   `message_dontfuckwithme` (m_menu.c:989), and `HU_Ticker` copies it into
    ///   `message_nottobefuckedwith` (hu_stuff.c:529);
    /// - `st_oldhealth` -1 at tic 0 (st_stuff.c:1268) and the player's health after every tic
    ///   (st_stuff.c:994), at most 200 (p_inter.c:385-386, 402-403, 412);
    /// - after tic 0, the key boxes and the ready ammunition as `ST_updateWidgets` derives them
    ///   from the player (st_stuff.c:932-958);
    /// - under [`RandomContract::WithSound`] (`DDSP` version 2) only, the wipe flag: set at tic 0,
    ///   clear from tic 2 (either at tic 1, between the first observe and the first draw): the
    ///   wipe is due once, before the first `D_Display` (d_main.c:187, 223-227, 300; g_game.c:
    ///   472-475), and its 320 draws would otherwise be made again; M5b's contract accepts any
    ///   flag, as it always has;
    /// - `lastattackdown` -1 or 1..=`ST_RAMPAGEDELAY`: it is reset to 1 in the tic it reaches 0
    ///   (st_stuff.c:880-893);
    /// - the face, by `priority` (st_stuff.c:760-921): 0 a straight face (offset 0-2) with
    ///   `st_facecount` below `ST_STRAIGHTFACECOUNT`; 4 the god face; 5 the rampage face; 6 the
    ///   rampage face; 7 a turned, head-on or ouch face; 8 the evil grin; 6-8 with a count below
    ///   their set count; 4, 5 and 9 with count 0; 9 exactly the dead face, while the player's
    ///   health is 0 after tic 0. A pain band is 0-4 (`ST_calcPainOffset` of a health of 0 or
    ///   more), and the ouch face, which needs a health above 19, is in bands 0-3.
    ///
    /// Accepted on trust: the message line's text, `showMessages`, the palette and the widgets'
    /// old values (which follow the last frame drawn, not the inputs), `oldweaponsowned`,
    /// `st_firsttime` and `bg_drawn` against the embedded screen, the `M_Random` index and the
    /// face's pain band against the current health (a face stays up after the health changes).
    /// Also accepted, although no run produces them (the checks are of ranges and of relations
    /// that hold in every tic, not a replay of the tic that set the face):
    ///
    /// - at tic 0, everything that is not pinned above: the key boxes and the ready ammunition
    ///   (not derived until the first tic), and the fresh values of the face (`faceindex`,
    ///   `facecount` and `priority` 0), of `lastattackdown` (-1) and of the message (off with
    ///   counter 0) are not required, so a tic-0 state with key boxes 0, 1 and 2 and priority 7
    ///   at count 34, or a message showing with counter 100, passes;
    /// - after tic 0: priority 4 without god mode or invulnerability in the inputs; priority 5
    ///   with `lastattackdown` other than 1; priority 6 or 7 at count 34, or 8 at count 69,
    ///   while the damage or bonus count that sets it that tic is 0.
    pub fn check(&self, sim: &Simulation) -> Result<StatusInputs, DisplayRestoreError> {
        if self.render.tic != sim.tic() {
            return Err(DisplayRestoreError::Tic {
                state: self.render.tic,
                simulation: sim.tic(),
            });
        }
        if let Some(v) = value_error(&self.status, &self.hud) {
            return Err(DisplayRestoreError::Value(v));
        }
        let inputs = status_inputs(sim).map_err(DisplayRestoreError::Observe)?;
        match consistency_error(self, sim.tic(), &inputs) {
            Some(e) => Err(DisplayRestoreError::Inconsistent(e)),
            None => Ok(inputs),
        }
    }
}

/// The first cross-field check of [`DisplayState::check`] that fails.
fn consistency_error(d: &DisplayState, tic: u64, i: &StatusInputs) -> Option<&'static str> {
    let (s, h) = (&d.status, &d.hud);
    let mut ticked = s;
    let derived;
    if tic != 0 {
        let mut t = *s;
        t.tick_widgets(i);
        derived = t;
        ticked = &derived;
    }
    let (band, offset) = (s.faceindex / 8, s.faceindex % 8);
    let plain = s.faceindex < st_stuff::GODFACE;
    let face = match s.priority {
        0 => plain && offset <= 2 && s.facecount < 17,
        4 => s.faceindex == st_stuff::GODFACE && s.facecount == 0,
        5 => plain && offset == 7 && s.facecount == 0,
        6 => plain && offset == 7 && s.facecount < 35,
        7 => plain && (matches!(offset, 3 | 4 | 7) || offset == 5 && band <= 3) && s.facecount < 35,
        8 => plain && offset == 6 && s.facecount < 70,
        9 => s.faceindex == st_stuff::DEADFACE && s.facecount == 0,
        _ => false,
    };
    // Under the M5c contract the wipe flag moves the M_Random index (the wipe's 320 draws after
    // the frame), so it must be one the original could hold at the tic: a straight-on-map start
    // makes `gamestate != wipegamestate` true once (wipegamestate starts as GS_DEMOSCREEN,
    // d_main.c:187; G_DoLoadLevel, g_game.c:472-475) and the first D_Display clears it
    // (d_main.c:223-227, 300), which the declared schedule runs after tic 0. So it is set at tic
    // 0, either at tic 1 (between the first observe and the first draw), clear from tic 2.
    // Version 1 (M5b's contract) is unchanged: there the flag changes only `redrawsbar`.
    let wipe = match (d.contract, tic) {
        (RandomContract::SoundFreeWipeFree, _) | (RandomContract::WithSound, 1) => true,
        (RandomContract::WithSound, 0) => d.wipe,
        (RandomContract::WithSound, _) => !d.wipe,
    };
    let checks: [(bool, &'static str); 8] = [
        (
            h.message_on == (h.message_counter != 0),
            "message_on and message_counter",
        ),
        (wipe, "wipe"),
        (!h.must_show && !h.protected, "must_show or protected"),
        (
            s.oldhealth == if tic == 0 { -1 } else { i.health } && s.oldhealth <= 200,
            "oldhealth",
        ),
        (
            ticked.keyboxes == s.keyboxes && ticked.ready_ammo == s.ready_ammo,
            "keyboxes or ready_ammo",
        ),
        (
            s.lastattackdown == -1 || (1..=70).contains(&s.lastattackdown),
            "lastattackdown",
        ),
        (face, "face"),
        (
            (s.priority == 9) == (tic != 0 && i.health == 0),
            "dead face",
        ),
    ];
    checks.iter().find(|(ok, _)| !ok).map(|(_, name)| *name)
}

/// Encoding magic of a display state.
pub const MAGIC: &[u8; 4] = b"DDSP";
/// Encoding version of [`RandomContract::SoundFreeWipeFree`] (M5b), the only version
/// [`DisplayState::decode`] reads.
pub const VERSION: u32 = 1;
/// Encoding version of [`RandomContract::WithSound`] (M5c): version 1's layout with one contract
/// byte after the version (M5c note §4.8.1); read by [`DisplayState::decode_any`].
pub const VERSION_WITH_SOUND: u32 = 2;
/// The contract byte of a version-2 encoding: 1 for [`RandomContract::WithSound`], the only value
/// written (a version-2 encoding of the M5b contract is never written and is refused).
const CONTRACT_WITH_SOUND: u8 = 1;

impl DisplayState {
    /// The `DDSP` encoding (docs/STATE_FORMAT.md, "Display state"): the magic, the version, under
    /// [`RandomContract::WithSound`] one contract byte (version 2, M5c), the DRST version 2 bytes
    /// length-prefixed, then the display, status bar and HUD fields (little-endian). Under
    /// [`RandomContract::SoundFreeWipeFree`] it is version 1, byte for byte as M5b wrote it.
    pub fn encode(&self) -> Vec<u8> {
        let drst = self.render.encode();
        let mut o = Vec::with_capacity(drst.len() + 256);
        o.extend_from_slice(MAGIC);
        match self.contract {
            RandomContract::SoundFreeWipeFree => o.extend(VERSION.to_le_bytes()),
            RandomContract::WithSound => {
                o.extend(VERSION_WITH_SOUND.to_le_bytes());
                o.push(CONTRACT_WITH_SOUND);
            }
        }
        o.extend((drst.len() as u32).to_le_bytes());
        o.extend(drst);
        let s = &self.status;
        let flags = |b: &[bool]| {
            b.iter()
                .enumerate()
                .fold(0u16, |a, (i, v)| a | u16::from(*v) << i)
        };
        o.push(self.screenblocks);
        o.push(flags(&[self.fullscreen, self.wipe]) as u8);
        o.push(s.rndindex);
        o.extend(flags(&s.oldweaponsowned).to_le_bytes());
        for v in [
            s.oldhealth,
            s.facecount,
            s.faceindex,
            s.priority,
            s.lastattackdown,
            s.palette,
        ] {
            o.extend(v.to_le_bytes());
        }
        for v in s
            .keyboxes
            .iter()
            .chain(&s.arms_old)
            .chain([&s.face_old])
            .chain(&s.keys_old)
        {
            o.push(*v as i8 as u8);
        }
        o.push(s.ready_ammo.unwrap_or(0xff));
        o.push(flags(&[s.firsttime, s.bg_drawn, s.armsbg_old]) as u8);
        let h = &self.hud;
        o.push(flags(&[h.message_on, h.protected, h.must_show, h.laston]) as u8);
        for v in [h.message_counter, h.show_messages, h.line.needsupdate] {
            o.extend(v.to_le_bytes());
        }
        o.push(h.stored as u8);
        o.push(h.line.text.len() as u8);
        o.extend(&h.line.text);
        o
    }

    /// Decodes a `DDSP` version 1 or version 2 encoding (M5c): version 1 as [`Self::decode`]
    /// ([`RandomContract::SoundFreeWipeFree`]); version 2 with contract byte 1 as the same layout
    /// and field rules under [`RandomContract::WithSound`]. A version-2 contract byte other than 1
    /// is refused as `Invalid("contract")`, any other version as `Version`.
    pub fn decode_any(bytes: &[u8]) -> Result<Self, StateDecodeError> {
        if bytes.get(..4).ok_or(StateDecodeError::Truncated)? != MAGIC {
            return Err(StateDecodeError::Magic);
        }
        let version = u32::from_le_bytes(
            bytes
                .get(4..8)
                .ok_or(StateDecodeError::Truncated)?
                .try_into()
                .expect("4"),
        );
        match version {
            VERSION => Self::decode(bytes),
            VERSION_WITH_SOUND => {
                let contract = *bytes.get(8).ok_or(StateDecodeError::Truncated)?;
                if contract != CONTRACT_WITH_SOUND {
                    return Err(StateDecodeError::Invalid("contract"));
                }
                // The rest is version 1's layout: decode it with version 1's rules.
                let mut v1 = Vec::with_capacity(bytes.len() - 1);
                v1.extend_from_slice(MAGIC);
                v1.extend(VERSION.to_le_bytes());
                v1.extend_from_slice(&bytes[9..]);
                let mut state = Self::decode(&v1)?;
                state.contract = RandomContract::WithSound;
                Ok(state)
            }
            v => Err(StateDecodeError::Version(v)),
        }
    }

    /// Decodes a `DDSP` version 1 encoding (M5b's contract): the embedded DRST with its own
    /// decoder, then every field with its bounds. Any other version, version 2 included, is
    /// refused as `Version` ([`Self::decode_any`] reads both).
    pub fn decode(bytes: &[u8]) -> Result<Self, StateDecodeError> {
        let mut at = 0usize;
        let mut take = |n: usize| -> Result<&[u8], StateDecodeError> {
            let b = bytes.get(at..at + n).ok_or(StateDecodeError::Truncated)?;
            at += n;
            Ok(b)
        };
        if take(4)? != MAGIC {
            return Err(StateDecodeError::Magic);
        }
        let u32_of = |b: &[u8]| u32::from_le_bytes(b.try_into().expect("4"));
        let version = u32_of(take(4)?);
        if version != VERSION {
            return Err(StateDecodeError::Version(version));
        }
        let n = u32_of(take(4)?) as usize;
        let render = RenderState::decode(take(n)?)?;
        let head = take(5)?;
        let (screenblocks, dflags, rndindex) = (head[0], head[1], head[2]);
        let owned = u16::from_le_bytes([head[3], head[4]]);
        let flag = |f: u8, i: u8| f >> i & 1 == 1;
        let mut ints = [0i32; 6];
        for v in &mut ints {
            *v = i32::from_le_bytes(take(4)?.try_into().expect("4"));
        }
        let small: Vec<i32> = take(13)?.iter().map(|b| i32::from(*b as i8)).collect();
        let tail = take(3)?;
        let (ready, sflags, hflags) = (tail[0], tail[1], tail[2]);
        let mut hints = [0i32; 3];
        for v in &mut hints {
            *v = i32::from_le_bytes(take(4)?.try_into().expect("4"));
        }
        let lens = take(2)?;
        let stored = hu_stuff::ALL_MESSAGES
            .iter()
            .copied()
            .find(|m| *m as u8 == lens[0])
            .ok_or(StateDecodeError::Invalid("stored"))?;
        let text = take(usize::from(lens[1]))?.to_vec();
        if at != bytes.len() {
            return Err(StateDecodeError::Trailing);
        }
        let bad = |name| StateDecodeError::Invalid(name);
        if !(10..=11).contains(&screenblocks) || dflags >> 2 != 0 || owned >> 9 != 0 {
            return Err(bad("display flags"));
        }
        if (ready > 3 && ready != 0xff) || sflags >> 3 != 0 || hflags >> 4 != 0 {
            return Err(bad("status flags"));
        }
        let status = StatusBarState {
            rndindex,
            oldhealth: ints[0],
            oldweaponsowned: std::array::from_fn(|i| owned >> i & 1 == 1),
            facecount: ints[1],
            faceindex: ints[2],
            priority: ints[3],
            lastattackdown: ints[4],
            keyboxes: [small[0], small[1], small[2]],
            ready_ammo: (ready != 0xff).then_some(ready),
            palette: ints[5],
            firsttime: flag(sflags, 0),
            bg_drawn: flag(sflags, 1),
            arms_old: std::array::from_fn(|i| small[3 + i]),
            face_old: small[9],
            keys_old: [small[10], small[11], small[12]],
            armsbg_old: flag(sflags, 2),
        };
        let hud = HudState {
            message_on: flag(hflags, 0),
            message_counter: hints[0],
            protected: flag(hflags, 1),
            must_show: flag(hflags, 2),
            show_messages: hints[1],
            line: crate::hu_lib::TextLine {
                text,
                needsupdate: hints[2],
            },
            laston: flag(hflags, 3),
            stored,
        };
        if let Some(v) = value_error(&status, &hud) {
            return Err(bad(v));
        }
        Ok(Self {
            screenblocks,
            render,
            status,
            hud,
            fullscreen: flag(dflags, 0),
            wipe: flag(dflags, 1),
            contract: RandomContract::SoundFreeWipeFree,
        })
    }
}
