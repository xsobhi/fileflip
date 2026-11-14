//! Turning the platform-independent menu tree into Win32 popup menus.

use crate::DLL_INSTANCE;
use fileflip_core::menu::Node;
use std::ffi::c_void;
use std::sync::atomic::Ordering;
use windows::core::{Result, PCWSTR, PWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub(crate) fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) unsafe fn insert_separator(hmenu: HMENU, pos: u32) -> Result<()> {
    let info = MENUITEMINFOW {
        cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
        fMask: MIIM_FTYPE,
        fType: MFT_SEPARATOR,
        ..Default::default()
    };
    InsertMenuItemW(hmenu, pos, TRUE, &info)
}

pub(crate) unsafe fn build_popup(children: &[Node], next_id: &mut u32) -> Result<HMENU> {
    let menu = CreatePopupMenu()?;
    for (pos, child) in children.iter().enumerate() {
        let pos = pos as u32;
        match child {
            Node::Separator => insert_separator(menu, pos)?,
            Node::Item { label, .. } => {
                let mut text = wide(label);
                let info = MENUITEMINFOW {
                    cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                    fMask: MIIM_STRING | MIIM_ID,
                    wID: *next_id,
                    dwTypeData: PWSTR(text.as_mut_ptr()),
                    ..Default::default()
                };
                *next_id += 1;
                InsertMenuItemW(menu, pos, TRUE, &info)?;
            }
            Node::Sub { label, children } => {
                let sub = build_popup(children, next_id)?;
                let mut text = wide(label);
                let info = MENUITEMINFOW {
                    cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                    fMask: MIIM_STRING | MIIM_SUBMENU,
                    hSubMenu: sub,
                    dwTypeData: PWSTR(text.as_mut_ptr()),
                    ..Default::default()
                };
                InsertMenuItemW(menu, pos, TRUE, &info)?;
            }
        }
    }
    Ok(menu)
}

/// The app icon as a 32-bit ARGB bitmap, which menus draw with alpha.
pub(crate) unsafe fn menu_icon() -> Option<HBITMAP> {
    let cx = GetSystemMetrics(SM_CXSMICON);
    let cy = GetSystemMetrics(SM_CYSMICON);
    let hinst = HINSTANCE(DLL_INSTANCE.load(Ordering::SeqCst) as *mut c_void);
    let icon = LoadImageW(hinst, PCWSTR(1 as *const u16), IMAGE_ICON, cx, cy, LR_DEFAULTCOLOR).ok()?;
    let icon = HICON(icon.0);
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: cx,
            biHeight: -cy,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let dc = CreateCompatibleDC(None);
    let mut bits = std::ptr::null_mut();
    let bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0).ok();
    if let Some(bmp) = bmp {
        let old = SelectObject(dc, bmp);
        let _ = DrawIconEx(dc, 0, 0, icon, cx, cy, 0, None, DI_NORMAL);
        SelectObject(dc, old);
    }
    let _ = DeleteDC(dc);
    let _ = DestroyIcon(icon);
    bmp
}
