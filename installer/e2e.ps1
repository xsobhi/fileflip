# End-to-end test: runs real conversions through fileflip.exe with the bundled FFmpeg.
# Usage: ./installer/e2e.ps1 -App dist/FileFlip/fileflip.exe
param([Parameter(Mandatory)] [string]$App)
$ErrorActionPreference = "Stop"
$env:FILEFLIP_AUTOCLOSE = "1"

$App = (Resolve-Path $App).Path
$ffdir = Join-Path (Split-Path $App) "ffmpeg"
$ff = Join-Path $ffdir "ffmpeg.exe"
$probe = Join-Path $ffdir "ffprobe.exe"
$work = Join-Path ([IO.Path]::GetTempPath()) "fileflip-e2e"
Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $work | Out-Null

# Test media: 3 s 640x360 H.264/AAC video, a WAV, and a PNG
& $ff -hide_banner -loglevel error -f lavfi -i "testsrc2=s=640x360:r=30:d=3" -f lavfi -i "sine=f=440:d=3" `
    -c:v libx264 -pix_fmt yuv420p -b:v 1500k -c:a aac -b:a 160k -shortest "$work\clip.mp4"
& $ff -hide_banner -loglevel error -f lavfi -i "sine=f=330:d=3" -ac 2 "$work\tone.wav"
& $ff -hide_banner -loglevel error -f lavfi -i "testsrc2=s=320x240" -frames:v 1 "$work\pic.png"

function Convert-With([string]$file, [string[]]$extra) {
    $list = Join-Path $work "list.txt"
    Set-Content -Path $list -Value (Join-Path $work $file) -Encoding utf8
    $argv = @("convert", "--list", $list) + $extra
    $err = Join-Path $work "stderr.txt"
    $p = Start-Process -FilePath $App -ArgumentList $argv -Wait -PassThru -RedirectStandardError $err
    if ($p.ExitCode -ne 0) { throw "fileflip $($extra -join ' ') on $file exited $($p.ExitCode): $(Get-Content $err -Raw)" }
}

function Expect([string]$file, [string]$codecs) {
    $path = Join-Path $work $file
    if (-not (Test-Path $path)) { throw "$file was not created" }
    $got = (& $probe -v error -show_entries stream=codec_name -of csv=p=0 $path) -join ","
    Write-Host ("{0,-14} {1,10:N0} bytes  {2}" -f $file, (Get-Item $path).Length, $got)
    if ($got -ne $codecs) { throw "$file has streams '$got', expected '$codecs'" }
}

Convert-With "clip.mp4" @("--to", "mkv")       ; Expect "clip.mkv" "h264,aac"      # remux, no re-encode
Convert-With "clip.mp4" @("--to", "webm")      ; Expect "clip.webm" "av1,opus"     # SVT-AV1 + Opus
Convert-With "clip.mp4" @("--to", "mp3")       ; Expect "clip.mp3" "mp3"           # audio extraction
Convert-With "clip.mp4" @("--to", "vgif")      ; Expect "clip.gif" "gif"
Convert-With "clip.mp4" @("--to", "mov", "--height", "240") ; Expect "clip.mov" "h264,aac"
Convert-With "tone.wav" @("--to", "flac")      ; Expect "tone.flac" "flac"
Convert-With "tone.wav" @("--to", "opus", "--bitrate", "96") ; Expect "tone.opus" "opus"
Convert-With "pic.png" @("--to", "avif")       ; Expect "pic.avif" "av1"
Write-Host "All end-to-end conversions passed."
