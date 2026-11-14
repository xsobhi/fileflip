//! Creating the Custom window's controls and reading them back into jobs.

use super::options::*;
use super::*;
use crate::engine::{compatible, Options};
use fileflip_core::formats::{ext_matches_format, ext_of, FORMATS};

impl Win {
    pub(super) fn layout(&mut self, hwnd: HWND) {
        let counts: Vec<String> = self
            .files
            .iter()
            .map(|(c, v)| {
                let n = v.len();
                match c {
                    Category::Image => format!("{n} image{}", if n == 1 { "" } else { "s" }),
                    Category::Video => format!("{n} video{}", if n == 1 { "" } else { "s" }),
                    Category::Audio => format!("{n} audio file{}", if n == 1 { "" } else { "s" }),
                }
            })
            .collect();
        let head = label(hwnd, &format!("Selected: {}", counts.join(", ")), 12, 12, W - 24, 20);
        set_bold(head);

        let mixed = self.files.len() > 1;
        let mut y = 40;
        let cats: Vec<Category> = self.files.keys().copied().collect();
        for (gi, cat) in cats.into_iter().enumerate() {
            let exts: Vec<String> = self.files[&cat].iter().map(|p| ext_of(p)).collect();
            let gh = group_height(cat);
            let title = match cat {
                Category::Image => "Images",
                Category::Video => "Videos",
                Category::Audio => "Audio",
            };
            group(hwnd, title, 12, y, W - 24, gh);
            let base = 400 + gi as i32 * 20;
            let mut g = Group::default();
            if mixed {
                g.formats.push(None);
            }
            for f in FORMATS.iter().filter(|f| compatible(cat, f)) {
                g.formats.push(Some(f));
            }
            let labels: Vec<String> = g
                .formats
                .iter()
                .map(|f| match f {
                    None => "Don't convert".to_string(),
                    Some(f) if cat == Category::Video && f.category == Category::Audio => format!("{} (audio only)", f.label),
                    Some(f) => f.label.to_string(),
                })
                .collect();
            let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            // default: first format that isn't what every file already is
            let default = g
                .formats
                .iter()
                .position(|f| f.is_some_and(|f| !exts.iter().all(|e| ext_matches_format(e, f))))
                .unwrap_or(0);
            let r1 = y + 22;
            label(hwnd, "Format:", 24, r1 + 3, 52, 18);
            g.format = combo(hwnd, &label_refs, default, 78, r1, 150, base);
            match cat {
                Category::Image => {
                    label(hwnd, "Quality:", 244, r1 + 3, 50, 18);
                    g.quality = combo(hwnd, &names(QUALITY), 0, 296, r1, 110, base + 1);
                    label(hwnd, "Size:", 418, r1 + 3, 30, 18);
                    g.size = combo(hwnd, &names(SIZES), 0, 450, r1, 96, base + 2);
                }
                Category::Video => {
                    label(hwnd, "Quality:", 244, r1 + 3, 50, 18);
                    g.quality = combo(hwnd, &names(QUALITY), 0, 296, r1, 110, base + 1);
                    label(hwnd, "Size:", 418, r1 + 3, 30, 18);
                    g.height = combo(hwnd, &names(HEIGHTS), 0, 450, r1, 96, base + 2);
                    let r2 = r1 + 30;
                    label(hwnd, "Frames:", 24, r2 + 3, 52, 18);
                    g.fps = combo(hwnd, &names(FPS), 0, 78, r2, 150, base + 3);
                    label(hwnd, "Speed:", 244, r2 + 3, 50, 18);
                    g.encoder = combo(hwnd, ENCODER, if self.settings.hw_accel { 0 } else { 1 }, 296, r2, 160, base + 4);
                    g.no_audio = checkbox(hwnd, "No audio", 466, r2 + 2, 80, 20, base + 5, false);
                }
                Category::Audio => {
                    let aq: Vec<&str> = AUDIO_Q.iter().map(|(n, _, _)| *n).collect();
                    label(hwnd, "Quality:", 244, r1 + 3, 50, 18);
                    g.quality = combo(hwnd, &aq, 0, 296, r1, 110, base + 1);
                    g.channels = combo(hwnd, &names(CHANNELS), 0, 418, r1, 128, base + 2);
                }
            }
            self.groups.insert(cat, g);
            y += gh + 10;
        }
        self.output = OutputControls::create(hwnd, &self.settings, 12, y, W - 24);
        y += output::HEIGHT + 14;
        button(hwnd, "&Settings…", 12, y, 96, 28, ID_SETTINGS, false);
        button(hwnd, "Convert", W - 212, y, 96, 28, IDOK_, true);
        button(hwnd, "Cancel", W - 108, y, 96, 28, IDCANCEL_, false);
    }

    pub(super) fn collect(&self) -> Result<(Vec<Job>, Settings), String> {
        let mut settings = self.settings.clone();
        self.output.read(&mut settings)?;
        let mut jobs = Vec::new();
        for (cat, g) in &self.groups {
            let Some(target) = g.formats.get(combo_sel(g.format)).copied().flatten() else { continue };
            let mut opts = Options { hw: settings.hw_accel, ..Default::default() };
            match cat {
                Category::Image => {
                    opts.quality = QUALITY[combo_sel(g.quality)].1;
                    opts.scale = SIZES[combo_sel(g.size)].1;
                }
                Category::Video => {
                    opts.quality = QUALITY[combo_sel(g.quality)].1;
                    opts.height = HEIGHTS[combo_sel(g.height)].1;
                    opts.fps = FPS[combo_sel(g.fps)].1;
                    opts.hw = combo_sel(g.encoder) == 0;
                    opts.remove_audio = is_checked(g.no_audio);
                }
                Category::Audio => {
                    let (_, q, k) = AUDIO_Q[combo_sel(g.quality)];
                    opts.quality = q;
                    opts.audio_kbps = k;
                    opts.channels = CHANNELS[combo_sel(g.channels)].1;
                }
            }
            for f in &self.files[cat] {
                jobs.push(Job { input: f.clone(), source: *cat, target, opts: opts.clone() });
            }
        }
        if jobs.is_empty() {
            return Err("Choose a format for at least one group of files.".into());
        }
        Ok((jobs, settings))
    }
}
