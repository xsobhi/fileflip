//! Output naming: where converted files go and how name clashes are resolved.

use super::Job;
use fileflip_core::settings::{IfExists, OutputMode};
use fileflip_core::Settings;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
static RESERVED: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();

pub(super) fn reserved() -> &'static Mutex<HashSet<PathBuf>> {
    RESERVED.get_or_init(Default::default)
}

pub fn output_dir(input: &Path, settings: &Settings) -> PathBuf {
    let parent = input.parent().map(Path::to_path_buf).unwrap_or_default();
    match settings.output_mode {
        OutputMode::Same => parent,
        OutputMode::Subfolder => parent.join(&settings.output_subfolder),
        OutputMode::Folder if !settings.output_folder.trim().is_empty() => PathBuf::from(settings.output_folder.trim()),
        OutputMode::Folder => parent,
    }
}

/// Decide the final output path. `Ok(None)` = skip (file exists and policy is Skip).
pub(super) fn choose_output(job: &Job, settings: &Settings) -> Result<Option<PathBuf>, String> {
    let dir = output_dir(&job.input, settings);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let stem = job.input.file_stem().and_then(|s| s.to_str()).unwrap_or("output").to_string();
    let ext = job.target.ext;
    let first = dir.join(format!("{stem}.{ext}"));
    let same_as_input = |p: &Path| p.to_string_lossy().eq_ignore_ascii_case(&job.input.to_string_lossy());
    let mut taken = reserved().lock().unwrap();
    let free = |p: &PathBuf, taken: &HashSet<PathBuf>| !p.exists() && !taken.contains(p);

    if !same_as_input(&first) && !taken.contains(&first) {
        match (first.exists(), settings.if_exists) {
            (false, _) | (true, IfExists::Overwrite) => {
                taken.insert(first.clone());
                return Ok(Some(first));
            }
            (true, IfExists::Skip) => return Ok(None),
            (true, IfExists::Rename) => {}
        }
    }
    for n in 1..10_000 {
        let p = dir.join(format!("{stem} ({n}).{ext}"));
        if free(&p, &taken) {
            taken.insert(p.clone());
            return Ok(Some(p));
        }
    }
    Err("could not find a free output name".into())
}

pub(super) fn temp_path(out: &Path) -> PathBuf {
    let name = out.file_name().and_then(|n| n.to_str()).unwrap_or("out");
    out.with_file_name(format!("{name}.fileflip-part"))
}
