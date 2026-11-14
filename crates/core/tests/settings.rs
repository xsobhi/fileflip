use fileflip_core::settings::{IfExists, OutputMode, Settings};

#[test]
fn roundtrip() {
    let mut s = Settings::default();
    s.output_mode = OutputMode::Folder;
    s.output_folder = r"D:\Out put".into();
    s.if_exists = IfExists::Skip;
    s.max_jobs = 4;
    s.menu_audio = false;
    assert_eq!(Settings::parse(&s.to_ini()), s);
    assert_eq!(Settings::parse(""), Settings::default());
}
