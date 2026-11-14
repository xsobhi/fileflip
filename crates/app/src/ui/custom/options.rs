//! The choices offered in the Custom window's combo boxes.

pub(super) const QUALITY: &[(&str, Option<u8>)] = &[
    ("Same as source", None),
    ("Best (100)", Some(100)),
    ("High (85)", Some(85)),
    ("Medium (70)", Some(70)),
    ("Small file (50)", Some(50)),
    ("Smallest (30)", Some(30)),
];
pub(super) const VIDEO_QUALITY: &[(&str, Option<u8>)] =
    &[("Same as source", None), ("Full bitrate", Some(100)), ("75% bitrate", Some(75)), ("Half bitrate", Some(50)), ("Quarter bitrate", Some(25))];
pub(super) const SIZES: &[(&str, Option<u8>)] =
    &[("Original size", None), ("75% size", Some(75)), ("Half size", Some(50)), ("Quarter size", Some(25)), ("10% size", Some(10))];
pub(super) const HEIGHTS: &[(&str, Option<u32>)] = &[
    ("Original", None),
    ("2160p (4K)", Some(2160)),
    ("1440p", Some(1440)),
    ("1080p", Some(1080)),
    ("720p", Some(720)),
    ("480p", Some(480)),
    ("360p", Some(360)),
];
pub(super) const FPS: &[(&str, Option<u32>)] = &[("Original", None), ("60 fps", Some(60)), ("30 fps", Some(30)), ("24 fps", Some(24)), ("15 fps", Some(15))];
pub(super) const ENCODER: &[&str] = &["Fastest (GPU)", "CPU only"];
pub(super) const AUDIO_Q: &[(&str, Option<u8>, Option<u32>)] = &[
    ("Same as source", None, None),
    ("75%", Some(75), None),
    ("50%", Some(50), None),
    ("320 kbps", None, Some(320)),
    ("256 kbps", None, Some(256)),
    ("192 kbps", None, Some(192)),
    ("128 kbps", None, Some(128)),
    ("96 kbps", None, Some(96)),
    ("64 kbps", None, Some(64)),
];
pub(super) const CHANNELS: &[(&str, Option<u32>)] = &[("Original channels", None), ("Stereo", Some(2)), ("Mono", Some(1))];

pub(super) fn names<T>(items: &[(&'static str, T)]) -> Vec<&'static str> {
    items.iter().map(|(n, _)| *n).collect()
}
