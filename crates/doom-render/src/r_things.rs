//! r_things.c: sprite definitions (`R_InitSprites`, `R_InitSpriteDefs`,
//! `R_InstallSpriteLump`), vissprites (`R_AddSprites`, `R_ProjectSprite`, `R_NewVisSprite`,
//! `R_SortVisSprites`), masked drawing (`R_DrawMasked`, `R_DrawSprite`, `R_DrawVisSprite`,
//! `R_DrawMaskedColumn`) and the weapon sprites (`R_DrawPlayerSprites`, `R_DrawPSprite`).
use crate::r_data::{
    ResourceError, Resources, SpriteDef, SpriteFrame, name_text, patch_column_offset, patch_header,
};
use crate::r_main::{LIGHTLEVELS, LIGHTSCALESHIFT, MAXLIGHTSCALE, SCREENWIDTH};
use crate::r_segs::{Clip, LIGHTSEGSHIFT, SIL_BOTTOM, SIL_TOP};
use crate::{Feature, RenderRefusal};
use doom_sim::PlayerSlot;
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
/// `BASEYCENTER` (r_things.c:47).
const BASEYCENTER: i32 = 100;
/// `FF_FULLBRIGHT` (p_pspr.h:50).
const FF_FULLBRIGHT: u32 = 0x8000;
/// `FF_FRAMEMASK` (p_pspr.h:51).
const FF_FRAMEMASK: u32 = 0x7fff;
/// `MF_SHADOW` (p_mobj.h:167).
const MF_SHADOW: u32 = 0x40000;
/// `MF_TRANSLATION` (p_mobj.h:199).
const MF_TRANSLATION: u32 = 0xc00_0000;
/// `pw_invisibility` (doomdef.h:229-236: invulnerability, strength, invisibility, ...).
const PW_INVISIBILITY: usize = 2;
/// `ANG45` (tables.h:68).
const ANG45: u32 = 0x2000_0000;

/// `vissprite_t` (r_defs.h:375-409) without the sort links, plus the sprite and frame for
/// first-difference reports.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VisSprite {
    pub x1: i32,
    pub x2: i32,
    pub gx: i32,
    pub gy: i32,
    pub gz: i32,
    pub gzt: i32,
    pub startfrac: i32,
    pub scale: i32,
    pub xiscale: i32,
    pub texturemid: i32,
    /// `patch`, relative to `firstspritelump`.
    pub patch: i16,
    /// `colormap` as a colour-map number; None is the NULL colormap (shadow draw).
    pub colormap: Option<u8>,
    pub mobjflags: u32,
    sprite: u16,
    frame: u16,
}

/// `colfunc` as `R_DrawVisSprite` selects it (r_things.c:410-422).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ColFunc {
    /// `basecolfunc` (`R_DrawColumn`).
    Base,
    /// `fuzzcolfunc` (`R_DrawFuzzColumn`).
    Fuzz,
    /// `R_DrawTranslatedColumn`: refused (player translation, decision R1).
    Translated,
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
    /// `spritelights` as a `scalelight` row (r_things.c:630-635, 757-762).
    spritelights: usize,
    /// `R_DrawSprite`'s locals `clipbot` and `cliptop` (r_things.c:845-846).
    clipbot: [i16; SCREENWIDTH],
    cliptop: [i16; SCREENWIDTH],
}

impl ThingState {
    pub(crate) fn new(sectors: usize) -> Self {
        Self {
            vissprites: Vec::with_capacity(MAXVISSPRITES),
            vissprite_drop: false,
            added: vec![false; sectors],
            spritelights: 0,
            clipbot: [0; SCREENWIDTH],
            cliptop: [0; SCREENWIDTH],
        }
    }
}

/// One `R_DrawMaskedColumn` call's inputs (the `dc_*` and sprite globals, r_things.c:345-349).
pub(crate) struct MaskedColumn<'r> {
    /// The block holding the column's posts (a patch lump).
    pub bytes: &'r [u8],
    /// The offset of the first post header in `bytes`.
    pub at: usize,
    /// `dc_x`.
    pub x: i32,
    /// `dc_texturemid` on entry.
    pub texturemid: i32,
    pub sprtopscreen: i32,
    pub spryscale: i32,
    /// `dc_iscale`.
    pub iscale: i32,
    /// `dc_colormap` (unused by the fuzz drawer).
    pub colormap: u8,
    /// `colfunc`.
    pub colfunc: ColFunc,
    pub mfloorclip: Clip,
    pub mceilingclip: Clip,
    /// The taint origin of an undefined texel read (nonzero; decision R4).
    pub origin: u32,
    /// The stage, for first-difference reports.
    pub feature: Feature,
}

/// The `scalelight` row of a sector light plus `extralight`, clamped (r_things.c:628-635,
/// 753-762).
fn light_row(lightlevel: i32, extralight: i32) -> usize {
    ((lightlevel >> LIGHTSEGSHIFT) + extralight).clamp(0, LIGHTLEVELS as i32 - 1) as usize
}

impl crate::FrameContext<'_> {
    /// A clip array entry `clip[x]` (`screenheightarray`, `negonearray`, openings, or
    /// `R_DrawSprite`'s `clipbot`/`cliptop`).
    pub(crate) fn clip_at(&self, clip: Clip, x: i32) -> Result<i32, RenderRefusal> {
        let local = |a: &[i16; SCREENWIDTH]| {
            usize::try_from(x)
                .ok()
                .and_then(|i| a.get(i))
                .map(|v| i32::from(*v))
                .ok_or(RenderRefusal::RangeCheck("R_DrawSprite clip array"))
        };
        match clip {
            Clip::ScreenHeight => Ok(self.view.viewheight),
            Clip::NegOne => Ok(-1),
            Clip::Openings(base) => self.read_opening(base.wrapping_add(x)).map(i32::from),
            Clip::SpriteBottom => local(&self.things.clipbot),
            Clip::SpriteTop => local(&self.things.cliptop),
            Clip::None => Err(RenderRefusal::RangeCheck("NULL clip array")),
        }
    }

    /// `R_DrawMaskedColumn` (r_things.c:351-388): every post of the column, clipped to
    /// `mceilingclip[dc_x]`/`mfloorclip[dc_x]`, through `colfunc`.
    pub(crate) fn draw_masked_column(&mut self, m: &MaskedColumn<'_>) -> Result<(), RenderRefusal> {
        let bytes = m.bytes;
        let mut at = m.at;
        loop {
            let topdelta = *bytes.get(at).ok_or(RenderRefusal::MaskedColumnUndefined)?;
            if topdelta == 0xff {
                break;
            }
            let length = *bytes
                .get(at + 1)
                .ok_or(RenderRefusal::MaskedColumnUndefined)?;
            // Unclipped screen coordinates of the post (r_things.c:363-367).
            let topscreen = m
                .sprtopscreen
                .wrapping_add(m.spryscale.wrapping_mul(i32::from(topdelta)));
            let bottomscreen = topscreen.wrapping_add(m.spryscale.wrapping_mul(i32::from(length)));
            let mut yl = topscreen.wrapping_add((1 << 16) - 1) >> 16;
            let mut yh = bottomscreen.wrapping_sub(1) >> 16;
            // r_things.c:369-372
            let floor = self.clip_at(m.mfloorclip, m.x)?;
            if yh >= floor {
                yh = floor - 1;
            }
            let ceiling = self.clip_at(m.mceilingclip, m.x)?;
            if yl <= ceiling {
                yl = ceiling + 1;
            }
            if yl <= yh {
                match m.colfunc {
                    ColFunc::Base => self.draw_column(&crate::r_draw::ColumnDraw {
                        x: m.x,
                        yl,
                        yh,
                        iscale: m.iscale,
                        texturemid: m.texturemid.wrapping_sub(i32::from(topdelta) << 16),
                        colormap: m.colormap,
                        source: Resources::lump_source(bytes, at + 3),
                        origin: m.origin,
                        feature: m.feature,
                    })?,
                    ColFunc::Fuzz => self.draw_fuzz_column(m.x, yl, yh, m.feature)?,
                    // The genuine runner aborts at the call (decision R1).
                    ColFunc::Translated => return Err(RenderRefusal::PlayerTranslation),
                }
            }
            at += usize::from(length) + 4;
        }
        Ok(())
    }

    /// `R_AddSprites` (r_things.c:613-640): each sector's things once per frame, on the first
    /// of its subsectors the BSP traversal reaches, in thinglist order (most recently linked
    /// first, `P_SetThingPosition` p_maputl.c:414-420).
    pub(crate) fn add_sprites(&mut self, sector: usize) -> Result<(), RenderRefusal> {
        if self.things.added[sector] {
            return Ok(());
        }
        self.things.added[sector] = true;
        self.things.spritelights =
            light_row(self.sectors[sector].lightlevel, self.frame.extralight);
        let sim = self.sim;
        let order = sim
            .sector_actor_order(sector as u16)
            .map_err(|e| RenderRefusal::Snapshot(format!("sector {sector} things: {e:?}")))?;
        for id in order {
            let actor = sim
                .actor(id)
                .ok_or_else(|| RenderRefusal::Snapshot("sector thing".into()))?;
            let life = actor
                .life()
                .ok_or_else(|| RenderRefusal::Snapshot("thing without a state".into()))?;
            let [x, y] = actor.position().map(|f| f.bits());
            self.project_sprite(&Thing {
                x,
                y,
                z: actor.z().bits(),
                angle: actor.angle(),
                sprite: life.sprite,
                frame: life.frame,
                flags: life.flags,
            })?;
        }
        Ok(())
    }

    /// The sprite frame `sprites[sprite].spriteframes[frame & FF_FRAMEMASK]` with the RANGECHECK
    /// of r_things.c:506-516 (and 659-669 for weapon sprites).
    fn sprite_frame(&self, sprite: u16, frame: u32) -> Result<SpriteFrame, RenderRefusal> {
        let def = self
            .res
            .sprites
            .get(usize::from(sprite))
            .ok_or(RenderRefusal::RangeCheck(
                "R_ProjectSprite: invalid sprite number",
            ))?;
        def.frames
            .get((frame & FF_FRAMEMASK) as usize)
            .copied()
            .ok_or(RenderRefusal::RangeCheck(
                "R_ProjectSprite: invalid sprite frame",
            ))
    }

    /// `spritewidth`, `spriteoffset`, `spritetopoffset` of sprite lump `lump` (relative to
    /// `firstspritelump`).
    fn sprite_lump(&self, lump: i16) -> Result<[i32; 3], RenderRefusal> {
        usize::try_from(lump)
            .ok()
            .and_then(|l| self.res.sprite_lumps.get(l))
            .copied()
            .ok_or(RenderRefusal::RangeCheck("sprite lump"))
    }

    /// `R_ProjectSprite` (r_things.c:452-604) with `R_NewVisSprite` (r_things.c:326-335).
    fn project_sprite(&mut self, thing: &Thing) -> Result<(), RenderRefusal> {
        use crate::r_main::{fixed_div, fixed_mul, point_to_angle};
        let f = self.frame;
        // Transform the origin point (r_things.c:483-489).
        let tr_x = thing.x.wrapping_sub(f.viewx);
        let tr_y = thing.y.wrapping_sub(f.viewy);
        let gxt = fixed_mul(tr_x, f.viewcos);
        let gyt = fixed_mul(tr_y, f.viewsin).wrapping_neg();
        let tz = gxt.wrapping_sub(gyt);
        // Thing is behind view plane? (r_things.c:492-493)
        if tz < MINZ {
            return Ok(());
        }
        let xscale = fixed_div(self.view.projection, tz)?;
        let gxt = fixed_mul(tr_x, f.viewsin).wrapping_neg();
        let gyt = fixed_mul(tr_y, f.viewcos);
        let mut tx = gyt.wrapping_add(gxt).wrapping_neg();
        // Too far off the side? (r_things.c:502-503; `tz<<2` wraps as in the strict build)
        if tx.wrapping_abs() > tz.wrapping_shl(2) {
            return Ok(());
        }
        let sprframe = self.sprite_frame(thing.sprite, thing.frame)?;
        let (lump, flip) = if sprframe.rotate {
            // Choose a rotation based on the player view (r_things.c:519-526).
            let ang = point_to_angle(thing.x, thing.y, f.viewx, f.viewy);
            let rot = (ang
                .wrapping_sub(thing.angle)
                .wrapping_add((ANG45 / 2).wrapping_mul(9))
                >> 29) as usize;
            (sprframe.lump[rot], sprframe.flip[rot])
        } else {
            (sprframe.lump[0], sprframe.flip[0])
        };
        let [width, offset, topoffset] = self.sprite_lump(lump)?;
        // Edges of the shape (r_things.c:535-547).
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
        // R_NewVisSprite: the 129th and later write `overflowsprite`, which is never sorted
        // or drawn (r_things.c:330-331; a silent drop, decision R3).
        let full = self.things.vissprites.len() == MAXVISSPRITES;
        let gzt = thing.z.wrapping_add(topoffset);
        let vx1 = x1.max(0);
        let iscale = fixed_div(1 << 16, xscale)?;
        let (mut startfrac, xiscale) = if flip {
            (width - 1, iscale.wrapping_neg())
        } else {
            (0, iscale)
        };
        if vx1 > x1 {
            startfrac = startfrac.wrapping_add(xiscale.wrapping_mul(vx1 - x1));
        }
        // Light level (r_things.c:578-603), first match wins.
        let colormap = if thing.flags & MF_SHADOW != 0 {
            None
        } else if f.fixedcolormap != 0 {
            Some(f.fixedcolormap as u8)
        } else if thing.frame & FF_FULLBRIGHT != 0 {
            Some(0)
        } else {
            let index = xscale >> (LIGHTSCALESHIFT - self.view.detailshift);
            let index = usize::try_from(index)
                .map_err(|_| RenderRefusal::TableRead("spritelights (negative index)"))?
                .min(MAXLIGHTSCALE - 1);
            Some(self.view.scalelight[self.things.spritelights][index])
        };
        if full {
            self.things.vissprite_drop = true;
            return Ok(());
        }
        self.things.vissprites.push(VisSprite {
            x1: vx1,
            x2: x2.min(self.view.viewwidth - 1),
            gx: thing.x,
            gy: thing.y,
            gz: thing.z,
            gzt,
            startfrac,
            scale: xscale << self.view.detailshift,
            xiscale,
            texturemid: gzt.wrapping_sub(f.viewz),
            patch: lump,
            colormap,
            mobjflags: thing.flags,
            sprite: thing.sprite,
            frame: (thing.frame & FF_FRAMEMASK) as u16,
        });
        Ok(())
    }

    /// `R_SortVisSprites` (r_things.c:784-835): `count` passes, each taking the first
    /// remaining vissprite (in `vissprites[]` order) whose scale is strictly below the best so
    /// far and appending it at the tail of the sorted list. Returns the `vissprites[]` indices
    /// far to near; equal scales keep projection order.
    fn sort_vis_sprites(&self) -> Result<Vec<usize>, RenderRefusal> {
        let count = self.things.vissprites.len();
        let mut unsorted: Vec<usize> = (0..count).collect();
        let mut sorted = Vec::with_capacity(count);
        for _ in 0..count {
            let mut bestscale = i32::MAX;
            let mut best = None;
            for (pos, &ds) in unsorted.iter().enumerate() {
                if self.things.vissprites[ds].scale < bestscale {
                    bestscale = self.things.vissprites[ds].scale;
                    best = Some(pos);
                }
            }
            // No scale below MAXINT: the genuine `best` is uninitialised or stale
            // (r_things.c:815) and the list is corrupted.
            let pos = best.ok_or(RenderRefusal::StaleGlobal("R_SortVisSprites best"))?;
            sorted.push(unsorted.remove(pos));
        }
        Ok(sorted)
    }

    /// `R_DrawSprite` (r_things.c:842-950) for `vissprites[v]`: clip against every drawseg
    /// from the last emitted backwards, drawing the masked midtextures of segs behind the
    /// sprite first, then `R_DrawVisSprite`.
    fn draw_sprite(&mut self, v: usize) -> Result<(), RenderRefusal> {
        use crate::r_main::point_on_side;
        let spr = self.things.vissprites[v];
        for x in spr.x1..=spr.x2 {
            self.things.clipbot[x as usize] = -2;
            self.things.cliptop[x as usize] = -2;
        }
        // Scan drawsegs from end to start for obscuring segs (r_things.c:860-933).
        for i in (0..self.bsp.drawsegs.len()).rev() {
            let ds = self.bsp.drawsegs[i];
            if ds.x1 > spr.x2
                || ds.x2 < spr.x1
                || (ds.silhouette == 0 && ds.maskedtexturecol.is_none())
            {
                continue;
            }
            let r1 = if ds.x1 < spr.x1 { spr.x1 } else { ds.x1 };
            let r2 = if ds.x2 > spr.x2 { spr.x2 } else { ds.x2 };
            let (lowscale, scale) = if ds.scale1 > ds.scale2 {
                (ds.scale2, ds.scale1)
            } else {
                (ds.scale1, ds.scale2)
            };
            let behind = scale < spr.scale
                || (lowscale < spr.scale && {
                    // R_PointOnSegSide (r_main.c:217-273) is R_PointOnSide on the seg.
                    let seg = &self.level.geometry.segs[ds.curline];
                    let node = [
                        seg.v1[0],
                        seg.v1[1],
                        seg.v2[0].wrapping_sub(seg.v1[0]),
                        seg.v2[1].wrapping_sub(seg.v1[1]),
                    ];
                    point_on_side(spr.gx, spr.gy, node) == 0
                });
            if behind {
                // Masked mid texture of a seg behind the sprite (r_things.c:891-893).
                if ds.maskedtexturecol.is_some() {
                    self.render_masked_seg_range(i, r1, r2)?;
                }
                continue;
            }
            // Clip this piece of the sprite (r_things.c:898-931).
            let mut silhouette = ds.silhouette;
            if spr.gz >= ds.bsilheight {
                silhouette &= !SIL_BOTTOM;
            }
            if spr.gzt <= ds.tsilheight {
                silhouette &= !SIL_TOP;
            }
            let bottom = silhouette == 1 || silhouette == 3;
            let top = silhouette == 2 || silhouette == 3;
            for x in r1..=r2 {
                if bottom && self.things.clipbot[x as usize] == -2 {
                    self.things.clipbot[x as usize] = self.clip_at(ds.sprbottomclip, x)? as i16;
                }
                if top && self.things.cliptop[x as usize] == -2 {
                    self.things.cliptop[x as usize] = self.clip_at(ds.sprtopclip, x)? as i16;
                }
            }
        }
        // Unclipped columns (r_things.c:938-945).
        for x in spr.x1..=spr.x2 {
            let x = x as usize;
            if self.things.clipbot[x] == -2 {
                self.things.clipbot[x] = self.view.viewheight as i16;
            }
            if self.things.cliptop[x] == -2 {
                self.things.cliptop[x] = -1;
            }
        }
        let feature = if spr.colormap.is_none() {
            Feature::Fuzz {
                psprite: false,
                vissprite: v as u16,
            }
        } else {
            Feature::Sprite {
                vissprite: v as u16,
                sprite: spr.sprite,
                frame: spr.frame,
            }
        };
        self.draw_vis_sprite(&spr, Clip::SpriteBottom, Clip::SpriteTop, feature)
    }

    /// `R_DrawVisSprite (vis, vis->x1, vis->x2)` (r_things.c:396-443) with `mfloorclip` and
    /// `mceilingclip` set by the caller.
    fn draw_vis_sprite(
        &mut self,
        vis: &VisSprite,
        mfloorclip: Clip,
        mceilingclip: Clip,
        feature: Feature,
    ) -> Result<(), RenderRefusal> {
        use crate::r_main::fixed_mul;
        let res = self.res;
        let lump = res.firstspritelump + vis.patch as usize;
        let patch = res.lump(lump);
        let width = patch_header(patch)
            .ok_or(RenderRefusal::RangeCheck("sprite patch header"))?
            .width;
        let colfunc = match vis.colormap {
            None => ColFunc::Fuzz,
            Some(_) if vis.mobjflags & MF_TRANSLATION != 0 => ColFunc::Translated,
            Some(_) => ColFunc::Base,
        };
        let iscale = vis.xiscale.wrapping_abs() >> self.view.detailshift;
        let texturemid = vis.texturemid;
        let mut frac = vis.startfrac;
        let spryscale = vis.scale;
        let sprtopscreen = self
            .view
            .centeryfrac
            .wrapping_sub(fixed_mul(texturemid, spryscale));
        for x in vis.x1..=vis.x2 {
            let texturecolumn = frac >> 16;
            // RANGECHECK (r_things.c:432-436).
            if texturecolumn < 0 || texturecolumn >= i32::from(width) {
                return Err(RenderRefusal::RangeCheck(
                    "R_DrawSpriteRange: bad texturecolumn",
                ));
            }
            let at = patch_column_offset(patch, texturecolumn)
                .and_then(|o| usize::try_from(o).ok())
                .ok_or(RenderRefusal::MaskedColumnUndefined)?;
            self.draw_masked_column(&MaskedColumn {
                bytes: patch,
                at,
                x,
                texturemid,
                sprtopscreen,
                spryscale,
                iscale,
                colormap: vis.colormap.unwrap_or(0),
                colfunc,
                mfloorclip,
                mceilingclip,
                origin: lump as u32 | 0x4000_0000,
                feature,
            })?;
            frac = frac.wrapping_add(vis.xiscale);
        }
        Ok(())
    }

    /// `R_DrawPlayerSprites` (r_things.c:744-776) with `R_DrawPSprite` (r_things.c:643-739):
    /// the weapon, then the flash, each with a state.
    fn draw_player_sprites(
        &mut self,
        weapons: &doom_sim::PlayerWeaponState,
    ) -> Result<(), RenderRefusal> {
        use crate::r_main::fixed_mul;
        let sim = self.sim;
        let player = sim
            .original_player(PlayerSlot::One)
            .map_err(|_| RenderRefusal::NoPlayer)?;
        let mo = player
            .mo
            .and_then(|id| sim.actor(id))
            .ok_or(RenderRefusal::NoPlayer)?;
        let sector = usize::from(mo.location().ok_or(RenderRefusal::NoPlayer)?.sector);
        let lightlevel = self
            .sectors
            .get(sector)
            .ok_or(RenderRefusal::NoPlayer)?
            .lightlevel;
        // Light level (r_things.c:753-762).
        self.things.spritelights = light_row(lightlevel, self.frame.extralight);
        let invisibility = sim
            .original_player_powers(PlayerSlot::One)
            .map_err(|_| RenderRefusal::NoPlayer)?
            .powers[PW_INVISIBILITY];
        let registry = sim
            .actor_registry()
            .ok_or_else(|| RenderRefusal::Snapshot("actor registry".into()))?;
        for (i, psp) in weapons.psprites.iter().enumerate() {
            let Some(state) = psp.state else { continue };
            let def = registry
                .state(state)
                .ok_or_else(|| RenderRefusal::Snapshot("psprite state".into()))?;
            let sprframe = self.sprite_frame(def.sprite, def.frame)?;
            let (lump, flip) = (sprframe.lump[0], sprframe.flip[0]);
            let [width, offset, topoffset] = self.sprite_lump(lump)?;
            // Calculate edges of the shape (r_things.c:676-690).
            let [sx, sy] = psp.position.map(|p| p.bits());
            let mut tx = sx.wrapping_sub(160 << 16);
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
            // The local `avis` (r_things.c:693-714).
            let vx1 = x1.max(0);
            let (mut startfrac, xiscale) = if flip {
                (width - 1, self.view.pspriteiscale.wrapping_neg())
            } else {
                (0, self.view.pspriteiscale)
            };
            if vx1 > x1 {
                startfrac = startfrac.wrapping_add(xiscale.wrapping_mul(vx1 - x1));
            }
            // r_things.c:716-736, first match wins.
            let colormap = if invisibility > 4 * 32 || invisibility & 8 != 0 {
                None
            } else if self.frame.fixedcolormap != 0 {
                Some(self.frame.fixedcolormap as u8)
            } else if def.frame & FF_FULLBRIGHT != 0 {
                Some(0)
            } else {
                Some(self.view.scalelight[self.things.spritelights][MAXLIGHTSCALE - 1])
            };
            let vis = VisSprite {
                x1: vx1,
                x2: x2.min(self.view.viewwidth - 1),
                gx: 0,
                gy: 0,
                gz: 0,
                gzt: 0,
                startfrac,
                scale: self.view.pspritescale << self.view.detailshift,
                xiscale,
                texturemid: ((BASEYCENTER << 16) + (1 << 15))
                    .wrapping_sub(sy.wrapping_sub(topoffset)),
                patch: lump,
                colormap,
                mobjflags: 0,
                sprite: def.sprite,
                frame: (def.frame & FF_FRAMEMASK) as u16,
            };
            let feature = if colormap.is_none() {
                Feature::Fuzz {
                    psprite: true,
                    vissprite: i as u16,
                }
            } else {
                Feature::PSprite {
                    psprite: i as u8,
                    sprite: vis.sprite,
                    frame: vis.frame,
                }
            };
            // mfloorclip = screenheightarray, mceilingclip = negonearray (r_things.c:765-766).
            self.draw_vis_sprite(&vis, Clip::ScreenHeight, Clip::NegOne, feature)?;
        }
        Ok(())
    }

    /// `R_DrawMasked` (r_things.c:958-987): the vissprites back to front, every remaining
    /// masked midtexture from the last drawseg down, then the weapon sprites
    /// (`viewangleoffset` is always 0, investigation §2.1.3).
    pub(crate) fn draw_masked(
        &mut self,
        weapons: &doom_sim::PlayerWeaponState,
    ) -> Result<(), RenderRefusal> {
        for v in self.sort_vis_sprites()? {
            self.draw_sprite(v)?;
        }
        for i in (0..self.bsp.drawsegs.len()).rev() {
            let ds = self.bsp.drawsegs[i];
            if ds.maskedtexturecol.is_some() {
                self.render_masked_seg_range(i, ds.x1, ds.x2)?;
            }
        }
        self.draw_player_sprites(weapons)
    }
}

/// What `R_ProjectSprite` reads of a thing (r_things.c:483-603).
struct Thing {
    x: i32,
    y: i32,
    z: i32,
    angle: u32,
    sprite: u16,
    frame: u32,
    flags: u32,
}

/// Guard tests below the public API: the post walk, clip arrays and vissprite sort are reached
/// by `Renderer::render` only with data the projection and the BSP have already validated (every
/// committed golden frame passes these guards), so the refusals are exercised by calling the
/// crate-private functions directly on the `amp.wad` fixture (`test_support::with_ctx`).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{with_ctx, with_ctx_res};

    /// One post (topdelta 0, length 4, pad, four texels, pad) and the 0xff terminator
    /// (the patch column format, investigation §2.6).
    const COLUMN: [u8; 9] = [0, 4, 0, 1, 2, 3, 4, 0, 0xff];

    fn masked<'a>(bytes: &'a [u8], colfunc: ColFunc) -> MaskedColumn<'a> {
        MaskedColumn {
            bytes,
            at: 0,
            x: 10,
            texturemid: 0,
            // The post's rows are 10..=13: `topscreen = sprtopscreen + spryscale * topdelta`,
            // `yl = (topscreen + FRACUNIT - 1) >> 16`, `yh = (bottomscreen - 1) >> 16`
            // (r_things.c:363-367).
            sprtopscreen: 10 << 16,
            spryscale: 1 << 16,
            iscale: 1 << 16,
            colormap: 0,
            colfunc,
            mfloorclip: Clip::ScreenHeight,
            mceilingclip: Clip::NegOne,
            origin: 1,
            feature: Feature::Untouched,
        }
    }

    #[test]
    fn translated_column_is_refused_as_player_translation() {
        // Guard: R_DrawTranslatedColumn (r_draw.c:382-447) belongs to MF_TRANSLATION things,
        // which only players 2-4 carry (p_mobj.c:668-669, investigation §2.2.5); decision R1
        // refuses it. The public API cannot reach it: player 1 and every spawned thing have no
        // translation bits.
        with_ctx(11, |ctx| {
            assert_eq!(
                ctx.draw_masked_column(&masked(&COLUMN, ColFunc::Translated)),
                Err(RenderRefusal::PlayerTranslation)
            );
            // The same column through the base drawer is drawn.
            assert_eq!(
                ctx.draw_masked_column(&masked(&COLUMN, ColFunc::Base)),
                Ok(())
            );
        });
    }

    #[test]
    fn post_walk_that_leaves_the_lump_is_refused_as_undefined() {
        // Guard: r_things.c:359-385 walks `column->topdelta != 0xff` with no bound; a column
        // whose terminator or length byte lies outside the patch lump reads zone memory
        // (`MaskedColumnUndefined`, decision R3). The golden rooms' patches are well formed.
        with_ctx(11, |ctx| {
            for bytes in [&[][..], &[0][..], &COLUMN[..8]] {
                assert_eq!(
                    ctx.draw_masked_column(&masked(bytes, ColFunc::Base)),
                    Err(RenderRefusal::MaskedColumnUndefined),
                    "{bytes:?}"
                );
            }
            // A post starting past the end of the lump.
            let mut m = masked(&COLUMN, ColFunc::Base);
            m.at = 50;
            assert_eq!(
                ctx.draw_masked_column(&m),
                Err(RenderRefusal::MaskedColumnUndefined)
            );
        });
    }

    #[test]
    fn clip_arrays_refuse_what_the_genuine_code_reads_outside() {
        // Guard: `clipbot`/`cliptop` are stack arrays of SCREENWIDTH entries (r_things.c:845-846)
        // and a NULL `mfloorclip`/`mceilingclip` is a null dereference (investigation §2.9).
        with_ctx(11, |ctx| {
            assert_eq!(
                ctx.clip_at(Clip::None, 0),
                Err(RenderRefusal::RangeCheck("NULL clip array"))
            );
            for clip in [Clip::SpriteBottom, Clip::SpriteTop] {
                for x in [-1, 320] {
                    assert_eq!(
                        ctx.clip_at(clip, x),
                        Err(RenderRefusal::RangeCheck("R_DrawSprite clip array")),
                        "{clip:?} {x}"
                    );
                }
                assert_eq!(ctx.clip_at(clip, 319), Ok(0));
            }
            // `screenheightarray` and `negonearray` (r_things.c:303-306).
            assert_eq!(ctx.clip_at(Clip::ScreenHeight, 7), Ok(200));
            assert_eq!(ctx.clip_at(Clip::NegOne, 7), Ok(-1));
        });
    }

    fn sprite(scale: i32, x1: i32) -> VisSprite {
        VisSprite {
            x1,
            x2: x1,
            gx: 0,
            gy: 0,
            gz: 0,
            gzt: 0,
            startfrac: 0,
            scale,
            xiscale: 0,
            texturemid: 0,
            patch: 0,
            colormap: Some(0),
            mobjflags: 0,
            sprite: 0,
            frame: 0,
        }
    }

    #[test]
    fn sort_orders_far_to_near_with_projection_order_ties() {
        // R_SortVisSprites (r_things.c:784-835), investigation §2.1.7 item 4: ascending scale
        // (far to near); among equal scales the earliest projected is drawn first.
        with_ctx(11, |ctx| {
            for (scale, x1) in [(5, 0), (3, 1), (5, 2), (3, 3), (4, 4)] {
                ctx.things.vissprites.push(sprite(scale, x1));
            }
            assert_eq!(ctx.sort_vis_sprites(), Ok(vec![1, 3, 4, 0, 2]));
        });
    }

    #[test]
    fn sort_with_no_scale_below_maxint_is_a_stale_global() {
        // Guard: `best` is uninitialised before the first pass (r_things.c:815, investigation
        // §2.1.7 item 5): a vissprite whose scale is not below MAXINT leaves the genuine list
        // corrupted. `tz >= MINZ` bounds `xscale` (r_things.c:492-495), so projection
        // never yields MAXINT and the public API cannot reach it.
        with_ctx(11, |ctx| {
            ctx.things.vissprites.push(sprite(i32::MAX, 0));
            assert_eq!(
                ctx.sort_vis_sprites(),
                Err(RenderRefusal::StaleGlobal("R_SortVisSprites best"))
            );
        });
    }

    // ---- R_InitSpriteDefs / R_InstallSpriteLump (r_things.c:108-157, 166-270): the error paths
    // of the sprite lump naming rules, reached by the public API only with well-formed sprite
    // sets (every golden loads), so they are exercised on hand-made lump directories.

    use crate::r_data::{LumpDirectory, Resources};

    /// A `Resources` whose sprite range holds lumps named `names` (empty patches; the sprite
    /// definitions read only the names).
    fn sprite_res(names: &[&str]) -> Resources {
        let mut dir = LumpDirectory::new();
        dir.add_single_lump("S_START", &[]).unwrap();
        for n in names {
            dir.add_single_lump(n, &[0; 8]).unwrap();
        }
        dir.add_single_lump("S_END", &[]).unwrap();
        Resources {
            dir,
            colormaps: vec![],
            palette: None,
            textures: vec![],
            firstflat: 0,
            numflats: 0,
            firstspritelump: 1,
            sprite_lumps: vec![[0; 3]; names.len()],
            sprites: vec![],
            switchlist: vec![],
            anims: vec![],
            skytexture: 0,
            skyflatnum: None,
        }
    }

    fn sprite_err(names: &[&str]) -> String {
        match init_sprite_defs(&sprite_res(names)) {
            Err(ResourceError::Sprite(m)) => m,
            other => panic!("{names:?}: {other:?}"),
        }
    }

    #[test]
    fn a_frame_with_no_patch_below_the_last_frame_is_refused_by_letter() {
        // `for (frame = 0; frame < maxframe; frame++) case -1: I_Error("R_InitSprites: No
        // patches found for %s frame %c", ..., frame+'A')` (r_things.c:239-247): TROO has
        // frames A and C but no B.
        assert_eq!(
            sprite_err(&["TROOA0", "TROOC0"]),
            "No patches found for TROO frame B"
        );
        assert_eq!(
            sprite_err(&["TROOA0", "TROOB0", "TROOD0"]),
            "No patches found for TROO frame C"
        );
    }

    #[test]
    fn a_rotated_frame_needs_all_eight_rotations() {
        // `case 1: for (rotation = 0; rotation < 8; rotation++) if (lump[rotation] == -1)
        // I_Error("... Sprite %s frame %c is missing rotations")` (r_things.c:249-257).
        assert_eq!(
            sprite_err(&[
                "TROOA1", "TROOA2", "TROOA3", "TROOA4", "TROOA5", "TROOA6", "TROOA7"
            ]),
            "Sprite TROO frame A is missing rotations"
        );
        let all: Vec<String> = (1..=8).map(|r| format!("TROOA{r}")).collect();
        let refs: Vec<&str> = all.iter().map(String::as_str).collect();
        let defs = init_sprite_defs(&sprite_res(&refs)).expect("complete rotations");
        let troo = defs.iter().find(|d| !d.frames.is_empty()).expect("TROO");
        assert_eq!(troo.frames.len(), 1);
        assert!(troo.frames[0].rotate);
        assert_eq!(troo.frames[0].lump, [0, 1, 2, 3, 4, 5, 6, 7]);
        // A mirrored pair `TROOA2A8` fills rotations 2 and 8 (name[6..8], r_things.c:215-221).
        let defs = init_sprite_defs(&sprite_res(&[
            "TROOA1", "TROOA2A8", "TROOA3A7", "TROOA4A6", "TROOA5",
        ]))
        .expect("mirrored rotations");
        let f = &defs.iter().find(|d| !d.frames.is_empty()).unwrap().frames[0];
        assert_eq!(f.lump, [0, 1, 2, 3, 4, 3, 2, 1]);
        assert_eq!(
            f.flip,
            [false, false, false, false, false, true, true, true]
        );
    }

    #[test]
    fn install_refuses_bad_names_and_duplicate_lumps_naming_the_frame_letter() {
        // R_InstallSpriteLump (r_things.c:108-157): `frame >= 29 || rotation > 8` is "Bad frame
        // characters in lump %i" (either alone); the others name the frame as 'A'+frame.
        // Lump numbers count from the start of the directory: S_START is lump 0.
        assert_eq!(sprite_err(&["TROO^1"]), "Bad frame characters in lump 1");
        assert_eq!(sprite_err(&["TROOA9"]), "Bad frame characters in lump 1");
        assert_eq!(
            sprite_err(&["TROOA0", "TROOB0", "TROOB0"]),
            "Sprite TROO frame B has multip rot=0 lump"
        );
        assert_eq!(
            sprite_err(&["TROOA0", "TROOB1", "TROOB0"]),
            "Sprite TROO frame B has rotations and a rot=0 lump"
        );
        assert_eq!(
            sprite_err(&["TROOA0", "TROOB0", "TROOB1"]),
            "Sprite TROO frame B has rotations and a rot=0 lump"
        );
        assert_eq!(
            sprite_err(&["TROOA0", "TROOB3", "TROOB3"]),
            "Sprite TROO : B : 3 has two lumps mapped to it"
        );
    }

    // ---- R_DrawSprite's drawseg clipping (r_things.c:860-933) on hand-made drawsegs.

    use crate::r_segs::DrawSeg;

    const BOTTOM_CLIP: i16 = 123;
    const TOP_CLIP: i16 = 37;

    /// A drawseg over columns 10-13 of seg 0 whose clip arrays are `openings` rows holding
    /// `bottom`/`top`.
    fn drawseg(
        ctx: &mut crate::FrameContext<'_>,
        k: usize,
        scales: [i32; 2],
        silhouette: i32,
        bsil: i32,
        tsil: i32,
        (bottom, top): (i16, i16),
    ) -> DrawSeg {
        let (bb, tb) = (k as i32 * 1000, k as i32 * 1000 + 500);
        for x in 0..320 {
            ctx.planes.openings[(bb + x) as usize] = bottom;
            ctx.planes.openings[(tb + x) as usize] = top;
        }
        DrawSeg {
            curline: 0,
            x1: 10,
            x2: 13,
            scale1: scales[0],
            scale2: scales[1],
            scalestep: 0,
            silhouette,
            bsilheight: bsil,
            tsilheight: tsil,
            sprtopclip: Clip::Openings(tb),
            sprbottomclip: Clip::Openings(bb),
            maskedtexturecol: None,
        }
    }

    /// A sprite over columns 11-12 (inside the drawsegs' 10-13) at (`gx`, `gy`), z from 0 to
    /// `gzt`, drawn from every column 0 of sprite lump 0.
    fn spr_at(scale: i32, gx: i32, gy: i32, gz: i32, gzt: i32) -> VisSprite {
        VisSprite {
            x1: 11,
            x2: 12,
            gx,
            gy,
            gz,
            gzt,
            startfrac: 0,
            scale,
            xiscale: 0,
            texturemid: 0,
            patch: 0,
            colormap: Some(0),
            mobjflags: 0,
            sprite: 0,
            frame: 0,
        }
    }

    /// A point on side `side` of seg 0 (`R_PointOnSegSide`, r_main.c:217-273).
    fn point_on_seg0_side(ctx: &crate::FrameContext<'_>, side: usize) -> (i32, i32) {
        let seg = &ctx.level.geometry.segs[0];
        let node = [
            seg.v1[0],
            seg.v1[1],
            seg.v2[0].wrapping_sub(seg.v1[0]),
            seg.v2[1].wrapping_sub(seg.v1[1]),
        ];
        for gx in (-8..8).map(|i| (i * 256) << 16) {
            for gy in (-8..8).map(|i| (i * 256) << 16) {
                if crate::r_main::point_on_side(gx, gy, node) == side {
                    return (gx, gy);
                }
            }
        }
        panic!("no point on side {side}");
    }

    /// Draws `spr` against `segs` and returns its columns 11 and 12 of (`clipbot`, `cliptop`).
    fn clips(
        ctx: &mut crate::FrameContext<'_>,
        spr: VisSprite,
        segs: Vec<DrawSeg>,
    ) -> [(i16, i16); 2] {
        ctx.bsp.drawsegs = segs;
        ctx.things.vissprites.clear();
        ctx.things.vissprites.push(spr);
        ctx.draw_sprite(0).expect("draw");
        let t = &ctx.things;
        [
            (t.clipbot[11], t.cliptop[11]),
            (t.clipbot[12], t.cliptop[12]),
        ]
    }

    #[test]
    fn a_drawseg_clips_a_sprite_unless_it_is_behind_it() {
        // r_things.c:870-891: the drawseg is `behind` the sprite when its nearer end is
        // farther (`scale < spr->scale`) or its far end is farther and the sprite is on the
        // seg's front (`lowscale < spr->scale && !R_PointOnSegSide`); `scale` is the larger of
        // the two end scales. A behind drawseg does not clip. Unclipped bounds are
        // clipbot = viewheight (200 here), cliptop = -1 (r_things.c:938-945).
        with_ctx(11, |ctx| {
            let on_back = point_on_seg0_side(ctx, 1);
            let on_front = point_on_seg0_side(ctx, 0);
            let sil =
                |ctx: &mut crate::FrameContext<'_>, sc: [i32; 2], at: (i32, i32), spr_scale| {
                    let ds = drawseg(ctx, 0, sc, 1, i32::MAX, i32::MIN, (BOTTOM_CLIP, TOP_CLIP));
                    clips(ctx, spr_at(spr_scale, at.0, at.1, 0, 1 << 16), vec![ds])[0].0
                };
            let (clipped, unclipped) = (BOTTOM_CLIP, 200);
            // The larger end scale (100, in either order) beyond the sprite's 50 and the
            // smaller one before it: behind only on the seg's front side (side 0).
            for sc in [[100, 1], [1, 100]] {
                assert_eq!(sil(ctx, sc, on_back, 50), clipped, "{sc:?} back side");
                assert_eq!(sil(ctx, sc, on_front, 50), unclipped, "{sc:?} front side");
            }
            // Both ends nearer than the sprite (`scale < spr->scale`): behind, whatever the side.
            assert_eq!(sil(ctx, [10, 20], on_back, 50), unclipped);
            // The larger end scale equal to the sprite's is not `<`: not behind via the first
            // test, and the second needs `lowscale < spr->scale`.
            assert_eq!(sil(ctx, [50, 50], on_back, 50), clipped);
            assert_eq!(sil(ctx, [30, 50], on_back, 50), clipped);
            assert_eq!(sil(ctx, [30, 50], on_front, 50), unclipped);
            // The smaller end scale equal to the sprite's: `lowscale < spr->scale` is false.
            assert_eq!(sil(ctx, [50, 100], on_front, 50), clipped);
        });
    }

    #[test]
    fn silhouette_bits_select_the_clip_arrays_and_gz_gzt_remove_them() {
        // r_things.c:898-931: `if (spr->gz >= ds->bsilheight) silhouette &= ~SIL_BOTTOM; if
        // (spr->gzt <= ds->tsilheight) silhouette &= ~SIL_TOP;` then silhouette 1 clips from
        // the bottom array, 2 from the top array, 3 from both.
        with_ctx(11, |ctx| {
            let (gx, gy) = point_on_seg0_side(ctx, 1); // never behind at these scales
            let near = 1_000_000;
            let run = |ctx: &mut crate::FrameContext<'_>, silhouette, bsil, tsil, gz, gzt| {
                let ds = drawseg(
                    ctx,
                    0,
                    [near, near],
                    silhouette,
                    bsil,
                    tsil,
                    (BOTTOM_CLIP, TOP_CLIP),
                );
                clips(ctx, spr_at(50, gx, gy, gz, gzt), vec![ds])[0]
            };
            let (b, t) = (BOTTOM_CLIP, TOP_CLIP);
            let (open_b, open_t) = (200, -1);
            // Bottom only, top only, both.
            assert_eq!(run(ctx, 1, i32::MAX, i32::MIN, 0, 10), (b, open_t));
            assert_eq!(run(ctx, 2, i32::MAX, i32::MIN, 0, 10), (open_b, t));
            assert_eq!(run(ctx, 3, i32::MAX, i32::MIN, 0, 10), (b, t));
            // `gz >= bsilheight` drops the bottom bit (equality included) and leaves the top.
            assert_eq!(run(ctx, 3, 0, i32::MIN, 0, 10), (open_b, t));
            assert_eq!(run(ctx, 3, 1, i32::MIN, 0, 10), (b, t));
            // `gzt <= tsilheight` drops the top bit (equality included) and leaves the bottom.
            assert_eq!(run(ctx, 3, i32::MAX, 10, 0, 10), (b, open_t));
            assert_eq!(run(ctx, 3, i32::MAX, 9, 0, 10), (b, t));
            assert_eq!(run(ctx, 2, i32::MAX, 10, 0, 10), (open_b, open_t));
            // Both dropped.
            assert_eq!(run(ctx, 3, 0, 10, 0, 10), (open_b, open_t));
        });
    }

    #[test]
    fn the_nearest_drawseg_clips_first_and_a_clipped_column_is_not_rewritten() {
        // Drawsegs are scanned from the last emitted backwards and `if (clipbot[x] == -2)`
        // keeps the first (nearest) write (r_things.c:902-931): the later index wins.
        with_ctx(11, |ctx| {
            let (gx, gy) = point_on_seg0_side(ctx, 1);
            let near = 1_000_000;
            let first = drawseg(ctx, 0, [near, near], 3, i32::MAX, i32::MIN, (11, 12));
            let last = drawseg(ctx, 1, [near, near], 3, i32::MAX, i32::MIN, (21, 22));
            let got = clips(ctx, spr_at(50, gx, gy, 0, 10), vec![first, last]);
            assert_eq!(got, [(21, 22), (21, 22)]);
        });
    }

    // ---- R_DrawVisSprite (r_things.c:389-450).

    #[test]
    fn a_translated_sprite_is_refused_and_a_bad_texture_column_is_a_range_error() {
        // `else if (vis->mobjflags & MF_TRANSLATION)` selects R_DrawTranslatedColumn
        // (r_things.c:407-413), refused by decision R1; the genuine `texturecolumn < 0 ||
        // texturecolumn >= SHORT(patch->width)` is an I_Error (r_things.c:432-436).
        with_ctx(11, |ctx| {
            let mut v = spr_at(50, 0, 0, 0, 0);
            v.mobjflags = MF_TRANSLATION & 0x400_0000;
            let draw = |ctx: &mut crate::FrameContext<'_>, v: &VisSprite| {
                ctx.draw_vis_sprite(v, Clip::ScreenHeight, Clip::NegOne, Feature::Untouched)
            };
            assert_eq!(draw(ctx, &v), Err(RenderRefusal::PlayerTranslation));
            // The shadow colour map (NULL) wins over the translation flags.
            v.colormap = None;
            assert!(draw(ctx, &v).is_ok());
            v.colormap = Some(0);
            v.mobjflags = 0;
            assert!(draw(ctx, &v).is_ok());
            let width = i32::from(
                patch_header(ctx.res.lump(ctx.res.firstspritelump))
                    .expect("patch header")
                    .width,
            );
            let range = Err(RenderRefusal::RangeCheck(
                "R_DrawSpriteRange: bad texturecolumn",
            ));
            for startfrac in [-1, -(1 << 16), width << 16, (width << 16) + 5, i32::MIN] {
                v.startfrac = startfrac;
                assert_eq!(draw(ctx, &v), range, "startfrac {startfrac}");
            }
            for startfrac in [0, (width - 1) << 16, ((width - 1) << 16) + 0xffff] {
                v.startfrac = startfrac;
                assert!(draw(ctx, &v).is_ok(), "startfrac {startfrac}");
            }
        });
    }

    #[test]
    fn light_rows_clamp_to_the_sixteen_scalelight_rows() {
        // r_things.c:628-635, 753-762: `lightnum = (sec->lightlevel >> LIGHTSEGSHIFT) +
        // extralight; if (lightnum < 0) spritelights = scalelight[0]; else if (lightnum >=
        // LIGHTLEVELS) spritelights = scalelight[LIGHTLEVELS-1]`.
        for (light, extra, want) in [
            (255, 0, 15),
            (255, 1, 15),
            (255, 8, 15),
            (240, 0, 15),
            (224, 1, 15),
            (224, 0, 14),
            (160, 0, 10),
            (160, 2, 12),
            (0, 0, 0),
            (0, -1, 0),
            (16, -5, 0),
        ] {
            assert_eq!(light_row(light, extra), want, "{light} {extra}");
        }
    }

    // ---- R_ProjectSprite's culling edges (r_things.c:483-547).

    use crate::r_data::SpriteDef;
    use crate::r_main::fixed_mul;

    /// The first sprite whose frame 0 has one lump for every rotation, as (sprite number, lump).
    fn plain_sprite(res: &Resources) -> (u16, i16) {
        res.sprites
            .iter()
            .enumerate()
            .find_map(|(i, d): (usize, &SpriteDef)| {
                let f = d.frames.first()?;
                (!f.rotate).then_some((i as u16, f.lump[0]))
            })
            .expect("a non-rotating sprite in the fixture")
    }

    /// `tz` and `tx` of r_things.c:483-489 for a thing at (`x`, `y`) seen from the fixture's
    /// view at the origin.
    fn tz_tx(ctx: &crate::FrameContext<'_>, x: i32, y: i32) -> (i32, i32) {
        let f = ctx.frame;
        let gxt = fixed_mul(x, f.viewcos);
        let gyt = fixed_mul(y, f.viewsin).wrapping_neg();
        let tz = gxt.wrapping_sub(gyt);
        let gxt = fixed_mul(x, f.viewsin).wrapping_neg();
        let gyt = fixed_mul(y, f.viewcos);
        (tz, gyt.wrapping_add(gxt).wrapping_neg())
    }

    fn thing(sprite: u16, x: i32, y: i32, frame: u32) -> Thing {
        Thing {
            x,
            y,
            z: 0,
            angle: 0,
            sprite,
            frame,
            flags: 0,
        }
    }

    /// A (x, y) with exactly `tz` and `tx` (the view is at the origin, `viewsin` is not 0, so
    /// both depend on both coordinates), found in a window around the ideal values.
    fn find_xy(ctx: &crate::FrameContext<'_>, tz: i32, tx: i32) -> (i32, i32) {
        for y in (-tx - 2048)..(-tx + 2048) {
            for x in (tz - 2048)..(tz + 2048) {
                if tz_tx(ctx, x, y) == (tz, tx) {
                    return (x, y);
                }
            }
        }
        panic!("no thing position gives tz {tz} tx {tx}");
    }

    /// The fixture with sprite lump `plain_sprite` made 100 units wide, so that a thing 16 units
    /// to the side at the 4-unit minimum distance still reaches the view.
    fn wide(res: &mut Resources) {
        let (_, lump) = plain_sprite(res);
        res.sprite_lumps[lump as usize] = [100 << 16, 0, 50 << 16];
    }

    #[test]
    fn project_culls_at_exactly_minz_and_four_times_tz_to_the_side() {
        // r_things.c:492-493 `if (tz < MINZ) return` (MINZ 4 units) and 502-503 `if
        // (abs(tx) > (tz<<2)) return`: a thing at exactly MINZ, or exactly 4 tz to the side,
        // is kept; one fixed-point unit nearer, or farther to the side, is not.
        with_ctx_res(11, wide, |ctx| {
            let (sprite, _) = plain_sprite(ctx.res);
            let kept = |ctx: &mut crate::FrameContext<'_>, (x, y): (i32, i32)| {
                ctx.things.vissprites.clear();
                ctx.project_sprite(&thing(sprite, x, y, 0))
                    .expect("project");
                !ctx.things.vissprites.is_empty()
            };
            let limit = MINZ << 2;
            let centre = find_xy(ctx, MINZ, 0);
            assert!(
                kept(ctx, centre),
                "tz == MINZ is in front of the view plane"
            );
            assert!(
                !kept(ctx, find_xy(ctx, MINZ - 1, 0)),
                "tz < MINZ is behind it"
            );
            let edge = find_xy(ctx, MINZ, -limit);
            assert!(
                kept(ctx, edge),
                "abs(tx) == tz<<2 is not too far off the side"
            );
            let out = find_xy(ctx, MINZ, -limit - 1);
            assert!(!kept(ctx, out), "abs(tx) > tz<<2 is too far off the side");
        });
    }

    #[test]
    fn a_projected_sprites_frame_field_is_the_frame_number_without_the_full_bright_bit() {
        // R_ProjectSprite indexes `spriteframes[thing->frame & FF_FRAMEMASK]` (r_things.c:511)
        // and `FF_FULLBRIGHT` (0x8000) selects colour map 0 (r_things.c:593-596); the
        // vissprite keeps the plain frame number for first-difference reports.
        with_ctx_res(11, wide, |ctx| {
            let (sprite, _) = plain_sprite(ctx.res);
            let x = 8 << 16;
            ctx.project_sprite(&thing(sprite, x, 0, FF_FULLBRIGHT))
                .expect("project");
            let v = ctx.things.vissprites.last().expect("a vissprite");
            assert_eq!(v.frame, 0);
            assert_eq!(v.colormap, Some(0));
            assert_eq!(v.sprite, sprite);
        });
    }

    // ---- R_DrawPSprite (r_things.c:643-739) with the weapon taken from the fixture's player.

    /// The fixture player's weapon state with the first psprite's x set to `sx`.
    fn weapon_at(ctx: &crate::FrameContext<'_>, sx: i32) -> doom_sim::PlayerWeaponState {
        let mut w = ctx
            .sim
            .original_player_weapons(PlayerSlot::One)
            .expect("player")
            .expect("weapon state");
        assert!(
            w.psprites[0].state.is_some(),
            "the fixture's weapon psprite"
        );
        w.psprites[0].position = [
            doom_types::Fixed::from_bits(sx),
            doom_types::Fixed::from_bits(32 << 16),
        ];
        w
    }

    /// `x2` of r_things.c:676-690 for psprite x `sx`.
    fn psprite_x2(ctx: &crate::FrameContext<'_>, sx: i32, width: i32, offset: i32) -> i32 {
        let tx = sx
            .wrapping_sub(160 << 16)
            .wrapping_sub(offset)
            .wrapping_add(width);
        (ctx.view
            .centerxfrac
            .wrapping_add(fixed_mul(tx, ctx.view.pspritescale))
            >> 16)
            - 1
    }

    /// The weapon sprite's lump, width and offset in the fixture.
    fn weapon_lump(ctx: &crate::FrameContext<'_>) -> (i16, i32, i32) {
        let w = weapon_at(ctx, 0);
        let def = ctx
            .res
            .sprites
            .get(usize::from(
                ctx.sim
                    .actor_registry()
                    .expect("registry")
                    .state(w.psprites[0].state.unwrap())
                    .expect("state")
                    .sprite,
            ))
            .expect("sprite def");
        let lump = def.frames[0].lump[0];
        let [width, offset, _] = ctx.res.sprite_lumps[lump as usize];
        (lump, width, offset)
    }

    #[test]
    fn a_weapon_sprite_ending_exactly_at_column_zero_is_drawn() {
        // r_things.c:685-690 `x2 = ((centerxfrac + FixedMul(tx, pspritescale)) >> FRACBITS) - 1;
        // if (x2 < 0) return;`: a weapon whose last column is screen column 0 is drawn, one
        // column further left is not. The sprite is made one column wide (spritewidth 1 unit)
        // so that exactly column 0 shows the patch's column 0.
        with_ctx_res(
            11,
            |res| {
                // The fixture weapon's lump, found as `weapon_lump` does, one column wide.
                let w = crate::test_support::fixture_weapon_lump(res);
                res.sprite_lumps[w as usize][0] = 1 << 16;
            },
            |ctx| {
                let (lump, width, offset) = weapon_lump(ctx);
                assert_eq!(width, 1 << 16);
                let patch = ctx.res.lump(ctx.res.firstspritelump + lump as usize);
                let col0 = patch_column_offset(patch, 0).expect("column 0");
                assert!(patch[col0 as usize] != 0xff, "patch column 0 has a post");
                let find = |want: i32| {
                    (-(600i32 << 16)..(600 << 16))
                        .step_by(1 << 10)
                        .find(|sx| psprite_x2(ctx, *sx, width, offset) == want)
                        .expect("an sx")
                };
                let (sx0, sx_1) = (find(0), find(-1));
                let column0_pixels = |ctx: &mut crate::FrameContext<'_>, sx| {
                    ctx.screen.fill(0xAA);
                    let w = weapon_at(ctx, sx);
                    ctx.draw_player_sprites(&w).expect("psprites");
                    (0..200).filter(|y| ctx.screen[y * 320] != 0xAA).count()
                };
                assert!(column0_pixels(ctx, sx0) > 0, "x2 == 0: column 0 is drawn");
                assert_eq!(column0_pixels(ctx, sx_1), 0, "x2 == -1: off the left side");
            },
        );
    }

    #[test]
    fn a_mirrored_weapon_lump_draws_the_patch_columns_in_reverse() {
        // r_things.c:700-715: a flipped lump starts at `spritewidth[lump] - 1` and steps by
        // `-pspriteiscale`, so at scale 1 the columns of the patch appear in reverse order over
        // the same screen span as the unflipped drawing.
        let draw = |flip: bool| {
            with_ctx_res(
                11,
                |res| {
                    if flip {
                        let sprite = {
                            let w = crate::test_support::fixture_weapon_lump(res);
                            res.sprites
                                .iter_mut()
                                .find(|d| d.frames.first().is_some_and(|f| f.lump[0] == w))
                                .expect("weapon sprite")
                                .frames[0]
                                .flip = [true; 8];
                            w
                        };
                        assert!(sprite >= 0);
                    }
                },
                |ctx| {
                    let (_, width, offset) = weapon_lump(ctx);
                    let x2 = psprite_x2(ctx, 0, width, offset);
                    let x1 = x2 - (width >> 16) + 1;
                    ctx.screen.fill(0xAA);
                    let w = weapon_at(ctx, 0);
                    ctx.draw_player_sprites(&w).expect("psprites");
                    (x1, x2, ctx.screen.to_vec())
                },
            )
        };
        let (x1, x2, plain) = draw(false);
        let (fx1, fx2, flipped) = draw(true);
        assert_eq!((x1, x2), (fx1, fx2));
        assert!(
            x1 >= 0 && x2 < 320,
            "the fixture weapon is fully on screen: {x1} {x2}"
        );
        assert_ne!(plain, flipped, "mirroring changes the picture");
        for y in 0..200 {
            for x in 0..320 {
                let mirror = if (x1..=x2).contains(&x) {
                    x1 + x2 - x
                } else {
                    x
                };
                assert_eq!(
                    flipped[y * 320 + x as usize],
                    plain[y * 320 + mirror as usize],
                    "({x},{y})"
                );
            }
        }
    }

    /// Draws the fixture weapon with its x at `sx` and returns the screen (background 0xAA) and
    /// the span of columns it covers.
    fn weapon_screen(ctx: &mut crate::FrameContext<'_>, sx: i32) -> (Vec<u8>, i32, i32) {
        let (_, width, offset) = weapon_lump(ctx);
        let x2 = psprite_x2(ctx, sx, width, offset);
        ctx.screen.fill(0xAA);
        let w = weapon_at(ctx, sx);
        ctx.draw_player_sprites(&w).expect("psprites");
        (ctx.screen.to_vec(), x2 - (width >> 16) + 1, x2)
    }

    #[test]
    fn a_weapon_cut_by_the_left_or_right_edge_shows_the_same_columns_as_the_uncut_one() {
        // r_things.c:696-713: `vis->x1 = x1 < 0 ? 0 : x1; vis->x2 = x2 >= viewwidth ? viewwidth-1
        // : x2; ... if (vis->x1 > x1) vis->startfrac += vis->xiscale*(vis->x1-x1);` At scale 1
        // (screenblocks 11) moving the weapon k pixels left or right only hides columns: every
        // visible screen column shows the patch column it showed before, and nothing is written
        // outside columns 0-319 (the framebuffer is row-major, so column 320 is the next row).
        with_ctx(11, |ctx| {
            let (whole, x1, x2) = weapon_screen(ctx, 0);
            assert!(x1 >= 5 && x2 < 300, "the uncut weapon: {x1}-{x2}");
            // Left: the first five columns are off the screen.
            let k = x1 + 5;
            let (left, lx1, _) = weapon_screen(ctx, -(k << 16));
            assert_eq!(lx1, -5);
            for y in 0..200 {
                for x in 0..320 - k as usize {
                    assert_eq!(
                        left[y * 320 + x],
                        whole[y * 320 + x + k as usize],
                        "left ({x},{y})"
                    );
                }
            }
            // Right: only the first five columns of the patch fit.
            let k = 315 - x1;
            let (right, rx1, rx2) = weapon_screen(ctx, k << 16);
            assert_eq!(rx1, 315);
            assert!(rx2 >= 320);
            for y in 0..200 {
                assert_eq!(right[y * 320], 0xAA, "column 0 untouched ({y})");
                for x in 315..320usize {
                    assert_eq!(
                        right[y * 320 + x],
                        whole[y * 320 + x - 315 + x1 as usize],
                        "right ({x},{y})"
                    );
                }
            }
        });
    }

    #[test]
    fn the_weapon_vissprite_reports_the_plain_frame_number() {
        // The first-difference trace names `PSprite { psprite, sprite, frame }` with the
        // state's frame without `FF_FULLBRIGHT` (the vissprite's `frame & FF_FRAMEMASK`).
        with_ctx(11, |ctx| {
            let w = weapon_at(ctx, 0);
            let state = ctx
                .sim
                .actor_registry()
                .expect("registry")
                .state(w.psprites[0].state.unwrap())
                .expect("state");
            let (sprite, frame) = (state.sprite, state.frame);
            assert_eq!(
                frame & FF_FULLBRIGHT,
                0,
                "a ready weapon is not full bright"
            );
            ctx.draw_player_sprites(&w).expect("psprites");
            let mut seen: Vec<Feature> = ctx
                .trace
                .as_deref()
                .expect("trace")
                .iter()
                .copied()
                .filter(|f| matches!(f, Feature::PSprite { .. }))
                .collect();
            seen.dedup();
            assert_eq!(
                seen,
                [Feature::PSprite {
                    psprite: 0,
                    sprite,
                    frame: (frame & FF_FRAMEMASK) as u16
                }]
            );
        });
    }

    #[test]
    fn a_masked_seg_behind_a_sprite_is_drawn_only_over_the_columns_they_share() {
        // r_things.c:870-891 `r1 = ds->x1 < spr->x1 ? spr->x1 : ds->x1; r2 = ds->x2 > spr->x2 ?
        // spr->x2 : ds->x2; ... R_RenderMaskedSegRange(ds, r1, r2)`: the masked columns of a
        // seg behind the sprite are drawn (and cleared to MAXSHORT in the openings) for the
        // overlap only; the rest are drawn later by R_DrawMasked. The seg covers columns
        // 8-15, the sprite 11-12; every masked column is pending (0) at 9, 11 and 14.
        const MAXSHORT: i16 = 0x7fff;
        crate::test_support::with_room(
            "hom.wad",
            11,
            |_| {},
            |ctx| {
                let seg = ctx
                    .level
                    .geometry
                    .segs
                    .iter()
                    .position(|s| s.backsector.is_some())
                    .expect("a two-sided seg");
                let base = 2000;
                for x in 0..320 {
                    ctx.planes.openings[base + x] = MAXSHORT;
                }
                for x in [9, 11, 14] {
                    ctx.planes.openings[base + x] = 0;
                }
                ctx.bsp.drawsegs = vec![DrawSeg {
                    curline: seg,
                    x1: 8,
                    x2: 15,
                    scale1: 10,
                    scale2: 10,
                    scalestep: 0,
                    silhouette: 0,
                    bsilheight: 0,
                    tsilheight: 0,
                    sprtopclip: Clip::NegOne,
                    sprbottomclip: Clip::ScreenHeight,
                    maskedtexturecol: Some(base as i32),
                }];
                ctx.things.vissprites.clear();
                ctx.things.vissprites.push(spr_at(50, 0, 0, 0, 0));
                ctx.draw_sprite(0).expect("draw");
                let open = |x: usize| ctx.planes.openings[base + x];
                assert_eq!(
                    open(11),
                    MAXSHORT,
                    "an overlapping column is drawn and cleared"
                );
                assert_eq!(
                    open(9),
                    0,
                    "a column left of the sprite is left for R_DrawMasked"
                );
                assert_eq!(
                    open(14),
                    0,
                    "a column right of the sprite is left for R_DrawMasked"
                );
            },
        );
    }
}
