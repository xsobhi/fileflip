use fileflip_core::menu::{build, leaves, render, Action, Selection};
use fileflip_core::Settings;
use std::path::PathBuf;

fn sel(paths: &[&str]) -> Selection {
    Selection::new(paths.iter().map(PathBuf::from), &Settings::default())
}

#[test]
fn nothing_for_unsupported() {
    assert!(build(&sel(&["a.txt", "b.exe"])).is_none());
}

#[test]
fn single_category_hides_own_format() {
    let m = render(&build(&sel(&["a.png", "b.PNG"])).unwrap());
    assert!(!m.contains("  PNG    [--to png"), "{m}");
    assert!(m.contains("JPG    [--to jpg --only image]"));
    // but PNG is still offered under More options (resize)
    assert!(m.contains("--to png --scale 50"));
    assert!(m.contains("--to jpg --quality 25"));
}

#[test]
fn mixed_selection_groups_by_category() {
    let m = render(&build(&sel(&["a.png", "b.mp4", "c.mp3", "d.txt"])).unwrap());
    assert!(m.contains("Images (1) >"));
    assert!(m.contains("Videos (1) >"));
    assert!(m.contains("Audio (1) >"));
    assert!(m.contains("Custom… (all 3 files)"));
    assert!(m.contains("--to mp3 --only video"), "video menu offers audio extraction");
}

#[test]
fn leaves_are_counted() {
    let n = build(&sel(&["a.mp4"])).unwrap();
    let mut v = Vec::new();
    leaves(&n, &mut v);
    assert!(v.len() > 20);
    assert_eq!(v.last(), Some(&Action::Settings));
}
