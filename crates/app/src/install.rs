//! Per-user install (no admin needed): copies the app to
//! `%LOCALAPPDATA%\Programs\FileFlip`, registers the Explorer extension under
//! HKCU, adds a Start-menu shortcut and an "Apps & features" entry.

use crate::winutil::*;
use fileflip_core::SHELL_CLSID;
use std::path::{Path, PathBuf};

pub const DLL_NAME: &str = "fileflip_shell.dll";
pub const EXE_NAME: &str = "fileflip.exe";
const HANDLER_KEY: &str = r"Software\Classes\*\shellex\ContextMenuHandlers\FileFlip";
const APPROVED_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Shell Extensions\Approved";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\FileFlip";

fn clsid_key() -> String {
    format!(r"Software\Classes\CLSID\{SHELL_CLSID}")
}

pub fn install_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Programs").join("FileFlip")
}

fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default()
}

fn start_menu_link() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default();
    base.join(r"Microsoft\Windows\Start Menu\Programs\FileFlip.lnk")
}

/// DLL path the context menu is currently registered with, if any.
pub fn registered_dll() -> Option<PathBuf> {
    reg_get(HANDLER_KEY, None)?;
    reg_get(&format!(r"{}\InprocServer32", clsid_key()), None).map(PathBuf::from)
}

/// Register the context menu for the DLL in `dir`.
pub fn register(dir: &Path) -> Result<(), String> {
    let dll = dir.join(DLL_NAME);
    if !dll.is_file() {
        return Err(format!("{} not found next to fileflip.exe", DLL_NAME));
    }
    reg_set(&clsid_key(), None, "FileFlip context menu")?;
    let inproc = format!(r"{}\InprocServer32", clsid_key());
    reg_set(&inproc, None, &dll.display().to_string())?;
    reg_set(&inproc, Some("ThreadingModel"), "Apartment")?;
    reg_set(HANDLER_KEY, None, SHELL_CLSID)?;
    reg_set(APPROVED_KEY, Some(SHELL_CLSID), "FileFlip context menu")?;
    notify_assoc_changed();
    Ok(())
}

pub fn unregister() {
    reg_delete_tree(HANDLER_KEY);
    reg_delete_tree(&clsid_key());
    reg_delete_value(APPROVED_KEY, SHELL_CLSID);
    notify_assoc_changed();
}

/// Copy a file, even over a DLL that Explorer has loaded (by renaming the old one away).
fn copy_over(src: &Path, dst: &Path) -> Result<(), String> {
    if std::fs::copy(src, dst).is_ok() {
        return Ok(());
    }
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let old = dst.with_extension(format!("old-{stamp}"));
    std::fs::rename(dst, &old).map_err(|e| format!("{} is in use: {e}", dst.display()))?;
    std::fs::copy(src, dst).map(|_| ()).map_err(|e| format!("cannot copy {}: {e}", dst.display()))
}

/// Copy a folder's files (one level, like the bundled `ffmpeg` folder). Missing source = no-op.
fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(src) else { return Ok(()) };
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for e in entries.flatten() {
        if e.path().is_file() {
            copy_over(&e.path(), &dst.join(e.file_name()))?;
        }
    }
    clean_old(dst);
    Ok(())
}

/// Remove leftovers from earlier updates (old DLLs that were locked by Explorer).
fn clean_old(dir: &Path) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().contains(".old-") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

pub fn install() -> Result<PathBuf, String> {
    let src = exe_dir();
    let dst = install_dir();
    std::fs::create_dir_all(&dst).map_err(|e| e.to_string())?;
    clean_old(&dst);
    if !same_dir(&src, &dst) {
        for name in [EXE_NAME, DLL_NAME] {
            let from = src.join(name);
            if !from.is_file() {
                return Err(format!("{name} not found next to the running program"));
            }
            copy_over(&from, &dst.join(name))?;
        }
        for extra in ["README.md", "LICENSE"] {
            let _ = std::fs::copy(src.join(extra), dst.join(extra));
        }
        // FFmpeg bundled by the setup / release zip
        copy_dir(&src.join("ffmpeg"), &dst.join("ffmpeg"))?;
    }
    register(&dst)?;
    let exe = dst.join(EXE_NAME);
    let _ = create_shortcut(&start_menu_link(), &exe, "settings", "FileFlip settings");
    let exe_s = exe.display().to_string();
    let _ = reg_set(UNINSTALL_KEY, Some("DisplayName"), "FileFlip");
    let _ = reg_set(UNINSTALL_KEY, Some("DisplayVersion"), env!("CARGO_PKG_VERSION"));
    let _ = reg_set(UNINSTALL_KEY, Some("Publisher"), "FileFlip");
    let _ = reg_set(UNINSTALL_KEY, Some("DisplayIcon"), &exe_s);
    let _ = reg_set(UNINSTALL_KEY, Some("InstallLocation"), &dst.display().to_string());
    let _ = reg_set(UNINSTALL_KEY, Some("UninstallString"), &format!("\"{exe_s}\" uninstall"));
    let _ = reg_set(UNINSTALL_KEY, Some("URLInfoAbout"), fileflip_core::REPO_URL);
    let _ = reg_set_dword(UNINSTALL_KEY, "NoModify", 1);
    let _ = reg_set_dword(UNINSTALL_KEY, "NoRepair", 1);
    Ok(dst)
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy()),
    }
}

/// Unregister and remove installed files. Returns a note for the user.
pub fn uninstall() -> String {
    unregister();
    let _ = std::fs::remove_file(start_menu_link());
    reg_delete_tree(UNINSTALL_KEY);
    let dir = install_dir();
    if !dir.exists() {
        return "FileFlip was removed from the context menu.".into();
    }
    // Explorer may still have the DLL loaded and we can't delete our own exe while running,
    // so let a detached cmd.exe finish the job a moment after we exit.
    // raw_arg: cmd.exe doesn't understand Rust's \" argument escaping.
    use std::os::windows::process::CommandExt;
    let script = format!("/c ping -n 3 127.0.0.1 >nul & rmdir /s /q \"{}\"", dir.display());
    let _ = std::process::Command::new("cmd.exe")
        .raw_arg(script)
        .creation_flags(crate::engine::ffmpeg::CREATE_NO_WINDOW)
        .spawn();
    "FileFlip was uninstalled. If Explorer still had it loaded, a few files stay in\n\
     %LOCALAPPDATA%\\Programs\\FileFlip until you sign out or restart Explorer."
        .into()
}
