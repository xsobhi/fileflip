//! Building and refreshing the progress window's controls.

use super::*;

impl Win {
    pub(super) fn layout(&mut self, hwnd: HWND) {
        let c = &mut self.c;
        c.file = label(hwnd, "Preparing…", 12, 12, W - 24, 18);
        set_bold(c.file);
        c.detail = label(hwnd, "", 12, 32, W - 24, 16);
        c.bar = progress_bar(hwnd, 12, 54, W - 24, 16);
        c.elapsed = label(hwnd, "", 12, 75, W - 74, 16);
        c.pct = label_right(hwnd, "", W - 62, 75, 50, 16);
        let mut y = 104;
        if self.multi {
            c.total_lbl = label(hwnd, "", 12, 102, W - 124, 16);
            c.remaining = label_right(hwnd, "", W - 112, 102, 100, 16);
            c.total_bar = progress_bar(hwnd, 12, 122, W - 24, 16);
            c.total_pct = label_right(hwnd, "", W - 62, 142, 50, 16);
            y = 170;
        }
        c.bg = button(hwnd, "&Background", 12, y, BTN, 26, ID_BACKGROUND, false);
        c.pause = button(hwnd, "&Pause", PAUSE_X, y, BTN, 26, ID_PAUSE, false);
        c.cancel = button(hwnd, "Cancel", CANCEL_X, y, BTN, 26, IDCANCEL_, true);
    }

    pub(super) fn elapsed(&self) -> f64 {
        let paused = self.paused_for + self.paused_at.map(|p| p.elapsed()).unwrap_or_default();
        (self.start.elapsed().saturating_sub(paused)).as_secs_f64()
    }

    pub(super) fn refresh(&mut self, hwnd: HWND) {
        let items = self.progress.snapshot();
        let (overall, current) = Progress::overall(&items);
        let paused = self.progress.is_paused();
        let finished = self.progress.finished.load(Ordering::SeqCst);

        if !self.shown && (self.start.elapsed() >= SHOW_DELAY) && !finished {
            self.show(hwnd);
        }
        if finished && !self.done {
            self.done = true;
            self.finish(hwnd, &items);
            return;
        }
        if self.done {
            return;
        }

        // current file
        let cur = current.or_else(|| items.iter().rposition(|i| i.state != State::Pending));
        if let Some(i) = cur {
            let it = &items[i];
            set_text(self.c.file, &format!("{}  →  {}", it.name, it.target));
            let detail = if !it.detail.is_empty() {
                it.detail.clone()
            } else if paused {
                "Paused".into()
            } else {
                "Converting…".into()
            };
            set_text(self.c.detail, &detail);
            set_progress(self.c.bar, it.fraction);
            set_text(self.c.pct, &format!("{:.0}%", it.fraction * 100.0));
            let file_secs = it.started.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0);
            if self.multi {
                set_text(self.c.elapsed, &format!("Elapsed {}", fmt_duration(file_secs)));
            }
        }
        let elapsed = self.elapsed();
        let remaining = if overall > 0.02 { Some(elapsed * (1.0 - overall) / overall) } else { None };
        let rem_text = remaining.map(fmt_duration).unwrap_or_else(|| "--:--".into());
        if self.multi {
            let done = items.iter().filter(|i| !matches!(i.state, State::Pending | State::Running)).count();
            let failed = items.iter().filter(|i| matches!(i.state, State::Failed(_))).count();
            let mut t = format!("{done} of {} files", items.len());
            if failed > 0 {
                t.push_str(&format!(", {failed} failed"));
            }
            set_text(self.c.total_lbl, &t);
            set_text(self.c.remaining, &format!("{rem_text} left"));
            set_progress(self.c.total_bar, overall);
            set_text(self.c.total_pct, &format!("{:.0}%", overall * 100.0));
        } else {
            set_text(self.c.elapsed, &format!("Elapsed {}  ·  {rem_text} left", fmt_duration(elapsed)));
        }
        set_text(hwnd, &format!("{:.0}%{} - FileFlip", overall * 100.0, if paused { " paused" } else { "" }));
        if let Some(tb) = &self.taskbar {
            unsafe {
                let _ = tb.SetProgressState(hwnd, if paused { TBPF_PAUSED } else { TBPF_NORMAL });
                let _ = tb.SetProgressValue(hwnd, (overall * 1000.0) as u64, 1000);
            }
        }
    }

    pub(super) fn show(&mut self, hwnd: HWND) {
        self.shown = true;
        unsafe {
            let _ = ShowWindow(hwnd, if self.background { SW_SHOWMINNOACTIVE } else { SW_SHOW });
            self.taskbar = CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER).ok();
            if let Some(tb) = &self.taskbar {
                let _ = tb.HrInit();
            }
        }
    }
}
