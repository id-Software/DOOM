//! The version-110 demo (`.lmp`) format of linuxdoom-1.10, restricted to the
//! games the level policies run: medium skill, single player, episode 1 map 1
//! ([`parse_demo`], [`write_demo`]: the wire-22 fixtures) or any episode 1 map
//! 1–9 ([`parse_demo_episode1`], [`write_demo_episode1`]: the Episode 1 policy,
//! decision H10).
//!
//! Layout (`G_DoPlayDemo`, g_game.c:1582–1620; `G_BeginRecording`, 1549–1567):
//! thirteen header bytes (version, skill, episode, map, deathmatch, respawn,
//! fast, nomonsters, consoleplayer, then `playeringame[0..4]`), then one 4-byte
//! ticcmd per tic and player in the game (`G_ReadDemoTiccmd`, 1491–1503):
//! `forwardmove` as a signed byte, `sidemove` as a signed byte, `angleturn` as
//! its high byte (`(unsigned char)byte << 8`), `buttons` as an unsigned byte.
//! The stream ends at `DEMOMARKER` 0x80 (1488), tested on the byte where the next
//! `forwardmove` would be (1493) before any field is read.
use crate::TicCommand;
use std::{error::Error, fmt, ops::RangeInclusive};

/// g_game.c:1488 `DEMOMARKER`.
pub const DEMO_MARKER: u8 = 0x80;
/// doomdef.h `VERSION` 110, the first header byte (g_game.c:1589).
pub const DEMO_VERSION: u8 = 110;
/// The header this reader admits and [`write_demo`] writes: version 110, skill 2
/// (`sk_medium`), episode 1, map 1, deathmatch 0, respawn 0, fast 0,
/// nomonsters 0, consoleplayer 0, `playeringame` `[1, 0, 0, 0]`.
pub const DEMO_HEADER: [u8; 13] = [DEMO_VERSION, 2, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0];
/// The offset of the map byte in the header (g_game.c:1592, `startmap`).
pub const DEMO_MAP_INDEX: usize = 3;
/// The maps [`parse_demo_episode1`] and [`write_demo_episode1`] admit: E1M1–E1M9
/// (decision H10).
pub const EPISODE1_DEMO_MAPS: RangeInclusive<u8> = 1..=9;

/// The offset of the skill byte in the header (g_game.c:1590, `skill`).
pub const DEMO_SKILL_INDEX: usize = 1;
/// The skills [`parse_demo_episode1_v2`] and [`write_demo_episode1_v2`] admit
/// (M4b decision I4): 2 (`sk_medium`) and 3 (`sk_hard`). The wire-24 constructor
/// decides which of them it executes.
pub const EPISODE1_V2_DEMO_SKILLS: RangeInclusive<u8> = 2..=3;

/// The varying part of an Episode 1 demo header (decision H10): every other
/// byte equals [`DEMO_HEADER`]'s.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DemoHeader {
    /// The map number of `E1M<map>`, in [`EPISODE1_DEMO_MAPS`].
    pub map: u8,
}

/// The varying part of a wire-24 Episode 1 demo header (M4b decisions I2 and
/// I4): every other byte equals [`DEMO_HEADER`]'s.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DemoHeaderV2 {
    /// The map number of `E1M<map>`, in [`EPISODE1_DEMO_MAPS`].
    pub map: u8,
    /// The skill byte, in [`EPISODE1_V2_DEMO_SKILLS`].
    pub skill: u8,
}

/// A parsed demo: one command per tic for player 1.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Demo {
    /// Player 1's ticcmds in tic order, before the marker.
    pub commands: Vec<TicCommand>,
}

/// Why [`parse_demo`] or [`write_demo`] refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemoError {
    /// Fewer than the thirteen header bytes.
    Truncated,
    /// Header byte `index` (0 version, 1 skill, 2 episode, 3 map, 4 deathmatch,
    /// 5 respawn, 6 fast, 7 nomonsters, 8 consoleplayer, 9–12 `playeringame`)
    /// differs from [`DEMO_HEADER`] (for [`parse_demo_episode1`], the map byte
    /// is outside [`EPISODE1_DEMO_MAPS`]).
    Header {
        /// The header byte's offset.
        index: usize,
        /// The byte found.
        value: u8,
    },
    /// The stream ended inside a ticcmd, or without the marker.
    UnterminatedCommands,
    /// Bytes follow the marker.
    TrailingBytes,
    /// [`write_demo`]: tic `tic`'s `forward` is −128, whose byte 0x80 a reader
    /// would take for the marker.
    MarkerForward {
        /// The tic's index.
        tic: usize,
    },
    /// [`write_demo_episode1`]: tic `tic`'s `side` is −128, outside the Episode 1
    /// command domain (decisions H5 and D9).
    MarkerSide {
        /// The tic's index.
        tic: usize,
    },
    /// [`parse_demo_episode1`]: the header's map (admitted) is not the caller's.
    Map {
        /// The caller's map.
        expected: u8,
        /// The header's map byte.
        found: u8,
    },
    /// [`parse_demo_episode1`] or [`write_demo_episode1`]: the caller's map is
    /// outside [`EPISODE1_DEMO_MAPS`].
    MapArgument {
        /// The caller's map.
        map: u8,
    },
    /// [`write_demo`]: tic `tic`'s buttons carry bit 0x40 or `BT_SPECIAL` 0x80,
    /// outside the admitted command domain.
    Buttons {
        /// The tic's index.
        tic: usize,
    },
    /// [`write_demo`]: tic `tic`'s `angle_turn` has a nonzero low byte, which the
    /// format cannot carry (only `angleturn >> 8` is stored).
    AngleTurn {
        /// The tic's index.
        tic: usize,
    },
}
impl fmt::Display for DemoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "demo: {self:?}")
    }
}
impl Error for DemoError {}

/// Read a version-110 demo with exactly [`DEMO_HEADER`], then player 1's ticcmds
/// up to the marker, as `G_ReadDemoTiccmd` reads them (g_game.c:1491–1503):
/// a `forwardmove` byte of 0x80 is the marker, any other value starts a command.
/// Nothing may follow the marker.
pub fn parse_demo(bytes: &[u8]) -> Result<Demo, DemoError> {
    let header = bytes.get(..DEMO_HEADER.len()).ok_or(DemoError::Truncated)?;
    if let Some((index, &value)) = header
        .iter()
        .enumerate()
        .find(|(i, b)| **b != DEMO_HEADER[*i])
    {
        return Err(DemoError::Header { index, value });
    }
    parse_commands(&bytes[DEMO_HEADER.len()..])
}

/// Read a version-110 Episode 1 demo for map `E1M<map>` (decision H10): the
/// header must equal [`DEMO_HEADER`] except the map byte, which must lie in
/// [`EPISODE1_DEMO_MAPS`] (both checked byte by byte in order, the first
/// difference refused as [`DemoError::Header`]) and equal `map`
/// ([`DemoError::Map`]); then player 1's ticcmds as [`parse_demo`] reads them.
pub fn parse_demo_episode1(bytes: &[u8], map: u8) -> Result<(DemoHeader, Demo), DemoError> {
    if !EPISODE1_DEMO_MAPS.contains(&map) {
        return Err(DemoError::MapArgument { map });
    }
    let header = bytes.get(..DEMO_HEADER.len()).ok_or(DemoError::Truncated)?;
    if let Some((index, &value)) = header.iter().enumerate().find(|(i, b)| {
        if *i == DEMO_MAP_INDEX {
            !EPISODE1_DEMO_MAPS.contains(b)
        } else {
            **b != DEMO_HEADER[*i]
        }
    }) {
        return Err(DemoError::Header { index, value });
    }
    let found = header[DEMO_MAP_INDEX];
    if found != map {
        return Err(DemoError::Map {
            expected: map,
            found,
        });
    }
    let demo = parse_commands(&bytes[DEMO_HEADER.len()..])?;
    Ok((DemoHeader { map }, demo))
}

/// Read a version-110 demo for wire 24 (M4b decisions I2 and I4): as
/// [`parse_demo_episode1`], except that the skill byte may be any of
/// [`EPISODE1_V2_DEMO_SKILLS`] (2 or 3; any other value is
/// [`DemoError::Header`] at [`DEMO_SKILL_INDEX`]). [`parse_demo_episode1`] keeps
/// admitting skill 2 alone.
pub fn parse_demo_episode1_v2(bytes: &[u8], map: u8) -> Result<(DemoHeaderV2, Demo), DemoError> {
    if !EPISODE1_DEMO_MAPS.contains(&map) {
        return Err(DemoError::MapArgument { map });
    }
    let header = bytes.get(..DEMO_HEADER.len()).ok_or(DemoError::Truncated)?;
    if let Some((index, &value)) = header.iter().enumerate().find(|(i, b)| match *i {
        DEMO_MAP_INDEX => !EPISODE1_DEMO_MAPS.contains(b),
        DEMO_SKILL_INDEX => !EPISODE1_V2_DEMO_SKILLS.contains(b),
        _ => **b != DEMO_HEADER[*i],
    }) {
        return Err(DemoError::Header { index, value });
    }
    let found = header[DEMO_MAP_INDEX];
    if found != map {
        return Err(DemoError::Map {
            expected: map,
            found,
        });
    }
    let demo = parse_commands(&bytes[DEMO_HEADER.len()..])?;
    Ok((
        DemoHeaderV2 {
            map,
            skill: header[DEMO_SKILL_INDEX],
        },
        demo,
    ))
}

/// Player 1's ticcmds up to the marker, as `G_ReadDemoTiccmd` reads them
/// (g_game.c:1491–1503); nothing may follow the marker.
fn parse_commands(mut rest: &[u8]) -> Result<Demo, DemoError> {
    let mut commands = Vec::new();
    loop {
        match rest {
            [DEMO_MARKER, tail @ ..] => {
                return if tail.is_empty() {
                    Ok(Demo { commands })
                } else {
                    Err(DemoError::TrailingBytes)
                };
            }
            [forward, side, turn, buttons, tail @ ..] => {
                commands.push(TicCommand {
                    forward: *forward as i8,
                    side: *side as i8,
                    angle_turn: (u16::from(*turn) << 8) as i16,
                    buttons: *buttons,
                });
                rest = tail;
            }
            _ => return Err(DemoError::UnterminatedCommands),
        }
    }
}

/// Write `commands` as a version-110 demo with [`DEMO_HEADER`] and the marker.
/// Refuses a `forward` of −128 (its byte is the marker), buttons with bit 0x40
/// or 0x80, and an `angle_turn` with a nonzero low byte (not representable).
pub fn write_demo(commands: &[TicCommand]) -> Result<Vec<u8>, DemoError> {
    write_with(DEMO_HEADER, commands, false)
}

/// Write `commands` as a version-110 Episode 1 demo for map `E1M<map>`
/// (decision H10): [`DEMO_HEADER`] with the map byte `map` (in
/// [`EPISODE1_DEMO_MAPS`]), the commands and the marker. Refuses what
/// [`write_demo`] refuses and also a `side` of −128 (decisions H5 and D9).
pub fn write_demo_episode1(map: u8, commands: &[TicCommand]) -> Result<Vec<u8>, DemoError> {
    if !EPISODE1_DEMO_MAPS.contains(&map) {
        return Err(DemoError::MapArgument { map });
    }
    let mut header = DEMO_HEADER;
    header[DEMO_MAP_INDEX] = map;
    write_with(header, commands, true)
}

/// Write `commands` as a version-110 wire-24 demo for map `E1M<map>` at `skill`
/// (M4b decisions I2 and I4): as [`write_demo_episode1`] with the skill byte
/// `skill` (in [`EPISODE1_V2_DEMO_SKILLS`], else [`DemoError::Header`] at
/// [`DEMO_SKILL_INDEX`]). With skill 2 the bytes equal
/// [`write_demo_episode1`]'s.
pub fn write_demo_episode1_v2(
    map: u8,
    skill: u8,
    commands: &[TicCommand],
) -> Result<Vec<u8>, DemoError> {
    if !EPISODE1_DEMO_MAPS.contains(&map) {
        return Err(DemoError::MapArgument { map });
    }
    if !EPISODE1_V2_DEMO_SKILLS.contains(&skill) {
        return Err(DemoError::Header {
            index: DEMO_SKILL_INDEX,
            value: skill,
        });
    }
    let mut header = DEMO_HEADER;
    header[DEMO_MAP_INDEX] = map;
    header[DEMO_SKILL_INDEX] = skill;
    write_with(header, commands, true)
}

fn write_with(
    header: [u8; 13],
    commands: &[TicCommand],
    refuse_side_marker: bool,
) -> Result<Vec<u8>, DemoError> {
    let mut out = Vec::with_capacity(header.len() + 4 * commands.len() + 1);
    out.extend_from_slice(&header);
    for (tic, c) in commands.iter().enumerate() {
        if c.forward == i8::MIN {
            return Err(DemoError::MarkerForward { tic });
        }
        if refuse_side_marker && c.side == i8::MIN {
            return Err(DemoError::MarkerSide { tic });
        }
        if c.buttons & 0xc0 != 0 {
            return Err(DemoError::Buttons { tic });
        }
        if c.angle_turn & 0xff != 0 {
            return Err(DemoError::AngleTurn { tic });
        }
        let [turn, _] = c.angle_turn.to_be_bytes();
        out.extend([c.forward as u8, c.side as u8, turn, c.buttons]);
    }
    out.push(DEMO_MARKER);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(forward: i8, side: i8, angle_turn: i16, buttons: u8) -> TicCommand {
        TicCommand {
            forward,
            side,
            angle_turn,
            buttons,
        }
    }

    #[test]
    fn round_trip_and_field_mapping() {
        let commands = vec![
            cmd(0, 0, 0, 0),
            cmd(50, -50, 0x0100, 1),
            cmd(-127, 127, -256, 2 | 4 | (3 << 3)),
            cmd(25, 0, i16::MIN, 0),
            cmd(0, 24, 0x7f00, 0x3f),
        ];
        let bytes = write_demo(&commands).unwrap();
        assert_eq!(bytes.len(), 13 + 4 * commands.len() + 1);
        assert_eq!(&bytes[..13], &DEMO_HEADER);
        assert_eq!(&bytes[17..21], &[50, 0xce, 0x01, 1]);
        assert_eq!(bytes[21 + 2], 0xff);
        assert_eq!(*bytes.last().unwrap(), DEMO_MARKER);
        assert_eq!(parse_demo(&bytes).unwrap().commands, commands);
        // Empty demo: header and marker.
        let empty = write_demo(&[]).unwrap();
        assert_eq!(empty.len(), 14);
        assert!(parse_demo(&empty).unwrap().commands.is_empty());
    }

    #[test]
    fn each_header_byte_is_checked() {
        let good = write_demo(&[cmd(1, 0, 0, 0)]).unwrap();
        for index in 0..13 {
            let mut bad = good.clone();
            bad[index] ^= 0x01;
            assert_eq!(
                parse_demo(&bad),
                Err(DemoError::Header {
                    index,
                    value: bad[index]
                })
            );
        }
        assert_eq!(parse_demo(&good[..12]), Err(DemoError::Truncated));
    }

    #[test]
    fn marker_and_stream_errors() {
        let mut bytes = write_demo(&[cmd(1, 2, 0, 0)]).unwrap();
        // The marker ends the stream where a forwardmove would be read.
        let mut early = DEMO_HEADER.to_vec();
        early.push(DEMO_MARKER);
        assert!(parse_demo(&early).unwrap().commands.is_empty());
        // A 0x80 side, turn or buttons byte is data, not a marker.
        let mut inner = DEMO_HEADER.to_vec();
        inner.extend([0, 0x80, 0x80, 0, DEMO_MARKER]);
        assert_eq!(
            parse_demo(&inner).unwrap().commands,
            vec![cmd(0, -128, i16::MIN, 0)]
        );
        bytes.pop();
        assert_eq!(parse_demo(&bytes), Err(DemoError::UnterminatedCommands));
        bytes.push(1);
        assert_eq!(parse_demo(&bytes), Err(DemoError::UnterminatedCommands));
        bytes.pop();
        bytes.extend([DEMO_MARKER, 0]);
        assert_eq!(parse_demo(&bytes), Err(DemoError::TrailingBytes));
    }

    #[test]
    fn writer_refusals() {
        assert_eq!(
            write_demo(&[cmd(0, 0, 0, 0), cmd(-128, 0, 0, 0)]),
            Err(DemoError::MarkerForward { tic: 1 })
        );
        assert_eq!(
            write_demo(&[cmd(0, 0, 0, 0x40)]),
            Err(DemoError::Buttons { tic: 0 })
        );
        assert_eq!(
            write_demo(&[cmd(0, 0, 0, 0x80)]),
            Err(DemoError::Buttons { tic: 0 })
        );
        assert_eq!(
            write_demo(&[cmd(0, 0, 1, 0)]),
            Err(DemoError::AngleTurn { tic: 0 })
        );
        assert!(write_demo(&[cmd(-127, -128, 0, 0x3f)]).is_ok());
    }

    #[test]
    fn episode1_round_trip_for_every_map() {
        let commands = vec![
            cmd(127, -127, 0x0100, 1),
            cmd(-127, 127, -256, 2 | 4 | (7 << 3)),
            cmd(100, 0, i16::MIN, 4),
            cmd(0, -128 + 1, 0x7f00, 0x3f),
        ];
        for map in 1..=9 {
            let bytes = write_demo_episode1(map, &commands).unwrap();
            assert_eq!(bytes.len(), 13 + 4 * commands.len() + 1);
            let mut header = DEMO_HEADER;
            header[3] = map;
            assert_eq!(&bytes[..13], &header);
            assert_eq!(&bytes[13..17], &[127, 0x81, 0x01, 1]);
            assert_eq!(*bytes.last().unwrap(), DEMO_MARKER);
            let (h, demo) = parse_demo_episode1(&bytes, map).unwrap();
            assert_eq!(h, DemoHeader { map });
            assert_eq!(demo.commands, commands);
            // The other maps' callers refuse it.
            for other in (1..=9).filter(|m| *m != map) {
                assert_eq!(
                    parse_demo_episode1(&bytes, other),
                    Err(DemoError::Map {
                        expected: other,
                        found: map
                    })
                );
            }
            // Only map 1 is a wire-22 demo.
            assert_eq!(parse_demo(&bytes).is_ok(), map == 1);
        }
        // Map 1 writes the wire-22 bytes exactly.
        assert_eq!(
            write_demo_episode1(1, &commands).unwrap(),
            write_demo(&commands).unwrap()
        );
        let empty = write_demo_episode1(9, &[]).unwrap();
        assert_eq!(empty.len(), 14);
        assert!(
            parse_demo_episode1(&empty, 9)
                .unwrap()
                .1
                .commands
                .is_empty()
        );
    }

    #[test]
    fn episode1_header_refusals() {
        let good = write_demo_episode1(4, &[cmd(1, 0, 0, 0)]).unwrap();
        for index in 0..13 {
            for value in 0..=255u8 {
                if index == 3 && (1..=9).contains(&value) || index != 3 && value == good[index] {
                    continue;
                }
                let mut bad = good.clone();
                bad[index] = value;
                assert_eq!(
                    parse_demo_episode1(&bad, 4),
                    Err(DemoError::Header { index, value }),
                    "byte {index} value {value}"
                );
            }
        }
        // The first refused byte in header order is reported.
        let mut two = good.clone();
        two[3] = 0;
        two[1] = 3;
        assert_eq!(
            parse_demo_episode1(&two, 4),
            Err(DemoError::Header { index: 1, value: 3 })
        );
        assert_eq!(
            parse_demo_episode1(&good[..12], 4),
            Err(DemoError::Truncated)
        );
        for map in [0, 10, 255] {
            assert_eq!(
                parse_demo_episode1(&good, map),
                Err(DemoError::MapArgument { map })
            );
            assert_eq!(
                write_demo_episode1(map, &[]),
                Err(DemoError::MapArgument { map })
            );
        }
        // The command stream rules are the wire-22 reader's.
        let mut bytes = good.clone();
        bytes.pop();
        assert_eq!(
            parse_demo_episode1(&bytes, 4),
            Err(DemoError::UnterminatedCommands)
        );
        bytes.extend([DEMO_MARKER, 0]);
        assert_eq!(
            parse_demo_episode1(&bytes, 4),
            Err(DemoError::TrailingBytes)
        );
    }

    #[test]
    fn episode1_writer_refusals() {
        assert_eq!(
            write_demo_episode1(2, &[cmd(0, 0, 0, 0), cmd(-128, 0, 0, 0)]),
            Err(DemoError::MarkerForward { tic: 1 })
        );
        assert_eq!(
            write_demo_episode1(2, &[cmd(0, 0, 0, 0), cmd(0, -128, 0, 0)]),
            Err(DemoError::MarkerSide { tic: 1 })
        );
        for buttons in [0x40, 0x80, 0xc0] {
            assert_eq!(
                write_demo_episode1(2, &[cmd(0, 0, 0, buttons)]),
                Err(DemoError::Buttons { tic: 0 })
            );
        }
        assert_eq!(
            write_demo_episode1(2, &[cmd(0, 0, 0x80, 0)]),
            Err(DemoError::AngleTurn { tic: 0 })
        );
        assert!(write_demo_episode1(2, &[cmd(-127, -127, 0, 0x3f)]).is_ok());
        // The wire-22 writer keeps admitting side −128.
        assert!(write_demo(&[cmd(0, -128, 0, 0)]).is_ok());
    }

    #[test]
    fn episode1_v2_header_admits_skill_2_and_3_and_every_map() {
        let commands = vec![cmd(25, -3, 0x0100, 1), cmd(0, 0, 0, 0)];
        for map in EPISODE1_DEMO_MAPS {
            // Skill 2: the bytes of the wire-23 writer, read back by both readers.
            let medium = write_demo_episode1_v2(map, 2, &commands).unwrap();
            assert_eq!(medium, write_demo_episode1(map, &commands).unwrap());
            let (header, demo) = parse_demo_episode1_v2(&medium, map).unwrap();
            assert_eq!(header, DemoHeaderV2 { map, skill: 2 });
            assert_eq!(demo.commands, commands);
            // Skill 3: the wire-24 reader alone admits it.
            let hard = write_demo_episode1_v2(map, 3, &commands).unwrap();
            assert_eq!(hard[DEMO_SKILL_INDEX], 3);
            assert_eq!(
                parse_demo_episode1_v2(&hard, map).unwrap().0,
                DemoHeaderV2 { map, skill: 3 }
            );
            assert_eq!(
                parse_demo_episode1(&hard, map),
                Err(DemoError::Header {
                    index: DEMO_SKILL_INDEX,
                    value: 3
                })
            );
        }
    }

    #[test]
    fn episode1_v2_header_refusals() {
        let good = write_demo_episode1_v2(5, 2, &[cmd(0, 0, 0, 0)]).unwrap();
        for skill in [0, 1, 4, 5, 255] {
            let mut bytes = good.clone();
            bytes[DEMO_SKILL_INDEX] = skill;
            assert_eq!(
                parse_demo_episode1_v2(&bytes, 5),
                Err(DemoError::Header {
                    index: DEMO_SKILL_INDEX,
                    value: skill
                })
            );
            assert_eq!(
                write_demo_episode1_v2(5, skill, &[]),
                Err(DemoError::Header {
                    index: DEMO_SKILL_INDEX,
                    value: skill
                })
            );
        }
        for map in [0, 10] {
            assert_eq!(
                parse_demo_episode1_v2(&good, map),
                Err(DemoError::MapArgument { map })
            );
            assert_eq!(
                write_demo_episode1_v2(map, 2, &[]),
                Err(DemoError::MapArgument { map })
            );
            let mut bytes = good.clone();
            bytes[DEMO_MAP_INDEX] = map;
            assert_eq!(
                parse_demo_episode1_v2(&bytes, 5),
                Err(DemoError::Header {
                    index: DEMO_MAP_INDEX,
                    value: map
                })
            );
        }
        assert_eq!(
            parse_demo_episode1_v2(&good, 4),
            Err(DemoError::Map {
                expected: 4,
                found: 5
            })
        );
        // Version 109 (DEMO1's byte), another episode, and every other fixed byte.
        for (index, value) in [
            (0, 109),
            (2, 2),
            (4, 1),
            (5, 1),
            (6, 1),
            (7, 1),
            (8, 1),
            (10, 1),
        ] {
            let mut bytes = good.clone();
            bytes[index] = value;
            assert_eq!(
                parse_demo_episode1_v2(&bytes, 5),
                Err(DemoError::Header { index, value })
            );
        }
        assert_eq!(
            parse_demo_episode1_v2(&good[..12], 5),
            Err(DemoError::Truncated)
        );
        let mut bytes = good.clone();
        bytes.push(0);
        assert_eq!(
            parse_demo_episode1_v2(&bytes, 5),
            Err(DemoError::TrailingBytes)
        );
        assert_eq!(
            write_demo_episode1_v2(5, 3, &[cmd(0, -128, 0, 0)]),
            Err(DemoError::MarkerSide { tic: 0 })
        );
    }
}
