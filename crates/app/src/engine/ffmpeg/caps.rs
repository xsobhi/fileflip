//! Which encoders this FFmpeg build has, and which GPU encoders actually work.

use super::FFmpeg;
use fileflip_core::Settings;
use std::collections::HashSet;
use std::process::Stdio;

impl FFmpeg {
    pub fn encoders(&self) -> HashSet<String> {
        let out = Self::cmd(&self.ffmpeg).args(["-hide_banner", "-encoders"]).output();
        let text = out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
        text.lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let flags = it.next()?;
                let name = it.next()?;
                (flags.len() == 6 && !flags.contains('=')).then(|| name.to_string())
            })
            .collect()
    }

    /// Does this encoder actually work on this machine (GPU present, driver ok)?
    fn encoder_works(&self, enc: &str) -> bool {
        Self::cmd(&self.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=c=black:s=256x256:r=30", "-frames:v", "3", "-c:v", enc, "-f", "null", "-"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Encoder capabilities, with GPU probing cached in the settings file.
    pub fn caps(&self, settings: &Settings) -> Caps {
        let encoders = self.encoders();
        let key = self.cache_key();
        let cached: Vec<&str> = settings.hw_cache.split('|').collect();
        let (hw_h264, hw_av1) = if cached.len() == 3 && cached[0] == key {
            (opt(cached[1]), opt(cached[2]))
        } else {
            let pick = |cands: &[&str]| cands.iter().find(|e| encoders.contains(**e) && self.encoder_works(e)).map(|e| e.to_string());
            let h = pick(&["h264_nvenc", "h264_qsv", "h264_amf"]);
            let a = pick(&["av1_nvenc", "av1_qsv", "av1_amf"]);
            let mut s = Settings::load();
            s.hw_cache = format!("{key}|{}|{}", h.clone().unwrap_or_default(), a.clone().unwrap_or_default());
            let _ = s.save();
            (h, a)
        };
        Caps { encoders, hw_h264, hw_av1 }
    }
}

fn opt(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

#[derive(Clone, Debug, Default)]
pub struct Caps {
    pub encoders: HashSet<String>,
    pub hw_h264: Option<String>,
    pub hw_av1: Option<String>,
}

impl Caps {
    pub fn has(&self, enc: &str) -> bool {
        self.encoders.contains(enc)
    }
}
