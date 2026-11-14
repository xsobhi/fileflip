//! Conversion engine: builds jobs, picks output names, runs them on a worker pool.

pub mod ffmpeg;
pub mod image;
pub mod media;
pub mod progress;

mod convert;
mod output;
mod runner;

pub use runner::{progress_items, Runner};

use fileflip_core::formats::{category_of, ext_of, format_by_id, Category, Format};
use fileflip_core::{Preset, Settings};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Options {
    /// 1..=100, None = same as source.
    pub quality: Option<u8>,
    /// Image size in percent.
    pub scale: Option<u8>,
    /// Max video height.
    pub height: Option<u32>,
    pub fps: Option<u32>,
    pub audio_kbps: Option<u32>,
    pub channels: Option<u32>,
    pub remove_audio: bool,
    /// Allow GPU encoders/decoders.
    pub hw: bool,
}

impl Options {
    fn is_default_quality(&self) -> bool {
        self.quality.is_none() && self.scale.is_none() && self.height.is_none() && self.fps.is_none() && self.audio_kbps.is_none() && self.channels.is_none()
    }
}

#[derive(Clone, Debug)]
pub struct Job {
    pub input: PathBuf,
    pub source: Category,
    pub target: &'static Format,
    pub opts: Options,
}

/// Can a file of category `src` be converted to `target`?
pub fn compatible(src: Category, target: &Format) -> bool {
    match target.category {
        Category::Image => src == Category::Image,
        Category::Video => src == Category::Video,
        Category::Audio => matches!(src, Category::Audio | Category::Video),
    }
}

/// Jobs for a quick context-menu conversion.
pub fn jobs_from_preset(files: &[PathBuf], preset: &Preset, settings: &Settings) -> Vec<Job> {
    let Some(target) = format_by_id(&preset.target) else { return vec![] };
    let opts = Options {
        quality: preset.quality,
        scale: preset.scale,
        height: preset.height,
        hw: settings.hw_accel,
        ..Default::default()
    };
    files
        .iter()
        .filter_map(|f| {
            let c = category_of(f)?;
            if preset.only.is_some_and(|o| o != c) || !compatible(c, target) {
                return None;
            }
            Some(Job { input: f.clone(), source: c, target, opts: opts.clone() })
        })
        .collect()
}

pub fn needs_ffmpeg(job: &Job) -> bool {
    job.source != Category::Image || job.target.id == "avif" || !image::native_input(&ext_of(&job.input))
}
