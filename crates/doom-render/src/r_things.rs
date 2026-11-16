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
    use crate::test_support::with_ctx;

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
}
