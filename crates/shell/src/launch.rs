//! Starting `fileflip.exe` (next to this DLL) for a clicked menu item.

use crate::module_dir;
use fileflip_core::menu::Action;
use fileflip_core::request;
use std::path::PathBuf;

/// Start fileflip.exe for the chosen action.
pub fn launch(action: &Action, files: &[PathBuf]) -> std::io::Result<()> {
    let exe = module_dir().join("fileflip.exe");
    let mut args: Vec<String> = Vec::new();
    match action {
        Action::Convert(preset) => {
            let list = request::write_list(files)?;
            args.extend(["convert".into(), "--list".into(), list.display().to_string()]);
            args.extend(preset.args());
        }
        Action::Custom => {
            let list = request::write_list(files)?;
            args.extend(["custom".into(), "--list".into(), list.display().to_string()]);
        }
        Action::Settings => args.push("settings".into()),
    }
    std::process::Command::new(exe).args(args).spawn().map(|_| ())
}
