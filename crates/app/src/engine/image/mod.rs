//! In-process image conversion (no FFmpeg needed): decode with `image`,
//! encode JPEG with `jpeg-encoder` (SIMD), WebP with libwebp, the rest with `image`.

mod encode;
mod jpeg_quality;

pub use jpeg_quality::jpeg_quality;

use fileflip_core::settings::PngCompression;
use fileflip_core::Format;
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageReader};
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
/// What we know about the source's quality.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Source {
    Lossless,
    /// JPEG with its estimated quality (from the quantization tables).
    Jpeg(Option<u8>),
    Lossy,
}

pub fn native_input(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "jpe" | "jfif" | "webp" | "bmp" | "dib" | "gif" | "tif" | "tiff" | "ico" | "tga" | "qoi" | "pnm" | "pbm" | "pgm" | "ppm"
    )
}

pub fn native_output(f: &Format) -> bool {
    f.id != "avif"
}

pub fn source_kind(path: &Path, ext: &str) -> Source {
    match ext {
        "jpg" | "jpeg" | "jpe" | "jfif" => Source::Jpeg(jpeg_quality(path)),
        "webp" => {
            let mut head = vec![0u8; 64 * 1024];
            let n = File::open(path).and_then(|mut f| f.read(&mut head)).unwrap_or(0);
            if head[..n].windows(4).any(|w| w == b"VP8 ") {
                Source::Lossy
            } else {
                Source::Lossless
            }
        }
        "avif" | "heic" | "heif" | "jxl" => Source::Lossy,
        _ => Source::Lossless,
    }
}

/// Quality to use for a lossy target when the user asked for "same quality".
/// `None` means "encode losslessly" (WebP from a lossless source).
pub fn default_quality(src: Source, target: &Format) -> Option<u8> {
    match (src, target.id) {
        (Source::Jpeg(q), _) => Some(q.unwrap_or(92).max(50)),
        (Source::Lossy, _) => Some(90),
        (Source::Lossless, "webp") => None,
        (Source::Lossless, "avif") => Some(92),
        (Source::Lossless, _) => Some(95),
    }
}

pub struct ImageJob<'a> {
    pub input: &'a Path,
    pub output: &'a Path,
    pub target: &'a Format,
    pub quality: Option<u8>,
    pub scale: Option<u8>,
    pub png: PngCompression,
}

pub fn decode(path: &Path) -> Result<(DynamicImage, Option<Vec<u8>>), String> {
    let mut reader = ImageReader::open(path).map_err(|e| e.to_string())?.with_guessed_format().map_err(|e| e.to_string())?;
    reader.no_limits();
    let mut dec = reader.into_decoder().map_err(|e| e.to_string())?;
    let orientation = dec.orientation().ok();
    let icc = dec.icc_profile().ok().flatten();
    let mut img = DynamicImage::from_decoder(dec).map_err(|e| e.to_string())?;
    if let Some(o) = orientation {
        img.apply_orientation(o);
    }
    Ok((img, icc))
}

pub fn convert(job: &ImageJob, on_progress: &dyn Fn(f64)) -> Result<(), String> {
    let (mut img, icc) = decode(job.input)?;
    on_progress(0.4);
    if let Some(s) = job.scale.filter(|s| *s < 100) {
        let w = ((img.width() as u64 * s as u64) / 100).max(1) as u32;
        let h = ((img.height() as u64 * s as u64) / 100).max(1) as u32;
        img = img.resize_exact(w, h, FilterType::CatmullRom);
    }
    on_progress(0.6);
    if job.target.id == "jpg" {
        return encode::jpeg(&img, &icc, job);
    }

    let mut out = BufWriter::new(File::create(job.output).map_err(encode::io)?);
    match job.target.id {
        "webp" => encode::webp(&img, job.quality, &mut out)?,
        "png" => encode::png(img, icc, job.png, &mut out)?,
        "bmp" => encode::bmp(&img, &mut out)?,
        "tiff" => encode::tiff(&img, &mut out)?,
        "gif" => encode::gif(&img, &mut out)?,
        "ico" => encode::ico(&img, &mut out)?,
        other => return Err(format!("{other} is not supported for images")),
    }
    out.flush().map_err(encode::io)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lossless_sources_stay_lossless_in_webp() {
        let webp = fileflip_core::formats::format_by_id("webp").unwrap();
        assert_eq!(default_quality(Source::Lossless, webp), None);
        assert_eq!(default_quality(Source::Jpeg(Some(83)), webp), Some(83));
    }
}
