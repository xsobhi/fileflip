//! Estimating a JPEG's quality setting from its quantization tables.

use std::fs::File;
use std::io::Read;
use std::path::Path;
/// Estimate libjpeg's quality setting from the luminance quantization table.
pub fn jpeg_quality(path: &Path) -> Option<u8> {
    const STD_LUMA: [u32; 64] = [
        16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56, 14, 17, 22, 29, 51, 87, 80, 62, 18,
        22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113, 92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100,
        103, 99,
    ];
    let mut data = Vec::new();
    File::open(path).ok()?.take(512 * 1024).read_to_end(&mut data).ok()?;
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }
    let mut i = 2;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = data[i + 1];
        if marker == 0xD8 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) || marker == 0xFF {
            i += if marker == 0xFF { 1 } else { 2 };
            continue;
        }
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        let seg = data.get(i + 4..i + 2 + len)?;
        if marker == 0xDA {
            return None; // start of scan, no DQT seen
        }
        if marker == 0xDB {
            let mut j = 0;
            while j < seg.len() {
                let pq = seg[j] >> 4;
                let tq = seg[j] & 0x0F;
                let size = if pq == 0 { 64 } else { 128 };
                let table = seg.get(j + 1..j + 1 + size)?;
                if tq == 0 {
                    let sum: u32 = if pq == 0 {
                        table.iter().map(|&v| v as u32).sum()
                    } else {
                        table.chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]]) as u32).sum()
                    };
                    let scale = sum as f64 * 100.0 / STD_LUMA.iter().sum::<u32>() as f64;
                    let q = if scale <= 100.0 { (200.0 - scale) / 2.0 } else { 5000.0 / scale };
                    return Some(q.round().clamp(1.0, 100.0) as u8);
                }
                j += 1 + size;
            }
        }
        i += 2 + len;
    }
    None
}
