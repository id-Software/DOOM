//! A small hand-written PNG writer for palette-indexed frames (viewer and diff output only;
//! decision R14: no new dependency). Colour type 3 (indexed), bit depth 8, PLTE from a
//! 768-byte palette, IDAT as a zlib stream of stored (uncompressed) deflate blocks with
//! Adler-32, and a CRC-32 per chunk.

const fn crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        t[n] = c;
        n += 1;
    }
    t
}
static CRC_TABLE: [u32; 256] = crc_table();

/// CRC-32 (ISO 3309, as PNG uses it).
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for b in bytes {
        c = CRC_TABLE[((c ^ u32::from(*b)) & 0xff) as usize] ^ (c >> 8);
    }
    c ^ 0xffff_ffff
}

/// Adler-32 (RFC 1950).
pub fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for x in bytes {
        a = (a + u32::from(*x)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend(crc.to_be_bytes());
}

/// Encodes `pixels` (`width * height` palette indices, row major) with `palette` (256 RGB
/// triples; a shorter palette is padded with black).
pub fn encode_indexed(width: u32, height: u32, pixels: &[u8], palette: &[u8]) -> Vec<u8> {
    assert_eq!(
        pixels.len(),
        width as usize * height as usize,
        "pixel count"
    );
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend(width.to_be_bytes());
    ihdr.extend(height.to_be_bytes());
    ihdr.extend([8, 3, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    let mut plte = palette.iter().copied().take(768).collect::<Vec<_>>();
    plte.resize(768, 0);
    chunk(&mut out, b"PLTE", &plte);
    let mut raw = Vec::with_capacity(pixels.len() + height as usize);
    for row in pixels.chunks(width as usize) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, block) in blocks.iter().enumerate() {
        z.push(u8::from(i + 1 == blocks.len()));
        let len = block.len() as u16;
        z.extend(len.to_le_bytes());
        z.extend((!len).to_le_bytes());
        z.extend_from_slice(block);
    }
    z.extend(adler32(&raw).to_be_bytes());
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksums_match_known_values() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
        let png = encode_indexed(2, 1, &[0, 1], &[0, 0, 0, 255, 255, 255]);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }

    /// Splits a PNG into (kind, data) chunks, checking the signature and every chunk's CRC
    /// (PNG specification, section 5.3 and annex D).
    fn chunks(png: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        let mut at = 8;
        let mut out = Vec::new();
        while at < png.len() {
            let n = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
            let kind: [u8; 4] = png[at + 4..at + 8].try_into().unwrap();
            let data = png[at + 8..at + 8 + n].to_vec();
            let crc = u32::from_be_bytes(png[at + 8 + n..at + 12 + n].try_into().unwrap());
            assert_eq!(crc, crc32(&png[at + 4..at + 8 + n]), "crc of {kind:?}");
            out.push((kind, data));
            at += 12 + n;
        }
        out
    }

    /// Inflates a zlib stream made only of stored blocks (RFC 1950/1951), checking the header,
    /// each block's `LEN`/`NLEN`, that exactly the last block is flagged final, and the Adler-32.
    fn inflate_stored(z: &[u8]) -> Vec<u8> {
        assert_eq!(&z[..2], &[0x78, 0x01]);
        assert_eq!((u16::from(z[0]) << 8 | u16::from(z[1])) % 31, 0, "FCHECK");
        let (mut at, mut raw) = (2, Vec::new());
        loop {
            let header = z[at];
            assert_eq!(header & 0xfe, 0, "stored block type");
            let len = u16::from_le_bytes([z[at + 1], z[at + 2]]);
            let nlen = u16::from_le_bytes([z[at + 3], z[at + 4]]);
            assert_eq!(nlen, !len, "NLEN");
            raw.extend_from_slice(&z[at + 5..at + 5 + usize::from(len)]);
            at += 5 + usize::from(len);
            if header == 1 {
                break;
            }
        }
        assert_eq!(
            z[at..],
            adler32(&raw).to_be_bytes(),
            "Adler-32 and no trailing bytes"
        );
        raw
    }

    #[test]
    fn indexed_png_round_trips_through_an_independent_decoder() {
        // 70,000 x 1 pixels: 70,001 filtered bytes, so two stored blocks (65,535 + 4,466): the
        // first not final, the last final. Palette entries beyond the given ones are black.
        let (w, h) = (70_000u32, 1u32);
        let pixels: Vec<u8> = (0..w).map(|i| (i % 253) as u8).collect();
        let palette = [10u8, 20, 30, 40, 50, 60];
        let png = encode_indexed(w, h, &pixels, &palette);
        let c = chunks(&png);
        let kinds: Vec<&[u8]> = c.iter().map(|(k, _)| &k[..]).collect();
        assert_eq!(kinds, [&b"IHDR"[..], b"PLTE", b"IDAT", b"IEND"]);
        let mut ihdr = Vec::new();
        ihdr.extend(w.to_be_bytes());
        ihdr.extend(h.to_be_bytes());
        ihdr.extend([8, 3, 0, 0, 0]);
        assert_eq!(c[0].1, ihdr);
        let mut plte = palette.to_vec();
        plte.resize(768, 0);
        assert_eq!(c[1].1, plte);
        assert_eq!(c[3].1, Vec::<u8>::new());
        let raw = inflate_stored(&c[2].1);
        assert_eq!(raw[0], 0, "filter type None");
        assert_eq!(&raw[1..], pixels.as_slice());
        // Several rows, one block: each row is preceded by its filter byte.
        let png = encode_indexed(3, 2, &[1, 2, 3, 4, 5, 6], &[]);
        let c = chunks(&png);
        assert_eq!(inflate_stored(&c[2].1), [0, 1, 2, 3, 0, 4, 5, 6]);
    }
}
