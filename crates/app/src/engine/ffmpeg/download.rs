//! Downloading the FFmpeg "essentials" build with the curl/tar that ship with Windows.

use super::{last_lines, managed_dir, FFmpeg, CREATE_NO_WINDOW, FFMPEG_URL};
use crate::engine::progress::Progress;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
/// Download the FFmpeg "essentials" build into [`managed_dir`], reporting progress on `job`.
pub fn download(job: usize, progress: &Arc<Progress>) -> Result<FFmpeg, String> {
    let sys = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let curl = Path::new(&sys).join(r"System32\curl.exe");
    let tar = Path::new(&sys).join(r"System32\tar.exe");
    let work = managed_dir().with_file_name("ffmpeg-download");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let zip = work.join("ffmpeg.zip");

    // Size for the progress bar (best effort).
    let total: Option<u64> = Command::new(&curl)
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-sIL", FFMPEG_URL])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|v| v.trim().parse().ok()))
                .last()
        });

    let mut child = Command::new(&curl)
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-L", "--fail", "-s", "-S", "-o"])
        .arg(&zip)
        .arg(FFMPEG_URL)
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start curl: {e}"))?;
    progress.register_child(job, child.id());
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            progress.unregister_child(job);
            if progress.is_cancelled() {
                return Err("Cancelled".into());
            }
            if !status.success() {
                let mut e = String::new();
                if let Some(mut s) = child.stderr.take() {
                    let _ = s.read_to_string(&mut e);
                }
                return Err(format!("download failed: {}", last_lines(&e, 2)));
            }
            break;
        }
        if let (Some(t), Ok(m)) = (total, std::fs::metadata(&zip)) {
            progress.set_fraction(job, (m.len() as f64 / t as f64 * 0.9).min(0.9));
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    progress.set_detail(job, "Extracting…");
    let out = Command::new(&tar)
        .creation_flags(CREATE_NO_WINDOW)
        .arg("-xf")
        .arg(&zip)
        .arg("-C")
        .arg(&work)
        .output()
        .map_err(|e| format!("could not extract: {e}"))?;
    if !out.status.success() {
        return Err(format!("could not extract: {}", last_lines(&String::from_utf8_lossy(&out.stderr), 2)));
    }
    let bin = find_file(&work, "ffmpeg.exe").and_then(|p| p.parent().map(Path::to_path_buf)).ok_or("ffmpeg.exe not found in download")?;
    let dest = managed_dir();
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    for name in ["ffmpeg.exe", "ffprobe.exe"] {
        std::fs::copy(bin.join(name), dest.join(name)).map_err(|e| format!("{name}: {e}"))?;
    }
    let _ = std::fs::remove_dir_all(&work);
    progress.set_fraction(job, 1.0);
    Ok(FFmpeg { ffmpeg: dest.join("ffmpeg.exe"), ffprobe: dest.join("ffprobe.exe") })
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(f) = find_file(&p, name) {
                return Some(f);
            }
        } else if p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.eq_ignore_ascii_case(name)) {
            return Some(p);
        }
    }
    None
}
