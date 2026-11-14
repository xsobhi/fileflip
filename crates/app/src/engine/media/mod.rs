//! Turns "convert this file to format X with options Y" into FFmpeg arguments.
//!
//! Default ("same quality") strategy:
//! * If the target container can hold the source streams, copy them (lossless and very fast).
//! * Otherwise re-encode with a bitrate matched to the source, adjusted for how efficient the
//!   source and target codecs are, preferring GPU encoders when available.
//! * Quality presets scale that matched bitrate (75% quality = 75% of the bitrate).

mod audio;
mod codecs;
mod still;
#[cfg(test)]
mod tests;
mod video;

pub use audio::plan_audio;
pub use still::plan_image;
pub use video::plan_video;
#[derive(Debug, Default)]
pub struct Plan {
    /// Arguments placed before `-i`.
    pub pre_input: Vec<String>,
    /// Arguments after the input, before `-f <muxer> <output>`.
    pub args: Vec<String>,
    pub muxer: &'static str,
    /// Re-encodes video: limited to one at a time since encoders use every core.
    pub heavy: bool,
    /// Copies cover art (retried without it if FFmpeg fails).
    pub with_cover: bool,
}

fn s<T: ToString>(v: T) -> String {
    v.to_string()
}

fn push(args: &mut Vec<String>, items: &[&str]) {
    args.extend(items.iter().map(|x| x.to_string()));
}

fn muxer(target: &str) -> &'static str {
    match target {
        "mp4" => "mp4",
        "mkv" => "matroska",
        "webm" => "webm",
        "mov" => "mov",
        "avi" => "avi",
        "vgif" => "gif",
        "mp3" => "mp3",
        "m4a" => "ipod",
        "wav" => "wav",
        "flac" => "flac",
        "ogg" => "ogg",
        "opus" => "opus",
        _ => "matroska",
    }
}
