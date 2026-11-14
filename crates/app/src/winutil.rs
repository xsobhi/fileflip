//! Small Win32 helpers: strings, message boxes, registry, shell operations.

use std::path::{Path, PathBuf};
use windows::core::{Interface, HSTRING, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn msgbox(owner: HWND, text: &str, style: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    unsafe { MessageBoxW(owner, &HSTRING::from(text), &HSTRING::from("FileFlip"), style) }
}

pub fn info(owner: HWND, text: &str) {
    msgbox(owner, text, MB_OK | MB_ICONINFORMATION);
}

pub fn error(owner: HWND, text: &str) {
    msgbox(owner, text, MB_OK | MB_ICONERROR);
}

pub fn confirm(owner: HWND, text: &str) -> bool {
    msgbox(owner, text, MB_YESNO | MB_ICONQUESTION) == IDYES
}

/// Move a file to the Recycle Bin.
pub fn recycle(path: &Path) -> bool {
    let mut from: Vec<u16> = path.as_os_str().to_string_lossy().encode_utf16().collect();
    from.extend([0, 0]); // double-null terminated list
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI).0 as u16,
        ..Default::default()
    };
    unsafe { SHFileOperationW(&mut op) == 0 }
}

/// Open Explorer with `path` selected.
pub fn reveal(path: &Path) {
    let _ = std::process::Command::new("explorer.exe").arg(format!("/select,{}", path.display())).spawn();
}

pub fn open_url(url: &str) {
    unsafe {
        ShellExecuteW(None, &HSTRING::from("open"), &HSTRING::from(url), None, None, SW_SHOWNORMAL);
    }
}

/// Folder picker. Returns None if cancelled.
pub fn browse_folder(owner: HWND, title: &str) -> Option<PathBuf> {
    unsafe {
        let dlg: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let opts = dlg.GetOptions().ok()?;
        dlg.SetOptions(opts | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM).ok()?;
        let _ = dlg.SetTitle(&HSTRING::from(title));
        dlg.Show(owner).ok()?;
        let item = dlg.GetResult().ok()?;
        let p = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as *const _));
        s.map(PathBuf::from)
    }
}

/// File picker for ffmpeg.exe.
pub fn browse_exe(owner: HWND, title: &str) -> Option<PathBuf> {
    unsafe {
        let dlg: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let _ = dlg.SetTitle(&HSTRING::from(title));
        let name = wide("ffmpeg.exe");
        let spec = wide("ffmpeg.exe");
        let filter = [windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC { pszName: PCWSTR(name.as_ptr()), pszSpec: PCWSTR(spec.as_ptr()) }];
        let _ = dlg.SetFileTypes(&filter);
        dlg.Show(owner).ok()?;
        let item = dlg.GetResult().ok()?;
        let p = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as *const _));
        s.map(PathBuf::from)
    }
}

/// Create a .lnk shortcut.
pub fn create_shortcut(lnk: &Path, target: &Path, args: &str, description: &str) -> windows::core::Result<()> {
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        link.SetPath(&HSTRING::from(target.as_os_str()))?;
        link.SetArguments(&HSTRING::from(args))?;
        link.SetDescription(&HSTRING::from(description))?;
        if let Some(dir) = target.parent() {
            link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()))?;
        }
        let file: IPersistFile = link.cast()?;
        file.Save(&HSTRING::from(lnk.as_os_str()), TRUE)
    }
}

pub fn notify_assoc_changed() {
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
}

// ---------------------------------------------------------------- registry (HKCU)

pub fn reg_set(subkey: &str, name: Option<&str>, value: &str) -> Result<(), String> {
    unsafe {
        let mut key = HKEY::default();
        let r = RegCreateKeyExW(HKEY_CURRENT_USER, &HSTRING::from(subkey), 0, None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None);
        if r != ERROR_SUCCESS {
            return Err(format!("cannot open HKCU\\{subkey}: {r:?}"));
        }
        let data = wide(value);
        let bytes = std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2);
        let name_w = name.map(HSTRING::from);
        let r = match &name_w {
            Some(n) => RegSetValueExW(key, n, 0, REG_SZ, Some(bytes)),
            None => RegSetValueExW(key, PCWSTR::null(), 0, REG_SZ, Some(bytes)),
        };
        let _ = RegCloseKey(key);
        if r != ERROR_SUCCESS {
            return Err(format!("cannot write HKCU\\{subkey}: {r:?}"));
        }
        Ok(())
    }
}

pub fn reg_set_dword(subkey: &str, name: &str, value: u32) -> Result<(), String> {
    unsafe {
        let mut key = HKEY::default();
        let r = RegCreateKeyExW(HKEY_CURRENT_USER, &HSTRING::from(subkey), 0, None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None);
        if r != ERROR_SUCCESS {
            return Err(format!("cannot open HKCU\\{subkey}: {r:?}"));
        }
        let r = RegSetValueExW(key, &HSTRING::from(name), 0, REG_DWORD, Some(&value.to_le_bytes()));
        let _ = RegCloseKey(key);
        if r != ERROR_SUCCESS {
            return Err(format!("cannot write HKCU\\{subkey}: {r:?}"));
        }
        Ok(())
    }
}

pub fn reg_get(subkey: &str, name: Option<&str>) -> Option<String> {
    unsafe {
        let mut buf = vec![0u16; 2048];
        let mut size = (buf.len() * 2) as u32;
        let name_w = name.map(HSTRING::from);
        let pname = name_w.as_ref().map(|n| PCWSTR(n.as_ptr())).unwrap_or(PCWSTR::null());
        let r = RegGetValueW(HKEY_CURRENT_USER, &HSTRING::from(subkey), pname, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr() as *mut _), Some(&mut size));
        if r != ERROR_SUCCESS {
            return None;
        }
        let len = (size as usize / 2).saturating_sub(1);
        Some(String::from_utf16_lossy(&buf[..len]))
    }
}

pub fn reg_delete_tree(subkey: &str) {
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(subkey));
        let _ = windows::Win32::System::Registry::RegDeleteKeyW(HKEY_CURRENT_USER, &HSTRING::from(subkey));
    }
}

pub fn reg_delete_value(subkey: &str, name: &str) {
    unsafe {
        let mut key = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(subkey), 0, KEY_WRITE, &mut key) == ERROR_SUCCESS {
            let _ = RegDeleteValueW(key, &HSTRING::from(name));
            let _ = RegCloseKey(key);
        }
    }
}
