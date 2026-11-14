//! Shared progress state between the worker threads and the progress window,
//! plus pause/cancel control of running FFmpeg processes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_SUSPEND_RESUME, PROCESS_TERMINATE};
use windows::core::{s, w};

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Pending,
    Running,
    Done,
    Skipped(String),
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct Item {
    /// Display name (file name).
    pub name: String,
    /// Target label, e.g. "MP4".
    pub target: String,
    /// Relative cost used for the overall bar (input size).
    pub weight: f64,
    pub fraction: f64,
    pub state: State,
    pub output: Option<PathBuf>,
    /// Extra status text ("Extracting…").
    pub detail: String,
    pub started: Option<std::time::Instant>,
}

#[derive(Default)]
pub struct Progress {
    pub items: Mutex<Vec<Item>>,
    cancelled: AtomicBool,
    paused: AtomicBool,
    pub finished: AtomicBool,
    children: Mutex<HashMap<usize, u32>>,
}

impl Progress {
    pub fn new(items: Vec<Item>) -> Self {
        Progress { items: Mutex::new(items), ..Default::default() }
    }

    pub fn snapshot(&self) -> Vec<Item> {
        self.items.lock().unwrap().clone()
    }

    pub fn update(&self, job: usize, f: impl FnOnce(&mut Item)) {
        if let Some(it) = self.items.lock().unwrap().get_mut(job) {
            f(it)
        }
    }

    pub fn set_fraction(&self, job: usize, v: f64) {
        self.update(job, |it| it.fraction = v);
    }

    pub fn set_detail(&self, job: usize, d: &str) {
        self.update(job, |it| it.detail = d.to_string());
    }

    pub fn set_state(&self, job: usize, s: State) {
        self.update(job, |it| {
            if s == State::Running {
                it.started = Some(std::time::Instant::now());
            }
            if matches!(s, State::Done | State::Skipped(_) | State::Failed(_)) {
                it.fraction = 1.0;
            }
            it.state = s;
        });
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn register_child(&self, job: usize, pid: u32) {
        self.children.lock().unwrap().insert(job, pid);
        if self.is_paused() {
            suspend(pid, true);
        }
        if self.is_cancelled() {
            kill(pid);
        }
    }

    pub fn unregister_child(&self, job: usize) {
        self.children.lock().unwrap().remove(&job);
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        let pids: Vec<u32> = self.children.lock().unwrap().values().copied().collect();
        for pid in pids {
            if self.is_paused() {
                suspend(pid, false);
            }
            kill(pid);
        }
        self.paused.store(false, Ordering::SeqCst);
    }

    pub fn set_paused(&self, paused: bool) {
        if self.paused.swap(paused, Ordering::SeqCst) == paused {
            return;
        }
        for pid in self.children.lock().unwrap().values() {
            suspend(*pid, paused);
        }
    }

    /// Lower (or restore) the CPU priority of running FFmpeg processes.
    pub fn set_child_priority(&self, background: bool) {
        use windows::Win32::System::Threading::{SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS, IDLE_PRIORITY_CLASS, PROCESS_SET_INFORMATION};
        let class = if background { IDLE_PRIORITY_CLASS } else { BELOW_NORMAL_PRIORITY_CLASS };
        for pid in self.children.lock().unwrap().values() {
            unsafe {
                if let Ok(h) = OpenProcess(PROCESS_SET_INFORMATION, false, *pid) {
                    let _ = SetPriorityClass(h, class);
                    let _ = CloseHandle(h);
                }
            }
        }
    }

    /// Block a worker while paused. Returns false if cancelled meanwhile.
    pub fn wait_if_paused(&self) -> bool {
        while self.is_paused() && !self.is_cancelled() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        !self.is_cancelled()
    }

    /// (overall 0..1, index of the "current" item)
    pub fn overall(items: &[Item]) -> (f64, Option<usize>) {
        let total: f64 = items.iter().map(|i| i.weight).sum();
        let done: f64 = items.iter().map(|i| i.weight * i.fraction).sum();
        let current = items.iter().position(|i| i.state == State::Running);
        (if total > 0.0 { done / total } else { 0.0 }, current)
    }
}

fn kill(pid: u32) {
    unsafe {
        if let Ok(h) = OpenProcess(PROCESS_TERMINATE, false, pid) {
            let _ = TerminateProcess(h, 1);
            let _ = CloseHandle(h);
        }
    }
}

/// Suspend or resume a whole process through ntdll's NtSuspendProcess/NtResumeProcess.
fn suspend(pid: u32, on: bool) {
    type NtFn = unsafe extern "system" fn(HANDLE) -> i32;
    unsafe {
        let Ok(ntdll) = GetModuleHandleW(w!("ntdll.dll")) else { return };
        let name = if on { s!("NtSuspendProcess") } else { s!("NtResumeProcess") };
        let Some(f) = GetProcAddress(ntdll, name) else { return };
        let f: NtFn = std::mem::transmute(f);
        if let Ok(h) = OpenProcess(PROCESS_SUSPEND_RESUME, false, pid) {
            f(h);
            let _ = CloseHandle(h);
        }
    }
}
