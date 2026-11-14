//! Converting a single job: images in-process, everything else through FFmpeg.

use super::ffmpeg::{Caps, FFmpeg};
use super::output::{choose_output, reserved, temp_path};
use super::progress::Progress;
use super::runner::Semaphore;
use super::{image, media, Job};
use fileflip_core::formats::{ext_matches_format, ext_of, Category};
use fileflip_core::settings::PngCompression;
use fileflip_core::Settings;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
pub(super) struct Ctx<'a> {
    pub(super) ff: Option<&'a FFmpeg>,
    pub(super) caps: &'a OnceLock<Caps>,
    pub(super) settings: &'a Settings,
    pub(super) slots: &'a Semaphore,
    pub(super) progress: &'a Arc<Progress>,
    pub(super) idx: usize,
}

pub(super) enum Outcome {
    Done(PathBuf),
    Skipped(String),
}

pub(super) fn convert_one(job: &Job, ctx: &Ctx) -> Result<Outcome, String> {
    let ext = ext_of(&job.input);
    if job.opts.is_default_quality() && ext_matches_format(&ext, job.target) {
        return Ok(Outcome::Skipped(format!("already {}", job.target.label)));
    }
    if !job.input.is_file() {
        return Err("file not found".into());
    }
    let Some(out) = choose_output(job, ctx.settings)? else {
        return Ok(Outcome::Skipped("output exists".into()));
    };
    let tmp = temp_path(&out);
    let result = convert_to(job, &tmp, ctx);
    let result = result.and_then(|_| {
        if ctx.progress.is_cancelled() {
            return Err("Cancelled".into());
        }
        std::fs::rename(&tmp, &out).map_err(|e| format!("could not write {}: {e}", out.display()))
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
        reserved().lock().unwrap().remove(&out);
    }
    result.map(|_| Outcome::Done(out))
}

fn ffmpeg_or_err<'a>(ctx: &Ctx<'a>) -> Result<&'a FFmpeg, String> {
    ctx.ff.ok_or_else(|| "FFmpeg is needed for this file but was not found (see FileFlip settings)".into())
}

fn caps<'a>(ctx: &Ctx<'a>, ff: &FFmpeg) -> &'a Caps {
    ctx.caps.get_or_init(|| ff.caps(ctx.settings))
}

fn convert_to(job: &Job, tmp: &Path, ctx: &Ctx) -> Result<(), String> {
    let ext = ext_of(&job.input);
    let size = std::fs::metadata(&job.input).map(|m| m.len()).unwrap_or(0);

    if job.source == Category::Image {
        let src = image::source_kind(&job.input, &ext);
        let quality = if job.target.lossy { job.opts.quality.or_else(|| image::default_quality(src, job.target)) } else { None };
        let png = ctx.settings.png_compression;
        if image::native_input(&ext) && image::native_output(job.target) {
            let ij = image::ImageJob { input: &job.input, output: tmp, target: job.target, quality, scale: job.opts.scale, png };
            let idx = ctx.idx;
            return image::convert(&ij, &|f| ctx.progress.set_fraction(idx, f));
        }
        let ff = ffmpeg_or_err(ctx)?;
        let caps = caps(ctx, ff);
        let level = match png {
            PngCompression::Fast => 1,
            PngCompression::Balanced => 5,
            PngCompression::Best => 9,
        };
        let plan = media::plan_image(job.target, &job.opts, job.opts.quality.or(quality), src == image::Source::Lossless, caps, level);
        return run_plan(ff, &job.input, tmp, &plan, None, ctx);
    }

    let ff = ffmpeg_or_err(ctx)?;
    let probe = ff.probe(&job.input)?;
    let duration = probe.duration();
    let caps = caps(ctx, ff);
    match job.target.category {
        Category::Audio => {
            let plan = media::plan_audio(job.target, &job.opts, &probe, job.source == Category::Audio)?;
            let r = run_plan(ff, &job.input, tmp, &plan, duration, ctx);
            if r.is_err() && plan.with_cover && !ctx.progress.is_cancelled() {
                let plan = media::plan_audio(job.target, &job.opts, &probe, false)?;
                return run_plan(ff, &job.input, tmp, &plan, duration, ctx);
            }
            r
        }
        _ => {
            let plan = media::plan_video(job.target, &job.opts, &probe, caps, size)?;
            if plan.heavy {
                ctx.progress.set_detail(ctx.idx, "Waiting for encoder…");
                ctx.slots.acquire();
                ctx.progress.set_detail(ctx.idx, "");
                let r = run_plan(ff, &job.input, tmp, &plan, duration, ctx);
                ctx.slots.release();
                // A GPU encoder can fail on unusual input (e.g. 4:4:4, huge sizes): retry on the CPU.
                if r.is_err() && job.opts.hw && !ctx.progress.is_cancelled() {
                    let mut opts = job.opts.clone();
                    opts.hw = false;
                    let plan = media::plan_video(job.target, &opts, &probe, caps, size)?;
                    ctx.slots.acquire();
                    let r2 = run_plan(ff, &job.input, tmp, &plan, duration, ctx);
                    ctx.slots.release();
                    return r2;
                }
                r
            } else {
                run_plan(ff, &job.input, tmp, &plan, duration, ctx)
            }
        }
    }
}

fn run_plan(ff: &FFmpeg, input: &Path, out: &Path, plan: &media::Plan, duration: Option<f64>, ctx: &Ctx) -> Result<(), String> {
    let mut args: Vec<String> = plan.pre_input.clone();
    args.push("-i".into());
    args.push(input.display().to_string());
    args.extend(plan.args.iter().cloned());
    args.extend(["-f".into(), plan.muxer.into(), out.display().to_string()]);
    ff.run(&args, duration, ctx.idx, ctx.progress)
}
