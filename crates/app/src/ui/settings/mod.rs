//! Settings window (what you get when starting FileFlip without a file).

mod view;

use super::output::{self, OutputControls};
use super::*;
use crate::install;
use fileflip_core::Settings;

const W: i32 = 560;
const ID_INSTALL: i32 = 201;
const ID_UNINSTALL: i32 = 202;
const ID_DOWNLOAD: i32 = 203;
const ID_BROWSE_FF: i32 = 204;
const ID_AUTO_FF: i32 = 205;
const ID_GITHUB: i32 = 206;
const TIMER: usize = 2;

const JOBS: &[(&str, u32)] = &[("Automatic", 0), ("1 at a time", 1), ("2", 2), ("4", 4), ("8", 8)];

#[derive(Default)]
struct Win {
    settings: Settings,
    menu_status: HWND,
    install: HWND,
    uninstall: HWND,
    images: HWND,
    videos: HWND,
    audio: HWND,
    ff_status: HWND,
    ff_path: HWND,
    download: HWND,
    gpu: HWND,
    jobs: HWND,
    png: HWND,
    close_done: HWND,
    output: OutputControls,
}

pub fn run() {
    let win = Win { settings: Settings::load(), ..Default::default() };
    let h = 12 + 92 + 10 + 86 + 10 + output::HEIGHT + 10 + 82 + 56;
    let hwnd = create_window("FileFlipSettings", "FileFlip Settings", W, h, Box::new(win));
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
    run_loop();
}

impl Handler for Win {
    fn handle(&mut self, hwnd: HWND, msg: u32, wp: WPARAM, _lp: LPARAM) -> Option<LRESULT> {
        match msg {
            WM_CREATE => {
                self.layout(hwnd);
                // pick up an FFmpeg download finishing in the other window
                unsafe { SetTimer(hwnd, TIMER, 1500, None) };
                Some(LRESULT(0))
            }
            WM_TIMER => {
                self.refresh();
                Some(LRESULT(0))
            }
            WM_COMMAND => {
                let (id, _) = command(wp);
                if self.output.command(hwnd, id) {
                    return Some(LRESULT(0));
                }
                match id {
                    IDOK_ => match self.save() {
                        Ok(()) => close(hwnd),
                        Err(e) => crate::winutil::error(hwnd, &e),
                    },
                    IDCANCEL_ => close(hwnd),
                    ID_INSTALL => match install::install() {
                        Ok(dir) => {
                            self.refresh();
                            crate::winutil::info(hwnd, &format!("Installed to {}.\n\nRight-click any image, video or audio file and choose \"Convert to\".", dir.display()));
                        }
                        Err(e) => crate::winutil::error(hwnd, &format!("Install failed: {e}")),
                    },
                    ID_UNINSTALL => {
                        if crate::winutil::confirm(hwnd, "Remove FileFlip from the context menu and delete the installed files?") {
                            let note = install::uninstall();
                            self.refresh();
                            crate::winutil::info(hwnd, &note);
                        }
                    }
                    ID_DOWNLOAD => {
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = std::process::Command::new(exe).arg("download-ffmpeg").spawn();
                        }
                    }
                    ID_BROWSE_FF => {
                        if let Some(p) = crate::winutil::browse_exe(hwnd, "Locate ffmpeg.exe (ffprobe.exe must be next to it)") {
                            if p.with_file_name("ffprobe.exe").is_file() {
                                self.set_ffmpeg_path(&p.display().to_string());
                            } else {
                                crate::winutil::error(hwnd, "ffprobe.exe was not found next to that ffmpeg.exe.");
                            }
                        }
                    }
                    ID_AUTO_FF => self.set_ffmpeg_path(""),
                    ID_GITHUB => crate::winutil::open_url(fileflip_core::REPO_URL),
                    _ => {}
                }
                Some(LRESULT(0))
            }
            WM_CLOSE => {
                close(hwnd);
                Some(LRESULT(0))
            }
            WM_DESTROY => {
                quit();
                Some(LRESULT(0))
            }
            _ => None,
        }
    }
}
