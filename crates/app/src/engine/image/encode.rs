//! One encoder per output format, plus the pixel helpers they need.

use super::ImageJob;
use fileflip_core::settings::PngCompression;
use image::codecs::gif::GifEncoder;
use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder};
use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageFormat, RgbaImage};
use std::fs::File;
use std::io::{BufWriter, Write};

type Out = BufWriter<File>;

fn err(e: image::ImageError) -> String {
    e.to_string()
}

pub(super) fn io(e: std::io::Error) -> String {
    e.to_string()
}

/// JPEG with `jpeg-encoder` (SIMD); writes `job.output` itself.
pub(super) fn jpeg(img: &DynamicImage, icc: &Option<Vec<u8>>, job: &ImageJob) -> Result<(), String> {
    if img.width() > u16::MAX as u32 || img.height() > u16::MAX as u32 {
        return Err("image is too large for JPEG (max 65535 px)".into());
    }
    let rgb = flatten(img);
    let q = job.quality.unwrap_or(95);
    let mut enc = jpeg_encoder::Encoder::new_file(job.output, q).map_err(|e| e.to_string())?;
    if q >= 90 {
        enc.set_sampling_factor(jpeg_encoder::SamplingFactor::R_4_4_4);
    }
    if let Some(icc) = icc {
        let _ = enc.add_icc_profile(icc);
    }
    enc.encode(rgb.as_raw(), rgb.width() as u16, rgb.height() as u16, jpeg_encoder::ColorType::Rgb)
        .map_err(|e| e.to_string())
}

pub(super) fn webp(img: &DynamicImage, quality: Option<u8>, out: &mut Out) -> Result<(), String> {
    // Explicit settings instead of libwebp's defaults: much less CPU,
    // especially for lossless, for slightly larger files.
    let mut config = webp::WebPConfig::new().map_err(|_| "could not set up the WebP encoder".to_string())?;
    match quality {
        None => {
            config.lossless = 1;
            config.quality = 25.0; // compression effort; output stays lossless
            config.method = 1;
            config.exact = 1;
        }
        Some(q) => {
            config.quality = q as f32;
            config.method = 3;
        }
    }
    let mem = if img.color().has_alpha() {
        let rgba = img.to_rgba8();
        webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height()).encode_advanced(&config)
    } else {
        let rgb = img.to_rgb8();
        webp::Encoder::from_rgb(rgb.as_raw(), rgb.width(), rgb.height()).encode_advanced(&config)
    }
    .map_err(|e| format!("WebP encoding failed: {e:?}"))?;
    out.write_all(&mem).map_err(io)
}

pub(super) fn png(img: DynamicImage, icc: Option<Vec<u8>>, png: PngCompression, out: &mut Out) -> Result<(), String> {
    let compression = match png {
        PngCompression::Fast => CompressionType::Fast,
        PngCompression::Balanced => CompressionType::Default,
        PngCompression::Best => CompressionType::Best,
    };
    let mut enc = PngEncoder::new_with_quality(&mut *out, compression, PngFilter::Adaptive);
    if let Some(icc) = icc {
        let _ = enc.set_icc_profile(icc);
    }
    let img = match img {
        DynamicImage::ImageRgb32F(_) | DynamicImage::ImageRgba32F(_) => DynamicImage::ImageRgba16(img.to_rgba16()),
        other => other,
    };
    img.write_with_encoder(enc).map_err(err)
}

pub(super) fn bmp(img: &DynamicImage, out: &mut Out) -> Result<(), String> {
    let img = if img.color().has_alpha() { DynamicImage::ImageRgba8(img.to_rgba8()) } else { DynamicImage::ImageRgb8(img.to_rgb8()) };
    img.write_to(out, ImageFormat::Bmp).map_err(err)
}

pub(super) fn tiff(img: &DynamicImage, out: &mut Out) -> Result<(), String> {
    let img = if img.color().has_alpha() { DynamicImage::ImageRgba8(img.to_rgba8()) } else { DynamicImage::ImageRgb8(img.to_rgb8()) };
    // TIFF needs Seek, so encode in memory first.
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Tiff).map_err(err)?;
    out.write_all(buf.get_ref()).map_err(io)
}

pub(super) fn gif(img: &DynamicImage, out: &mut Out) -> Result<(), String> {
    let rgba = img.to_rgba8();
    let mut enc = GifEncoder::new_with_speed(&mut *out, 10);
    enc.encode(rgba.as_raw(), rgba.width(), rgba.height(), ExtendedColorType::Rgba8).map_err(err)
}

pub(super) fn ico(img: &DynamicImage, out: &mut Out) -> Result<(), String> {
    let square = pad_square(&img.to_rgba8());
    let side = square.width();
    let mut frames = Vec::new();
    for size in [16u32, 24, 32, 48, 64, 128, 256] {
        if size > side && size != 16 {
            continue;
        }
        let resized = image::imageops::resize(&square, size, size, FilterType::Lanczos3);
        let mut png = Vec::new();
        PngEncoder::new(&mut png).write_image(resized.as_raw(), size, size, ExtendedColorType::Rgba8).map_err(err)?;
        frames.push(IcoFrame::with_encoded(png, size, size, ExtendedColorType::Rgba8).map_err(err)?);
    }
    IcoEncoder::new(&mut *out).encode_images(&frames).map_err(err)
}

/// RGB with transparent areas composited onto white (for JPEG).
fn flatten(img: &DynamicImage) -> image::RgbImage {
    if !img.color().has_alpha() {
        return img.to_rgb8();
    }
    let rgba = img.to_rgba8();
    let mut rgb = image::RgbImage::new(rgba.width(), rgba.height());
    for (d, s) in rgb.pixels_mut().zip(rgba.pixels()) {
        let a = s[3] as u32;
        for c in 0..3 {
            d[c] = ((s[c] as u32 * a + 255 * (255 - a) + 127) / 255) as u8;
        }
    }
    rgb
}

fn pad_square(img: &RgbaImage) -> RgbaImage {
    if img.width() == img.height() {
        return img.clone();
    }
    let side = img.width().max(img.height());
    let mut sq = RgbaImage::new(side, side);
    image::imageops::overlay(&mut sq, img, ((side - img.width()) / 2) as i64, ((side - img.height()) / 2) as i64);
    sq
}
