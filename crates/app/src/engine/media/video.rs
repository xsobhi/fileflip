//! Video encoder choice and video plans.

use super::audio::audio_encode;
use super::codecs::{audio_copy_ok, is_text_sub, video_copy_ok, video_eff};
use super::still::plan_gif;
use super::{muxer, push, Plan};
use crate::engine::ffmpeg::{Caps, Probe};
use crate::engine::Options;
use fileflip_core::Format;
enum VCodec {
    H264,
    Av1OrVp9,
    Mpeg4,
}

/// Pick the fastest encoder that produces what the container needs.
fn video_encoder(target: &str, caps: &Caps, hw: bool) -> (&'static str, String) {
    let codec = match target {
        "webm" => VCodec::Av1OrVp9,
        "avi" => VCodec::Mpeg4,
        _ => VCodec::H264,
    };
    match codec {
        VCodec::H264 => {
            if hw {
                if let Some(e) = &caps.hw_h264 {
                    return ("h264", e.clone());
                }
            }
            ("h264", "libx264".into())
        }
        VCodec::Av1OrVp9 => {
            if hw {
                if let Some(e) = &caps.hw_av1 {
                    return ("av1", e.clone());
                }
            }
            if caps.has("libsvtav1") {
                ("av1", "libsvtav1".into())
            } else if caps.has("libvpx-vp9") {
                ("vp9", "libvpx-vp9".into())
            } else {
                ("av1", "libaom-av1".into())
            }
        }
        VCodec::Mpeg4 => ("mpeg4", "mpeg4".into()),
    }
}

fn video_encoder_args(args: &mut Vec<String>, enc: &str, kbps: u32) {
    let b = format!("{kbps}k");
    let max = format!("{}k", kbps * 3 / 2);
    let buf = format!("{}k", kbps * 2);
    let rate = ["-b:v", b.as_str(), "-maxrate", max.as_str(), "-bufsize", buf.as_str()];
    match enc {
        "libx264" => push(args, &["-c:v", "libx264", "-preset", "veryfast"]),
        "h264_nvenc" | "av1_nvenc" => push(args, &["-c:v", enc, "-preset", "p4", "-tune", "hq", "-rc", "vbr"]),
        "h264_qsv" | "av1_qsv" => push(args, &["-c:v", enc, "-preset", "veryfast"]),
        "h264_amf" | "av1_amf" => push(args, &["-c:v", enc, "-quality", "speed", "-rc", "vbr_peak"]),
        "libsvtav1" => push(args, &["-c:v", "libsvtav1", "-preset", "10"]),
        "libvpx-vp9" => push(args, &["-c:v", "libvpx-vp9", "-deadline", "realtime", "-cpu-used", "8", "-row-mt", "1"]),
        "libaom-av1" => push(args, &["-c:v", "libaom-av1", "-usage", "realtime", "-cpu-used", "8", "-row-mt", "1"]),
        "mpeg4" => push(args, &["-c:v", "mpeg4", "-vtag", "xvid", "-threads", "0"]),
        other => push(args, &["-c:v", other]),
    }
    if matches!(enc, "libsvtav1" | "libvpx-vp9" | "libaom-av1") {
        push(args, &["-b:v", &b]);
    } else {
        push(args, &rate);
    }
    let pix = if enc.ends_with("_qsv") || enc.ends_with("_amf") { "nv12" } else { "yuv420p" };
    push(args, &["-pix_fmt", pix]);
}

pub fn plan_video(target: &Format, opts: &Options, probe: &Probe, caps: &Caps, file_size: u64) -> Result<Plan, String> {
    if target.id == "vgif" {
        return plan_gif(opts, probe);
    }
    let mut p = Plan { muxer: muxer(target.id), ..Default::default() };
    let v = probe.video();
    if v.is_none() && probe.main_audio().is_none() {
        return Err("no video or audio track found".into());
    }
    let reencode_video = opts.quality.is_some() || opts.height.is_some() || opts.fps.is_some();
    let mut args = Vec::new();

    if let Some(v) = v {
        push(&mut args, &["-map", &format!("0:{}", v.index)]);
        if !reencode_video && video_copy_ok(target.id, v.codec()) {
            push(&mut args, &["-c:v", "copy"]);
            if v.codec() == "hevc" && matches!(target.id, "mp4" | "mov") {
                push(&mut args, &["-tag:v", "hvc1"]);
            }
            if v.codec() == "mpeg4" && target.id != "avi" {
                push(&mut args, &["-bsf:v", "mpeg4_unpack_bframes"]);
            }
        } else {
            p.heavy = true;
            let (tcodec, enc) = video_encoder(target.id, caps, opts.hw);
            if opts.hw {
                push(&mut p.pre_input, &["-hwaccel", "auto"]);
            }
            let (w, h) = (v.width.unwrap_or(1280) as f64, v.height.unwrap_or(720) as f64);
            let src_fps = v.fps().unwrap_or(30.0);
            let mut out_h = h;
            let mut filters = Vec::new();
            if let Some(max_h) = opts.height {
                if h > max_h as f64 {
                    filters.push(format!("scale=-2:{max_h}"));
                    out_h = max_h as f64;
                }
            }
            if filters.is_empty() && (v.width.unwrap_or(0) % 2 == 1 || v.height.unwrap_or(0) % 2 == 1) {
                filters.push("scale=trunc(iw/2)*2:trunc(ih/2)*2".into());
            }
            let mut out_fps = src_fps;
            if let Some(f) = opts.fps {
                if (f as f64) < src_fps {
                    filters.push(format!("fps={f}"));
                    out_fps = f as f64;
                }
            }
            let pixel_ratio = (out_h / h).powi(2);
            let out_w = w * out_h / h;

            let src_bps = probe.video_bitrate(file_size).unwrap_or(out_w * out_h * out_fps * 0.1);
            let mut bps = src_bps * video_eff(tcodec) / video_eff(v.codec());
            bps *= pixel_ratio * (out_fps / src_fps).max(0.3);
            if !enc.starts_with("lib") && enc != "mpeg4" {
                bps *= 1.15; // GPU encoders need a few more bits for the same quality
            }
            bps *= opts.quality.unwrap_or(100) as f64 / 100.0;
            let cap = out_w * out_h * out_fps * 0.25 * video_eff(tcodec);
            let kbps = (bps.min(cap) / 1000.0).round().max(150.0) as u32;

            if !filters.is_empty() {
                push(&mut args, &["-vf", &filters.join(",")]);
            }
            video_encoder_args(&mut args, &enc, kbps);
        }
    }

    if !opts.remove_audio {
        let audio_enc = match target.id {
            "webm" => "libopus",
            "avi" => "libmp3lame",
            _ => "aac",
        };
        for (n, a) in probe.audios().enumerate() {
            push(&mut args, &["-map", &format!("0:{}", a.index)]);
            if opts.audio_kbps.is_none() && opts.channels.is_none() && audio_copy_ok(target.id, a.codec()) {
                push(&mut args, &[&format!("-c:a:{n}"), "copy"]);
            } else {
                audio_encode(&mut args, n, a, audio_enc, opts);
            }
        }
    }

    let mut sn = 0;
    for sub in probe.subtitles() {
        let c = sub.codec();
        let conv = match (target.id, is_text_sub(c)) {
            ("mkv", true) if c == "mov_text" => "srt",
            ("mkv", _) => "copy",
            ("mp4" | "mov", true) => "mov_text",
            ("webm", true) => "webvtt",
            _ => continue,
        };
        push(&mut args, &["-map", &format!("0:{}", sub.index), &format!("-c:s:{sn}"), conv]);
        sn += 1;
    }

    push(&mut args, &["-map_metadata", "0"]);
    if matches!(target.id, "mkv" | "mp4" | "mov" | "webm") {
        push(&mut args, &["-map_chapters", "0"]);
    }
    if matches!(target.id, "mp4" | "mov") {
        // Some sources have negative start times that break copied MP4s.
        push(&mut args, &["-avoid_negative_ts", "make_zero"]);
    }
    p.args = args;
    Ok(p)
}
