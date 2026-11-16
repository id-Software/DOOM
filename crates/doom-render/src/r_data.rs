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
    /// A PNAMES or TEXTURE count or directory entry is negative or exceeds the lump.
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
    /// A texture's width, height or patch count is not positive.
    BadTextureSize(String),
    /// A patch header or column directory entry lies outside its lump.
    PatchBounds {
        /// The patch's lump number.
        lump: usize,
    },
    /// `R_GenerateLookup: texture %i is >64k` (r_data.c:367).
    CompositeTooLarge(usize),
    /// The F_START/F_END or S_START/S_END markers do not enclose a positive range.
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
    /// A map refers to something the renderer cannot resolve (`ML_TWOSIDED` without a back
    /// side, `p_setup.c:189-192`, would read `sides[-1]`).
    Map(String),
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
}

impl ColumnSource<'_> {
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
    /// `skyflatnum` (`R_FlatNumForName(SKYFLATNAME)`, g_game.c:454).
    pub skyflatnum: usize,
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
            skyflatnum: 0,
        };
        res.switchlist = crate::p_spec::init_switch_list(&res, mode)?;
        res.anims = crate::p_spec::init_pic_anims(&res)?;
        res.sprites = crate::r_things::init_sprite_defs(&res)?;
        res.skytexture = res.texture_num_for_name(b"SKY1")?;
        res.skyflatnum = res.flat_num_for_name(crate::r_sky::SKYFLATNAME)?;
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
        match t.columns[col] {
            ColumnEntry::Lump { lump, ofs } => ColumnSource {
                block: self.dir.bytes(lump),
                defined: None,
                start: usize::from(ofs),
                undefined_column: false,
            },
            ColumnEntry::Composite { ofs } | ColumnEntry::LumpZero { ofs } => ColumnSource {
                block: &t.composite,
                defined: Some(&t.defined),
                start: usize::from(ofs),
                undefined_column: false,
            },
            ColumnEntry::Undefined => ColumnSource {
                block: &[],
                defined: None,
                start: 0,
                undefined_column: true,
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
}

impl Resources {
    /// The posts a masked midtexture column walks (`R_GetColumn` then `- 3`, r_segs.c:181-182).
    pub fn masked_posts(&self, tex: usize, col: i32) -> MaskedPosts<'_> {
        let t = &self.textures[tex];
        let col = (col & t.widthmask) as usize;
        match t.columns[col] {
            ColumnEntry::Lump { lump, ofs } => match usize::from(ofs).checked_sub(3) {
                Some(at) => MaskedPosts::Lump {
                    bytes: self.dir.bytes(lump),
                    at,
                },
                None => MaskedPosts::Undefined,
            },
            ColumnEntry::Composite { .. } | ColumnEntry::LumpZero { .. } => MaskedPosts::Composite,
            ColumnEntry::Undefined => MaskedPosts::Undefined,
        }
    }
    /// A column source starting at byte `start` of lump `bytes` (a masked post's pixels).
    pub fn lump_source(bytes: &[u8], start: usize) -> ColumnSource<'_> {
        ColumnSource {
            block: bytes,
            defined: None,
            start,
            undefined_column: false,
        }
    }
}

fn marker_range(
    dir: &LumpDirectory,
    start: &[u8],
    end: &[u8],
    what: &'static str,
) -> Result<(usize, usize), ResourceError> {
    let first = dir.get_num_for_name(start)? + 1;
    let last = dir.get_num_for_name(end)?;
    if last <= first {
        return Err(ResourceError::BadMarkerRange(what));
    }
    Ok((first, last - first))
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
    let mut lists: Vec<(&[u8], &'static str)> =
        vec![(dir.bytes(dir.get_num_for_name(b"TEXTURE1")?), "TEXTURE1")];
    if let Some(t2) = dir.check_num_for_name(b"TEXTURE2") {
        lists.push((dir.bytes(t2), "TEXTURE2"));
    }
    let mut textures = Vec::new();
    for (lump, what) in lists {
        let n = read_i32(lump, 0)
            .filter(|n| *n >= 0)
            .ok_or_else(|| ResourceError::BadTextureDirectory(what.into()))?;
        for i in 0..n as usize {
            let offset = read_i32(lump, 4 + 4 * i)
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
            if width <= 0 || height <= 0 || patchcount <= 0 {
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
    let mut patchcount = vec![0u32; width];
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
            patchcount[x as usize] += 1;
            // `colofs[x] = LONG(columnofs[x-x1]) + 3`, stored as unsigned short.
            cols[x as usize] = Some((p.patch, ofs.wrapping_add(3) as u16));
        }
    }
    let mut entries = Vec::with_capacity(width);
    let mut size = 0usize;
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
            stopped = true;
            entries.push(ColumnEntry::Undefined);
            continue;
        }
        if patchcount[x] > 1 {
            if size > 0x10000 - t.height as usize {
                return Err(ResourceError::CompositeTooLarge(num));
            }
            entries.push(ColumnEntry::Composite { ofs: size as u16 });
            size += t.height as usize;
        } else {
            let (lump, ofs) = cols[x].expect("covered column");
            entries.push(lump_entry(lump, ofs));
        }
    }
    t.columns = entries;
    t.compositesize = size;
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
}
