//! Creating, refreshing and saving the Settings window's controls.

use super::*;
use crate::engine::ffmpeg::FFmpeg;
use fileflip_core::settings::PngCompression;

impl Win {
    pub(super) fn layout(&mut self, hwnd: HWND) {
        let s = self.settings.clone();
        let mut y = 12;
        group(hwnd, "Explorer context menu", 12, y, W - 24, 92);
        self.menu_status = label_wrap(hwnd, "", 24, y + 20, W - 260, 36);
        self.install = button(hwnd, "&Install", W - 228, y + 20, 100, 26, ID_INSTALL, false);
        self.uninstall = button(hwnd, "&Uninstall", W - 120, y + 20, 96, 26, ID_UNINSTALL, false);
        label(hwnd, "Show for:", 24, y + 64, 60, 18);
        self.images = checkbox(hwnd, "Images", 90, y + 62, 80, 20, 0, s.menu_images);
        self.videos = checkbox(hwnd, "Videos", 176, y + 62, 80, 20, 0, s.menu_videos);
        self.audio = checkbox(hwnd, "Audio", 262, y + 62, 80, 20, 0, s.menu_audio);
        y += 92 + 10;

        group(hwnd, "FFmpeg (used for video, audio, AVIF and HEIC)", 12, y, W - 24, 86);
        self.ff_status = label(hwnd, "", 24, y + 22, W - 48, 18);
        self.ff_path = label(hwnd, "", 24, y + 40, W - 48, 18);
        self.download = button(hwnd, "&Download FFmpeg", 24, y + 54, 130, 26, ID_DOWNLOAD, false);
        button(hwnd, "&Browse…", 162, y + 54, 90, 26, ID_BROWSE_FF, false);
        button(hwnd, "Auto-detect", 260, y + 54, 90, 26, ID_AUTO_FF, false);
        y += 86 + 10;

        self.output = OutputControls::create(hwnd, &s, 12, y, W - 24);
        y += output::HEIGHT + 10;

        group(hwnd, "Speed", 12, y, W - 24, 82);
        self.gpu = checkbox(hwnd, "Use the GPU for video when available (much faster)", 24, y + 20, 330, 20, 0, s.hw_accel);
        self.close_done = checkbox(hwnd, "Close the progress window when done", 24, y + 50, 250, 20, 0, s.close_when_done);
        label(hwnd, "Files at once:", 360, y + 23, 80, 18);
        let ji = JOBS.iter().position(|(_, v)| *v == s.max_jobs).unwrap_or(0);
        self.jobs = combo(hwnd, &JOBS.iter().map(|(n, _)| *n).collect::<Vec<_>>(), ji, 444, y + 19, 92, 0);
        label(hwnd, "PNG:", 360, y + 53, 80, 18);
        let pi = match s.png_compression {
            PngCompression::Fast => 0,
            PngCompression::Balanced => 1,
            PngCompression::Best => 2,
        };
        self.png = combo(hwnd, &["Fastest", "Balanced", "Smallest"], pi, 444, y + 49, 92, 0);
        y += 82 + 14;

        button(hwnd, "GitHub page", 12, y, 96, 28, ID_GITHUB, false);
        button(hwnd, "OK", W - 212, y, 96, 28, IDOK_, true);
        button(hwnd, "Cancel", W - 108, y, 96, 28, IDCANCEL_, false);
        self.refresh();
    }

    pub(super) fn refresh(&self) {
        match install::registered_dll() {
            Some(dll) if dll.is_file() => {
                set_text(self.menu_status, &format!("Installed — right-click a file and choose \"Convert to\".\n{}", dll.parent().unwrap_or(&dll).display()));
                set_text(self.install, "Re&install");
                enable(self.uninstall, true);
            }
            _ => {
                set_text(self.menu_status, "Not installed. Install adds \"Convert to\" to the right-click menu\n(for your user only, no admin rights needed).");
                set_text(self.install, "&Install");
                enable(self.uninstall, install::install_dir().exists());
            }
        }
        match FFmpeg::locate(&Settings::load()) {
            Some(ff) => {
                set_text(self.ff_status, "Found ✓");
                set_text(self.ff_path, &ff.ffmpeg.display().to_string());
                set_text(self.download, "Re&download");
            }
            None => {
                set_text(self.ff_status, "Not found — needed only for videos and audio (images work without it).");
                set_text(self.ff_path, "");
                set_text(self.download, "&Download FFmpeg");
            }
        }
    }

    pub(super) fn save(&mut self) -> Result<(), String> {
        // Reload so values changed elsewhere (FFmpeg path, GPU cache) aren't lost.
        let mut s = Settings::load();
        self.output.read(&mut s)?;
        s.menu_images = is_checked(self.images);
        s.menu_videos = is_checked(self.videos);
        s.menu_audio = is_checked(self.audio);
        s.hw_accel = is_checked(self.gpu);
        s.close_when_done = is_checked(self.close_done);
        s.max_jobs = JOBS[combo_sel(self.jobs)].1;
        s.png_compression = match combo_sel(self.png) {
            1 => PngCompression::Balanced,
            2 => PngCompression::Best,
            _ => PngCompression::Fast,
        };
        s.save().map_err(|e| format!("Could not save settings: {e}"))?;
        self.settings = s;
        Ok(())
    }

    pub(super) fn set_ffmpeg_path(&self, path: &str) {
        let mut s = Settings::load();
        s.ffmpeg_path = path.to_string();
        s.hw_cache.clear();
        let _ = s.save();
        self.refresh();
    }
}
