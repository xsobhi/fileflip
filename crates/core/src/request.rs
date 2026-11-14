//! The tiny protocol between the Explorer extension and `fileflip.exe`.
//!
//! The extension writes the selected paths (one per line, UTF-8) to a temp
//! file and starts e.g. `fileflip.exe convert --list <file> --to jpg --quality 75`.
//! Using a list file avoids the 32K command-line limit for big selections.

use crate::formats::Category;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Preset {
    /// Target format id (see `formats::FORMATS`).
    pub target: String,
    /// 1..=100. `None` = keep the source quality as closely as possible.
    pub quality: Option<u8>,
    /// Image size in percent of the original.
    pub scale: Option<u8>,
    /// Maximum video height (e.g. 720), never upscales.
    pub height: Option<u32>,
    /// Audio bitrate in kbit/s.
    pub bitrate: Option<u32>,
    /// Only convert source files of this category (used for mixed selections).
    pub only: Option<Category>,
}

impl Preset {
    pub fn new(target: &str) -> Self {
        Preset { target: target.to_string(), ..Default::default() }
    }

    pub fn args(&self) -> Vec<String> {
        let mut a = vec!["--to".to_string(), self.target.clone()];
        if let Some(q) = self.quality {
            a.extend(["--quality".into(), q.to_string()]);
        }
        if let Some(s) = self.scale {
            a.extend(["--scale".into(), s.to_string()]);
        }
        if let Some(h) = self.height {
            a.extend(["--height".into(), h.to_string()]);
        }
        if let Some(b) = self.bitrate {
            a.extend(["--bitrate".into(), b.to_string()]);
        }
        if let Some(c) = self.only {
            a.extend(["--only".into(), c.id().to_string()]);
        }
        a
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Quick conversion from the context menu.
    Convert { list: PathBuf, preset: Preset },
    /// Open the "Custom" window for these files.
    Custom { list: PathBuf },
    /// Open the settings window (also the default with no arguments).
    Settings,
    /// Install to %LOCALAPPDATA% and register the context menu.
    Install { quiet: bool },
    /// Remove the context menu and installed files.
    Uninstall { quiet: bool },
    /// Register the context menu for the DLL next to the exe (no copying).
    Register,
    Unregister,
    /// Download FFmpeg with a progress window.
    DownloadFfmpeg,
    /// Developer helper: print the context menu the DLL builds for these files.
    DebugMenu { files: Vec<PathBuf>, invoke: Option<String> },
}

pub fn parse_args(args: &[String]) -> Result<Command, String> {
    let mut it = args.iter().peekable();
    let Some(cmd) = it.next() else { return Ok(Command::Settings) };
    let mut list = None;
    let mut preset = Preset::default();
    let mut quiet = false;
    let mut files = Vec::new();
    let mut invoke = None;
    while let Some(a) = it.next() {
        let mut val = |name: &str| it.next().cloned().ok_or_else(|| format!("missing value for {name}"));
        match a.as_str() {
            "--list" => list = Some(PathBuf::from(val("--list")?)),
            "--to" => preset.target = val("--to")?,
            "--quality" => preset.quality = Some(num(&val("--quality")?, 1, 100)? as u8),
            "--scale" => preset.scale = Some(num(&val("--scale")?, 1, 100)? as u8),
            "--height" => preset.height = Some(num(&val("--height")?, 16, 10_000)?),
            "--bitrate" => preset.bitrate = Some(num(&val("--bitrate")?, 8, 1_000)?),
            "--only" => {
                let v = val("--only")?;
                preset.only = Some(Category::from_id(&v).ok_or(format!("bad category {v}"))?)
            }
            "--invoke" => invoke = Some(val("--invoke")?),
            "--quiet" | "/quiet" | "-q" => quiet = true,
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => files.push(PathBuf::from(other)),
        }
    }
    let need_list = |list: Option<PathBuf>| list.ok_or_else(|| "missing --list".to_string());
    Ok(match cmd.as_str() {
        "convert" => {
            if crate::formats::format_by_id(&preset.target).is_none() {
                return Err(format!("unknown target format '{}'", preset.target));
            }
            Command::Convert { list: need_list(list)?, preset }
        }
        "custom" => Command::Custom { list: need_list(list)? },
        "settings" => Command::Settings,
        "install" => Command::Install { quiet },
        "uninstall" => Command::Uninstall { quiet },
        "register" => Command::Register,
        "unregister" => Command::Unregister,
        "download-ffmpeg" => Command::DownloadFfmpeg,
        "debug-menu" => Command::DebugMenu { files, invoke },
        other => return Err(format!("unknown command '{other}'")),
    })
}

fn num(s: &str, min: u32, max: u32) -> Result<u32, String> {
    let n: u32 = s.trim_end_matches('%').parse().map_err(|_| format!("'{s}' is not a number"))?;
    if n < min || n > max {
        return Err(format!("{n} is out of range {min}..{max}"));
    }
    Ok(n)
}

pub fn list_dir() -> PathBuf {
    std::env::temp_dir().join("FileFlip")
}

/// Write the selection to a fresh temp file and return its path.
pub fn write_list(paths: &[PathBuf]) -> std::io::Result<PathBuf> {
    let dir = list_dir();
    std::fs::create_dir_all(&dir)?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = dir.join(format!("sel-{}-{nanos}.txt", std::process::id()));
    let mut f = std::fs::File::create(&path)?;
    for p in paths {
        writeln!(f, "{}", p.display())?;
    }
    Ok(path)
}

/// Read (and delete) a list file written by [`write_list`].
pub fn read_list(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let text = std::fs::read_to_string(path)?;
    if path.starts_with(list_dir()) {
        let _ = std::fs::remove_file(path);
    }
    Ok(text.lines().map(str::trim).filter(|l| !l.is_empty()).map(PathBuf::from).collect())
}
