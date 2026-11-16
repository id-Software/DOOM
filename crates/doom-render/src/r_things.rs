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
