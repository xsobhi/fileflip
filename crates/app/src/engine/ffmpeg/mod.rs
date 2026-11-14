//! Finding, probing, running and downloading FFmpeg.

mod caps;
mod download;
mod probe;

pub use caps::Caps;
pub use download::download;
pub use probe::{Probe, Stream};

use super::progress::Progress;
use fileflip_core::Settings;
use std::io::{BufRead, BufReader, Read};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
pub const FFMPEG_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

/// Folder FileFlip downloads FFmpeg into.
pub fn managed_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("FileFlip").join("ffmpeg")
}

#[derive(Clone, Debug)]
pub struct FFmpeg {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

impl FFmpeg {
    pub fn locate(settings: &Settings) -> Option<FFmpeg> {
        let mut dirs: Vec<PathBuf> = Vec::new();
        if !settings.ffmpeg_path.trim().is_empty() {
            let p = PathBuf::from(settings.ffmpeg_path.trim());
            dirs.push(if p.is_file() { p.parent().map(Path::to_path_buf).unwrap_or_default() } else { p });
        }
        if let Some(d) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
            dirs.push(d.clone());
            dirs.push(d.join("ffmpeg"));
        }
        dirs.push(managed_dir());
        if let Some(path) = std::env::var_os("PATH") {
            dirs.extend(std::env::split_paths(&path));
        }
        dirs.into_iter().find_map(|d| {
            let ffmpeg = d.join("ffmpeg.exe");
            let ffprobe = d.join("ffprobe.exe");
            (ffmpeg.is_file() && ffprobe.is_file()).then_some(FFmpeg { ffmpeg, ffprobe })
        })
    }

    fn cmd(exe: &Path) -> Command {
        let mut c = Command::new(exe);
        c.creation_flags(CREATE_NO_WINDOW).stdin(Stdio::null());
        c
    }

    pub fn probe(&self, input: &Path) -> Result<Probe, String> {
        let out = Self::cmd(&self.ffprobe)
            .args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"])
            .arg(input)
            .output()
            .map_err(|e| format!("could not run ffprobe: {e}"))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(format!("could not read file: {}", last_lines(&err, 2)));
        }
        serde_json::from_slice(&out.stdout).map_err(|e| format!("bad ffprobe output: {e}"))
    }

    /// Run FFmpeg, reporting progress for `job` (0..1 of `duration` seconds).
    pub fn run(&self, args: &[String], duration: Option<f64>, job: usize, progress: &Arc<Progress>) -> Result<(), String> {
        let mut child = Self::cmd(&self.ffmpeg)
            .args(["-hide_banner", "-nostdin", "-y", "-loglevel", "error", "-progress", "pipe:1", "-nostats"])
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("could not run ffmpeg: {e}"))?;
        progress.register_child(job, child.id());

        let mut stderr = child.stderr.take().unwrap();
        let err_thread = std::thread::spawn(move || {
            let mut s = Vec::new();
            let _ = stderr.read_to_end(&mut s);
            String::from_utf8_lossy(&s).into_owned()
        });
        let stdout = BufReader::new(child.stdout.take().unwrap());
        for line in stdout.lines() {
            let Ok(line) = line else { break };
            if let Some(us) = line.strip_prefix("out_time_us=").or_else(|| line.strip_prefix("out_time_ms=")) {
                if let (Ok(us), Some(d)) = (us.trim().parse::<f64>(), duration) {
                    if d > 0.0 {
                        progress.set_fraction(job, (us / 1e6 / d).clamp(0.0, 0.995));
                    }
                }
            }
        }
        let status = child.wait().map_err(|e| e.to_string());
        progress.unregister_child(job);
        let err = err_thread.join().unwrap_or_default();
        if progress.is_cancelled() {
            return Err("Cancelled".into());
        }
        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(_) => Err(if err.trim().is_empty() { "ffmpeg failed".into() } else { last_lines(&err, 3) }),
            Err(e) => Err(e),
        }
    }

    fn cache_key(&self) -> String {
        let meta = std::fs::metadata(&self.ffmpeg).ok();
        let mtime = meta.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
        format!("{}#{}#{}", self.ffmpeg.display(), meta.map(|m| m.len()).unwrap_or(0), mtime)
    }
}

pub fn last_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    lines[lines.len().saturating_sub(n)..].join(" | ")
}
