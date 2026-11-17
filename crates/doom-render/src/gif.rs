//! A small hand-written GIF89a writer for palette-indexed frames (local viewer output only;
//! decision R16, no new dependency). One global 256-colour table, a looping animation, and
//! variable-width LZW (minimum code size 8, codes up to 12 bits, a clear code whenever the
//! table fills). No Rust or simulation state depends on it.
//!
//! GIF frame delays are whole hundredths of a second, so 35 tics/second (2.857 cs a tic) cannot
//! be written exactly per frame. [`delay_cs`] spreads the rounding: tic `n` lasts
//! `floor((n+1)*100/fps) - floor(n*100/fps)` cs, which is 2, 3, 3, 3, 3, 3, 3 repeating at
//! 35 fps (20 cs per 7 tics), so every whole second of tics lasts exactly one second. Viewers
//! that clamp very short delays (some treat 1 cs as 10 cs) are not affected, as the minimum is 2.

use std::collections::HashMap;

/// The delay in hundredths of a second of frame `n` (counting from 0) at `fps` frames a second
/// (`fps` > 0): the running total of delays equals `floor(n * 100 / fps)` after every frame.
pub fn delay_cs(n: u64, fps: u32) -> u16 {
    let fps = u64::from(fps.max(1));
    let a = (n + 1) * 100 / fps;
    let b = n * 100 / fps;
    (a - b).clamp(1, u64::from(u16::MAX)) as u16
}

/// `pixels` (`width * height` indices) enlarged by the integer `factor` (nearest neighbour).
pub fn scale_nearest(width: usize, height: usize, pixels: &[u8], factor: usize) -> Vec<u8> {
    assert_eq!(pixels.len(), width * height, "pixel count");
    assert!(factor >= 1, "scale factor");
    let mut out = Vec::with_capacity(pixels.len() * factor * factor);
    for row in pixels.chunks(width) {
        let start = out.len();
        for &p in row {
            out.extend(std::iter::repeat_n(p, factor));
        }
        let line = out[start..].to_vec();
        for _ in 1..factor {
            out.extend_from_slice(&line);
        }
    }
    out
}

/// Packs variable-width codes into GIF data sub-blocks.
struct BitWriter {
    bytes: Vec<u8>,
    acc: u32,
    bits: u32,
}

impl BitWriter {
    fn put(&mut self, code: u16, width: u32) {
        self.acc |= u32::from(code) << self.bits;
        self.bits += width;
        while self.bits >= 8 {
            self.bytes.push(self.acc as u8);
            self.acc >>= 8;
            self.bits -= 8;
        }
    }
    fn finish(mut self) -> Vec<u8> {
        if self.bits > 0 {
            self.bytes.push(self.acc as u8);
        }
        self.bytes
    }
}

/// LZW-compresses `pixels` (indices below 256) with minimum code size 8.
fn lzw(pixels: &[u8]) -> Vec<u8> {
    const CLEAR: u16 = 256;
    const END: u16 = 257;
    let mut out = BitWriter {
        bytes: Vec::new(),
        acc: 0,
        bits: 0,
    };
    let mut table: HashMap<(u16, u8), u16> = HashMap::new();
    let (mut next, mut width) = (258u16, 9u32);
    out.put(CLEAR, width);
    let mut it = pixels.iter().copied();
    let Some(first) = it.next() else {
        out.put(END, width);
        return out.finish();
    };
    let mut prefix = u16::from(first);
    for k in it {
        if let Some(&code) = table.get(&(prefix, k)) {
            prefix = code;
            continue;
        }
        out.put(prefix, width);
        if next < 4096 {
            table.insert((prefix, k), next);
            // The decoder is one entry behind: the width grows once code 2^width exists.
            if u32::from(next) == 1 << width && width < 12 {
                width += 1;
            }
            next += 1;
        } else {
            out.put(CLEAR, width);
            table.clear();
            next = 258;
            width = 9;
        }
        prefix = u16::from(k);
    }
    out.put(prefix, width);
    // The decoder adds an entry for that last code too, which may widen the end code.
    if next < 4096 && u32::from(next) + 1 > 1 << width && width < 12 {
        width += 1;
    }
    out.put(END, width);
    out.finish()
}

/// An animated GIF being written into memory.
pub struct GifWriter {
    out: Vec<u8>,
    width: u16,
    height: u16,
}

impl GifWriter {
    /// The header, the logical screen with a 256-entry global colour table from `palette`
    /// (RGB triples; a shorter palette is padded with black) and the looping extension.
    pub fn new(width: u16, height: u16, palette: &[u8]) -> Self {
        let mut out = b"GIF89a".to_vec();
        out.extend(width.to_le_bytes());
        out.extend(height.to_le_bytes());
        out.extend([0xf7, 0, 0]); // global table, 8 bits of colour, 256 entries
        let mut table = palette.iter().copied().take(768).collect::<Vec<_>>();
        table.resize(768, 0);
        out.extend(table);
        out.extend(b"\x21\xff\x0bNETSCAPE2.0\x03\x01\x00\x00\x00"); // loop forever
        Self { out, width, height }
    }

    /// Appends one full-size frame of `width * height` indices shown for `delay` hundredths
    /// of a second.
    pub fn frame(&mut self, pixels: &[u8], delay: u16) {
        assert_eq!(
            pixels.len(),
            usize::from(self.width) * usize::from(self.height),
            "pixel count"
        );
        self.out.extend([0x21, 0xf9, 4, 0x04]); // graphic control: leave in place
        self.out.extend(delay.to_le_bytes());
        self.out.extend([0, 0]);
        self.out.push(0x2c);
        self.out.extend([0, 0, 0, 0]);
        self.out.extend(self.width.to_le_bytes());
        self.out.extend(self.height.to_le_bytes());
        self.out.extend([0, 8]); // no local table, minimum code size 8
        for block in lzw(pixels).chunks(255) {
            self.out.push(block.len() as u8);
            self.out.extend_from_slice(block);
        }
        self.out.push(0);
    }

    /// The trailer; returns the file's bytes.
    pub fn finish(mut self) -> Vec<u8> {
        self.out.push(0x3b);
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the data sub-blocks at `*at` into one buffer.
    fn blocks(b: &[u8], at: &mut usize) -> Vec<u8> {
        let mut data = Vec::new();
        loop {
            let n = usize::from(b[*at]);
            *at += 1;
            if n == 0 {
                return data;
            }
            data.extend_from_slice(&b[*at..*at + n]);
            *at += n;
        }
    }

    /// An independent LZW decoder (minimum code size 8) for the tests.
    fn unlzw(data: &[u8], expect: usize) -> Vec<u8> {
        unlzw_codes(data, expect).0
    }

    /// The decoder of [`unlzw`], also returning every code read with the decoder's code width
    /// at that point. A code may be at most the next table index (the decoder knows every
    /// earlier entry; the next one only by the `KwKwK` rule), the GIF89a LZW rule.
    fn unlzw_codes(data: &[u8], expect: usize) -> (Vec<u8>, Vec<(usize, usize)>) {
        let mut codes = Vec::new();
        let mut table: Vec<Vec<u8>> = Vec::new();
        let reset = |t: &mut Vec<Vec<u8>>| {
            t.clear();
            t.extend((0..=255u8).map(|b| vec![b]));
            t.push(Vec::new()); // clear
            t.push(Vec::new()); // end
        };
        reset(&mut table);
        let (mut width, mut pos) = (9usize, 0usize);
        let mut out = Vec::new();
        let mut prev: Option<Vec<u8>> = None;
        loop {
            let mut code = 0usize;
            for i in 0..width {
                let bit = pos + i;
                code |= usize::from((data[bit / 8] >> (bit % 8)) & 1) << i;
            }
            pos += width;
            codes.push((code, width));
            assert!(
                code <= table.len(),
                "code {code} beyond table {}",
                table.len()
            );
            match code {
                256 => {
                    reset(&mut table);
                    width = 9;
                    prev = None;
                    continue;
                }
                257 => break,
                _ => {}
            }
            let entry = if code < table.len() {
                table[code].clone()
            } else {
                let mut e = prev.clone().expect("first code");
                e.push(e[0]);
                e
            };
            if let Some(p) = &prev {
                let mut n = p.clone();
                n.push(entry[0]);
                if table.len() < 4096 {
                    table.push(n);
                }
            }
            out.extend_from_slice(&entry);
            prev = Some(entry);
            if table.len() == 1 << width && width < 12 {
                width += 1;
            }
        }
        assert_eq!(out.len(), expect);
        (out, codes)
    }

    fn decode_frame(file: &[u8], at: &mut usize, expect: usize) -> (u16, Vec<u8>) {
        assert_eq!(&file[*at..*at + 4], &[0x21, 0xf9, 4, 0x04]);
        let delay = u16::from_le_bytes([file[*at + 4], file[*at + 5]]);
        *at += 8;
        assert_eq!(file[*at], 0x2c);
        *at += 9;
        assert_eq!(&file[*at..*at + 2], &[0, 8]);
        *at += 2;
        let data = blocks(file, at);
        (delay, unlzw(&data, expect))
    }

    /// Deterministic pixel patterns: flat, ramp, noise (fills the table many times) and runs.
    fn patterns() -> Vec<Vec<u8>> {
        let mut x = 0x1234_5678u32;
        let mut noise = || {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x >> 8) as u8
        };
        vec![
            vec![7; 64000],
            (0..64000u32).map(|i| (i % 251) as u8).collect(),
            (0..64000).map(|_| noise()).collect(),
            (0..64000u32)
                .map(|i| if (i / 37) % 2 == 0 { 3 } else { (i % 5) as u8 })
                .collect(),
            vec![1],
            vec![1, 2],
            (0..300u32).map(|i| (i % 3) as u8).collect(),
        ]
    }

    #[test]
    fn lzw_round_trips() {
        for p in patterns() {
            let packed = lzw(&p);
            assert_eq!(unlzw(&packed, p.len()), p, "length {}", p.len());
        }
    }

    /// The GIF89a variable-width rules, pinned on the stream itself.
    fn noise(n: usize) -> Vec<u8> {
        let mut x = 0x9e37_79b9u32;
        (0..n)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                (x >> 8) as u8
            })
            .collect()
    }

    #[test]
    fn end_code_width_round_trips_at_every_table_size_boundary() {
        // Pins `lzw`'s end-code width (the decoder, one entry behind, widens once code 2^width
        // exists; GIF89a LZW): the final prefix and END codes are written at the width the
        // decoder has then reached. Noise over 256 values almost never repeats a pair, so
        // every pixel adds one entry and the lengths below straddle the 9->10, 10->11, 11->12
        // bit changes and the table-full clear (an independent decoder checks every length).
        let data = noise(9000);
        for w in 9..=12u32 {
            let around = (1usize << w) - 258;
            for n in around.saturating_sub(4)..=around + 4 {
                let p = &data[..n];
                assert_eq!(unlzw(&lzw(p), n), p, "length {n}");
            }
        }
        for n in 3830..=3850 {
            assert_eq!(unlzw(&lzw(&data[..n]), n), &data[..n], "length {n}");
        }
        for n in 7670..=7690 {
            assert_eq!(unlzw(&lzw(&data[..n]), n), &data[..n], "length {n}");
        }
    }

    #[test]
    fn compression_and_clear_period() {
        // A run of one value must actually compress (an encoder that clears at once also
        // round-trips).
        assert!(lzw(&[7; 64000]).len() < 1000);
        // Every pixel of noise emits one code and adds one entry; the table holds 4096 entries
        // (codes 0-4095), so after the clear the stream has exactly 4096 - 258 = 3838 entries
        // added plus the emission that finds the table full: 3839 data codes between clears.
        let p = noise(20000);
        let (_, codes) = unlzw_codes(&lzw(&p), p.len());
        assert_eq!(codes[0], (256, 9));
        let clears: Vec<usize> = (0..codes.len()).filter(|&i| codes[i].0 == 256).collect();
        assert!(clears.len() >= 4);
        for pair in clears.windows(2) {
            assert_eq!(pair[1] - pair[0] - 1, 3839);
        }
        // No code is ever written wider than 12 bits.
        assert!(codes.iter().all(|&(c, w)| w <= 12 && c < 4096 || c == 257));
        assert_eq!(codes.iter().map(|c| c.1).max(), Some(12));
    }

    #[test]
    fn data_ending_on_a_byte_boundary_has_no_padding_byte() {
        // CLEAR, five emissions of all-distinct pixels, the last prefix and END are eight
        // 9-bit codes: exactly 72 bits = 9 bytes, with no trailing zero byte.
        assert_eq!(lzw(&[1, 2, 3, 4, 5, 6]).len(), 9);
        // Seven codes (63 bits) need 8 bytes.
        assert_eq!(lzw(&[1, 2, 3, 4, 5]).len(), 8);
    }

    #[test]
    fn file_structure_and_frames_round_trip() {
        let palette: Vec<u8> = (0..768u32).map(|i| (i % 256) as u8).collect();
        let (w, h) = (16u16, 8u16);
        let frames: Vec<Vec<u8>> = (0..3u8)
            .map(|f| (0..128u32).map(|i| (i as u8).wrapping_mul(f + 1)).collect())
            .collect();
        let mut g = GifWriter::new(w, h, &palette);
        for (i, f) in frames.iter().enumerate() {
            g.frame(f, delay_cs(i as u64, 35));
        }
        let file = g.finish();
        assert_eq!(&file[..6], b"GIF89a");
        assert_eq!(u16::from_le_bytes([file[6], file[7]]), w);
        assert_eq!(u16::from_le_bytes([file[8], file[9]]), h);
        assert_eq!(file[10], 0xf7);
        assert_eq!(&file[13..13 + 768], palette.as_slice());
        assert_eq!(*file.last().unwrap(), 0x3b);
        let mut at = 13 + 768;
        assert_eq!(&file[at..at + 3], &[0x21, 0xff, 0x0b]);
        at += 19;
        for (i, f) in frames.iter().enumerate() {
            let (delay, pixels) = decode_frame(&file, &mut at, f.len());
            assert_eq!(delay, delay_cs(i as u64, 35));
            assert_eq!(&pixels, f);
        }
        assert_eq!(at, file.len() - 1);
    }

    #[test]
    fn large_frame_round_trips() {
        let mut g = GifWriter::new(320, 200, &[]);
        for p in patterns().into_iter().take(4) {
            g.frame(&p, 3);
        }
        let file = g.finish();
        let mut at = 13 + 768 + 19;
        for p in patterns().into_iter().take(4) {
            let (_, pixels) = decode_frame(&file, &mut at, p.len());
            assert_eq!(pixels, p);
        }
    }

    #[test]
    fn delays_average_the_frame_rate() {
        let d: Vec<u16> = (0..7).map(|n| delay_cs(n, 35)).collect();
        assert_eq!(d, [2, 3, 3, 3, 3, 3, 3]);
        for fps in [1u32, 10, 24, 35, 60] {
            let total: u64 = (0..u64::from(fps) * 5)
                .map(|n| u64::from(delay_cs(n, fps)))
                .sum();
            assert_eq!(total, 500, "fps {fps}");
        }
    }

    #[test]
    fn scaling_repeats_pixels() {
        assert_eq!(
            scale_nearest(2, 2, &[1, 2, 3, 4], 2),
            [1, 1, 2, 2, 1, 1, 2, 2, 3, 3, 4, 4, 3, 3, 4, 4]
        );
        assert_eq!(scale_nearest(2, 1, &[5, 6], 1), [5, 6]);
    }
}
