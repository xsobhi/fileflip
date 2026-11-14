//! "Custom…" window: per-category format and options for the selected files,
//! plus output options — like WinRAR's "Archive name and parameters".

mod options;
mod view;

use super::output::{self, OutputControls};
use super::*;
use crate::engine::Job;
use fileflip_core::formats::{category_of, Category, Format};
use fileflip_core::Settings;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

const W: i32 = 560;
const ID_SETTINGS: i32 = 120;

/// Controls for one category group.
#[derive(Default)]
struct Group {
    formats: Vec<Option<&'static Format>>,
    format: HWND,
    quality: HWND,
    size: HWND,
    height: HWND,
    fps: HWND,
    encoder: HWND,
    no_audio: HWND,
    channels: HWND,
}

struct Win {
    files: BTreeMap<Category, Vec<PathBuf>>,
    settings: Settings,
    groups: BTreeMap<Category, Group>,
    output: OutputControls,
    result: Rc<RefCell<Option<(Vec<Job>, Settings)>>>,
}

pub fn run(files: Vec<PathBuf>) {
    let mut by_cat: BTreeMap<Category, Vec<PathBuf>> = BTreeMap::new();
    for f in files {
        if let Some(c) = category_of(&f) {
            by_cat.entry(c).or_default().push(f);
        }
    }
    if by_cat.is_empty() {
        crate::winutil::info(HWND::default(), "None of the selected files are images, videos or audio files FileFlip can convert.");
        return;
    }
    let result = Rc::new(RefCell::new(None));
    let total: usize = by_cat.values().map(Vec::len).sum();
    let height = 44 + by_cat.keys().map(|c| group_height(*c) + 10).sum::<i32>() + output::HEIGHT + 56;
    let win = Win { files: by_cat, settings: Settings::load(), groups: BTreeMap::new(), output: OutputControls::default(), result: result.clone() };
    let title = if total == 1 { "Convert 1 file - FileFlip".to_string() } else { format!("Convert {total} files - FileFlip") };
    let hwnd = create_window("FileFlipCustom", &title, W, height, Box::new(win));
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
    run_loop();
    let taken = result.borrow_mut().take();
    if let Some((jobs, settings)) = taken {
        crate::convert(jobs, settings);
    }
}

fn group_height(c: Category) -> i32 {
    if c == Category::Video {
        86
    } else {
        58
    }
}

impl Handler for Win {
    fn handle(&mut self, hwnd: HWND, msg: u32, wp: WPARAM, _lp: LPARAM) -> Option<LRESULT> {
        match msg {
            WM_CREATE => {
                self.layout(hwnd);
                Some(LRESULT(0))
            }
            WM_COMMAND => {
                let (id, _) = command(wp);
                if self.output.command(hwnd, id) {
                    return Some(LRESULT(0));
                }
                match id {
                    IDOK_ => match self.collect() {
                        Ok(r) => {
                            *self.result.borrow_mut() = Some(r);
                            close(hwnd);
                        }
                        Err(e) => crate::winutil::error(hwnd, &e),
                    },
                    IDCANCEL_ => close(hwnd),
                    ID_SETTINGS => {
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = std::process::Command::new(exe).arg("settings").spawn();
                        }
                    }
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
