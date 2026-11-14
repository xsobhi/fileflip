//! WinRAR-style progress window: one bar for the current file and, when
//! converting several files, a second bar for the whole batch.

mod finish;
mod view;

use super::*;
use crate::engine::progress::{Progress, State};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::Threading::{GetCurrentProcess, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS, IDLE_PRIORITY_CLASS};
use windows::Win32::UI::Shell::{ITaskbarList3, TaskbarList, TBPF_ERROR, TBPF_NOPROGRESS, TBPF_NORMAL, TBPF_PAUSED};

/// Compact, tool-like width.
const W: i32 = 264;
const BTN: i32 = 76;
const PAUSE_X: i32 = W - 12 - BTN - 6 - BTN;
const CANCEL_X: i32 = W - 12 - BTN;
const ID_BACKGROUND: i32 = 101;
const ID_PAUSE: i32 = 102;
const TIMER: usize = 1;
/// Don't flash a window for conversions that finish almost instantly.
const SHOW_DELAY: Duration = Duration::from_millis(300);

struct Win {
    progress: Arc<Progress>,
    close_when_done: bool,
    multi: bool,
    start: Instant,
    paused_for: Duration,
    paused_at: Option<Instant>,
    shown: bool,
    done: bool,
    close_requested: bool,
    background: bool,
    taskbar: Option<ITaskbarList3>,
    last_output: Option<PathBuf>,
    c: Controls,
}

#[derive(Default)]
struct Controls {
    file: HWND,
    detail: HWND,
    bar: HWND,
    elapsed: HWND,
    pct: HWND,
    total_lbl: HWND,
    remaining: HWND,
    total_bar: HWND,
    total_pct: HWND,
    bg: HWND,
    pause: HWND,
    cancel: HWND,
    log: HWND,
}

pub fn run(progress: Arc<Progress>, close_when_done: bool) {
    let multi = progress.snapshot().len() > 1;
    let win = Win {
        progress,
        close_when_done,
        multi,
        start: Instant::now(),
        paused_for: Duration::ZERO,
        paused_at: None,
        shown: false,
        done: false,
        close_requested: false,
        background: false,
        taskbar: None,
        last_output: None,
        c: Controls::default(),
    };
    let h = if multi { 206 } else { 140 };
    create_window("FileFlipProgress", "FileFlip", W, h, Box::new(win));
    run_loop();
}

impl Handler for Win {
    fn handle(&mut self, hwnd: HWND, msg: u32, wp: WPARAM, _lp: LPARAM) -> Option<LRESULT> {
        match msg {
            WM_CREATE => {
                self.layout(hwnd);
                unsafe { SetTimer(hwnd, TIMER, 100, None) };
                Some(LRESULT(0))
            }
            WM_TIMER => {
                self.refresh(hwnd);
                Some(LRESULT(0))
            }
            WM_COMMAND => {
                match command(wp).0 {
                    IDCANCEL_ => self.cancel(hwnd),
                    ID_PAUSE if self.done => {
                        if let Some(o) = &self.last_output {
                            crate::winutil::reveal(o);
                        }
                    }
                    ID_PAUSE => self.toggle_pause(),
                    ID_BACKGROUND => self.toggle_background(hwnd),
                    _ => {}
                }
                Some(LRESULT(0))
            }
            WM_CLOSE => {
                if self.done {
                    close(hwnd);
                } else {
                    self.close_requested = true;
                    self.cancel(hwnd);
                }
                Some(LRESULT(0))
            }
            WM_DESTROY => {
                if !self.done {
                    self.progress.cancel();
                }
                quit();
                Some(LRESULT(0))
            }
            _ => None,
        }
    }
}
