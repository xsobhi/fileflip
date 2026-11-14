//! Audio bitrates, encoder arguments and audio-only plans.

use super::codecs::{audio_copy_ok, audio_eff, is_lossless_audio};
use super::{muxer, push, s, Plan};
use crate::engine::ffmpeg::{Probe, Stream};
use crate::engine::Options;
use fileflip_core::Format;
fn audio_max_kbps(codec: &str) -> u32 {
    match codec {
        "libmp3lame" => 320,
        "aac" => 320,
        "libvorbis" => 320,
        "libopus" => 256,
        _ => 320,
    }
}

fn audio_target_eff(encoder: &str) -> f64 {
    match encoder {
        "libmp3lame" => 1.0,
        "aac" => 0.8,
        "libvorbis" => 0.8,
        "libopus" => 0.6,
        _ => 1.0,
    }
}

const MP3_RATES: [u32; 14] = [32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320];

/// Bitrate (kbit/s) that keeps the source's audio quality in `encoder`.
fn audio_kbps(src: &Stream, encoder: &str, opts: &Options) -> u32 {
    if let Some(k) = opts.audio_kbps {
        return k;
    }
    let max = audio_max_kbps(encoder);
    let codec = src.codec();
    let base = if is_lossless_audio(codec) {
        max as f64
    } else if let Some(b) = src.bitrate() {
        b / 1000.0 * audio_target_eff(encoder) / audio_eff(codec)
    } else {
        // unknown bitrate: assume a good-quality source
        max as f64 * 0.8
    };
    let q = opts.quality.unwrap_or(100) as f64 / 100.0;
    let mut kbps = ((base * q).round() as u32).clamp(32, max);
    if encoder == "libmp3lame" {
        kbps = *MP3_RATES.iter().min_by_key(|r| r.abs_diff(kbps)).unwrap();
    }
    kbps
}

fn wav_codec(src: &Stream) -> &'static str {
    let fmt = src.sample_fmt.as_deref().unwrap_or("");
    match src.raw_bits() {
        Some(24) => "pcm_s24le",
        Some(32) if fmt.starts_with("s32") => "pcm_s32le",
        _ if src.codec() == "pcm_f32le" => "pcm_f32le",
        _ => "pcm_s16le",
    }
}

/// Encoder arguments for one output audio stream (`n` = output stream number).
pub(super) fn audio_encode(args: &mut Vec<String>, n: usize, src: &Stream, encoder: &str, opts: &Options) {
    let spec = format!("-c:a:{n}");
    match encoder {
        "flac" => push(args, &[&spec, "flac"]),
        "wav" => push(args, &[&spec, wav_codec(src)]),
        enc => {
            let k = audio_kbps(src, enc, opts);
            push(args, &[&spec, enc, &format!("-b:a:{n}"), &format!("{k}k")]);
            if enc == "libopus" {
                push(args, &["-vbr", "on"]);
            }
        }
    }
    let ch = src.channels.unwrap_or(2);
    if let Some(c) = opts.channels {
        push(args, &[&format!("-ac:a:{n}"), &s(c)]);
    } else if ch > 2 && matches!(encoder, "libmp3lame" | "libopus") {
        push(args, &[&format!("-ac:a:{n}"), "2"]);
    }
}

pub fn plan_audio(target: &Format, opts: &Options, probe: &Probe, allow_cover: bool) -> Result<Plan, String> {
    let a = probe.main_audio().ok_or("the file has no audio track")?;
    let mut p = Plan { muxer: muxer(target.id), ..Default::default() };
    let args = &mut p.args;
    push(args, &["-map", &format!("0:{}", a.index)]);
    let forced = opts.quality.is_some() || opts.audio_kbps.is_some() || opts.channels.is_some();
    if !forced && audio_copy_ok(target.id, a.codec()) {
        push(args, &["-c:a", "copy"]);
    } else {
        let encoder = match target.id {
            "mp3" => "libmp3lame",
            "m4a" => "aac",
            "ogg" => "libvorbis",
            "opus" => "libopus",
            "flac" => "flac",
            _ => "wav",
        };
        audio_encode(args, 0, a, encoder, opts);
    }
    if allow_cover && matches!(target.id, "mp3" | "m4a" | "flac") {
        if let Some(c) = probe.cover() {
            push(args, &["-map", &format!("0:{}", c.index), "-c:v", "copy", "-disposition:v:0", "attached_pic"]);
            p.with_cover = true;
        }
    }
    push(args, &["-map_metadata", "0"]);
    if target.id == "mp3" {
        push(args, &["-id3v2_version", "3"]);
    }
    Ok(p)
}
