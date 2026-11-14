//! "Output" group shared by the Custom and Settings windows.

use super::*;
use fileflip_core::settings::{IfExists, OutputMode};
use fileflip_core::Settings;

pub const ID_SAME: i32 = 301;
pub const ID_SUB: i32 = 302;
pub const ID_FOLDER: i32 = 303;
pub const ID_BROWSE: i32 = 304;

pub const HEIGHT: i32 = 116;

#[derive(Default)]
pub struct OutputControls {
    same: HWND,
    sub: HWND,
    sub_name: HWND,
    folder: HWND,
    folder_path: HWND,
    browse: HWND,
    exists: HWND,
    recycle: HWND,
}

impl OutputControls {
    /// Create the group at (x, y) with width w.
    pub fn create(parent: HWND, s: &Settings, x: i32, y: i32, w: i32) -> Self {
        group(parent, "Output", x, y, w, HEIGHT);
        let l = x + 12;
        let mut c = OutputControls {
            same: radio(parent, "Same folder as the file", l, y + 20, 170, 20, ID_SAME, true),
            sub: radio(parent, "Subfolder:", l + 180, y + 20, 80, 20, ID_SUB, false),
            ..Default::default()
        };
        c.sub_name = edit(parent, &s.output_subfolder, l + 262, y + 19, w - 286, 22, 0);
        c.folder = radio(parent, "Folder:", l, y + 50, 70, 20, ID_FOLDER, false);
        c.folder_path = edit(parent, &s.output_folder, l + 72, y + 49, w - 134, 22, 0);
        c.browse = button(parent, "…", x + w - 44, y + 48, 32, 24, ID_BROWSE, false);
        label(parent, "If the file exists:", l, y + 84, 100, 18);
        c.exists = combo(
            parent,
            &["Rename the new file", "Overwrite", "Skip"],
            match s.if_exists {
                IfExists::Rename => 0,
                IfExists::Overwrite => 1,
                IfExists::Skip => 2,
            },
            l + 102,
            y + 80,
            150,
            0,
        );
        c.recycle = checkbox(parent, "Move originals to the Recycle Bin", l + 264, y + 82, w - 288, 20, 0, s.delete_source);
        let r = match s.output_mode {
            OutputMode::Same => c.same,
            OutputMode::Subfolder => c.sub,
            OutputMode::Folder => c.folder,
        };
        set_checked(r, true);
        c.sync();
        c
    }

    fn sync(&self) {
        enable(self.sub_name, is_checked(self.sub));
        enable(self.folder_path, is_checked(self.folder));
        enable(self.browse, is_checked(self.folder));
    }

    /// Handle WM_COMMAND for our controls. Returns true if handled.
    pub fn command(&mut self, hwnd: HWND, id: i32) -> bool {
        match id {
            ID_SAME | ID_SUB | ID_FOLDER => {
                self.sync();
                true
            }
            ID_BROWSE => {
                if let Some(p) = crate::winutil::browse_folder(hwnd, "Choose the output folder") {
                    set_text(self.folder_path, &p.display().to_string());
                }
                true
            }
            _ => false,
        }
    }

    pub fn read(&self, s: &mut Settings) -> Result<(), String> {
        s.output_mode = if is_checked(self.sub) {
            OutputMode::Subfolder
        } else if is_checked(self.folder) {
            OutputMode::Folder
        } else {
            OutputMode::Same
        };
        let sub = get_text(self.sub_name).trim().to_string();
        if s.output_mode == OutputMode::Subfolder && (sub.is_empty() || sub.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|'])) {
            return Err("Please enter a valid subfolder name.".into());
        }
        if !sub.is_empty() {
            s.output_subfolder = sub;
        }
        s.output_folder = get_text(self.folder_path).trim().to_string();
        if s.output_mode == OutputMode::Folder && s.output_folder.is_empty() {
            return Err("Please choose an output folder.".into());
        }
        s.if_exists = match combo_sel(self.exists) {
            1 => IfExists::Overwrite,
            2 => IfExists::Skip,
            _ => IfExists::Rename,
        };
        s.delete_source = is_checked(self.recycle);
        Ok(())
    }
}
