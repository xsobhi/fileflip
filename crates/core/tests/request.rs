use fileflip_core::formats::Category;
use fileflip_core::request::{parse_args, Command, Preset};

#[test]
fn roundtrip() {
    let p = Preset { target: "jpg".into(), quality: Some(75), scale: Some(50), height: None, only: Some(Category::Image) };
    let mut args = vec!["convert".to_string(), "--list".into(), "x.txt".into()];
    args.extend(p.args());
    assert_eq!(parse_args(&args), Ok(Command::Convert { list: "x.txt".into(), preset: p }));
    assert_eq!(parse_args(&[]), Ok(Command::Settings));
    assert!(parse_args(&["convert".into(), "--list".into(), "a".into(), "--to".into(), "nope".into()]).is_err());
}
