use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum Category {
    Image,
    Video,
    Audio,
}

impl Category {
    pub const ALL: [Category; 3] = [Category::Image, Category::Video, Category::Audio];

    pub fn id(self) -> &'static str {
        match self {
            Category::Image => "image",
            Category::Video => "video",
            Category::Audio => "audio",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        Category::ALL.into_iter().find(|c| c.id() == s)
    }

    pub fn plural(self) -> &'static str {
        match self {
            Category::Image => "Images",
            Category::Video => "Videos",
            Category::Audio => "Audio",
        }
    }

    /// Target formats offered for files of this category, in menu order.
    pub fn targets(self) -> impl Iterator<Item = &'static Format> {
        FORMATS.iter().filter(move |f| f.category == self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Format {
    /// Unique id used on the command line (`--to <id>`).
    pub id: &'static str,
    pub label: &'static str,
    /// Output file extension.
    pub ext: &'static str,
    /// Category of the *output*.
    pub category: Category,
    /// Lossy formats get "Quality N%" presets.
    pub lossy: bool,
    /// Image formats that can be resized get "Size N%" presets.
    pub resizable: bool,
}

const fn f(id: &'static str, label: &'static str, ext: &'static str, category: Category, lossy: bool, resizable: bool) -> Format {
    Format { id, label, ext, category, lossy, resizable }
}

use Category::*;

pub static FORMATS: &[Format] = &[
    // Images
    f("png", "PNG", "png", Image, false, true),
    f("jpg", "JPG", "jpg", Image, true, true),
    f("webp", "WEBP", "webp", Image, true, true),
    f("avif", "AVIF", "avif", Image, true, true),
    f("bmp", "BMP", "bmp", Image, false, true),
    f("gif", "GIF", "gif", Image, false, true),
    f("tiff", "TIFF", "tiff", Image, false, true),
    f("ico", "ICO", "ico", Image, false, false),
    // Videos
    f("mp4", "MP4", "mp4", Video, true, false),
    f("mkv", "MKV", "mkv", Video, true, false),
    f("webm", "WEBM", "webm", Video, true, false),
    f("mov", "MOV", "mov", Video, true, false),
    f("avi", "AVI", "avi", Video, true, false),
    f("vgif", "GIF (animated)", "gif", Video, true, false),
    // Audio
    f("mp3", "MP3", "mp3", Audio, true, false),
    f("m4a", "M4A (AAC)", "m4a", Audio, true, false),
    f("wav", "WAV", "wav", Audio, false, false),
    f("flac", "FLAC", "flac", Audio, false, false),
    f("ogg", "OGG (Vorbis)", "ogg", Audio, true, false),
    f("opus", "OPUS", "opus", Audio, true, false),
];

pub fn format_by_id(id: &str) -> Option<&'static Format> {
    let id = id.to_ascii_lowercase();
    FORMATS.iter().find(|f| f.id == id)
}

const IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "jpe", "jfif", "webp", "bmp", "dib", "gif", "tif", "tiff", "ico", "tga", "qoi",
    "pnm", "pbm", "pgm", "ppm", "avif", "heic", "heif", "jxl",
];
const VIDEO_EXTS: &[&str] = &[
    "mp4", "m4v", "mkv", "webm", "avi", "mov", "qt", "wmv", "asf", "flv", "mpg", "mpeg", "m2v", "ts", "mts",
    "m2ts", "3gp", "3g2", "ogv", "vob", "mxf", "divx", "f4v",
];
const AUDIO_EXTS: &[&str] = &[
    "mp3", "wav", "flac", "m4a", "m4b", "aac", "ogg", "oga", "opus", "wma", "aiff", "aif", "aifc", "ape", "wv",
    "ac3", "eac3", "dts", "amr", "mka", "caf", "mp2", "mpc", "tta",
];

pub fn ext_of(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

pub fn category_of_ext(ext: &str) -> Option<Category> {
    let ext = ext.to_ascii_lowercase();
    let ext = ext.as_str();
    if IMAGE_EXTS.contains(&ext) {
        Some(Image)
    } else if VIDEO_EXTS.contains(&ext) {
        Some(Video)
    } else if AUDIO_EXTS.contains(&ext) {
        Some(Audio)
    } else {
        None
    }
}

pub fn category_of(path: &Path) -> Option<Category> {
    category_of_ext(&ext_of(path))
}

/// True when a file with extension `ext` already is in `format`
/// (e.g. `jpeg` is already `jpg`).
pub fn ext_matches_format(ext: &str, format: &Format) -> bool {
    let ext = ext.to_ascii_lowercase();
    let canonical = match ext.as_str() {
        "jpeg" | "jpe" | "jfif" => "jpg",
        "tif" => "tiff",
        "oga" => "ogg",
        e => e,
    };
    canonical == format.ext && format.id != "vgif"
}
