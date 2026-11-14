use super::{plan_audio, plan_video};
use crate::engine::ffmpeg::{Caps, Probe};
use crate::engine::Options;
use fileflip_core::formats::format_by_id;

fn probe(json: &str) -> Probe {
    serde_json::from_str(json).unwrap()
}

const H264_AAC: &str = r#"{"streams":[
    {"index":0,"codec_type":"video","codec_name":"h264","width":1920,"height":1080,"bit_rate":"8000000","avg_frame_rate":"30/1"},
    {"index":1,"codec_type":"audio","codec_name":"aac","bit_rate":"160000","channels":2}],
    "format":{"duration":"10.0","bit_rate":"8200000"}}"#;

#[test]
fn mp4_to_mkv_is_a_copy() {
    let p = plan_video(format_by_id("mkv").unwrap(), &Options::default(), &probe(H264_AAC), &Caps::default(), 0).unwrap();
    let a = p.args.join(" ");
    assert!(a.contains("-c:v copy") && a.contains("-c:a:0 copy"), "{a}");
    assert!(!p.heavy);
}

#[test]
fn webm_reencodes_with_matched_bitrate() {
    let caps = Caps { encoders: ["libsvtav1".to_string()].into(), ..Default::default() };
    let p = plan_video(format_by_id("webm").unwrap(), &Options::default(), &probe(H264_AAC), &caps, 0).unwrap();
    let a = p.args.join(" ");
    // 8 Mbit/s H.264 -> AV1 at half the bits
    assert!(a.contains("libsvtav1") && a.contains("-b:v 4000k"), "{a}");
    assert!(a.contains("-c:a:0 libopus"), "{a}");
    assert!(p.heavy);
}

#[test]
fn quality_and_height_scale_bitrate() {
    let opts = Options { quality: Some(50), height: Some(720), ..Default::default() };
    let p = plan_video(format_by_id("mp4").unwrap(), &opts, &probe(H264_AAC), &Caps::default(), 0).unwrap();
    let a = p.args.join(" ");
    // 8000k * (720/1080)^2 * 0.5 = 1778k
    assert!(a.contains("scale=-2:720") && a.contains("-b:v 1778k"), "{a}");
}

#[test]
fn audio_bitrates() {
    let pr = probe(H264_AAC);
    let p = plan_audio(format_by_id("mp3").unwrap(), &Options::default(), &pr, true).unwrap();
    // 160k AAC ~ 200k MP3 -> nearest standard rate 192k
    assert!(p.args.join(" ").contains("-b:a:0 192k"), "{:?}", p.args);
    let p = plan_audio(format_by_id("m4a").unwrap(), &Options::default(), &pr, true).unwrap();
    assert!(p.args.join(" ").contains("-c:a copy"));
    let flac = probe(r#"{"streams":[{"index":0,"codec_type":"audio","codec_name":"flac","channels":2,"bits_per_raw_sample":"24"}],"format":{}}"#);
    let p = plan_audio(format_by_id("wav").unwrap(), &Options::default(), &flac, true).unwrap();
    assert!(p.args.join(" ").contains("pcm_s24le"));
    let p = plan_audio(format_by_id("opus").unwrap(), &Options { quality: Some(50), ..Default::default() }, &flac, true).unwrap();
    assert!(p.args.join(" ").contains("-b:a:0 128k"), "{:?}", p.args);
}
