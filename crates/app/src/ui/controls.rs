//! DPI-scaled child controls and helpers to read and update them.

use super::*;
/// Create a child control. Coordinates are in 96-DPI units.
pub fn child(parent: HWND, class: &str, text: &str, style: u32, x: i32, y: i32, w: i32, h: i32, id: i32) -> HWND {
    unsafe {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            &HSTRING::from(class),
            &HSTRING::from(text),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(style),
            px(x),
            px(y),
            px(w),
            px(h),
            parent,
            HMENU(id as isize as *mut c_void),
            hinstance(),
            None,
        )
        .unwrap_or_default();
        SendMessageW(hwnd, WM_SETFONT, WPARAM(font().0 as usize), LPARAM(1));
        hwnd
    }
}

pub fn label(parent: HWND, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {
    child(parent, "STATIC", text, SS_LEFT.0 | SS_NOPREFIX.0 | SS_ENDELLIPSIS.0, x, y, w, h, 0)
}

/// Multi-line label (wraps, no ellipsis).
pub fn label_wrap(parent: HWND, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {
    child(parent, "STATIC", text, SS_LEFT.0 | SS_NOPREFIX.0, x, y, w, h, 0)
}

pub fn label_right(parent: HWND, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {
    child(parent, "STATIC", text, SS_RIGHT.0 | SS_NOPREFIX.0, x, y, w, h, 0)
}

pub fn button(parent: HWND, text: &str, x: i32, y: i32, w: i32, h: i32, id: i32, default: bool) -> HWND {
    let style = if default { BS_DEFPUSHBUTTON } else { BS_PUSHBUTTON };
    child(parent, "BUTTON", text, WS_TABSTOP.0 | style as u32, x, y, w, h, id)
}

pub fn checkbox(parent: HWND, text: &str, x: i32, y: i32, w: i32, h: i32, id: i32, checked: bool) -> HWND {
    let c = child(parent, "BUTTON", text, WS_TABSTOP.0 | BS_AUTOCHECKBOX as u32, x, y, w, h, id);
    set_checked(c, checked);
    c
}

pub fn radio(parent: HWND, text: &str, x: i32, y: i32, w: i32, h: i32, id: i32, first: bool) -> HWND {
    let mut style = BS_AUTORADIOBUTTON as u32;
    if first {
        style |= WS_GROUP.0 | WS_TABSTOP.0;
    }
    child(parent, "BUTTON", text, style, x, y, w, h, id)
}

pub fn group(parent: HWND, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {
    child(parent, "BUTTON", text, BS_GROUPBOX as u32, x, y, w, h, 0)
}

pub fn edit(parent: HWND, text: &str, x: i32, y: i32, w: i32, h: i32, id: i32) -> HWND {
    unsafe {
        let e = child(parent, "EDIT", text, WS_TABSTOP.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32, x, y, w, h, id);
        let _ = SetWindowLongW(e, GWL_EXSTYLE, WS_EX_CLIENTEDGE.0 as i32);
        let _ = SetWindowPos(e, None, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED);
        e
    }
}

pub fn combo(parent: HWND, items: &[&str], selected: usize, x: i32, y: i32, w: i32, id: i32) -> HWND {
    let c = child(parent, "COMBOBOX", "", WS_TABSTOP.0 | WS_VSCROLL.0 | CBS_DROPDOWNLIST as u32, x, y, w, 300, id);
    for it in items {
        let s = wide(it);
        unsafe { SendMessageW(c, CB_ADDSTRING, WPARAM(0), LPARAM(s.as_ptr() as isize)) };
    }
    combo_set(c, selected);
    c
}

pub fn progress_bar(parent: HWND, x: i32, y: i32, w: i32, h: i32) -> HWND {
    let p = child(parent, "msctls_progress32", "", 0, x, y, w, h, 0);
    unsafe { SendMessageW(p, PBM_SETRANGE32, WPARAM(0), LPARAM(1000)) };
    p
}

pub fn set_text(hwnd: HWND, text: &str) {
    if get_text(hwnd) != text {
        unsafe {
            let _ = SetWindowTextW(hwnd, &HSTRING::from(text));
        }
    }
}

pub fn get_text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        let mut buf = vec![0u16; len as usize + 1];
        let n = GetWindowTextW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf[..n as usize])
    }
}

pub fn is_checked(hwnd: HWND) -> bool {
    unsafe { SendMessageW(hwnd, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1 }
}

pub fn set_checked(hwnd: HWND, on: bool) {
    unsafe { SendMessageW(hwnd, BM_SETCHECK, WPARAM(on as usize), LPARAM(0)) };
}

pub fn combo_sel(hwnd: HWND) -> usize {
    unsafe { SendMessageW(hwnd, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0.max(0) as usize }
}

pub fn combo_set(hwnd: HWND, i: usize) {
    unsafe { SendMessageW(hwnd, CB_SETCURSEL, WPARAM(i), LPARAM(0)) };
}

pub fn enable(hwnd: HWND, on: bool) {
    unsafe {
        let _ = EnableWindow(hwnd, on);
    }
}

pub fn show(hwnd: HWND, on: bool) {
    unsafe {
        let _ = ShowWindow(hwnd, if on { SW_SHOW } else { SW_HIDE });
    }
}

pub fn set_bold(hwnd: HWND) {
    unsafe { SendMessageW(hwnd, WM_SETFONT, WPARAM(bold_font().0 as usize), LPARAM(1)) };
}

pub fn set_progress(bar: HWND, fraction: f64) {
    let v = (fraction.clamp(0.0, 1.0) * 1000.0) as usize;
    unsafe {
        if SendMessageW(bar, PBM_GETPOS, WPARAM(0), LPARAM(0)).0 as usize != v {
            SendMessageW(bar, PBM_SETPOS, WPARAM(v), LPARAM(0));
        }
    }
}

pub fn set_progress_state(bar: HWND, state: u32) {
    unsafe { SendMessageW(bar, PBM_SETSTATE, WPARAM(state as usize), LPARAM(0)) };
}

/// `WM_COMMAND` → (control id, notification code)
pub fn command(wp: WPARAM) -> (i32, u32) {
    ((wp.0 & 0xFFFF) as i32, ((wp.0 >> 16) & 0xFFFF) as u32)
}

pub fn close(hwnd: HWND) {
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

pub fn quit() {
    unsafe { PostQuitMessage(0) };
}
