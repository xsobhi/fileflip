//! A very small native Win32 toolkit: windows with a boxed message handler,
//! DPI-scaled child controls using the system message font, and helpers.

pub mod custom;
pub mod output;
pub mod progress;
pub mod settings;

mod controls;
mod window;

pub use controls::*;
pub use window::*;

use crate::winutil::wide;
use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::c_void;
use std::sync::OnceLock;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemServices::{SS_ENDELLIPSIS, SS_LEFT, SS_NOPREFIX, SS_RIGHT};
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const IDOK_: i32 = 1;
pub const IDCANCEL_: i32 = 2;

struct Theme {
    scale: f32,
    font: isize,
    bold: isize,
}

static THEME: OnceLock<Theme> = OnceLock::new();

pub fn init() {
    unsafe {
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES | ICC_PROGRESS_CLASS | ICC_BAR_CLASSES | ICC_WIN95_CLASSES,
        };
        let _ = InitCommonControlsEx(&icc);
        let dpi = GetDpiForSystem().max(96);
        let mut ncm = NONCLIENTMETRICSW { cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32, ..Default::default() };
        let _ = SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, ncm.cbSize, Some(&mut ncm as *mut _ as *mut c_void), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
        let font = CreateFontIndirectW(&ncm.lfMessageFont);
        let mut lf = ncm.lfMessageFont;
        lf.lfWeight = 600;
        let bold = CreateFontIndirectW(&lf);
        let _ = THEME.set(Theme { scale: dpi as f32 / 96.0, font: font.0 as isize, bold: bold.0 as isize });
    }
}

fn theme() -> &'static Theme {
    THEME.get().expect("ui::init not called")
}

/// Scale a value in 96-DPI units to pixels.
pub fn px(v: i32) -> i32 {
    (v as f32 * theme().scale).round() as i32
}

pub fn font() -> HFONT {
    HFONT(theme().font as *mut c_void)
}

pub fn bold_font() -> HFONT {
    HFONT(theme().bold as *mut c_void)
}

fn hinstance() -> HINSTANCE {
    unsafe { GetModuleHandleW(None).map(|m| HINSTANCE(m.0)).unwrap_or_default() }
}

pub fn run_loop() {
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let top = GetAncestor(msg.hwnd, GA_ROOT);
            if !top.0.is_null() && IsDialogMessageW(top, &msg).as_bool() {
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

pub fn fmt_duration(secs: f64) -> String {
    let s = secs.max(0.0).round() as u64;
    if s < 3600 {
        format!("{:02}:{:02}", s / 60, s % 60)
    } else {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    }
}
