//! The quick presets shown under "More options" in the context menu.

use crate::formats::{Category, Format};
use crate::menu::{Action, Node};
use crate::request::Preset;

/// Image encoder quality (JPEG/WebP/AVIF 1-100), with names people understand.
pub const IMAGE_QUALITY: [(&str, u8); 4] = [("Best quality (100)", 100), ("High (85)", 85), ("Medium (70)", 70), ("Small file (50)", 50)];
pub const IMAGE_SIZES: [(&str, u8); 3] = [("75% size", 75), ("Half size", 50), ("Quarter size", 25)];
/// Video quality is a share of the bitrate that matches the source.
pub const VIDEO_QUALITY: [(&str, u8); 4] = [("Full bitrate", 100), ("75% bitrate", 75), ("Half bitrate", 50), ("Quarter bitrate", 25)];
pub const VIDEO_HEIGHTS: [u32; 3] = [1080, 720, 480];

/// Bitrates (kbit/s) offered for a lossy audio format.
pub fn audio_bitrates(format_id: &str) -> &'static [u32] {
    match format_id {
        "opus" => &[256, 192, 128, 96, 64],
        "mp3" | "m4a" | "ogg" => &[320, 256, 192, 128, 96],
        _ => &[],
    }
}

fn base(target: &Format, cat: Category) -> Preset {
    Preset { only: Some(cat), ..Preset::new(target.id) }
}

fn item(label: String, p: Preset) -> Node {
    Node::Item { label, action: Action::Convert(p) }
}

/// Preset items for one target format (empty if it has none).
pub fn more_options(f: &Format, cat: Category) -> Vec<Node> {
    let mut opts = Vec::new();
    match cat {
        Category::Image => {
            if f.lossy {
                for (label, q) in IMAGE_QUALITY {
                    opts.push(item(label.into(), Preset { quality: Some(q), ..base(f, cat) }));
                }
            }
            if f.resizable {
                if !opts.is_empty() {
                    opts.push(Node::Separator);
                }
                for (label, s) in IMAGE_SIZES {
                    opts.push(item(label.into(), Preset { scale: Some(s), ..base(f, cat) }));
                }
            }
        }
        Category::Video => {
            for (label, q) in VIDEO_QUALITY {
                opts.push(item(label.into(), Preset { quality: Some(q), ..base(f, cat) }));
            }
            opts.push(Node::Separator);
            for h in VIDEO_HEIGHTS {
                opts.push(item(format!("Resize to {h}p"), Preset { height: Some(h), ..base(f, cat) }));
            }
        }
        Category::Audio => {
            for &k in audio_bitrates(f.id) {
                opts.push(item(format!("{k} kbps"), Preset { bitrate: Some(k), ..base(f, cat) }));
            }
        }
    }
    opts
}
