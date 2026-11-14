<p align="center"><img src="assets/fileflip.png" width="96" alt="FileFlip icon"></p>

<h1 align="center">FileFlip</h1>

<p align="center">Right-click any image, video or audio file in Explorer → <b>Convert to</b> → pick a format. Done.</p>

FileFlip is a small, fast file converter for Windows, written in Rust. Like WinRAR, it lives in the
Explorer context menu: it shows formats that fit the files you selected, converts them in the
background, and shows a compact progress window.

## Features

- **Context menu per file type**: images get image formats, videos get video formats (plus "Audio only"), audio gets audio formats. The format your files are already in is hidden.
- **Same quality by default**: clicking a format keeps the original quality as closely as possible:
  - Videos are **remuxed without re-encoding** when the new container supports the streams (e.g. MP4 → MKV takes seconds, zero quality loss).
  - When re-encoding is needed, the bitrate is matched to the source and adjusted for codec efficiency (H.264 → AV1 needs fewer bits).
  - JPEG quality is read from the file's quantization tables and reused; lossless sources become lossless WebP.
  - Audio bitrates are matched the same way; lossless sources get top-quality lossy encodes.
- **More options ▸** nested submenus with `Quality 100 / 75 / 50 / 25 %`, image `Size 75 / 50 / 25 %` and video `1080p / 720p / 480p`.
- **Mixed selections**: select photos, videos and songs together. The menu groups them (`Images (3) ▸`, `Videos (2) ▸`, …), and **Custom…** converts each group to its own format in one go.
- **Custom window**: format, quality, size, frame rate, encoder, audio bitrate and channels, output folder, overwrite rules.
- **Progress window**: one bar for the current file, plus a second bar for the batch, with Pause, Background (low priority) and Cancel, and taskbar progress.
- **Fast by default, polite to your PC**: images convert in-process (SIMD JPEG, libwebp), several files run in parallel, NVIDIA / Intel / AMD GPU encoders are detected and used automatically, and everything runs at below-normal CPU priority so your mouse and apps stay responsive.
- **Per-user install, no admin rights**. Uninstall from *Apps & features*.

### Formats

| Type   | Converts to |
|--------|-------------|
| Images | PNG, JPG, WEBP, AVIF, BMP, GIF, TIFF, ICO (multi-size) |
| Videos | MP4, MKV, WEBM, MOV, AVI, animated GIF, plus audio extraction to any audio format |
| Audio  | MP3, M4A (AAC), WAV, FLAC, OGG (Vorbis), OPUS |

Input: png jpg webp bmp gif tiff ico tga qoi pnm avif heic jxl · mp4 mkv webm avi mov wmv flv mpg ts m2ts 3gp ogv … · mp3 wav flac m4a aac ogg opus wma aiff ape wv ac3 …

## Install

1. Download `FileFlip-x.y.z-windows-x64.zip` from [Releases](../../releases) and extract it anywhere.
2. Run `fileflip.exe` and click **Install**.
   (Or run `fileflip.exe install` from a terminal.)
3. Right-click a file → **Convert to**.

Videos and audio need **FFmpeg**. The first time you convert one, FileFlip offers to download it
(the [gyan.dev](https://www.gyan.dev/ffmpeg/builds/) essentials build, about 100 MB, stored in
`%LOCALAPPDATA%\FileFlip\ffmpeg`). If you already have FFmpeg on your `PATH`, it's used automatically.
Images never need FFmpeg (except AVIF/HEIC).

> **Windows 11:** the classic context-menu extension appears under **Show more options**
> (or Shift + right-click), the same as for most third-party tools.

## Settings

Start **FileFlip** from the Start menu (or the **Settings…** menu item) to:

- install, reinstall or uninstall the context menu, and choose which file types it appears for
- download, locate or auto-detect FFmpeg
- choose the output location (same folder, a subfolder, or a fixed folder) and what happens if the file exists (rename, overwrite, skip)
- optionally move originals to the Recycle Bin after a successful conversion
- toggle GPU acceleration, files converted at once, PNG compression, and auto-closing the progress window

Settings are stored in `%APPDATA%\FileFlip\settings.ini`.

## Command line

```bat
fileflip.exe                                    :: settings window
fileflip.exe install | uninstall
fileflip.exe convert --list files.txt --to webp [--quality 75] [--scale 50] [--height 720] [--only image]
fileflip.exe custom  --list files.txt
fileflip.exe download-ffmpeg
fileflip.exe debug-menu a.png b.mp4 [--invoke "Convert to/More options/JPG/Quality 50%"]
```

`files.txt` holds one path per line.

## Building from source

Requires Rust (stable). Both MSVC and GNU toolchains work; the GNU toolchain also needs MinGW (`gcc`, `windres`).

```bat
cargo build --release
target\release\fileflip.exe register   :: registers the DLL next to the exe, without copying
cargo test --workspace
```

Layout:

- `crates/core`: format tables, the menu model, the exe ↔ DLL protocol and settings (no dependencies)
- `crates/shell`: `fileflip_shell.dll`, the Explorer `IContextMenu` extension
- `crates/app`: `fileflip.exe`: conversion engine (`engine/`) and native Win32 windows (`ui/`)

## License

MIT, see [LICENSE](LICENSE). FFmpeg is downloaded separately and has its own license.
