//! Top-level windows: the boxed message handler, class registration, sizing.

use super::*;
pub trait Handler {
    fn handle(&mut self, hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT>;
}

thread_local! {
    static CLASSES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

pub fn app_icon(size: i32) -> HICON {
    unsafe {
        LoadImageW(hinstance(), PCWSTR(1 as *const u16), IMAGE_ICON, size, size, LR_DEFAULTCOLOR)
            .map(|h| HICON(h.0))
            .unwrap_or_default()
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Box<dyn Handler>;
    if msg == WM_NCDESTROY {
        if !ptr.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(Box::from_raw(ptr));
        }
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    // Labels, checkboxes and radios draw on the window's own background.
    if msg == WM_CTLCOLORSTATIC || msg == WM_CTLCOLORBTN {
        let hdc = HDC(wp.0 as *mut c_void);
        SetBkMode(hdc, TRANSPARENT);
        return LRESULT(GetSysColorBrush(COLOR_WINDOW).0 as isize);
    }
    if !ptr.is_null() {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (*ptr).handle(hwnd, msg, wp, lp)));
        match r {
            Ok(Some(res)) => return res,
            Ok(None) => {}
            Err(_) => std::process::exit(3),
        }
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

/// Create a top-level window with a client area of `w`×`h` (96-DPI units), centered.
pub fn create_window(class: &str, title: &str, w: i32, h: i32, handler: Box<dyn Handler>) -> HWND {
    unsafe {
        let class_w = HSTRING::from(class);
        let registered = CLASSES.with(|c| !c.borrow_mut().insert(class.to_string()));
        if !registered {
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(wndproc),
                hInstance: hinstance(),
                hIcon: app_icon(GetSystemMetrics(SM_CXICON)),
                hIconSm: app_icon(GetSystemMetrics(SM_CXSMICON)),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as isize as *mut c_void),
                lpszClassName: PCWSTR(class_w.as_ptr()),
                ..Default::default()
            };
            RegisterClassExW(&wc);
        }
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN;
        let ex = WS_EX_CONTROLPARENT;
        let mut rc = RECT { left: 0, top: 0, right: px(w), bottom: px(h) };
        let _ = AdjustWindowRectEx(&mut rc, style, false, ex);
        let (ww, wh) = (rc.right - rc.left, rc.bottom - rc.top);
        let mut work = RECT::default();
        let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut work as *mut _ as *mut c_void), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
        let x = work.left + ((work.right - work.left) - ww) / 2;
        let y = work.top + ((work.bottom - work.top) - wh) / 2;
        let param = Box::into_raw(Box::new(handler));
        CreateWindowExW(ex, &class_w, &HSTRING::from(title), style, x, y, ww, wh, None, None, hinstance(), Some(param as *const c_void))
            .unwrap_or_default()
    }
}

/// Resize a window so its client area is `w`×`h` (96-DPI units), keeping its position.
pub fn set_client_size(hwnd: HWND, w: i32, h: i32) {
    unsafe {
        let style = WINDOW_STYLE(GetWindowLongW(hwnd, GWL_STYLE) as u32);
        let ex = WINDOW_EX_STYLE(GetWindowLongW(hwnd, GWL_EXSTYLE) as u32);
        let mut rc = RECT { left: 0, top: 0, right: px(w), bottom: px(h) };
        let _ = AdjustWindowRectEx(&mut rc, style, false, ex);
        let _ = SetWindowPos(hwnd, None, 0, 0, rc.right - rc.left, rc.bottom - rc.top, SWP_NOMOVE | SWP_NOZORDER);
    }
}
