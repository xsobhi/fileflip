//! What we know about codecs: relative efficiency and which containers can hold them.

/// Bits needed relative to H.264 for the same visual quality.
pub(super) fn video_eff(codec: &str) -> f64 {
    match codec {
        "h264" => 1.0,
        "hevc" => 0.6,
        "av1" => 0.5,
        "vp9" => 0.65,
        "vp8" => 1.1,
        "theora" => 1.4,
        "wmv3" | "vc1" => 1.2,
        "mpeg4" | "msmpeg4v3" | "msmpeg4v2" | "msmpeg4v1" | "wmv1" | "wmv2" | "flv1" | "h263" => 1.5,
        "mpeg2video" => 2.0,
        "mpeg1video" => 2.5,
        "mjpeg" => 5.0,
        c if c.starts_with("prores") || c.starts_with("dnx") => 6.0,
        "rawvideo" | "ffv1" | "huffyuv" | "utvideo" | "magicyuv" | "qtrle" | "png" => 12.0,
        _ => 1.5,
    }
}

pub(super) fn audio_eff(codec: &str) -> f64 {
    match codec {
        "mp3" => 1.0,
        "mp2" => 1.4,
        "aac" => 0.8,
        "vorbis" => 0.8,
        "opus" => 0.6,
        "wmav1" | "wmav2" | "wmapro" => 1.1,
        "ac3" => 1.3,
        "eac3" => 1.0,
        "dts" => 1.5,
        "amr_nb" | "amr_wb" => 0.5,
        _ => 1.0,
    }
}

pub(super) fn is_lossless_audio(codec: &str) -> bool {
    codec.starts_with("pcm_")
        || matches!(codec, "flac" | "alac" | "wavpack" | "ape" | "tta" | "truehd" | "mlp" | "tak" | "shorten")
}

pub(super) fn is_text_sub(codec: &str) -> bool {
    matches!(codec, "subrip" | "srt" | "ass" | "ssa" | "webvtt" | "mov_text" | "text")
}

// ---------------------------------------------------------------- containers

pub(super) fn video_copy_ok(target: &str, codec: &str) -> bool {
    match target {
        "mkv" => !codec.is_empty(),
        "mp4" => matches!(codec, "h264" | "hevc" | "av1" | "mpeg4" | "vp9" | "mpeg2video"),
        "mov" => matches!(codec, "h264" | "hevc" | "mpeg4" | "mjpeg" | "mpeg2video") || codec.starts_with("prores"),
        "webm" => matches!(codec, "vp8" | "vp9" | "av1"),
        "avi" => matches!(codec, "mpeg4" | "msmpeg4v3" | "msmpeg4v2" | "mjpeg"),
        _ => false,
    }
}

pub(super) fn audio_copy_ok(target: &str, codec: &str) -> bool {
    match target {
        "mkv" => !codec.is_empty(),
        "mp4" => matches!(codec, "aac" | "mp3" | "ac3" | "eac3" | "alac" | "opus" | "flac"),
        "mov" => matches!(codec, "aac" | "alac" | "mp3" | "ac3" | "pcm_s16le" | "pcm_s24le" | "pcm_s16be" | "pcm_s24be"),
        "webm" => matches!(codec, "opus" | "vorbis"),
        "avi" => matches!(codec, "mp3" | "ac3" | "pcm_s16le"),
        // audio-only targets
        "mp3" => codec == "mp3",
        "m4a" => matches!(codec, "aac" | "alac"),
        "flac" => codec == "flac",
        "ogg" => codec == "vorbis",
        "opus" => codec == "opus",
        "wav" => matches!(codec, "pcm_s16le" | "pcm_s24le" | "pcm_s32le" | "pcm_f32le" | "pcm_u8"),
        _ => false,
    }
}
