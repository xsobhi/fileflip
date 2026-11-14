//! Animated GIFs from video, and images that need FFmpeg.

use super::{push, s, Plan};
use crate::engine::ffmpeg::{Caps, Probe};
use crate::engine::Options;
use fileflip_core::Format;
pub(super) fn plan_gif(opts: &Options, probe: &Probe) -> Result<Plan, String> {
    let v = probe.video().ok_or("the file has no video track")?;
    let src_fps = v.fps().unwrap_or(30.0);
    let fps = opts.fps.map(|f| f as f64).unwrap_or(15.0).min(src_fps).max(1.0);
    let src_w = v.width.unwrap_or(640) as f64;
    let src_h = v.height.unwrap_or(480) as f64;
    let mut w = src_w.min(720.0);
    if let Some(h) = opts.height {
        w = w.min(src_w * h as f64 / src_h);
    }
    w *= opts.quality.unwrap_or(100) as f64 / 100.0;
    let w = (w.round() as u32).max(16);
    let vf = format!(
        "fps={fps},scale={w}:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle"
    );
    let mut p = Plan { muxer: "gif", heavy: true, ..Default::default() };
    push(&mut p.args, &["-map", &format!("0:{}", v.index), "-vf", &vf, "-loop", "0"]);
    Ok(p)
}

/// Used for AVIF output and for inputs the built-in decoders can't read (HEIC, AVIF, JXL…).
pub fn plan_image(target: &Format, opts: &Options, quality: Option<u8>, lossless_source: bool, caps: &Caps, png_level: u32) -> Plan {
    let mut p = Plan { muxer: "image2", ..Default::default() };
    let args = &mut p.args;
    push(args, &["-map", "0:v:0", "-frames:v", "1"]);
    let mut filters = Vec::new();
    if let Some(sc) = opts.scale {
        filters.push(format!("scale=trunc(iw*{sc}/100):-1:flags=lanczos"));
    }
    if target.id == "ico" {
        filters.push("scale='min(256,iw)':'min(256,ih)':force_original_aspect_ratio=decrease".into());
    }
    if !filters.is_empty() {
        push(args, &["-vf", &filters.join(",")]);
    }
    let q = quality.unwrap_or(if lossless_source { 95 } else { 90 }) as f64;
    match target.id {
        "jpg" => {
            let qv = (2.0 + (100.0 - q) * 29.0 / 100.0).round().clamp(2.0, 31.0);
            push(args, &["-c:v", "mjpeg", "-q:v", &s(qv), "-pix_fmt", "yuvj444p"]);
        }
        "webp" => {
            if quality.is_none() && lossless_source {
                push(args, &["-c:v", "libwebp", "-lossless", "1"]);
            } else {
                push(args, &["-c:v", "libwebp", "-quality", &s(q)]);
            }
        }
        "avif" => {
            p.muxer = "avif";
            let crf = ((100.0 - q) * 0.6).round().clamp(0.0, 63.0);
            if caps.has("libsvtav1") && !caps.has("libaom-av1") {
                push(args, &["-c:v", "libsvtav1", "-preset", "8", "-crf", &s(crf)]);
            } else {
                push(args, &["-c:v", "libaom-av1", "-still-picture", "1", "-cpu-used", "6", "-row-mt", "1", "-crf", &s(crf)]);
            }
            push(args, &["-pix_fmt", "yuv420p"]);
        }
        "png" => push(args, &["-c:v", "png", "-compression_level", &s(png_level)]),
        "bmp" => push(args, &["-c:v", "bmp"]),
        "gif" => push(args, &["-c:v", "gif"]),
        "tiff" => push(args, &["-c:v", "tiff"]),
        "ico" => {
            p.muxer = "ico";
            push(args, &["-c:v", "png"]);
        }
        _ => {}
    }
    if p.muxer == "image2" {
        push(args, &["-update", "1"]);
    }
    p
}
