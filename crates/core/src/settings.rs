//! Settings stored as a small INI-style file at `%APPDATA%\FileFlip\settings.ini`.
//! Hand-rolled so the Explorer DLL stays dependency-free.

use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputMode {
    /// Next to the source file.
    Same,
    /// In a subfolder next to the source file.
    Subfolder,
    /// In one fixed folder.
    Folder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IfExists {
    Rename,
    Overwrite,
    Skip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PngCompression {
    Fast,
    Balanced,
    Best,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub output_mode: OutputMode,
    pub output_subfolder: String,
    pub output_folder: String,
    pub if_exists: IfExists,
    /// Use GPU encoders/decoders when available.
    pub hw_accel: bool,
    /// 0 = automatic (number of CPU cores).
    pub max_jobs: u32,
    pub close_when_done: bool,
    /// Move source files to the Recycle Bin after a successful conversion.
    pub delete_source: bool,
    pub png_compression: PngCompression,
    pub menu_images: bool,
    pub menu_videos: bool,
    pub menu_audio: bool,
    /// Empty = auto-detect (next to the exe, the FileFlip data folder, then PATH).
    pub ffmpeg_path: String,
    /// Cached hardware-encoder probe results: "<ffmpeg id>|<h264 encoder>|<av1 encoder>".
    pub hw_cache: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            output_mode: OutputMode::Same,
            output_subfolder: "Converted".into(),
            output_folder: String::new(),
            if_exists: IfExists::Rename,
            hw_accel: true,
            max_jobs: 0,
            close_when_done: true,
            delete_source: false,
            png_compression: PngCompression::Fast,
            menu_images: true,
            menu_videos: true,
            menu_audio: true,
            ffmpeg_path: String::new(),
            hw_cache: String::new(),
        }
    }
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("FileFlip")
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.ini")
}

impl Settings {
    pub fn load() -> Settings {
        std::fs::read_to_string(settings_path()).map(|t| Settings::parse(&t)).unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(data_dir())?;
        let tmp = settings_path().with_extension("ini.tmp");
        std::fs::write(&tmp, self.to_ini())?;
        std::fs::rename(tmp, settings_path())
    }

    pub fn parse(text: &str) -> Settings {
        let map: BTreeMap<&str, &str> = text
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                if l.starts_with(['#', ';', '[']) {
                    return None;
                }
                let (k, v) = l.split_once('=')?;
                Some((k.trim(), v.trim()))
            })
            .collect();
        let d = Settings::default();
        let b = |k: &str, def: bool| map.get(k).map(|v| matches!(*v, "1" | "true" | "yes" | "on")).unwrap_or(def);
        let s = |k: &str, def: &str| map.get(k).map(|v| v.to_string()).unwrap_or_else(|| def.to_string());
        Settings {
            output_mode: match map.get("output_mode").copied() {
                Some("subfolder") => OutputMode::Subfolder,
                Some("folder") => OutputMode::Folder,
                _ => OutputMode::Same,
            },
            output_subfolder: {
                let v = s("output_subfolder", &d.output_subfolder);
                if v.trim().is_empty() { d.output_subfolder.clone() } else { v }
            },
            output_folder: s("output_folder", ""),
            if_exists: match map.get("if_exists").copied() {
                Some("overwrite") => IfExists::Overwrite,
                Some("skip") => IfExists::Skip,
                _ => IfExists::Rename,
            },
            hw_accel: b("hw_accel", d.hw_accel),
            max_jobs: map.get("max_jobs").and_then(|v| v.parse().ok()).unwrap_or(0).min(64),
            close_when_done: b("close_when_done", d.close_when_done),
            delete_source: b("delete_source", d.delete_source),
            png_compression: match map.get("png_compression").copied() {
                Some("balanced") => PngCompression::Balanced,
                Some("best") => PngCompression::Best,
                _ => PngCompression::Fast,
            },
            menu_images: b("menu_images", true),
            menu_videos: b("menu_videos", true),
            menu_audio: b("menu_audio", true),
            ffmpeg_path: s("ffmpeg_path", ""),
            hw_cache: s("hw_cache", ""),
        }
    }

    pub fn to_ini(&self) -> String {
        let b = |v: bool| if v { "1" } else { "0" };
        format!(
            "# FileFlip settings\n\
             output_mode={}\noutput_subfolder={}\noutput_folder={}\nif_exists={}\n\
             hw_accel={}\nmax_jobs={}\nclose_when_done={}\ndelete_source={}\npng_compression={}\n\
             menu_images={}\nmenu_videos={}\nmenu_audio={}\nffmpeg_path={}\nhw_cache={}\n",
            match self.output_mode {
                OutputMode::Same => "same",
                OutputMode::Subfolder => "subfolder",
                OutputMode::Folder => "folder",
            },
            self.output_subfolder,
            self.output_folder,
            match self.if_exists {
                IfExists::Rename => "rename",
                IfExists::Overwrite => "overwrite",
                IfExists::Skip => "skip",
            },
            b(self.hw_accel),
            self.max_jobs,
            b(self.close_when_done),
            b(self.delete_source),
            match self.png_compression {
                PngCompression::Fast => "fast",
                PngCompression::Balanced => "balanced",
                PngCompression::Best => "best",
            },
            b(self.menu_images),
            b(self.menu_videos),
            b(self.menu_audio),
            self.ffmpeg_path,
            self.hw_cache,
        )
    }

    pub fn menu_enabled(&self, c: crate::Category) -> bool {
        match c {
            crate::Category::Image => self.menu_images,
            crate::Category::Video => self.menu_videos,
            crate::Category::Audio => self.menu_audio,
        }
    }
}
