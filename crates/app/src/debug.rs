//! `fileflip debug-menu <files…> [--invoke "Convert to/JPG"]`
//! Loads fileflip_shell.dll exactly like Explorer does (DllGetClassObject →
//! IShellExtInit::Initialize → IContextMenu::QueryContextMenu), prints the menu
//! and optionally invokes an item. Used for testing without touching Explorer.

use crate::install::DLL_NAME;
use std::ffi::c_void;
use std::path::PathBuf;
use windows::core::{s, Interface, GUID, HRESULT, HSTRING, PCSTR};
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::Registry::HKEY;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

type GetClassObject = unsafe extern "system" fn(*const GUID, *const GUID, *mut *mut c_void) -> HRESULT;

pub fn run(files: Vec<PathBuf>, invoke: Option<String>) -> Result<String, String> {
    unsafe { run_inner(files, invoke).map_err(|e| e.to_string()) }
}

unsafe fn run_inner(files: Vec<PathBuf>, invoke: Option<String>) -> windows::core::Result<String> {
    let dll = std::env::current_exe().unwrap().with_file_name(DLL_NAME);
    let module = LoadLibraryW(&HSTRING::from(dll.as_os_str()))?;
    let proc = GetProcAddress(module, s!("DllGetClassObject")).ok_or_else(|| windows::core::Error::from(E_FAIL))?;
    let get: GetClassObject = std::mem::transmute(proc);
    let clsid = GUID::from_u128(0x4a8660f0_8a4e_45e6_86b1_77d553f9148a);
    let mut factory: Option<IClassFactory> = None;
    get(&clsid, &IClassFactory::IID, &mut factory as *mut _ as *mut *mut c_void).ok()?;
    let factory = factory.unwrap();
    let init: IShellExtInit = factory.CreateInstance(None)?;

    let mut pidls: Vec<*const ITEMIDLIST> = Vec::new();
    for f in &files {
        let abs = std::path::absolute(f).unwrap_or(f.clone());
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        SHParseDisplayName(&HSTRING::from(abs.as_os_str()), None, &mut pidl, 0, None)?;
        pidls.push(pidl);
    }
    let arr = SHCreateShellItemArrayFromIDLists(&pidls)?;
    let data: IDataObject = arr.BindToHandler(None, &BHID_DataObject)?;
    init.Initialize(None, &data, HKEY::default())?;
    let cm: IContextMenu = init.cast()?;
    let menu = CreatePopupMenu()?;
    let hr = (cm.vtable().QueryContextMenu)(cm.as_raw(), menu, 0, 1, 0x7FFF, CMF_NORMAL);
    let mut out = format!("QueryContextMenu -> {:#x} ({} command ids)\n", hr.0, hr.0 & 0xFFFF);
    let mut found = None;
    dump(menu, 0, "", invoke.as_deref(), &mut out, &mut found);
    if let Some(target) = invoke {
        let id = found.ok_or_else(|| windows::core::Error::new(E_INVALIDARG, format!("menu item '{target}' not found")))?;
        let info = CMINVOKECOMMANDINFO {
            cbSize: std::mem::size_of::<CMINVOKECOMMANDINFO>() as u32,
            lpVerb: PCSTR((id - 1) as usize as *const u8),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        cm.InvokeCommand(&info)?;
        out.push_str(&format!("invoked '{target}' (id {id})\n"));
    }
    let _ = DestroyMenu(menu);
    Ok(out)
}

unsafe fn dump(menu: HMENU, depth: usize, prefix: &str, want: Option<&str>, out: &mut String, found: &mut Option<u32>) {
    for i in 0..GetMenuItemCount(menu) {
        let mut buf = [0u16; 256];
        let mut info = MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_STRING | MIIM_SUBMENU | MIIM_ID | MIIM_FTYPE | MIIM_BITMAP,
            dwTypeData: windows::core::PWSTR(buf.as_mut_ptr()),
            cch: buf.len() as u32,
            ..Default::default()
        };
        if GetMenuItemInfoW(menu, i as u32, TRUE, &mut info).is_err() {
            continue;
        }
        let pad = "  ".repeat(depth);
        if info.fType.contains(MFT_SEPARATOR) {
            out.push_str(&format!("{pad}----\n"));
            continue;
        }
        let text = String::from_utf16_lossy(&buf[..info.cch as usize]);
        let path = if prefix.is_empty() { text.clone() } else { format!("{prefix}/{text}") };
        if !info.hSubMenu.0.is_null() {
            let icon = if info.hbmpItem.0.is_null() { "" } else { " [icon]" };
            out.push_str(&format!("{pad}{text} >{icon}\n"));
            dump(info.hSubMenu, depth + 1, &path, want, out, found);
        } else {
            out.push_str(&format!("{pad}{text}  (id {})\n", info.wID));
            if want == Some(path.as_str()) {
                *found = Some(info.wID);
            }
        }
    }
}
