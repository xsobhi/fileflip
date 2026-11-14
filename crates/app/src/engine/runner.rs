//! Running jobs on a pool of worker threads.

use super::convert::{convert_one, Ctx, Outcome};
use super::ffmpeg::{self, Caps, FFmpeg};
use super::progress::{Item, Progress, State};
use super::Job;
use fileflip_core::formats::Category;
use fileflip_core::Settings;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
pub(super) struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    pub(super) fn acquire(&self) {
        let mut c = self.count.lock().unwrap();
        while *c == 0 {
            c = self.cv.wait(c).unwrap();
        }
        *c -= 1;
    }
    pub(super) fn release(&self) {
        *self.count.lock().unwrap() += 1;
        self.cv.notify_one();
    }
}

pub struct Runner {
    pub jobs: Vec<Job>,
    pub settings: Settings,
    /// Download FFmpeg first (job index 0 is the download item).
    pub download_ffmpeg: bool,
}

pub fn progress_items(jobs: &[Job], download: bool) -> Vec<Item> {
    let mut items = Vec::new();
    if download {
        items.push(Item {
            name: "FFmpeg (one-time download)".into(),
            target: "Download".into(),
            weight: jobs.iter().map(weight).sum::<f64>().max(1.0) * 0.3,
            fraction: 0.0,
            state: State::Pending,
            output: None,
            detail: String::new(),
            started: None,
        });
    }
    for j in jobs {
        items.push(Item {
            name: j.input.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            target: j.target.label.to_string(),
            weight: weight(j),
            fraction: 0.0,
            state: State::Pending,
            output: None,
            detail: String::new(),
            started: None,
        });
    }
    items
}

fn weight(j: &Job) -> f64 {
    let size = std::fs::metadata(&j.input).map(|m| m.len()).unwrap_or(1) as f64;
    // Decoding images costs much more per byte than remuxing video.
    let factor = if j.source == Category::Image { 8.0 } else { 1.0 };
    (size * factor).max(1.0)
}

impl Runner {
    /// Start converting on background threads. Progress is reported to `progress`.
    pub fn start(self, progress: Arc<Progress>) {
        std::thread::spawn(move || {
            self.run(&progress);
            progress.finished.store(true, Ordering::SeqCst);
        });
    }

    fn run(self, progress: &Arc<Progress>) {
        let offset = usize::from(self.download_ffmpeg);
        let mut ff = FFmpeg::locate(&self.settings);
        if self.download_ffmpeg {
            progress.set_state(0, State::Running);
            match ffmpeg::download(0, progress) {
                Ok(f) => {
                    ff = Some(f);
                    progress.set_state(0, State::Done);
                }
                Err(e) => progress.set_state(0, State::Failed(e)),
            }
        }
        let caps: Arc<OnceLock<Caps>> = Arc::new(OnceLock::new());
        let jobs = Arc::new(self.jobs);
        let settings = Arc::new(self.settings);
        let next = Arc::new(AtomicUsize::new(0));
        let video_slots = Arc::new(Semaphore { count: Mutex::new(1), cv: Condvar::new() });
        let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        // About one file per physical core (logical/2), at most 6: fast, but leaves room
        // for everything else running on the PC.
        let auto = (cores / 2).clamp(1, 6);
        let workers = if settings.max_jobs > 0 { settings.max_jobs as usize } else { auto }.min(jobs.len()).max(1);

        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let (jobs, settings, next, slots, ff, caps, progress) =
                    (jobs.clone(), settings.clone(), next.clone(), video_slots.clone(), ff.clone(), caps.clone(), progress.clone());
                std::thread::spawn(move || loop {
                    if !progress.wait_if_paused() {
                        break;
                    }
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    let Some(job) = jobs.get(i) else { break };
                    let idx = i + offset;
                    progress.set_state(idx, State::Running);
                    let ctx = Ctx { ff: ff.as_ref(), caps: &caps, settings: &settings, slots: &slots, progress: &progress, idx };
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| convert_one(job, &ctx)))
                        .unwrap_or_else(|_| Err("internal error".into()));
                    let state = match result {
                        Ok(Outcome::Done(out)) => {
                            progress.update(idx, |it| it.output = Some(out.clone()));
                            if settings.delete_source && out != job.input {
                                crate::winutil::recycle(&job.input);
                            }
                            State::Done
                        }
                        Ok(Outcome::Skipped(why)) => State::Skipped(why),
                        Err(e) if progress.is_cancelled() => State::Skipped(if e == "Cancelled" { e } else { "Cancelled".into() }),
                        Err(e) => State::Failed(e),
                    };
                    progress.set_state(idx, state);
                })
            })
            .collect();
        for h in handles {
            let _ = h.join();
        }
        if progress.is_cancelled() {
            for i in 0..progress.snapshot().len() {
                progress.update(i, |it| {
                    if matches!(it.state, State::Pending | State::Running) {
                        it.state = State::Skipped("Cancelled".into());
                    }
                });
            }
        }
    }
}
