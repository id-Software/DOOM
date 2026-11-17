//! r_data.c (and the w_wad.c lookups it relies on): the lump directory, colour maps, patch
//! names, the TEXTURE1/TEXTURE2 definitions, texture column lookup and composition, flats and
//! sprite lumps.
//!
//! Every offset, size and count read from a lump is validated. Where the genuine code reads
//! outside a lump at startup (no check exists in the C), loading is refused with a typed
//! [`ResourceError`]; where the genuine code calls `I_Error`, loading is refused with the
//! matching variant. Texel reads during drawing that would be undefined in the genuine engine
//! (outside the source lump, or a composite byte no patch wrote) are reported by
//! [`ColumnSource::texel`] so the drawers can flag the column as non-comparable (decision R4).
use std::fmt;

/// Number of composite or lump bytes a wall column can address (`& 127`, r_draw.c:142).
pub const COLUMN_INDEX_MASK: usize = 127;

/// A load-time refusal: an `I_Error` of the genuine startup or a read outside a lump that the
/// genuine startup would make without a check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceError {
    /// A directory name holds a lower-case ASCII letter before its first NUL: the genuine
    /// `W_CheckNumForName` (w_wad.c) compares raw directory bytes against an upper-cased query,
    /// while doom-wad's lookup upper-cases both, so the two would disagree (decision R14).
    DirectoryNameCase {
        /// Directory position in the global lump list.
        lump: usize,
    },
    /// More lumps than a `short` lump number holds (`texturecolumnlump`, r_data.c:140).
    TooManyLumps,
    /// `W_GetNumForName` found nothing (`I_Error`, w_wad.c).
    LumpNotFound(String),
    /// A lump is shorter than a structure the genuine code reads from it.
    LumpTooShort {
        /// The lump's name.
        name: String,
        /// Bytes needed.
        needed: usize,
        /// Bytes present.
        length: usize,
    },
    /// A PNAMES count is negative, the combined TEXTURE1 and TEXTURE2 count is negative or
    /// overflows, or a count or directory entry exceeds its lump.
    BadTextureDirectory(String),
    /// A texture names a patch index outside PNAMES (unchecked read of `patchlookup`).
    PatchIndexOutOfRange {
        /// The texture's name.
        texture: String,
        /// The index.
        index: i32,
    },
    /// `R_InitTextures: Missing patch in texture %s` (r_data.c:544).
    MissingPatch(String),
    /// A texture's width or patch count is negative: `alloca` and `memset` of a negative
    /// width (r_data.c:322-323), or a `texture_t` allocation smaller than the fields
    /// `R_InitTextures` writes into it (r_data.c:524-531).
    BadTextureSize(String),
    /// A patch header or column directory entry lies outside its lump.
    PatchBounds {
        /// The patch's lump number.
        lump: usize,
    },
    /// `R_GenerateLookup: texture %i is >64k` (r_data.c:367).
    CompositeTooLarge(usize),
    /// The F_START/F_END or S_START/S_END markers are in the wrong order: the genuine count
    /// is negative and sizes a `Z_Malloc` with a negative byte count (r_data.c:585-590,
    /// 608-614; z_zone.c:195-203). An empty range (count 0) is admitted.
    BadMarkerRange(&'static str),
    /// `R_TextureNumForName: %s not found` (r_data.c:724).
    TextureNotFound(String),
    /// `R_FlatNumForName: %s not found` (r_data.c:687).
    FlatNotFound(String),
    /// A flat name resolves to a lump outside F_START..F_END (an out-of-bounds
    /// `flattranslation` index in the genuine engine).
    FlatOutOfRange(String),
    /// `P_InitPicAnims: bad cycle from %s to %s` (p_spec.c:175), or a cycle outside the
    /// texture or flat table.
    BadAnimCycle(String),
    /// One of the `I_Error`s of `R_InstallSpriteLump`/`R_InitSpriteDefs` (r_things.c:115-151,
    /// 255-267).
    Sprite(String),
    /// A map refers to something the renderer cannot resolve (a seg of an `ML_TWOSIDED` line
    /// without a back side, which `P_LoadSegs` would read as `sides[-1]`, p_setup.c:189-192).
    Map(String),
    /// The episode's sky is not `SKY1`: `G_InitNew` selects `SKY2`, `SKY3` or `SKY4` for
    /// episodes 2-4 (g_game.c:1464-1478) and the renderer admits Episode 1 only.
    SkyEpisode(u8),
    /// The configuration is outside the admitted domain (decision R2).
    Config(crate::ConfigError),
}
impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "renderer resources: {self:?}")
    }
}
impl std::error::Error for ResourceError {}

/// The `strncpy` form of an 8-byte name: the bytes up to the first NUL, zero padded.
pub fn c_name(bytes: &[u8]) -> [u8; 8] {
    let mut out = [0; 8];
    for (o, b) in out.iter_mut().zip(bytes.iter().take_while(|b| **b != 0)) {
        *o = *b;
    }
    out
}

/// A printable form of an 8-byte name (for errors only).
pub fn name_text(name: &[u8]) -> String {
    c_name(name)
        .iter()
        .take_while(|b| **b != 0)
        .map(|b| char::from(*b))
        .collect()
}

/// The genuine global lump list (`lumpinfo`, w_wad.c): every file's directory in load order,
/// names as `strncpy` stored them, and the lump bytes (owned copies).
#[derive(Clone, Debug, Default)]
pub struct LumpDirectory {
    names: Vec<[u8; 8]>,
    data: Vec<Vec<u8>>,
}

impl LumpDirectory {
    /// An empty list.
    pub fn new() -> Self {
        Self::default()
    }
    /// `W_AddFile` of a WAD: appends its directory in on-disk order.
    pub fn add_wad(&mut self, wad: &doom_wad::Wad<'_>) -> Result<(), ResourceError> {
        for (index, lump) in wad.lumps().iter().enumerate() {
            let data = wad
                .data(index)
                .ok_or(ResourceError::PatchBounds { lump: index })?;
            self.push(*lump.name().as_bytes(), data)?;
        }
        Ok(())
    }
    /// `W_AddFile` of a non-WAD file (w_wad.c:172-180): one lump named after the upper-cased
    /// base name of `file_name` (`ExtractFileBase`, up to 8 characters before the extension).
    pub fn add_single_lump(&mut self, file_name: &str, data: &[u8]) -> Result<(), ResourceError> {
        let base = file_name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(file_name)
            .split('.')
            .next()
            .unwrap_or("");
        let mut name = [0u8; 8];
        for (o, b) in name.iter_mut().zip(base.bytes()) {
            *o = b.to_ascii_uppercase();
        }
        self.push(name, data)
    }
    fn push(&mut self, raw: [u8; 8], data: &[u8]) -> Result<(), ResourceError> {
        let name = c_name(&raw);
        if name.iter().any(u8::is_ascii_lowercase) {
            return Err(ResourceError::DirectoryNameCase {
                lump: self.names.len(),
            });
        }
        if self.names.len() >= i16::MAX as usize {
            return Err(ResourceError::TooManyLumps);
        }
        self.names.push(name);
        self.data.push(data.to_vec());
        Ok(())
    }
    /// `numlumps`.
    pub fn len(&self) -> usize {
        self.names.len()
    }
    /// Whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
    /// The stored name of lump `index`.
    pub fn name(&self, index: usize) -> Option<&[u8; 8]> {
        self.names.get(index)
    }
    /// The bytes of lump `index`.
    pub fn data(&self, index: usize) -> Option<&[u8]> {
        self.data.get(index).map(Vec::as_slice)
    }
    /// `W_CheckNumForName` (w_wad.c): the query truncated to 8 bytes at its first NUL and
    /// upper-cased, compared with the raw stored names, scanning backwards (last match wins).
    pub fn check_num_for_name(&self, query: &[u8]) -> Option<usize> {
        let mut want = c_name(query);
        want.make_ascii_uppercase();
        self.names.iter().rposition(|n| *n == want)
    }
    /// `W_GetNumForName`: [`Self::check_num_for_name`] or `I_Error`.
    pub fn get_num_for_name(&self, query: &[u8]) -> Result<usize, ResourceError> {
        self.check_num_for_name(query)
            .ok_or_else(|| ResourceError::LumpNotFound(name_text(query)))
    }
    fn bytes(&self, index: usize) -> &[u8] {
        &self.data[index]
    }
}

fn read_i16(bytes: &[u8], at: usize) -> Option<i16> {
    Some(i16::from_le_bytes(
        bytes.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}
fn read_i32(bytes: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

/// The validated header of a patch lump (`patch_t`, r_defs.h:356-364).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PatchHeader {
    /// `width`.
    pub width: i16,
    /// `height`.
    pub height: i16,
    /// `leftoffset`.
    pub leftoffset: i16,
    /// `topoffset`.
    pub topoffset: i16,
}

/// Reads a patch header (8 bytes) from `bytes`.
pub fn patch_header(bytes: &[u8]) -> Option<PatchHeader> {
    Some(PatchHeader {
        width: read_i16(bytes, 0)?,
        height: read_i16(bytes, 2)?,
        leftoffset: read_i16(bytes, 4)?,
        topoffset: read_i16(bytes, 6)?,
    })
}

/// `LONG(patch->columnofs[col])`; None when the entry lies outside the lump.
pub fn patch_column_offset(bytes: &[u8], col: i32) -> Option<i32> {
    let at = 8usize.checked_add(usize::try_from(col).ok()?.checked_mul(4)?)?;
    read_i32(bytes, at)
}

/// One patch of a texture definition (`texpatch_t`, r_data.c:103-111).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TexPatch {
    /// `originx`.
    pub originx: i32,
    /// `originy`.
    pub originy: i32,
    /// The patch's lump number (`patchlookup[...]`).
    pub patch: usize,
}

/// How a texture column is fetched by `R_GetColumn` (r_data.c:382-401).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColumnEntry {
    /// `texturecolumnlump > 0`: the patch lump at the stored `unsigned short` offset.
    Lump {
        /// The lump (stored as a `short` in the genuine table).
        lump: usize,
        /// `texturecolumnofs` (the column directory entry plus 3, truncated to 16 bits).
        ofs: u16,
    },
    /// `texturecolumnlump == -1` (a column covered by more than one patch): the composite
    /// block at the stored offset.
    Composite {
        /// `texturecolumnofs`.
        ofs: u16,
    },
    /// `texturecolumnlump == 0` (a single-patch column whose patch is lump 0): `R_GetColumn`
    /// reads the composite block at the patch's column offset, and `R_GenerateComposite`
    /// skips it (`collump[x] >= 0`, r_data.c:270-272).
    LumpZero {
        /// `texturecolumnofs`.
        ofs: u16,
    },
    /// A column `R_GenerateLookup` left uninitialised after "column without a patch"
    /// (r_data.c:353-358): its lump and offset are undefined memory in the genuine engine,
    /// so every read from it is undefined.
    Undefined,
    /// A column whose genuine `R_GetColumn` reads memory no allocation holds: a column of a
    /// zero-width texture (its `texturecolumnlump`/`texturecolumnofs` tables are 0-byte
    /// `Z_Malloc` blocks, r_data.c:548-549, read at r_data.c:391-392), or a composite column of
    /// a texture with a negative height (`R_GenerateComposite` would `Z_Malloc` the negative
    /// `texturecompositesize`, r_data.c:244 and 365-371). Drawing it is refused.
    Unallocated,
}

/// A texture (`texture_t` plus its r_data.c side tables).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Texture {
    /// `name` (8 raw bytes).
    pub name: [u8; 8],
    /// `width`.
    pub width: i16,
    /// `height`.
    pub height: i16,
    /// `patches`.
    pub patches: Vec<TexPatch>,
    /// `texturewidthmask`.
    pub widthmask: i32,
    /// `textureheight` (16.16).
    pub textureheight: i32,
    /// `texturecolumnlump`/`texturecolumnofs` per column.
    pub columns: Vec<ColumnEntry>,
    /// `texturecompositesize`.
    pub compositesize: usize,
    /// The composite block (`R_GenerateComposite`); unwritten bytes are 0 (as the M5a
    /// runner's declared ZeroFill makes them) and flagged undefined in `defined`.
    pub composite: Vec<u8>,
    /// Per composite byte: whether a patch post wrote it.
    pub defined: Vec<bool>,
}

/// The block a wall column reads from, with its definedness.
#[derive(Clone, Copy, Debug)]
pub struct ColumnSource<'a> {
    block: &'a [u8],
    defined: Option<&'a [bool]>,
    start: usize,
    undefined_column: bool,
    unallocated: bool,
}

impl ColumnSource<'_> {
    /// Whether the genuine `R_GetColumn` read of this column's tables or composite reads
    /// memory no allocation holds (see [`ColumnEntry::Unallocated`]).
    pub fn unallocated(&self) -> bool {
        self.unallocated
    }
    /// The byte at `start + index` of the source block, and whether reading it is defined
    /// (inside the lump, or a composite byte some patch wrote). An undefined read returns 0.
    pub fn texel(&self, index: usize) -> (u8, bool) {
        if self.undefined_column {
            return (0, false);
        }
        let at = self.start + index;
        match self.block.get(at) {
            Some(b) => (*b, self.defined.is_none_or(|d| d[at])),
            None => (0, false),
        }
    }
}

/// A sprite frame (`spriteframe_t`, r_defs.h:430-440).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpriteFrame {
    /// `rotate`.
    pub rotate: bool,
    /// `lump[8]`, relative to `firstspritelump`.
    pub lump: [i16; 8],
    /// `flip[8]`.
    pub flip: [bool; 8],
}

/// A sprite definition (`spritedef_t`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SpriteDef {
    /// `spriteframes[numframes]`.
    pub frames: Vec<SpriteFrame>,
}

/// The game mode the switch list is built for (`P_InitSwitchList`, p_switch.c:107-140).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameMode {
    /// Shareware: episode-1 switches.
    Shareware,
    /// Registered: episode 1 and 2 switches (the M5a runner's mode, d_main.c:697-701).
    Registered,
    /// Retail (The Ultimate DOOM): episode-1 switches.
    Retail,
}

/// Validated renderer resources: the `R_InitData`, `R_InitSprites`, `P_InitSwitchList` and
/// `P_InitPicAnims` tables.
#[derive(Clone, Debug)]
pub struct Resources {
    pub(crate) dir: LumpDirectory,
    /// `colormaps` (the whole COLORMAP lump; at least 33 maps).
    pub colormaps: Vec<u8>,
    /// Palette 0 of PLAYPAL when present (only for PNG output).
    pub palette: Option<Vec<u8>>,
    /// `textures`.
    pub textures: Vec<Texture>,
    /// `firstflat`.
    pub firstflat: usize,
    /// `numflats`.
    pub numflats: usize,
    /// `firstspritelump`.
    pub firstspritelump: usize,
    /// `spritewidth`, `spriteoffset`, `spritetopoffset` (16.16) per sprite lump.
    pub sprite_lumps: Vec<[i32; 3]>,
    /// `sprites`, in `sprnames` order.
    pub sprites: Vec<SpriteDef>,
    /// `switchlist` (texture numbers, pairs).
    pub switchlist: Vec<usize>,
    /// `anims` of `P_InitPicAnims`.
    pub anims: Vec<crate::p_spec::Anim>,
    /// `skytexture` (`R_TextureNumForName("SKY1")`, g_game.c:1457-1475; episode 1).
    pub skytexture: usize,
    /// `skyflatnum` (`R_FlatNumForName(SKYFLATNAME)`, g_game.c:454), or `None` when the
    /// lump lies outside F_START..F_END: the genuine `R_FlatNumForName` returns that
    /// out-of-range number unchecked (r_data.c:672-687) and every reader only compares it
    /// with a sector's pic (r_bsp.c:526, r_plane.c:225 and 396, r_segs.c:530-531 and 672),
    /// which `Level::load` has resolved inside the range, so it never matches.
    pub skyflatnum: Option<usize>,
}

impl Resources {
    /// Loads everything `R_Init`, `P_InitSwitchList`, `P_InitPicAnims` and `R_InitSprites`
    /// build, in the genuine order, from the global lump list.
    pub fn load(dir: LumpDirectory, mode: GameMode) -> Result<Self, ResourceError> {
        let textures = init_textures(&dir)?;
        let (firstflat, numflats) = marker_range(&dir, b"F_START", b"F_END", "F_START/F_END")?;
        let (firstspritelump, numsprite) =
            marker_range(&dir, b"S_START", b"S_END", "S_START/S_END")?;
        let mut sprite_lumps = Vec::with_capacity(numsprite);
        for i in 0..numsprite {
            let lump = firstspritelump + i;
            let h = patch_header(dir.bytes(lump)).ok_or_else(|| ResourceError::LumpTooShort {
                name: name_text(&dir.names[lump]),
                needed: 8,
                length: dir.bytes(lump).len(),
            })?;
            sprite_lumps.push([
                i32::from(h.width) << 16,
                i32::from(h.leftoffset) << 16,
                i32::from(h.topoffset) << 16,
            ]);
        }
        let cm = dir.get_num_for_name(b"COLORMAP")?;
        let colormaps = dir.bytes(cm).to_vec();
        if colormaps.len() < 33 * 256 {
            return Err(ResourceError::LumpTooShort {
                name: "COLORMAP".into(),
                needed: 33 * 256,
                length: colormaps.len(),
            });
        }
        let palette = dir
            .check_num_for_name(b"PLAYPAL")
            .and_then(|l| dir.bytes(l).get(..768).map(<[u8]>::to_vec));
        let mut res = Self {
            dir,
            colormaps,
            palette,
            textures,
            firstflat,
            numflats,
            firstspritelump,
            sprite_lumps,
            sprites: Vec::new(),
            switchlist: Vec::new(),
            anims: Vec::new(),
            skytexture: 0,
            skyflatnum: None,
        };
        res.switchlist = crate::p_spec::init_switch_list(&res, mode)?;
        res.anims = crate::p_spec::init_pic_anims(&res)?;
        res.sprites = crate::r_things::init_sprite_defs(&res)?;
        res.skytexture = res.texture_num_for_name(b"SKY1")?;
        res.skyflatnum = match res.flat_num_for_name(crate::r_sky::SKYFLATNAME) {
            Ok(n) => Some(n),
            Err(ResourceError::FlatOutOfRange(_)) => None,
            Err(e) => return Err(e),
        };
        Ok(res)
    }

    /// The global lump list.
    pub fn directory(&self) -> &LumpDirectory {
        &self.dir
    }

    /// `R_CheckTextureNumForName` (r_data.c:700-712): `'-'` is texture 0, otherwise the first
    /// texture whose name matches with `strncasecmp(.., 8)`.
    pub fn check_texture_num_for_name(&self, name: &[u8]) -> Option<usize> {
        if name.first() == Some(&b'-') {
            return Some(0);
        }
        let want = c_name(name);
        self.textures.iter().position(|t| {
            let have = c_name(&t.name);
            have.eq_ignore_ascii_case(&want)
        })
    }
    /// `R_TextureNumForName` (r_data.c:718-730).
    pub fn texture_num_for_name(&self, name: &[u8]) -> Result<usize, ResourceError> {
        self.check_texture_num_for_name(name)
            .ok_or_else(|| ResourceError::TextureNotFound(name_text(name)))
    }
    /// `R_FlatNumForName` (r_data.c:680-692), refusing a lump outside the flat range.
    pub fn flat_num_for_name(&self, name: &[u8]) -> Result<usize, ResourceError> {
        let lump = self
            .dir
            .check_num_for_name(name)
            .ok_or_else(|| ResourceError::FlatNotFound(name_text(name)))?;
        lump.checked_sub(self.firstflat)
            .filter(|n| *n < self.numflats)
            .ok_or_else(|| ResourceError::FlatOutOfRange(name_text(name)))
    }
    /// The bytes of flat `flatnum` (`W_CacheLumpNum(firstflat + flatnum)`).
    pub fn flat(&self, flatnum: usize) -> &[u8] {
        self.dir.bytes(self.firstflat + flatnum)
    }
    /// The bytes of lump `lump`.
    pub fn lump(&self, lump: usize) -> &[u8] {
        self.dir.bytes(lump)
    }
    /// `R_GetColumn(tex, col)` (r_data.c:382-401): `col &= texturewidthmask`, then the patch
    /// lump at its offset when `texturecolumnlump > 0`, else the composite.
    pub fn get_column(&self, tex: usize, col: i32) -> ColumnSource<'_> {
        let t = &self.textures[tex];
        let col = (col & t.widthmask) as usize;
        let none = ColumnSource {
            block: &[],
            defined: None,
            start: 0,
            undefined_column: true,
            unallocated: false,
        };
        match t
            .columns
            .get(col)
            .copied()
            .unwrap_or(ColumnEntry::Unallocated)
        {
            ColumnEntry::Lump { lump, ofs } => ColumnSource {
                block: self.dir.bytes(lump),
                defined: None,
                start: usize::from(ofs),
                undefined_column: false,
                unallocated: false,
            },
            ColumnEntry::Composite { ofs } | ColumnEntry::LumpZero { ofs } => ColumnSource {
                block: &t.composite,
                defined: Some(&t.defined),
                start: usize::from(ofs),
                undefined_column: false,
                unallocated: false,
            },
            ColumnEntry::Undefined => none,
            ColumnEntry::Unallocated => ColumnSource {
                unallocated: true,
                ..none
            },
        }
    }
}

/// Where a masked column's posts start (`R_GetColumn (tex, col) - 3`, r_segs.c:181-182).
#[derive(Clone, Copy, Debug)]
pub enum MaskedPosts<'a> {
    /// A patch column: the patch lump and the offset of its first post header.
    Lump {
        /// The patch lump's bytes.
        bytes: &'a [u8],
        /// `texturecolumnofs - 3`.
        at: usize,
    },
    /// A composite column (or lump 0's): no post headers inside the block.
    Composite,
    /// A column `R_GenerateLookup` left uninitialised, or an offset below 3.
    Undefined,
    /// A column whose tables or composite no allocation holds ([`ColumnEntry::Unallocated`]).
    Unallocated,
}

impl Resources {
    /// The posts a masked midtexture column walks (`R_GetColumn` then `- 3`, r_segs.c:181-182).
    pub fn masked_posts(&self, tex: usize, col: i32) -> MaskedPosts<'_> {
        let t = &self.textures[tex];
        let col = (col & t.widthmask) as usize;
        match t
            .columns
            .get(col)
            .copied()
            .unwrap_or(ColumnEntry::Unallocated)
        {
            ColumnEntry::Lump { lump, ofs } => match usize::from(ofs).checked_sub(3) {
                Some(at) => MaskedPosts::Lump {
                    bytes: self.dir.bytes(lump),
                    at,
                },
                None => MaskedPosts::Undefined,
            },
            ColumnEntry::Composite { .. } | ColumnEntry::LumpZero { .. } => MaskedPosts::Composite,
            ColumnEntry::Undefined => MaskedPosts::Undefined,
            ColumnEntry::Unallocated => MaskedPosts::Unallocated,
        }
    }
    /// A column source starting at byte `start` of lump `bytes` (a masked post's pixels).
    pub fn lump_source(bytes: &[u8], start: usize) -> ColumnSource<'_> {
        ColumnSource {
            block: bytes,
            defined: None,
            start,
            undefined_column: false,
            unallocated: false,
        }
    }
}

fn marker_range(
    dir: &LumpDirectory,
    start: &[u8],
    end: &[u8],
    what: &'static str,
) -> Result<(usize, usize), ResourceError> {
    // `first = W_GetNumForName(start) + 1`, `last = W_GetNumForName(end) - 1`,
    // `count = last - first + 1` (r_data.c:585-587, 608-611): 0 when the markers are adjacent.
    let first = dir.get_num_for_name(start)? + 1;
    let end_lump = dir.get_num_for_name(end)?;
    let count = end_lump
        .checked_sub(first)
        .ok_or(ResourceError::BadMarkerRange(what))?;
    Ok((first, count))
}

/// `R_InitTextures` (r_data.c:411-574) and `R_GenerateLookup` (r_data.c:296-374) for every
/// texture, then `R_GenerateComposite` (r_data.c:228-289) eagerly (the genuine code builds
/// composites lazily; the result is the same except for the bytes no patch wrote, which are
/// tracked as undefined).
fn init_textures(dir: &LumpDirectory) -> Result<Vec<Texture>, ResourceError> {
    let pnames = dir.bytes(dir.get_num_for_name(b"PNAMES")?);
    let count = read_i32(pnames, 0)
        .filter(|n| *n >= 0)
        .ok_or_else(|| ResourceError::BadTextureDirectory("PNAMES".into()))?;
    let count = count as usize;
    if count
        .checked_mul(8)
        .and_then(|n| n.checked_add(4))
        .is_none_or(|n| n > pnames.len())
    {
        return Err(ResourceError::BadTextureDirectory("PNAMES".into()));
    }
    let lookup: Vec<Option<usize>> = (0..count)
        .map(|i| dir.check_num_for_name(&pnames[4 + 8 * i..12 + 8 * i]))
        .collect();
    let maptex1 = dir.bytes(dir.get_num_for_name(b"TEXTURE1")?);
    let numtextures1 = read_i32(maptex1, 0)
        .ok_or_else(|| ResourceError::BadTextureDirectory("TEXTURE1".into()))?;
    let maptex2 = dir.check_num_for_name(b"TEXTURE2").map(|l| dir.bytes(l));
    let numtextures2 = match maptex2 {
        Some(m) => {
            read_i32(m, 0).ok_or_else(|| ResourceError::BadTextureDirectory("TEXTURE2".into()))?
        }
        None => 0,
    };
    // `numtextures = numtextures1 + numtextures2` (r_data.c:480) sizes seven `Z_Malloc`s
    // (r_data.c:482-488): a negative total is a negative allocation (z_zone.c:195-203), and
    // an overflowing sum is undefined.
    let numtextures = numtextures1
        .checked_add(numtextures2)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| ResourceError::BadTextureDirectory("TEXTURE1+TEXTURE2".into()))?;
    let mut textures = Vec::new();
    let (mut lump, mut what, mut base_index) = (maptex1, "TEXTURE1", 0usize);
    for i in 0..numtextures {
        // The directory switches to TEXTURE2 exactly at `i == numtextures1` (r_data.c:509-515):
        // never when `numtextures1` is negative, so every entry is then read from TEXTURE1's
        // directory, as it is when `numtextures2` is negative.
        if i64::try_from(i).ok() == Some(i64::from(numtextures1)) {
            if let Some(m) = maptex2 {
                (lump, what, base_index) = (m, "TEXTURE2", i);
            }
        }
        {
            let offset = read_i32(lump, 4 + 4 * (i - base_index))
                .ok_or_else(|| ResourceError::BadTextureDirectory(what.into()))?;
            // r_data.c:520 checks only `offset > maxoff`; any read outside the lump is refused.
            let base = usize::try_from(offset)
                .ok()
                .filter(|o| *o <= lump.len())
                .ok_or_else(|| ResourceError::BadTextureDirectory(what.into()))?;
            let m = lump
                .get(base..)
                .filter(|m| m.len() >= 22)
                .ok_or_else(|| ResourceError::BadTextureDirectory(what.into()))?;
            let mut name = [0u8; 8];
            name.copy_from_slice(&m[..8]);
            let width = read_i16(m, 12).unwrap_or(0);
            let height = read_i16(m, 14).unwrap_or(0);
            let patchcount = read_i16(m, 20).unwrap_or(0);
            // A zero width, a zero patch count and any height are well defined at load
            // (r_data.c:524-559, 322-323); a negative width or patch count is not.
            if width < 0 || patchcount < 0 {
                return Err(ResourceError::BadTextureSize(name_text(&name)));
            }
            let mut patches = Vec::with_capacity(patchcount as usize);
            for j in 0..patchcount as usize {
                let p = 22 + 10 * j;
                let (Some(ox), Some(oy), Some(idx)) =
                    (read_i16(m, p), read_i16(m, p + 2), read_i16(m, p + 4))
                else {
                    return Err(ResourceError::BadTextureDirectory(what.into()));
                };
                let entry = usize::try_from(idx)
                    .ok()
                    .and_then(|k| lookup.get(k))
                    .ok_or_else(|| ResourceError::PatchIndexOutOfRange {
                        texture: name_text(&name),
                        index: i32::from(idx),
                    })?;
                let patch = entry.ok_or_else(|| ResourceError::MissingPatch(name_text(&name)))?;
                patches.push(TexPatch {
                    originx: i32::from(ox),
                    originy: i32::from(oy),
                    patch,
                });
            }
            let mut j = 1i32;
            while j * 2 <= i32::from(width) {
                j <<= 1;
            }
            textures.push(Texture {
                name,
                width,
                height,
                patches,
                widthmask: j - 1,
                textureheight: i32::from(height) << 16,
                columns: Vec::new(),
                compositesize: 0,
                composite: Vec::new(),
                defined: Vec::new(),
            });
        }
    }
    for (num, t) in textures.iter_mut().enumerate() {
        generate_lookup(dir, num, t)?;
        generate_composite(dir, t)?;
    }
    Ok(textures)
}

/// `R_GenerateLookup` (r_data.c:296-374).
fn generate_lookup(dir: &LumpDirectory, num: usize, t: &mut Texture) -> Result<(), ResourceError> {
    let width = t.width as usize;
    // `byte* patchcount` (r_data.c:299, 322-323): the count wraps modulo 256
    // (`patchcount[x]++`, r_data.c:343), so 256 patches read as 0 ("column without a patch")
    // and 257 as 1 (a single-patch column of the last patch).
    let mut patchcount = vec![0u8; width];
    // The genuine tables are uninitialised Z_Malloc memory; a column no patch covers is never
    // assigned (see `ColumnEntry::Undefined`).
    let mut cols: Vec<Option<(usize, u16)>> = vec![None; width];
    for p in &t.patches {
        let bytes = dir.bytes(p.patch);
        let h = patch_header(bytes).ok_or(ResourceError::PatchBounds { lump: p.patch })?;
        let x1 = p.originx;
        let x2 = (x1 + i32::from(h.width)).min(i32::from(t.width));
        for x in x1.max(0)..x2 {
            let ofs = patch_column_offset(bytes, x - x1)
                .ok_or(ResourceError::PatchBounds { lump: p.patch })?;
            patchcount[x as usize] = patchcount[x as usize].wrapping_add(1);
            // `colofs[x] = LONG(columnofs[x-x1]) + 3`, stored as unsigned short.
            cols[x as usize] = Some((p.patch, ofs.wrapping_add(3) as u16));
        }
    }
    let mut entries = Vec::with_capacity(width);
    let mut size = 0i64;
    let mut stopped = false;
    for x in 0..width {
        if stopped {
            // After "column without a patch" the genuine loop has returned: columns keep the
            // last patch's lump and offset (defined), or nothing.
            entries.push(match cols[x] {
                Some((lump, ofs)) => lump_entry(lump, ofs),
                None => ColumnEntry::Undefined,
            });
            continue;
        }
        if patchcount[x] == 0 {
            // "column without a patch" (r_data.c:350-356): the function returns, leaving this
            // and every later column with what the first loop stored (a column covered by a
            // multiple of 256 patches keeps its last patch's lump and offset).
            stopped = true;
            entries.push(match cols[x] {
                Some((lump, ofs)) => lump_entry(lump, ofs),
                None => ColumnEntry::Undefined,
            });
            continue;
        }
        if patchcount[x] > 1 {
            // `texturecompositesize > 0x10000 - height` (r_data.c:365-371), in `int`.
            if size > 0x10000 - i64::from(t.height) {
                return Err(ResourceError::CompositeTooLarge(num));
            }
            if t.height < 0 {
                // The composite would be `Z_Malloc`ed with the negative size (r_data.c:244).
                entries.push(ColumnEntry::Unallocated);
            } else {
                entries.push(ColumnEntry::Composite { ofs: size as u16 });
            }
            size += i64::from(t.height);
        } else {
            let (lump, ofs) = cols[x].expect("covered column");
            entries.push(lump_entry(lump, ofs));
        }
    }
    t.columns = entries;
    // A negative height leaves no composite column (they are all `Unallocated`).
    t.compositesize = usize::try_from(size).unwrap_or(0);
    Ok(())
}

/// A single-patch column: `R_GetColumn` takes the lump path only for `lump > 0`.
fn lump_entry(lump: usize, ofs: u16) -> ColumnEntry {
    if lump > 0 {
        ColumnEntry::Lump { lump, ofs }
    } else {
        ColumnEntry::LumpZero { ofs }
    }
}

/// `R_GenerateComposite` (r_data.c:228-289) with `R_DrawColumnInCache` (r_data.c:184-216).
/// A post walk that would leave the patch lump marks that composite column undefined.
fn generate_composite(dir: &LumpDirectory, t: &mut Texture) -> Result<(), ResourceError> {
    let size = t.compositesize;
    let mut block = vec![0u8; size];
    let mut defined = vec![false; size];
    let height = i32::from(t.height);
    for p in &t.patches {
        let bytes = dir.bytes(p.patch);
        let h = patch_header(bytes).ok_or(ResourceError::PatchBounds { lump: p.patch })?;
        let x1 = p.originx;
        let x2 = (x1 + i32::from(h.width)).min(i32::from(t.width));
        for x in x1.max(0)..x2 {
            let ColumnEntry::Composite { ofs } = t.columns[x as usize] else {
                continue;
            };
            let base = usize::from(ofs);
            let col = patch_column_offset(bytes, x - x1)
                .ok_or(ResourceError::PatchBounds { lump: p.patch })?;
            draw_column_in_cache(
                bytes,
                col,
                &mut block,
                &mut defined,
                base,
                p.originy,
                height,
            );
        }
    }
    t.composite = block;
    t.defined = defined;
    Ok(())
}

fn draw_column_in_cache(
    patch: &[u8],
    column: i32,
    block: &mut [u8],
    defined: &mut [bool],
    base: usize,
    originy: i32,
    cacheheight: i32,
) {
    let Ok(mut at) = usize::try_from(column) else {
        poison(defined, base, cacheheight);
        return;
    };
    loop {
        let Some(&topdelta) = patch.get(at) else {
            poison(defined, base, cacheheight);
            return;
        };
        if topdelta == 0xff {
            return;
        }
        let Some(&length) = patch.get(at + 1) else {
            poison(defined, base, cacheheight);
            return;
        };
        let source = at + 3;
        let mut count = i32::from(length);
        let mut position = originy + i32::from(topdelta);
        if position < 0 {
            count += position;
            position = 0;
        }
        if position + count > cacheheight {
            count = cacheheight - position;
        }
        if count > 0 {
            // memcpy (cache + position, source, count): the source starts at the post's data
            // (the genuine code copies from `source` even when position was clamped).
            for k in 0..count as usize {
                let dst = base + position as usize + k;
                match (patch.get(source + k), block.get_mut(dst)) {
                    (Some(b), Some(d)) => {
                        *d = *b;
                        defined[dst] = true;
                    }
                    (None, Some(_)) => defined[dst] = false,
                    _ => {}
                }
            }
        }
        at = at + usize::from(length) + 4;
    }
}

fn poison(defined: &mut [bool], base: usize, height: i32) {
    for d in defined.iter_mut().skip(base).take(height.max(0) as usize) {
        *d = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_lookup_is_exact() {
        let mut d = LumpDirectory::new();
        d.push(*b"FOO\0\0\0\0\0", &[1]).unwrap();
        d.push(*b"FOO\0XYZ\0", &[2]).unwrap();
        assert_eq!(d.check_num_for_name(b"foo"), Some(1));
        assert_eq!(d.check_num_for_name(b"FOOBAR"), None);
        assert_eq!(
            d.push(*b"bar\0\0\0\0\0", &[]),
            Err(ResourceError::DirectoryNameCase { lump: 2 })
        );
    }

    #[test]
    fn composite_clipping_copies_from_post_start() {
        // A patch column: post at topdelta 0, length 4, bytes 10..13, then 0xff.
        let patch = [0u8, 4, 0, 10, 11, 12, 13, 0, 0xff];
        let mut block = vec![0u8; 8];
        let mut defined = vec![false; 8];
        // originy -2: position -2 -> count 2 at 0, copied from the post's first byte.
        draw_column_in_cache(&patch, 0, &mut block, &mut defined, 0, -2, 8);
        assert_eq!(&block[..3], &[10, 11, 0]);
        assert_eq!(&defined[..3], &[true, true, false]);
    }

    // ---- Load-time refusals (investigation §2.3; each pins the genuine `I_Error` or the
    // unchecked read named in `ResourceError`). The public `Resources::load` reaches every one
    // of them; the committed graphics set is well formed, so the goldens reach none.

    fn dir(lumps: &[(&str, Vec<u8>)]) -> LumpDirectory {
        let mut d = LumpDirectory::new();
        for (name, data) in lumps {
            d.add_single_lump(name, data).unwrap();
        }
        d
    }
    fn pnames(names: &[&str]) -> Vec<u8> {
        let mut o = (names.len() as i32).to_le_bytes().to_vec();
        for n in names {
            let mut b = [0u8; 8];
            b[..n.len()].copy_from_slice(n.as_bytes());
            o.extend(b);
        }
        o
    }
    type TextureDef<'a> = (&'a str, i16, i16, &'a [(i16, i16, i16)]);
    /// A TEXTURE lump: `(name, width, height, [(originx, originy, pnames index)])` entries
    /// (the `maptexture_t` layout of §2.3.1: name, masked, width, height, columndirectory,
    /// patchcount, then 10-byte `mappatch_t`).
    fn texture_lump(entries: &[TextureDef]) -> Vec<u8> {
        let mut o = (entries.len() as i32).to_le_bytes().to_vec();
        let mut body = Vec::new();
        let base = 4 + 4 * entries.len();
        for (name, w, h, patches) in entries {
            o.extend(((base + body.len()) as i32).to_le_bytes());
            let mut b = [0u8; 8];
            b[..name.len()].copy_from_slice(name.as_bytes());
            body.extend(b);
            body.extend(0i32.to_le_bytes());
            body.extend(w.to_le_bytes());
            body.extend(h.to_le_bytes());
            body.extend(0i32.to_le_bytes());
            body.extend((patches.len() as i16).to_le_bytes());
            for (ox, oy, idx) in *patches {
                for v in [*ox, *oy, *idx, 1, 0] {
                    body.extend(v.to_le_bytes());
                }
            }
        }
        o.extend(body);
        o
    }
    /// A patch lump of `width` columns, every column an empty post list (`0xff`).
    fn patch(width: i16, height: i16) -> Vec<u8> {
        let mut o = Vec::new();
        for v in [width, height, 0, 0] {
            o.extend(v.to_le_bytes());
        }
        let first = 8 + 4 * width as usize;
        for x in 0..width as usize {
            o.extend(((first + x) as i32).to_le_bytes());
        }
        o.extend(vec![0xff; width as usize]);
        o
    }
    fn load(lumps: &[(&str, Vec<u8>)]) -> Result<Resources, ResourceError> {
        Resources::load(dir(lumps), GameMode::Registered)
    }
    fn err(lumps: &[(&str, Vec<u8>)]) -> ResourceError {
        load(lumps).expect_err("a refusal")
    }
    /// The lumps up to `TEXTURE1`.
    fn textures_only() -> Vec<(&'static str, Vec<u8>)> {
        vec![("PNAMES", pnames(&[])), ("TEXTURE1", texture_lump(&[]))]
    }

    #[test]
    fn startup_lumps_are_required_in_the_genuine_order() {
        // R_InitData: PNAMES and TEXTURE1 (r_data.c:411-479), then the flat and sprite marker
        // ranges, then COLORMAP; `W_GetNumForName` is an `I_Error` (w_wad.c).
        let nf = |n: &str| ResourceError::LumpNotFound(n.into());
        assert_eq!(err(&[]), nf("PNAMES"));
        assert_eq!(err(&[("PNAMES", pnames(&[]))]), nf("TEXTURE1"));
        assert_eq!(err(&textures_only()), nf("F_START"));
        let mut l = textures_only();
        l.extend([("F_START", vec![]), ("F_END", vec![])]);
        // F_START immediately followed by F_END encloses no flat: `numflats` 0 is well
        // defined (r_data.c:581-594), so the load goes on to the sprite markers.
        assert_eq!(err(&l), nf("S_START"));
        let mut l = textures_only();
        l.extend([
            ("F_START", vec![]),
            ("FLAT", vec![0; 4096]),
            ("F_END", vec![]),
        ]);
        assert_eq!(err(&l), nf("S_START"));
        l.extend([("S_START", vec![]), ("S_END", vec![])]);
        // No sprite lump: `numspritelumps` 0 is well defined too (r_data.c:603-626).
        assert_eq!(err(&l), nf("COLORMAP"));
        let mut l = textures_only();
        l.extend([
            ("F_START", vec![]),
            ("FLAT", vec![0; 4096]),
            ("F_END", vec![]),
            ("S_END", vec![]),
            ("S_START", vec![]),
        ]);
        assert_eq!(err(&l), ResourceError::BadMarkerRange("S_START/S_END"));
        // F_END before F_START: the range is negative.
        let mut l = textures_only();
        l.extend([
            ("F_END", vec![]),
            ("F_START", vec![]),
            ("FLAT", vec![0; 4096]),
        ]);
        assert_eq!(err(&l), ResourceError::BadMarkerRange("F_START/F_END"));
    }

    fn with_sprites(extra: &[(&'static str, Vec<u8>)]) -> Vec<(&'static str, Vec<u8>)> {
        let mut l = textures_only();
        l.extend([
            ("F_START", vec![]),
            ("FLAT", vec![0; 4096]),
            ("F_END", vec![]),
        ]);
        l.extend([
            ("S_START", vec![]),
            ("TROOA1", patch(1, 1)),
            ("S_END", vec![]),
        ]);
        l.extend(extra.iter().cloned());
        l
    }

    #[test]
    fn sprite_lump_shorter_than_its_header_is_refused() {
        // R_InitSpriteLumps reads `patch->width/leftoffset/topoffset` (§2.3.6, 8-byte
        // header) of every lump between S_START and S_END.
        let mut l = textures_only();
        l.extend([
            ("F_START", vec![]),
            ("FLAT", vec![0; 4096]),
            ("F_END", vec![]),
        ]);
        l.extend([
            ("S_START", vec![]),
            ("SHORT", vec![1, 2, 3]),
            ("S_END", vec![]),
        ]);
        assert_eq!(
            err(&l),
            ResourceError::LumpTooShort {
                name: "SHORT".into(),
                needed: 8,
                length: 3
            }
        );
    }

    #[test]
    fn colormap_is_required_and_holds_33_maps() {
        // R_InitColormaps (§2.3.6): `W_GetNumForName("COLORMAP")`; the renderer reads
        // maps 0..=32 (32 light maps and the inverse map, investigation §2.3.6).
        assert_eq!(
            err(&with_sprites(&[])),
            ResourceError::LumpNotFound("COLORMAP".into())
        );
        assert_eq!(
            err(&with_sprites(&[("COLORMAP", vec![0; 33 * 256 - 1])])),
            ResourceError::LumpTooShort {
                name: "COLORMAP".into(),
                needed: 33 * 256,
                length: 33 * 256 - 1
            }
        );
    }

    #[test]
    fn lump_directory_holds_at_most_a_short_lump_number() {
        // `texturecolumnlump` is a `short` array (r_data.c:140): a 32,768th lump cannot be
        // numbered (`W_AddFile` would overflow it).
        let mut d = LumpDirectory::new();
        for _ in 0..i16::MAX {
            d.add_single_lump("LUMP", &[]).unwrap();
        }
        assert_eq!(
            d.add_single_lump("LUMP", &[]),
            Err(ResourceError::TooManyLumps)
        );
    }

    #[test]
    fn pnames_and_texture_directories_are_validated() {
        let bad = |what: &str| ResourceError::BadTextureDirectory(what.into());
        // A negative or oversized PNAMES count (R_InitTextures reads `nummappatches` names, r_data.c:411-574).
        assert_eq!(
            err(&[("PNAMES", (-1i32).to_le_bytes().to_vec())]),
            bad("PNAMES")
        );
        assert_eq!(
            err(&[("PNAMES", 2i32.to_le_bytes().to_vec())]),
            bad("PNAMES")
        );
        assert_eq!(err(&[("PNAMES", vec![1, 2])]), bad("PNAMES"));
        // A negative count, or an entry offset outside the lump (r_data.c:520 checks only
        // `offset > maxoff`; the renderer refuses every read outside the lump).
        let t = |lump: Vec<u8>| vec![("PNAMES", pnames(&[])), ("TEXTURE1", lump)];
        assert_eq!(
            err(&t((-3i32).to_le_bytes().to_vec())),
            bad("TEXTURE1+TEXTURE2")
        );
        assert_eq!(err(&t(vec![])), bad("TEXTURE1"));
        let mut lump = 1i32.to_le_bytes().to_vec();
        lump.extend(1000i32.to_le_bytes());
        assert_eq!(err(&t(lump)), bad("TEXTURE1"));
        // An offset leaving fewer than the 22 header bytes of a `maptexture_t`.
        let mut lump = 1i32.to_le_bytes().to_vec();
        lump.extend(8i32.to_le_bytes());
        lump.extend([0; 10]);
        assert_eq!(err(&t(lump)), bad("TEXTURE1"));
        // A truncated patch list (`patchcount` 2, one `mappatch_t` present).
        let mut lump = texture_lump(&[("WALL", 8, 8, &[(0, 0, 0)])]);
        let at = 4 + 4 + 20;
        lump[at..at + 2].copy_from_slice(&2i16.to_le_bytes());
        let l = vec![
            ("PNAMES", pnames(&["P"])),
            ("P", patch(8, 8)),
            ("TEXTURE1", lump),
        ];
        assert_eq!(err(&l), bad("TEXTURE1"));
    }

    #[test]
    fn texture_definitions_are_validated() {
        let one = |w, h, patches: &[(i16, i16, i16)]| {
            vec![
                ("PNAMES", pnames(&["P", "ABSENT"])),
                ("P", patch(8, 8)),
                ("TEXTURE1", texture_lump(&[("WALL", w, h, patches)])),
            ]
        };
        // A negative width is refused (`alloca`/`memset` of a negative size, r_data.c:322-323);
        // a zero width, a zero or negative height and no patch at all load (the next lump
        // the load needs, F_START, is then missing).
        assert_eq!(
            err(&one(-8, 8, &[(0, 0, 0)])),
            ResourceError::BadTextureSize("WALL".into())
        );
        for (w, h, p) in [(0, 8, 1), (8, 0, 1), (8, -8, 1), (8, 8, 0)] {
            let patches: &[(i16, i16, i16)] = &[(0, 0, 0)][..p];
            assert_eq!(
                err(&one(w, h, patches)),
                ResourceError::LumpNotFound("F_START".into()),
                "{w} {h} {p}"
            );
        }
        // A `patch` index outside PNAMES: the genuine `patchlookup[patch]` read is unchecked.
        for index in [2, 100, -1] {
            assert_eq!(
                err(&one(8, 8, &[(0, 0, index)])),
                ResourceError::PatchIndexOutOfRange {
                    texture: "WALL".into(),
                    index: i32::from(index)
                }
            );
        }
        // `R_InitTextures: Missing patch in texture %s` (r_data.c:544): PNAMES entry 1 names
        // no lump.
        assert_eq!(
            err(&one(8, 8, &[(0, 0, 1)])),
            ResourceError::MissingPatch("WALL".into())
        );
        // A patch lump shorter than its 8-byte header, and a column directory entry outside
        // the lump (R_GenerateLookup reads `patch->columnofs[x - x1]`, r_data.c:296-374).
        let mut l = one(8, 8, &[(0, 0, 0)]);
        l[1].1 = vec![1, 2, 3];
        assert_eq!(err(&l), ResourceError::PatchBounds { lump: 1 });
        let mut l = one(8, 8, &[(0, 0, 0)]);
        l[1].1 = patch(8, 8)[..8 + 4 * 3].to_vec();
        assert_eq!(err(&l), ResourceError::PatchBounds { lump: 1 });
    }

    fn lookup(
        name: &str,
        w: i16,
        h: i16,
        patches: &[(i16, i16, i16)],
        names: &[(&str, Vec<u8>)],
    ) -> Result<Vec<Texture>, ResourceError> {
        let mut l = vec![(
            "PNAMES",
            pnames(&names.iter().map(|n| n.0).collect::<Vec<_>>()),
        )];
        l.extend(names.iter().cloned());
        l.push(("TEXTURE1", texture_lump(&[(name, w, h, patches)])));
        init_textures(&dir(&l))
    }

    #[test]
    fn column_without_a_patch_leaves_the_rest_undefined() {
        // R_GenerateLookup (r_data.c:349-358): the first column no patch covers prints
        // "R_GenerateLookup: column without a patch" and returns, leaving every later column's
        // lump and offset uninitialised (`ColumnEntry::Undefined`); investigation §2.10 item 8.
        let t = lookup("WALL", 6, 8, &[(0, 0, 0)], &[("P", patch(2, 8))]).unwrap();
        let cols = &t[0].columns;
        assert!(matches!(cols[0], ColumnEntry::Lump { lump: 1, .. }));
        assert!(matches!(cols[1], ColumnEntry::Lump { lump: 1, .. }));
        assert_eq!(&cols[2..], &[ColumnEntry::Undefined; 4]);
    }

    #[test]
    fn column_covered_by_two_patches_is_composite_and_a_lump_zero_patch_is_not() {
        // r_data.c:349-373: `patchcount[x] > 1` selects the composite (`collump[x] = -1`,
        // `colofs[x] = texturecompositesize`, size += height); a single patch keeps its lump
        // and `columnofs + 3`, and a single patch in lump 0 stays `collump == 0`, which
        // `R_GetColumn` reads as a composite (r_data.c:270-272, `ColumnEntry::LumpZero`).
        let p = ("P", patch(4, 8));
        let t = lookup(
            "WALL",
            4,
            8,
            &[(0, 0, 0), (2, 0, 0)],
            std::slice::from_ref(&p),
        )
        .unwrap();
        let t = &t[0];
        assert_eq!(t.compositesize, 2 * 8);
        assert!(matches!(t.columns[0], ColumnEntry::Lump { lump: 1, ofs } if ofs == 8 + 16 + 3));
        assert_eq!(t.columns[2], ColumnEntry::Composite { ofs: 0 });
        assert_eq!(t.columns[3], ColumnEntry::Composite { ofs: 8 });
        // Lump 0: the patch is the first lump of the directory.
        let l = vec![
            ("P", patch(4, 8)),
            ("PNAMES", pnames(&["P"])),
            ("TEXTURE1", texture_lump(&[("WALL", 4, 8, &[(0, 0, 0)])])),
        ];
        let t = init_textures(&dir(&l)).unwrap();
        assert!(matches!(t[0].columns[0], ColumnEntry::LumpZero { .. }));
    }

    #[test]
    fn composite_larger_than_64k_is_refused() {
        // `R_GenerateLookup: texture %i is >64k` (r_data.c:367): the guard fires when the
        // composite size so far exceeds 0x10000 - height. 300 doubly covered columns of 255 rows
        // pass 65,281 at column 257.
        let p = ("P", patch(300, 255));
        let e = lookup("WALL", 300, 255, &[(0, 0, 0), (0, 0, 0)], &[p]).err();
        assert_eq!(e, Some(ResourceError::CompositeTooLarge(0)));
    }

    #[test]
    fn masked_posts_follow_r_get_column_minus_three() {
        // `R_GetColumn (tex, col) - 3` (r_segs.c:181-182): a patch column starts at its stored
        // offset (`columnofs + 3`) minus 3; an offset below 3 cannot be a patch column; a
        // composite column (and lump 0's) has no post headers inside the block (r_segs.c:181-184,
        // r_things.c:359-385 would run to the end of zone memory); an uninitialised column is
        // undefined (r_data.c:353-358). Constructed with the table fields directly: the loader
        // builds the same entries from patch geometry (the tests above).
        let columns = vec![
            ColumnEntry::Lump { lump: 1, ofs: 20 },
            ColumnEntry::Lump { lump: 1, ofs: 2 },
            ColumnEntry::Composite { ofs: 0 },
            ColumnEntry::LumpZero { ofs: 0 },
            ColumnEntry::Undefined,
        ];
        let res = Resources {
            dir: dir(&[("A", vec![]), ("B", vec![9; 40])]),
            colormaps: vec![],
            palette: None,
            textures: vec![Texture {
                name: *b"WALL\0\0\0\0",
                width: 5,
                height: 8,
                patches: vec![],
                widthmask: 7,
                textureheight: 8 << 16,
                columns,
                compositesize: 0,
                composite: vec![],
                defined: vec![],
            }],
            firstflat: 0,
            numflats: 0,
            firstspritelump: 0,
            sprite_lumps: vec![],
            sprites: vec![],
            switchlist: vec![],
            anims: vec![],
            skytexture: 0,
            skyflatnum: None,
        };
        assert!(
            matches!(res.masked_posts(0, 0), MaskedPosts::Lump { at: 17, bytes } if bytes.len() == 40)
        );
        assert!(matches!(res.masked_posts(0, 1), MaskedPosts::Undefined));
        assert!(matches!(res.masked_posts(0, 2), MaskedPosts::Composite));
        assert!(matches!(res.masked_posts(0, 3), MaskedPosts::Composite));
        assert!(matches!(res.masked_posts(0, 4), MaskedPosts::Undefined));
        // `col &= texturewidthmask` (r_data.c:382-401): column 8 is column 0.
        assert!(matches!(
            res.masked_posts(0, 8),
            MaskedPosts::Lump { at: 17, .. }
        ));
        // An undefined column reads as undefined texels (decision R4).
        assert_eq!(res.get_column(0, 4).texel(0), (0, false));
        assert_eq!(res.get_column(0, 0).texel(0), (9, true));
        assert_eq!(res.get_column(0, 0).texel(100), (0, false));
    }

    // ---- Review findings (investigation §7 items 40 and 41).

    /// A `Resources` holding `textures` only (the accessors under test read nothing else).
    fn with_textures(textures: Vec<Texture>) -> Resources {
        // Lump 1 is the 4-column patch the tests' `lookup` directories also hold at 1.
        Resources {
            dir: dir(&[("A", vec![]), ("P", patch(4, 8))]),
            colormaps: vec![],
            palette: None,
            textures,
            firstflat: 0,
            numflats: 0,
            firstspritelump: 0,
            sprite_lumps: vec![],
            sprites: vec![],
            switchlist: vec![],
            anims: vec![],
            skytexture: 0,
            skyflatnum: None,
        }
    }

    #[test]
    fn patch_counts_wrap_in_a_byte() {
        // `byte* patchcount` (r_data.c:299, 322-323, 343): column 0 is covered by n patches,
        // the last of them Q (lump 2, two columns, column directory at 16, so `ofs` 19 and 20);
        // column 1 by Q alone. 255 is a composite; 256 wraps to 0, "column without a patch"
        // (r_data.c:350-356) returns and leaves Q's lump and offset in both columns; 257 wraps
        // to 1, a single-patch column of Q.
        let names = [("P", patch(1, 8)), ("Q", patch(2, 8))];
        for (n, col0, size) in [
            (255usize, ColumnEntry::Composite { ofs: 0 }, 8),
            (256, ColumnEntry::Lump { lump: 2, ofs: 19 }, 0),
            (257, ColumnEntry::Lump { lump: 2, ofs: 19 }, 0),
        ] {
            let mut patches = vec![(0i16, 0i16, 0i16); n - 1];
            patches.push((0, 0, 1));
            let t = &lookup("WALL", 2, 8, &patches, &names).unwrap()[0];
            assert_eq!(t.columns[0], col0, "{n}");
            assert_eq!(t.columns[1], ColumnEntry::Lump { lump: 2, ofs: 20 }, "{n}");
            assert_eq!(t.compositesize, size, "{n}");
        }
        // 258 wraps to 2: a composite again.
        let mut patches = vec![(0i16, 0i16, 0i16); 257];
        patches.push((0, 0, 1));
        let t = &lookup("WALL", 2, 8, &patches, &names).unwrap()[0];
        assert_eq!(t.columns[0], ColumnEntry::Composite { ofs: 0 });
    }

    #[test]
    fn a_texture_without_patches_loads_with_undefined_columns() {
        // `patchcount` 0: the `texture_t` allocation is exactly its header (r_data.c:524-527)
        // and R_GenerateLookup stops at column 0 (r_data.c:350-356).
        let t = &lookup("WALL", 4, 8, &[], &[]).unwrap()[0];
        assert_eq!(t.columns, vec![ColumnEntry::Undefined; 4]);
        assert_eq!(t.compositesize, 0);
    }

    #[test]
    fn a_zero_width_texture_loads_and_its_columns_are_unallocated() {
        // Width 0: `alloca(0)` and 0-byte column tables (r_data.c:322-323, 548-549), width mask
        // 0 (r_data.c:551-555); every `R_GetColumn` reads past the tables (r_data.c:391-392).
        let t = lookup("WALL", 0, 8, &[(0, 0, 0)], &[("P", patch(4, 8))]).unwrap();
        assert!(t[0].columns.is_empty());
        assert_eq!(t[0].widthmask, 0);
        let res = with_textures(t);
        for col in [0, 1, -1, 1000] {
            assert!(res.get_column(0, col).unallocated(), "{col}");
            assert!(matches!(res.masked_posts(0, col), MaskedPosts::Unallocated));
        }
    }

    #[test]
    fn heights_of_zero_and_below_load() {
        // Height 0: composite columns add nothing to the composite size (r_data.c:371) and every
        // composite read is outside the empty block (undefined, decision R4).
        let p = ("P", patch(4, 8));
        let t = lookup(
            "WALL",
            4,
            0,
            &[(0, 0, 0), (2, 0, 0)],
            std::slice::from_ref(&p),
        )
        .unwrap();
        assert_eq!(t[0].columns[2], ColumnEntry::Composite { ofs: 0 });
        assert_eq!(t[0].columns[3], ColumnEntry::Composite { ofs: 0 });
        assert_eq!(t[0].compositesize, 0);
        assert_eq!(t[0].textureheight, 0);
        let res = with_textures(t);
        assert_eq!(res.get_column(0, 2).texel(0), (0, false));
        assert!(!res.get_column(0, 2).unallocated());
        // A negative height: `textureheight = height<<FRACBITS` (r_data.c:556); a composite
        // column would be built in a negative-size `Z_Malloc` (r_data.c:244, 365-371), so it is
        // unallocated, while a single-patch column reads its lump as usual.
        let t = lookup("WALL", 4, -8, &[(0, 0, 0), (2, 0, 0)], &[p]).unwrap();
        assert_eq!(t[0].textureheight, -8 << 16);
        assert!(matches!(t[0].columns[0], ColumnEntry::Lump { lump: 1, .. }));
        assert_eq!(t[0].columns[2], ColumnEntry::Unallocated);
        assert_eq!(t[0].columns[3], ColumnEntry::Unallocated);
        assert_eq!(t[0].compositesize, 0);
        let res = with_textures(t);
        assert!(!res.get_column(0, 0).unallocated());
        assert!(res.get_column(0, 2).unallocated());
        assert!(matches!(res.masked_posts(0, 3), MaskedPosts::Unallocated));
    }

    #[test]
    fn negative_patch_counts_are_refused() {
        // `Z_Malloc(sizeof(texture_t) + sizeof(texpatch_t)*(patchcount-1))` is smaller than the
        // fields R_InitTextures writes for a negative count (r_data.c:524-531).
        let mut lump = texture_lump(&[("WALL", 8, 8, &[])]);
        let at = 4 + 4 + 20;
        lump[at..at + 2].copy_from_slice(&(-1i16).to_le_bytes());
        let l = vec![("PNAMES", pnames(&[])), ("TEXTURE1", lump)];
        assert_eq!(
            init_textures(&dir(&l)).err(),
            Some(ResourceError::BadTextureSize("WALL".into()))
        );
    }

    #[test]
    fn texture_counts_combine_as_the_genuine_directory_walk() {
        // `numtextures = numtextures1 + numtextures2` (r_data.c:480); the directory switches
        // to TEXTURE2 only at `i == numtextures1` (r_data.c:509-515).
        let with_count = |entries: &[TextureDef], count: i32| {
            let mut lump = texture_lump(entries);
            lump[..4].copy_from_slice(&count.to_le_bytes());
            lump
        };
        let names = |l: &[(&str, Vec<u8>)]| -> Result<Vec<String>, ResourceError> {
            Ok(init_textures(&dir(l))?
                .iter()
                .map(|t| name_text(&t.name))
                .collect())
        };
        let a: TextureDef = ("A", 8, 8, &[]);
        let b: TextureDef = ("B", 8, 8, &[]);
        let c: TextureDef = ("C", 8, 8, &[]);
        let d: TextureDef = ("D", 8, 8, &[]);
        let base = |t1: Vec<u8>, t2: Vec<u8>| {
            vec![("PNAMES", pnames(&[])), ("TEXTURE1", t1), ("TEXTURE2", t2)]
        };
        // The ordinary case: TEXTURE1's entries, then TEXTURE2's.
        assert_eq!(
            names(&base(texture_lump(&[a, b]), texture_lump(&[c, d]))),
            Ok(vec!["A".into(), "B".into(), "C".into(), "D".into()])
        );
        // numtextures1 -1, numtextures2 2: one texture, read from TEXTURE1's directory.
        assert_eq!(
            names(&base(with_count(&[a, b], -1), texture_lump(&[c, d]))),
            Ok(vec!["A".into()])
        );
        // numtextures1 3, numtextures2 -1: TEXTURE1's first two entries; TEXTURE2 unread.
        assert_eq!(
            names(&base(with_count(&[a, b, c], 3), with_count(&[d], -1))),
            Ok(vec!["A".into(), "B".into()])
        );
        // A negative total sizes negative allocations (r_data.c:482-488), and the sum may not
        // overflow.
        let bad = Err(ResourceError::BadTextureDirectory(
            "TEXTURE1+TEXTURE2".into(),
        ));
        assert_eq!(names(&base(with_count(&[a], -2), texture_lump(&[c]))), bad);
        assert_eq!(
            names(&base(with_count(&[a], i32::MAX), texture_lump(&[c]))),
            bad
        );
    }

    #[test]
    fn a_sky_flat_outside_the_flat_range_never_matches() {
        // R_FlatNumForName returns an out-of-range number unchecked (r_data.c:672-687) and
        // `skyflatnum` is only compared with sector pics; a later F_SKY1 outside
        // F_START..F_END (the last match wins, w_wad.c) loads with no sky flat number.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
        let gfx_bytes = std::fs::read(root.join("m5a_gfx.wad")).expect("graphics WAD");
        let gfx = doom_wad::Wad::parse(&gfx_bytes, doom_wad::Limits::default()).expect("gfx");
        let mut d = LumpDirectory::new();
        d.add_wad(&gfx).unwrap();
        let res = Resources::load(d.clone(), GameMode::Registered).unwrap();
        assert!(res.skyflatnum.is_some());
        d.add_single_lump("F_SKY1", &[0; 4096]).unwrap();
        let res = Resources::load(d, GameMode::Registered).unwrap();
        assert_eq!(res.skyflatnum, None);
    }

    #[test]
    fn the_lump_directory_reports_its_lumps() {
        // `numlumps` and the lump bytes by number (w_wad.c).
        let mut d = LumpDirectory::new();
        assert!(d.is_empty());
        assert_eq!(d.len(), 0);
        assert_eq!(d.data(0), None);
        d.add_single_lump("FIRST", &[1, 2]).unwrap();
        d.add_single_lump("SECOND", &[3]).unwrap();
        assert!(!d.is_empty());
        assert_eq!(d.len(), 2);
        assert_eq!(d.data(0), Some(&[1u8, 2][..]));
        assert_eq!(d.data(1), Some(&[3u8][..]));
        assert_eq!(d.data(2), None);
    }

    #[test]
    fn resource_errors_display_their_variant() {
        assert_eq!(
            ResourceError::TooManyLumps.to_string(),
            "renderer resources: TooManyLumps"
        );
    }

    #[test]
    fn a_flat_name_must_name_a_lump_inside_the_flat_range() {
        // R_FlatNumForName (r_data.c:680-692) is `W_GetNumForName(name) - firstflat`; the refusal
        // is for a lump outside F_START..F_END: the F_END marker itself (the lump after the
        // last flat) is outside it.
        let mut d = LumpDirectory::new();
        for (n, len) in [
            ("BEFORE", 4096),
            ("F_START", 0),
            ("A", 4096),
            ("B", 4096),
            ("F_END", 0),
            ("AFTER", 4096),
        ] {
            d.add_single_lump(n, &vec![0; len]).unwrap();
        }
        let res = Resources {
            dir: d,
            colormaps: vec![],
            palette: None,
            textures: vec![],
            firstflat: 2,
            numflats: 2,
            firstspritelump: 0,
            sprite_lumps: vec![],
            sprites: vec![],
            switchlist: vec![],
            anims: vec![],
            skytexture: 0,
            skyflatnum: None,
        };
        assert_eq!(res.flat_num_for_name(b"A"), Ok(0));
        assert_eq!(res.flat_num_for_name(b"B"), Ok(1));
        for n in [&b"F_END"[..], b"AFTER", b"BEFORE", b"F_START"] {
            assert!(
                matches!(
                    res.flat_num_for_name(n),
                    Err(ResourceError::FlatOutOfRange(_))
                ),
                "{n:?}"
            );
        }
        assert!(matches!(
            res.flat_num_for_name(b"NOPE"),
            Err(ResourceError::FlatNotFound(_))
        ));
    }

    #[test]
    fn a_composite_texture_may_fill_exactly_64k() {
        // R_GenerateLookup (r_data.c:365-371): `if (texturecompositesize[texnum] > 0x10000 -
        // texture->height) I_Error("R_GenerateLookup: texture %i is >64k")`, checked before each
        // multi-patch column adds its height. 128-high columns: the 512th column starts at
        // 511 x 128 = 0x10000 - 128, which is allowed; a 513th starts at 0x10000 and is not.
        let ok = |w: i16| {
            let p = ("P", patch(w, 128));
            lookup("WALL", w, 128, &[(0, 0, 0), (0, 0, 0)], &[p])
        };
        let t = ok(512).expect("512 columns of 128 rows fill the 64 KiB exactly");
        assert_eq!(t[0].compositesize, 0x10000);
        assert_eq!(ok(513).err(), Some(ResourceError::CompositeTooLarge(0)));
    }

    // ---- R_DrawColumnInCache (r_data.c:152-185) on hand-made patch columns.

    /// A post column: (topdelta, data) posts, each `topdelta, length, pad, data, pad`, then 0xff.
    fn posts(list: &[(u8, &[u8])]) -> Vec<u8> {
        let mut o = Vec::new();
        for (top, data) in list {
            o.extend([*top, data.len() as u8, 0]);
            o.extend(*data);
            o.push(0);
        }
        o.push(0xff);
        o
    }

    /// The block (length 16, base 4) and `defined` after drawing `patch` at `originy` into a
    /// cache of `height` rows.
    fn cache(patch: &[u8], column: i32, originy: i32, height: i32) -> (Vec<u8>, Vec<bool>) {
        let mut block = vec![0u8; 16];
        let mut defined = vec![false; 16];
        draw_column_in_cache(patch, column, &mut block, &mut defined, 4, originy, height);
        (block, defined)
    }

    #[test]
    fn posts_are_copied_at_originy_plus_topdelta_and_clipped_to_the_cache() {
        // `position = originy + patch->topdelta; if (position < 0) { count += position;
        // position = 0; } if (position + count > cacheheight) count = cacheheight - position;
        // if (count > 0) memcpy (cache + position, source, count);` The copy starts at the
        // post's first byte even when the front was clipped.
        let patch = posts(&[(2, &[10, 11, 12])]);
        let rows = |(b, d): (Vec<u8>, Vec<bool>)| -> Vec<(u8, bool)> {
            b.into_iter().zip(d).skip(4).take(8).collect()
        };
        let cell = |v: u8| (v, v != 0);
        // Plain: rows 2-4.
        assert_eq!(
            rows(cache(&patch, 0, 0, 8)),
            [0, 0, 10, 11, 12, 0, 0, 0].map(cell)
        );
        // Shifted down one row by originy.
        assert_eq!(
            rows(cache(&patch, 0, 1, 8)),
            [0, 0, 0, 10, 11, 12, 0, 0].map(cell)
        );
        // Shifted above the top: position -1 keeps count 3 - 1 = 2 bytes, copied from source[0];
        // position -2 keeps one.
        assert_eq!(
            rows(cache(&patch, 0, -3, 8)),
            [10, 11, 0, 0, 0, 0, 0, 0].map(cell)
        );
        assert_eq!(
            rows(cache(&patch, 0, -4, 8)),
            [10, 0, 0, 0, 0, 0, 0, 0].map(cell)
        );
        // Clipped at the bottom: rows 6-7 of an 8-row cache only (the block holds more rows,
        // which stay untouched).
        let low = posts(&[(6, &[1, 2, 3, 4])]);
        assert_eq!(
            rows(cache(&low, 0, 0, 8)),
            [0, 0, 0, 0, 0, 0, 1, 2].map(cell)
        );
        // A post wholly past the bottom or above the top copies nothing.
        assert_eq!(rows(cache(&low, 0, 8, 8)), [0; 8].map(cell));
        assert_eq!(rows(cache(&low, 0, -20, 8)), [0; 8].map(cell));
    }

    #[test]
    fn the_next_post_follows_the_length_plus_four() {
        // `patch = (column_t *)((byte *)patch + patch->length + 4)`: header, data, one pad byte.
        let patch = posts(&[(0, &[1, 2]), (4, &[3, 4, 5]), (9, &[6])]);
        let (b, d) = cache(&patch, 0, 0, 12);
        assert_eq!(&b[4..16], &[1, 2, 0, 0, 3, 4, 5, 0, 0, 6, 0, 0]);
        assert_eq!(
            &d[4..16],
            &[
                true, true, false, false, true, true, true, false, false, true, false, false
            ]
        );
        // Starting from the middle of the lump: a column offset past the first post.
        let (b, _) = cache(&patch, 6, 0, 12);
        assert_eq!(&b[4..16], &[0, 0, 0, 0, 3, 4, 5, 0, 0, 6, 0, 0]);
    }

    #[test]
    fn a_column_running_off_the_lump_leaves_the_whole_column_undefined() {
        // The genuine walk reads zone memory past the lump; the renderer marks every row of
        // the column undefined (`poison`), including rows an earlier post of it wrote.
        let mut patch = posts(&[(0, &[1, 2])]);
        patch.pop(); // no terminator
        let mut block = vec![7u8; 16];
        let mut defined = vec![true; 16];
        draw_column_in_cache(&patch, 0, &mut block, &mut defined, 4, 0, 8);
        assert_eq!(
            &defined[..4],
            &[true; 4],
            "rows before the column are not its own"
        );
        assert_eq!(&defined[4..12], &[false; 8]);
        assert_eq!(&defined[12..], &[true; 4]);
        // An offset outside the lump (or negative) is the same.
        for column in [100, -1] {
            let mut defined = vec![true; 16];
            draw_column_in_cache(&patch, column, &mut block, &mut defined, 4, 0, 8);
            assert_eq!(&defined[4..12], &[false; 8], "column {column}");
        }
    }
}
