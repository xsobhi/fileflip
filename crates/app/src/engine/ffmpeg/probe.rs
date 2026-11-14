//! ffprobe's JSON output and helpers to interpret it.

use serde::Deserialize;
use std::collections::HashMap;
#[derive(Debug, Deserialize, Default)]
pub struct Probe {
    #[serde(default)]
    pub streams: Vec<Stream>,
    #[serde(default)]
    pub format: ProbeFormat,
}

#[derive(Debug, Deserialize, Default)]
pub struct ProbeFormat {
    pub duration: Option<String>,
    pub bit_rate: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Stream {
    pub index: u32,
    pub codec_type: Option<String>,
    pub codec_name: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub bit_rate: Option<String>,
    pub avg_frame_rate: Option<String>,
    pub r_frame_rate: Option<String>,
    pub channels: Option<u32>,
    pub sample_fmt: Option<String>,
    pub bits_per_raw_sample: Option<String>,
    pub duration: Option<String>,
    #[serde(default)]
    pub disposition: HashMap<String, i64>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

fn num(s: &Option<String>) -> Option<f64> {
    s.as_deref().and_then(|v| v.parse::<f64>().ok()).filter(|v| v.is_finite() && *v > 0.0)
}

impl Stream {
    pub fn codec(&self) -> &str {
        self.codec_name.as_deref().unwrap_or("")
    }
    pub fn kind(&self) -> &str {
        self.codec_type.as_deref().unwrap_or("")
    }
    pub fn is_cover(&self) -> bool {
        self.disposition.get("attached_pic").copied().unwrap_or(0) == 1
    }
    /// Bits per second, also looking at Matroska's `BPS` statistics tags.
    pub fn bitrate(&self) -> Option<f64> {
        num(&self.bit_rate).or_else(|| {
            self.tags
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("BPS") || k.to_ascii_uppercase().starts_with("BPS-"))
                .and_then(|(_, v)| v.parse::<f64>().ok())
                .filter(|v| *v > 0.0)
        })
    }
    pub fn fps(&self) -> Option<f64> {
        let parse = |s: &Option<String>| -> Option<f64> {
            let s = s.as_deref()?;
            let (a, b) = s.split_once('/')?;
            let (a, b) = (a.parse::<f64>().ok()?, b.parse::<f64>().ok()?);
            (b > 0.0 && a > 0.0).then(|| a / b).filter(|f| *f < 1000.0)
        };
        parse(&self.avg_frame_rate).or_else(|| parse(&self.r_frame_rate))
    }
    pub fn raw_bits(&self) -> Option<u32> {
        self.bits_per_raw_sample.as_deref().and_then(|b| b.parse().ok()).filter(|b| *b > 0)
    }
}

impl Probe {
    pub fn duration(&self) -> Option<f64> {
        num(&self.format.duration).or_else(|| self.streams.iter().filter_map(|s| num(&s.duration)).reduce(f64::max))
    }
    pub fn video(&self) -> Option<&Stream> {
        self.streams.iter().find(|s| s.kind() == "video" && !s.is_cover())
    }
    pub fn cover(&self) -> Option<&Stream> {
        self.streams.iter().find(|s| s.kind() == "video" && s.is_cover())
    }
    pub fn audios(&self) -> impl Iterator<Item = &Stream> {
        self.streams.iter().filter(|s| s.kind() == "audio")
    }
    /// The default audio track, or the first one.
    pub fn main_audio(&self) -> Option<&Stream> {
        self.audios().find(|s| s.disposition.get("default").copied().unwrap_or(0) == 1).or_else(|| self.audios().next())
    }
    pub fn subtitles(&self) -> impl Iterator<Item = &Stream> {
        self.streams.iter().filter(|s| s.kind() == "subtitle")
    }
    /// Best guess of the video stream's bitrate in bits/s.
    pub fn video_bitrate(&self, file_size: u64) -> Option<f64> {
        let v = self.video()?;
        if let Some(b) = v.bitrate() {
            return Some(b);
        }
        let audio: f64 = self.audios().filter_map(|a| a.bitrate()).sum();
        let total = num(&self.format.bit_rate).or_else(|| {
            let d = self.duration()?;
            (file_size > 0).then(|| file_size as f64 * 8.0 / d)
        })?;
        Some((total - audio).max(total * 0.5))
    }
}
