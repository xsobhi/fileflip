#![windows_subsystem = "windows"]
//! FileFlip — right-click file converter for Windows.

mod debug;
mod engine;
mod install;
mod ui;
mod winutil;

use engine::ffmpeg::FFmpeg;
use engine::progress::Progress;
use engine::{Job, Runner};
use fileflip_core::request::{self, Command};
use fileflip_core::Settings;
use std::io::Write;
use std::sync::Arc;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
    }
    ui::init();
    let cmd = match request::parse_args(&args) {
        Ok(c) => c,
        Err(e) => {
            winutil::error(HWND::default(), &format!("{e}\n\nUsage: fileflip [settings | install | uninstall | convert --list FILE --to FORMAT]"));
            std::process::exit(2);
        }
    };
    let code = match cmd {
        Command::Convert { list, preset } => {
            let files = request::read_list(&list).unwrap_or_default();
            let settings = Settings::load();
            let jobs = engine::jobs_from_preset(&files, &preset, &settings);
            convert(jobs, settings)
        }
        Command::Custom { list } => {
            let files = request::read_list(&list).unwrap_or_default();
            ui::custom::run(files);
            0
        }
        Command::Settings => {
            ui::settings::run();
            0
        }
        Command::Install { quiet } => match install::install() {
            Ok(dir) => {
                if !quiet {
                    winutil::info(
                        HWND::default(),
                        &format!("FileFlip is installed in\n{}\n\nRight-click any image, video or audio file and choose \"Convert to\".", dir.display()),
                    );
                }
                0
            }
            Err(e) => {
                winutil::error(HWND::default(), &format!("Install failed: {e}"));
                1
            }
        },
        Command::Uninstall { quiet } => {
            let note = install::uninstall();
            if !quiet {
                winutil::info(HWND::default(), &note);
            }
            0
        }
        Command::Register => {
            let dir = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
            match install::register(&dir) {
                Ok(()) => 0,
                Err(e) => {
                    let _ = writeln!(std::io::stderr(), "{e}");
                    1
                }
            }
        }
        Command::DownloadFfmpeg => {
            let progress = Arc::new(Progress::new(engine::progress_items(&[], true)));
            Runner { jobs: Vec::new(), settings: Settings::load(), download_ffmpeg: true }.start(progress.clone());
            ui::progress::run(progress.clone(), false);
            i32::from(progress.snapshot().iter().any(|i| matches!(i.state, engine::progress::State::Failed(_))))
        }
        Command::Unregister => {
            install::unregister();
            0
        }
        Command::DebugMenu { files, invoke } => match debug::run(files, invoke) {
            Ok(s) => {
                let _ = std::io::stdout().write_all(s.as_bytes());
                0
            }
            Err(e) => {
                let _ = writeln!(std::io::stderr(), "error: {e}");
                1
            }
        },
    };
    std::process::exit(code);
}

/// Run jobs with the progress window. Asks to download FFmpeg if it's needed and missing.
pub fn convert(jobs: Vec<Job>, settings: Settings) -> i32 {
    if jobs.is_empty() {
        winutil::info(HWND::default(), "None of the selected files can be converted to that format.");
        return 1;
    }
    let mut download = false;
    if jobs.iter().any(engine::needs_ffmpeg) && FFmpeg::locate(&settings).is_none() {
        download = winutil::confirm(
            HWND::default(),
            "Converting video and audio needs FFmpeg, which isn't installed yet.\n\n\
             Download it now? (about 100 MB, one time, from gyan.dev)\n\n\
             Choose No to convert only the images.",
        );
    }
    // Below-normal priority: full speed when the PC is idle, but the mouse and other apps
    // always win. FFmpeg processes inherit this priority class.
    unsafe {
        use windows::Win32::System::Threading::{GetCurrentProcess, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS};
        let _ = SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS);
    }
    let progress = Arc::new(Progress::new(engine::progress_items(&jobs, download)));
    let close_when_done = settings.close_when_done;
    Runner { jobs, settings, download_ffmpeg: download }.start(progress.clone());
    ui::progress::run(progress.clone(), close_when_done);
    let mut failed = false;
    for item in progress.snapshot() {
        if let engine::progress::State::Failed(e) = &item.state {
            failed = true;
            let _ = writeln!(std::io::stderr(), "{}: {e}", item.name);
        }
    }
    i32::from(failed)
}
