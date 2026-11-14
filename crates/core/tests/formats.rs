use fileflip_core::formats::{category_of, ext_matches_format, format_by_id, Category::*, FORMATS};
use std::path::Path;

#[test]
fn ids_are_unique() {
    for (i, a) in FORMATS.iter().enumerate() {
        for b in &FORMATS[i + 1..] {
            assert_ne!(a.id, b.id);
        }
    }
}

#[test]
fn categories() {
    assert_eq!(category_of(Path::new("C:/a/B.JPEG")), Some(Image));
    assert_eq!(category_of(Path::new("x.mkv")), Some(Video));
    assert_eq!(category_of(Path::new("x.flac")), Some(Audio));
    assert_eq!(category_of(Path::new("x.txt")), None);
    assert!(ext_matches_format("jpeg", format_by_id("jpg").unwrap()));
    assert!(!ext_matches_format("gif", format_by_id("vgif").unwrap()));
}
