//! r_things.c: sprite definitions (`R_InitSprites`, `R_InitSpriteDefs`,
//! `R_InstallSpriteLump`) and, from step 2c, vissprites, masked drawing and weapon sprites.
use crate::r_data::{ResourceError, Resources, SpriteDef, SpriteFrame, name_text};
use doom_sim::identity::OriginalSprite;

/// `sprtemp` entry state (`spriteframe_t` after `memset(-1)`: `rotate` is -1).
#[derive(Clone, Copy)]
struct Temp {
    rotate: i8,
    lump: [i16; 8],
    flip: [bool; 8],
}

/// `R_InitSpriteDefs` (r_things.c:183-267) over the 138 `sprnames` entries (info.c:40-55, the
/// NULL-terminated copy of decision R11), with `modifiedgame` false (the M5a runner).
pub(crate) fn init_sprite_defs(res: &Resources) -> Result<Vec<SpriteDef>, ResourceError> {
    let dir = res.directory();
    let first = res.firstspritelump;
    let end = first + res.sprite_lumps.len();
    let mut out = Vec::new();
    for sprite in OriginalSprite::all() {
        let spritename = sprite.resource_name().as_bytes();
        let mut temp = [Temp {
            rotate: -1,
            lump: [-1; 8],
            flip: [true; 8],
        }; 29];
        let mut maxframe: i32 = -1;
        for l in first..end {
            let name = dir.name(l).expect("lump in range");
            if name[..4] != spritename[..4] {
                continue;
            }
            // `lumpinfo[l].name[4] - 'A'` with signed chars, passed as unsigned.
            let frame = (i32::from(name[4] as i8) - i32::from(b'A')) as u32;
            let rotation = (i32::from(name[5] as i8) - i32::from(b'0')) as u32;
            install(
                &mut temp,
                &mut maxframe,
                l,
                first,
                frame,
                rotation,
                false,
                spritename,
            )?;
            if name[6] != 0 {
                let frame = (i32::from(name[6] as i8) - i32::from(b'A')) as u32;
                let rotation = (i32::from(name[7] as i8) - i32::from(b'0')) as u32;
                install(
                    &mut temp,
                    &mut maxframe,
                    l,
                    first,
                    frame,
                    rotation,
                    true,
                    spritename,
                )?;
            }
        }
        if maxframe == -1 {
            out.push(SpriteDef::default());
            continue;
        }
        let count = (maxframe + 1) as usize;
        let text = name_text(spritename);
        let mut frames = Vec::with_capacity(count);
        for (frame, t) in temp.iter().take(count).enumerate() {
            let letter = char::from(b'A' + frame as u8);
            match t.rotate {
                -1 => {
                    return Err(ResourceError::Sprite(format!(
                        "No patches found for {text} frame {letter}"
                    )));
                }
                1 if t.lump.contains(&-1) => {
                    return Err(ResourceError::Sprite(format!(
                        "Sprite {text} frame {letter} is missing rotations"
                    )));
                }
                _ => {}
            }
            frames.push(SpriteFrame {
                rotate: t.rotate == 1,
                lump: t.lump,
                flip: t.flip,
            });
        }
        out.push(SpriteDef { frames });
    }
    Ok(out)
}

/// `R_InstallSpriteLump` (r_things.c:108-157).
#[expect(
    clippy::too_many_arguments,
    reason = "the C function's arguments plus its globals"
)]
fn install(
    temp: &mut [Temp; 29],
    maxframe: &mut i32,
    lump: usize,
    first: usize,
    frame: u32,
    rotation: u32,
    flipped: bool,
    spritename: &[u8],
) -> Result<(), ResourceError> {
    let text = name_text(spritename);
    if frame >= 29 || rotation > 8 {
        return Err(ResourceError::Sprite(format!(
            "Bad frame characters in lump {lump}"
        )));
    }
    let letter = char::from(b'A' + frame as u8);
    if frame as i32 > *maxframe {
        *maxframe = frame as i32;
    }
    let t = &mut temp[frame as usize];
    let rel = (lump - first) as i16;
    if rotation == 0 {
        if t.rotate == 0 {
            return Err(ResourceError::Sprite(format!(
                "Sprite {text} frame {letter} has multip rot=0 lump"
            )));
        }
        if t.rotate == 1 {
            return Err(ResourceError::Sprite(format!(
                "Sprite {text} frame {letter} has rotations and a rot=0 lump"
            )));
        }
        t.rotate = 0;
        t.lump = [rel; 8];
        t.flip = [flipped; 8];
        return Ok(());
    }
    if t.rotate == 0 {
        return Err(ResourceError::Sprite(format!(
            "Sprite {text} frame {letter} has rotations and a rot=0 lump"
        )));
    }
    t.rotate = 1;
    let r = (rotation - 1) as usize;
    if t.lump[r] != -1 {
        return Err(ResourceError::Sprite(format!(
            "Sprite {text} : {letter} : {} has two lumps mapped to it",
            char::from(b'1' + r as u8)
        )));
    }
    t.lump[r] = rel;
    t.flip[r] = flipped;
    Ok(())
}

/// `MAXVISSPRITES` (r_things.h:31).
pub const MAXVISSPRITES: usize = 128;
/// `MINZ` (r_things.c:46).
const MINZ: i32 = 4 << 16;
/// `FF_FRAMEMASK` (p_pspr.h:51).
const FF_FRAMEMASK: u32 = 0x7fff;
/// `ANG45` (tables.h:68).
const ANG45: u32 = 0x2000_0000;

/// The screen extent of a vissprite or weapon sprite (`vis->x1`, `vis->x2`). Step 2c adds the
/// rest of `vissprite_t` and the drawing.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VisSprite {
    pub x1: i32,
    pub x2: i32,
}

/// The r_things.c globals of one frame.
#[derive(Clone, Debug)]
pub(crate) struct ThingState {
    /// `vissprites[]` up to `vissprite_p`.
    pub vissprites: Vec<VisSprite>,
    /// `overflowsprite` was written (r_things.c:330-331).
    pub vissprite_drop: bool,
    /// `sec->validcount == validcount` for this frame (r_things.c:622-626).
    added: Vec<bool>,
    /// The weapon sprites' extents (`R_DrawPSprite`, r_things.c:676-698).
    psprites: Vec<VisSprite>,
}

impl ThingState {
    pub(crate) fn new(sectors: usize) -> Self {
        Self {
            vissprites: Vec::with_capacity(MAXVISSPRITES),
            vissprite_drop: false,
            added: vec![false; sectors],
            psprites: Vec::new(),
        }
    }
    /// The screen columns the frame's vissprites and weapon sprites span (step 2c draws them).
    pub(crate) fn pending_columns(&self, view: &crate::r_main::ViewSize) -> Vec<u16> {
        let mut cols = vec![false; crate::SCREENWIDTH];
        for v in self.vissprites.iter().chain(&self.psprites) {
            for x in v.x1..=v.x2 {
                cols[(x + view.viewwindowx) as usize] = true;
            }
        }
        (0..crate::SCREENWIDTH)
            .filter(|x| cols[*x])
            .map(|x| x as u16)
            .collect()
    }
}

/// One `R_DrawMaskedColumn` call's inputs (the `dc_*` and sprite globals, r_things.c:345-349).
pub(crate) struct MaskedColumn {
    /// The texture (`R_GetColumn (texnum, col)`).
    pub tex: usize,
    pub col: i32,
    /// `dc_x`.
    pub x: i32,
    /// `dc_texturemid` on entry.
    pub texturemid: i32,
    pub sprtopscreen: i32,
    pub spryscale: i32,
    /// `dc_iscale`.
    pub iscale: i32,
    /// `dc_colormap`.
    pub colormap: u8,
    pub mfloorclip: crate::r_segs::Clip,
    pub mceilingclip: crate::r_segs::Clip,
    pub feature: crate::Feature,
}

impl crate::FrameContext<'_> {
    /// A clip array entry `clip[x]` (`screenheightarray`, `negonearray` or openings).
    fn clip_at(&self, clip: crate::r_segs::Clip, x: i32) -> Result<i32, crate::RenderRefusal> {
        use crate::r_segs::Clip;
        match clip {
            Clip::ScreenHeight => Ok(self.view.viewheight),
            Clip::NegOne => Ok(-1),
            Clip::Openings(base) => self.read_opening(base.wrapping_add(x)).map(i32::from),
            Clip::None => Err(crate::RenderRefusal::RangeCheck("NULL clip array")),
        }
    }

    /// `R_DrawMaskedColumn` (r_things.c:351-388) for a masked midtexture column: every post of
    /// the patch column, clipped to `mceilingclip`/`mfloorclip`, through `R_DrawColumn`.
    pub(crate) fn draw_masked_column(
        &mut self,
        m: &MaskedColumn,
    ) -> Result<(), crate::RenderRefusal> {
        use crate::r_data::MaskedPosts;
        let res = self.res;
        let (bytes, mut at) = match res.masked_posts(m.tex, m.col) {
            MaskedPosts::Lump { bytes, at } => (bytes, at),
            MaskedPosts::Composite => return Err(crate::RenderRefusal::MaskedCompositeColumn),
            MaskedPosts::Undefined => return Err(crate::RenderRefusal::MaskedColumnUndefined),
        };
        let floor = self.clip_at(m.mfloorclip, m.x)?;
        let ceiling = self.clip_at(m.mceilingclip, m.x)?;
        loop {
            let topdelta = *bytes
                .get(at)
                .ok_or(crate::RenderRefusal::MaskedColumnUndefined)?;
            if topdelta == 0xff {
                break;
            }
            let length = *bytes
                .get(at + 1)
                .ok_or(crate::RenderRefusal::MaskedColumnUndefined)?;
            // Unclipped screen coordinates of the post (r_things.c:363-367).
            let topscreen = m
                .sprtopscreen
                .wrapping_add(m.spryscale.wrapping_mul(i32::from(topdelta)));
            let bottomscreen = topscreen.wrapping_add(m.spryscale.wrapping_mul(i32::from(length)));
            let mut yl = topscreen.wrapping_add((1 << 16) - 1) >> 16;
            let mut yh = bottomscreen.wrapping_sub(1) >> 16;
            if yh >= floor {
                yh = floor - 1;
            }
            if yl <= ceiling {
                yl = ceiling + 1;
            }
            if yl <= yh {
                self.draw_column(&crate::r_draw::ColumnDraw {
                    x: m.x,
                    yl,
                    yh,
                    iscale: m.iscale,
                    texturemid: m.texturemid.wrapping_sub(i32::from(topdelta) << 16),
                    colormap: m.colormap,
                    source: Resources::lump_source(bytes, at + 3),
                    origin: m.tex as u32 + 1,
                    feature: m.feature,
                })?;
            }
            at += usize::from(length) + 4;
        }
        Ok(())
    }

    /// `R_AddSprites` (r_things.c:613-640) with `R_ProjectSprite` (r_things.c:452-604) up to
    /// the vissprite's extent; the light and drawing fields are step 2c.
    pub(crate) fn add_sprites(&mut self, sector: usize) -> Result<(), crate::RenderRefusal> {
        if self.things.added[sector] {
            return Ok(());
        }
        self.things.added[sector] = true;
        let sim = self.sim;
        let order = sim.sector_actor_order(sector as u16).map_err(|e| {
            crate::RenderRefusal::Snapshot(format!("sector {sector} things: {e:?}"))
        })?;
        for id in order {
            let actor = sim
                .actor(id)
                .ok_or_else(|| crate::RenderRefusal::Snapshot("sector thing".into()))?;
            let life = actor
                .life()
                .ok_or_else(|| crate::RenderRefusal::Snapshot("thing without a state".into()))?;
            let [x, y] = actor.position().map(|f| f.bits());
            self.project_sprite(x, y, actor.angle(), life.sprite, life.frame)?;
        }
        Ok(())
    }

    /// The sprite frame `sprites[sprite].spriteframes[frame & FF_FRAMEMASK]` with the RANGECHECK
    /// of r_things.c:506-516 (and 659-669 for weapon sprites).
    fn sprite_frame(&self, sprite: u16, frame: u32) -> Result<SpriteFrame, crate::RenderRefusal> {
        let def =
            self.res
                .sprites
                .get(usize::from(sprite))
                .ok_or(crate::RenderRefusal::RangeCheck(
                    "R_ProjectSprite: invalid sprite number",
                ))?;
        def.frames
            .get((frame & FF_FRAMEMASK) as usize)
            .copied()
            .ok_or(crate::RenderRefusal::RangeCheck(
                "R_ProjectSprite: invalid sprite frame",
            ))
    }

    /// `spritewidth`, `spriteoffset` of sprite lump `lump` (relative to `firstspritelump`).
    fn sprite_lump(&self, lump: i16) -> Result<[i32; 3], crate::RenderRefusal> {
        usize::try_from(lump)
            .ok()
            .and_then(|l| self.res.sprite_lumps.get(l))
            .copied()
            .ok_or(crate::RenderRefusal::RangeCheck("sprite lump"))
    }

    /// `R_ProjectSprite` (r_things.c:452-604) through `R_NewVisSprite` (r_things.c:326-335).
    fn project_sprite(
        &mut self,
        x: i32,
        y: i32,
        angle: u32,
        sprite: u16,
        frame: u32,
    ) -> Result<(), crate::RenderRefusal> {
        use crate::r_main::{fixed_div, fixed_mul, point_to_angle};
        let f = self.frame;
        // Transform the origin point (r_things.c:484-488).
        let tr_x = x.wrapping_sub(f.viewx);
        let tr_y = y.wrapping_sub(f.viewy);
        let gxt = fixed_mul(tr_x, f.viewcos);
        let gyt = fixed_mul(tr_y, f.viewsin).wrapping_neg();
        let tz = gxt.wrapping_sub(gyt);
        // Thing is behind view plane? (r_things.c:491-493)
        if tz < MINZ {
            return Ok(());
        }
        let xscale = fixed_div(self.view.projection, tz)?;
        let gxt = fixed_mul(tr_x, f.viewsin).wrapping_neg();
        let gyt = fixed_mul(tr_y, f.viewcos);
        let mut tx = gyt.wrapping_add(gxt).wrapping_neg();
        // Too far off the side? (r_things.c:500-502)
        if tx.wrapping_abs() > tz.wrapping_shl(2) {
            return Ok(());
        }
        let sprframe = self.sprite_frame(sprite, frame)?;
        let lump = if sprframe.rotate {
            // Choose a rotation based on the player view (r_things.c:519-526).
            let ang = point_to_angle(x, y, f.viewx, f.viewy);
            let rot = (ang
                .wrapping_sub(angle)
                .wrapping_add((ANG45 / 2).wrapping_mul(9))
                >> 29) as usize;
            sprframe.lump[rot]
        } else {
            sprframe.lump[0]
        };
        let [width, offset, _top] = self.sprite_lump(lump)?;
        // Edges of the shape (r_things.c:535-548).
        tx = tx.wrapping_sub(offset);
        let x1 = self.view.centerxfrac.wrapping_add(fixed_mul(tx, xscale)) >> 16;
        if x1 > self.view.viewwidth {
            return Ok(());
        }
        tx = tx.wrapping_add(width);
        let x2 = (self.view.centerxfrac.wrapping_add(fixed_mul(tx, xscale)) >> 16) - 1;
        if x2 < 0 {
            return Ok(());
        }
        // `iscale = FixedDiv (FRACUNIT, xscale)` (r_things.c:561) can refuse.
        fixed_div(1 << 16, xscale)?;
        // R_NewVisSprite: the 129th and later write `overflowsprite` (r_things.c:330-331).
        if self.things.vissprites.len() == MAXVISSPRITES {
            self.things.vissprite_drop = true;
            return Ok(());
        }
        self.things.vissprites.push(VisSprite {
            x1: x1.max(0),
            x2: x2.min(self.view.viewwidth - 1),
        });
        Ok(())
    }

    /// The extent of each weapon sprite `R_DrawPlayerSprites` would draw (r_things.c:744-776,
    /// `R_DrawPSprite` r_things.c:643-698): psprites with a state, weapon then flash.
    fn project_psprites(
        &mut self,
        weapons: &doom_sim::PlayerWeaponState,
    ) -> Result<(), crate::RenderRefusal> {
        use crate::r_main::fixed_mul;
        let registry = self
            .sim
            .actor_registry()
            .ok_or_else(|| crate::RenderRefusal::Snapshot("actor registry".into()))?;
        for psp in &weapons.psprites {
            let Some(state) = psp.state else { continue };
            let def = registry
                .state(state)
                .ok_or_else(|| crate::RenderRefusal::Snapshot("psprite state".into()))?;
            let sprframe = self.sprite_frame(def.sprite, def.frame)?;
            let [width, offset, _top] = self.sprite_lump(sprframe.lump[0])?;
            let mut tx = psp.position[0].bits().wrapping_sub(160 << 16);
            tx = tx.wrapping_sub(offset);
            let x1 = self
                .view
                .centerxfrac
                .wrapping_add(fixed_mul(tx, self.view.pspritescale))
                >> 16;
            if x1 > self.view.viewwidth {
                continue;
            }
            tx = tx.wrapping_add(width);
            let x2 = (self
                .view
                .centerxfrac
                .wrapping_add(fixed_mul(tx, self.view.pspritescale))
                >> 16)
                - 1;
            if x2 < 0 {
                continue;
            }
            self.things.psprites.push(VisSprite {
                x1: x1.max(0),
                x2: x2.min(self.view.viewwidth - 1),
            });
        }
        Ok(())
    }

    /// `R_DrawMasked` (r_things.c:958-987): the vissprites back to front (step 2c), every
    /// remaining masked midtexture from the last drawseg down, then the weapon sprites (step
    /// 2c; only their extents are computed here).
    pub(crate) fn draw_masked(
        &mut self,
        weapons: &doom_sim::PlayerWeaponState,
    ) -> Result<(), crate::RenderRefusal> {
        for i in (0..self.bsp.drawsegs.len()).rev() {
            let ds = self.bsp.drawsegs[i];
            if ds.maskedtexturecol.is_some() {
                self.render_masked_seg_range(i, ds.x1, ds.x2)?;
            }
        }
        self.project_psprites(weapons)
    }
}
