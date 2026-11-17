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
//! game and only the status bar ticker advances it, so the idle face's glance can differ from a
//! vanilla session with sound; nothing else on screen depends on it.
//!
//! Use: [`Display::new`], [`Display::begin_level`] at tic 0, then after every committed
//! `Simulation::advance` [`Display::observe`] with the command fed, and [`Display::draw`] for a
//! frame and its palette number. [`Display::state`] and [`Display::restore`] save and restore.
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
    /// g_game.c:472-473). Only its `redrawsbar` effect is modelled; the melt is not drawn.
    pub wipe: bool,
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

    /// Observes the simulation's next committed tic (exactly once per tic, in order), with the
    /// command fed to `advance`: `ST_Ticker` and `HU_Ticker` with the derived message. Returns
    /// the message `HU_Ticker` took in this tic, if any.
    pub fn observe(
        &mut self,
        sim: &Simulation,
        fed: &TicCommand,
    ) -> Result<Option<Message>, DisplayObserveError> {
        self.observe_with(sim, fed, |_, _| {})
    }

    /// [`Self::observe`] with the derived inputs adjusted by `adjust` before the tickers. For
    /// the bespoke cheat scenario of decision S1 only: cheats are outside wire 24, so a test
    /// applies their effect on the status bar inputs (`powers`, `godmode`) and the message.
    pub fn observe_with(
        &mut self,
        sim: &Simulation,
        fed: &TicCommand,
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
        adjust(&mut inputs, &mut message);
        let mut status = self.status;
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
    /// unchanged and `out` is unspecified.
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
        })
    }

    /// Restores a saved state next to the simulation at the same tic (its status bar inputs are
    /// read from `sim`). On an error the display keeps its state.
    pub fn restore(
        &mut self,
        state: DisplayState,
        sim: &Simulation,
    ) -> Result<(), DisplayRestoreError> {
        if state.render.tic != sim.tic() {
            return Err(DisplayRestoreError::Tic {
                state: state.render.tic,
                simulation: sim.tic(),
            });
        }
        if let Some(v) = value_error(&state.status, &state.hud) {
            return Err(DisplayRestoreError::Value(v));
        }
        let inputs = status_inputs(sim).map_err(DisplayRestoreError::Observe)?;
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

/// Encoding magic of a display state.
pub const MAGIC: &[u8; 4] = b"DDSP";
/// Encoding version.
pub const VERSION: u32 = 1;

impl DisplayState {
    /// The `DDSP` version 1 encoding (docs/STATE_FORMAT.md, "Display state"): the magic, the
    /// version, the DRST version 2 bytes length-prefixed, then the display, status bar and HUD
    /// fields (little-endian).
    pub fn encode(&self) -> Vec<u8> {
        let drst = self.render.encode();
        let mut o = Vec::with_capacity(drst.len() + 256);
        o.extend_from_slice(MAGIC);
        o.extend(VERSION.to_le_bytes());
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

    /// Decodes a `DDSP` version 1 encoding: the embedded DRST with its own decoder, then every
    /// field with its bounds.
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
        })
    }
}
