#!/bin/bash
# Builds a slim, conversion-only FFmpeg for Windows (x64, shared DLLs).
#
# Runs inside BtbN's prebuilt cross-compile image, which already contains every
# codec library (x264, SVT-AV1, dav1d, LAME, Opus, Vorbis, libwebp, GPU SDKs…):
#   docker run --rm -v "$PWD/ffbuild:/ffbuild" -v "$PWD/ffmpeg:/fileflip" \
#       ghcr.io/btbn/ffmpeg-builds/win64-gpl-shared-7.1:latest bash /fileflip/build.sh
# Output: /ffbuild/out (ffmpeg.exe, ffprobe.exe, DLLs, LICENSE.txt)
#
# Everything is disabled, then only what FileFlip uses is enabled. See
# crates/app/src/engine/media/ for the encoders and filters it relies on.
set -xe

FFMPEG_BRANCH="${FFMPEG_BRANCH:-release/7.1}"

join() { local IFS=,; echo "$*"; }

DECODERS=(
  # video
  h264 hevc vp8 vp9 libdav1d av1 mpeg1video mpeg2video mpeg4 msmpeg4v1 msmpeg4v2 msmpeg4v3
  h263 h263p wmv1 wmv2 wmv3 vc1 flv vp6 vp6f vp6a theora mjpeg mjpegb prores dnxhd dvvideo
  rawvideo ffv1 huffyuv ffvhuff utvideo qtrle cinepak svq3 rv30 rv40 indeo5
  # images
  png apng gif webp bmp tiff libjxl targa qoi ppm pgm pbm pam
  # audio
  aac aac_latm mp3 mp3float mp2 mp2float mp1 ac3 eac3 dca truehd mlp flac alac vorbis opus
  wmav1 wmav2 wmapro wmalossless ape wavpack tta amrnb amrwb gsm cook 'pcm_*' 'adpcm_*' 'dsd_*'
  # subtitles (text ones are converted between containers)
  subrip srt ass ssa webvtt mov_text
)

ENCODERS=(
  libx264 h264_nvenc h264_qsv h264_amf av1_nvenc av1_qsv av1_amf libsvtav1 mpeg4
  aac libmp3lame libopus libvorbis flac pcm_s16le pcm_s24le pcm_s32le pcm_f32le
  png mjpeg libwebp bmp gif tiff
  mov_text srt subrip webvtt ass
  wrapped_avframe
)

MUXERS=(mp4 mov matroska webm avi gif mp3 ipod wav flac ogg opus image2 avif ico null)

FILTERS=(
  buffer buffersink abuffer abuffersink format aformat null anull copy
  scale fps split palettegen paletteuse aresample
  transpose hflip vflip crop setpts asetpts trim atrim
  hwdownload hwupload color
)

cd /ffbuild
rm -rf ffmpeg prefix out
git clone --depth 1 --branch "$FFMPEG_BRANCH" https://github.com/FFmpeg/FFmpeg.git ffmpeg
cd ffmpeg

./configure --prefix=/ffbuild/prefix --pkg-config-flags="--static" $FFBUILD_TARGET_FLAGS $FF_CONFIGURE \
    --extra-cflags="$FF_CFLAGS" --extra-cxxflags="$FF_CXXFLAGS" --extra-libs="$FF_LIBS" \
    --extra-ldflags="$FF_LDFLAGS" --extra-ldexeflags="$FF_LDEXEFLAGS" \
    --cc="$CC" --cxx="$CXX" --ar="$AR" --ranlib="$RANLIB" --nm="$NM" \
    --extra-version="fileflip" \
    --disable-everything --disable-doc --disable-network --disable-ffplay --disable-postproc \
    --enable-protocol=file,pipe \
    --enable-demuxers --enable-parsers --enable-bsfs --enable-hwaccels \
    --enable-indev=lavfi \
    --enable-decoder="$(join "${DECODERS[@]}")" \
    --enable-encoder="$(join "${ENCODERS[@]}")" \
    --enable-muxer="$(join "${MUXERS[@]}")" \
    --enable-filter="$(join "${FILTERS[@]}")" \
    || { tail -n 80 ffbuild/config.log; exit 1; }

make -j"$(nproc)"
make install

mkdir -p /ffbuild/out
cp /ffbuild/prefix/bin/*.exe /ffbuild/prefix/bin/*.dll /ffbuild/out/
cp COPYING.GPLv3 /ffbuild/out/LICENSE.txt
cat > /ffbuild/out/README.txt <<EOF
Slim FFmpeg build for FileFlip ($(git describe --tags --always)), GPLv3.
Source: https://github.com/FFmpeg/FFmpeg ($FFMPEG_BRANCH)
Build script: https://github.com/xsobhi/fileflip/blob/main/ffmpeg/build.sh
EOF
ls -la /ffbuild/out
du -sh /ffbuild/out
