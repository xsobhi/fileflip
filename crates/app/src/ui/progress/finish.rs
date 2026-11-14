//! The finished state and the Pause / Background / Cancel buttons.

use super::*;

impl Win {
    pub(super) fn finish(&mut self, hwnd: HWND, items: &[crate::engine::progress::Item]) {
        let ok = items.iter().filter(|i| i.state == State::Done).count();
        let failed: Vec<_> = items.iter().filter_map(|i| if let State::Failed(e) = &i.state { Some((i, e)) } else { None }).collect();
        // "already JPG" skips are expected in mixed selections and not worth stopping for.
        let skipped: Vec<_> = items
            .iter()
            .filter_map(|i| if let State::Skipped(e) = &i.state { Some((i, e)) } else { None })
            .filter(|(_, e)| !e.starts_with("already "))
            .collect();
        self.last_output = items.iter().rev().find_map(|i| i.output.clone());
        let cancelled = self.progress.is_cancelled();

        // FILEFLIP_AUTOCLOSE: never wait for the user (automated tests).
        let unattended = std::env::var_os("FILEFLIP_AUTOCLOSE").is_some();
        if unattended || (failed.is_empty() && skipped.is_empty() && self.close_when_done) || (cancelled && self.close_requested) {
            close(hwnd);
            return;
        }
        if !self.shown {
            self.show(hwnd);
        }
        let headline = if cancelled {
            format!("Cancelled, {ok} of {} done", items.len())
        } else if failed.is_empty() {
            match ok {
                0 => "Nothing was converted".to_string(),
                1 => "Done: 1 file converted".to_string(),
                n => format!("Done: {n} files converted"),
            }
        } else {
            format!("{ok} converted, {} failed", failed.len())
        };
        set_text(self.c.file, &headline);
        set_text(self.c.detail, &self.last_output.as_ref().map(|o| o.display().to_string()).unwrap_or_default());
        set_progress(self.c.bar, 1.0);
        set_text(self.c.pct, "100%");
        if self.multi {
            set_progress(self.c.total_bar, 1.0);
            set_text(self.c.total_pct, "100%");
            set_text(self.c.remaining, "");
            set_text(self.c.total_lbl, &format!("Took {}", fmt_duration(self.elapsed())));
        } else {
            set_text(self.c.elapsed, &format!("Took {}", fmt_duration(self.elapsed())));
        }
        if !failed.is_empty() {
            set_progress_state(self.c.bar, PBST_ERROR);
            if self.multi {
                set_progress_state(self.c.total_bar, PBST_ERROR);
            }
        }
        set_text(hwnd, if failed.is_empty() { "Done - FileFlip" } else { "Errors - FileFlip" });
        if let Some(tb) = &self.taskbar {
            unsafe {
                let _ = tb.SetProgressState(hwnd, if failed.is_empty() { TBPF_NOPROGRESS } else { TBPF_ERROR });
            }
        }
        show(self.c.bg, false);
        set_text(self.c.pause, "&Open folder");
        enable(self.c.pause, self.last_output.is_some());
        set_text(self.c.cancel, "Close");
        enable(self.c.cancel, true);

        if !failed.is_empty() || !skipped.is_empty() {
            let mut log = String::new();
            for (i, e) in &failed {
                log.push_str(&format!("✗ {}: {}\r\n", i.name, e));
            }
            for (i, e) in &skipped {
                log.push_str(&format!("– {} skipped: {}\r\n", i.name, e));
            }
            let base = if self.multi { 170 } else { 104 };
            self.c.log = child(hwnd, "EDIT", &log, (WS_BORDER | WS_VSCROLL).0 | (ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL) as u32, 12, base, W - 24, 110, 0);
            let y = base + 120;
            for (b, x) in [(self.c.pause, PAUSE_X), (self.c.cancel, CANCEL_X)] {
                unsafe {
                    let _ = SetWindowPos(b, None, px(x), px(y), 0, 0, SWP_NOSIZE | SWP_NOZORDER);
                }
            }
            set_client_size(hwnd, W, y + 38);
        }
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        }
    }

    pub(super) fn toggle_pause(&mut self) {
        let now = !self.progress.is_paused();
        self.progress.set_paused(now);
        if now {
            self.paused_at = Some(Instant::now());
        } else if let Some(p) = self.paused_at.take() {
            self.paused_for += p.elapsed();
        }
        set_text(self.c.pause, if now { "&Resume" } else { "&Pause" });
        let st = if now { PBST_PAUSED } else { PBST_NORMAL };
        set_progress_state(self.c.bar, st);
        if self.multi {
            set_progress_state(self.c.total_bar, st);
        }
    }

    pub(super) fn toggle_background(&mut self, hwnd: HWND) {
        self.background = !self.background;
        let class = if self.background { IDLE_PRIORITY_CLASS } else { BELOW_NORMAL_PRIORITY_CLASS };
        unsafe {
            let _ = SetPriorityClass(GetCurrentProcess(), class);
        }
        self.progress.set_child_priority(self.background);
        set_text(self.c.bg, if self.background { "&Foreground" } else { "&Background" });
        if self.background {
            unsafe {
                let _ = ShowWindow(hwnd, SW_MINIMIZE);
            }
        }
    }

    pub(super) fn cancel(&mut self, hwnd: HWND) {
        if self.done {
            close(hwnd);
            return;
        }
        self.progress.cancel();
        set_text(self.c.cancel, "Cancelling…");
        enable(self.c.cancel, false);
        enable(self.c.pause, false);
    }
}
