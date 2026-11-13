//! The three jobs (compress, extract audio, grab frames) as plain ffmpeg argument lists.
//! Everything that builds arguments is a pure function, so it is unit-tested without ffmpeg.

use serde_json::Value;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub const VIDEO_EXTS: &[&str] = &["mp4", "mov", "m4v", "avi", "mkv", "webm"];

// ---------------------------------------------------------------- options

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Codec {
    H265,
    H264,
}

impl Codec {
    pub fn lib(self) -> &'static str {
        match self {
            Codec::H265 => "libx265",
            Codec::H264 => "libx264",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Codec::H265 => "H.265 (smaller files)",
            Codec::H264 => "H.264 (plays everywhere)",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CompressOpts {
    pub crf: u32,
    pub preset: String,
    pub codec: Codec,
    pub audio_kbps: u32,
    pub max_width: Option<u32>,
    pub suffix: String,
    /// Replace the original, but only when the result is smaller.
    pub in_place: bool,
    pub overwrite: bool,
}

impl Default for CompressOpts {
    fn default() -> Self {
        Self { crf: 28, preset: "medium".into(), codec: Codec::H265, audio_kbps: 128, max_width: None, suffix: "_compressed".into(), in_place: false, overwrite: false }
    }
}

pub const PRESETS: &[&str] = &["ultrafast", "superfast", "veryfast", "faster", "fast", "medium", "slow", "slower", "veryslow"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AudioFormat {
    Mp3,
    M4a,
    Aac,
    Opus,
    Wav,
}

impl AudioFormat {
    pub const ALL: [AudioFormat; 5] = [AudioFormat::Mp3, AudioFormat::M4a, AudioFormat::Aac, AudioFormat::Opus, AudioFormat::Wav];
    pub fn ext(self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::M4a => "m4a",
            AudioFormat::Aac => "aac",
            AudioFormat::Opus => "opus",
            AudioFormat::Wav => "wav",
        }
    }
    pub fn encoder(self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "libmp3lame",
            AudioFormat::M4a | AudioFormat::Aac => "aac",
            AudioFormat::Opus => "libopus",
            AudioFormat::Wav => "pcm_s16le",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.ext() == s.to_ascii_lowercase())
    }
}

#[derive(Clone, Debug)]
pub struct AudioOpts {
    pub format: AudioFormat,
    pub kbps: u32,
    pub mono: bool,
    pub overwrite: bool,
}

impl Default for AudioOpts {
    fn default() -> Self {
        Self { format: AudioFormat::Mp3, kbps: 96, mono: false, overwrite: false }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImageFormat {
    Jpg,
    Png,
}

impl ImageFormat {
    pub fn ext(self) -> &'static str {
        match self {
            ImageFormat::Jpg => "jpg",
            ImageFormat::Png => "png",
        }
    }
}

#[derive(Clone, Debug)]
pub struct FramesOpts {
    /// Seconds between frames.
    pub interval: f64,
    pub max_width: u32,
    /// JPEG quality 1 (best) to 31 (worst), ffmpeg's -q:v scale.
    pub quality: u32,
    pub format: ImageFormat,
    /// Start this many seconds in.
    pub start: f64,
    /// Drop frames that look the same as the one before (slides, still shots).
    pub dedupe: bool,
    pub overwrite: bool,
}

impl Default for FramesOpts {
    fn default() -> Self {
        Self { interval: 10.0, max_width: 1920, quality: 5, format: ImageFormat::Jpg, start: 0.0, dedupe: false, overwrite: false }
    }
}

#[derive(Clone, Debug)]
pub enum Job {
    Compress(CompressOpts),
    Audio(AudioOpts),
    Frames(FramesOpts),
}

impl Job {
    pub fn name(&self) -> &'static str {
        match self {
            Job::Compress(_) => "Compress",
            Job::Audio(_) => "Extract audio",
            Job::Frames(_) => "Extract frames",
        }
    }
}

// ---------------------------------------------------------------- small helpers

pub fn human_size(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", units[i])
    }
}

pub fn is_video(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).map(|e| VIDEO_EXTS.contains(&e.to_ascii_lowercase().as_str())).unwrap_or(false)
}

/// Files and folders in, a sorted list of video files out (folders are searched recursively).
/// Files this program made itself (`*_frames` folders) are skipped so a re-run does not eat its own output.
pub fn collect_inputs(paths: &[PathBuf]) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>, depth: u32) {
        if depth > 12 {
            return;
        }
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(&p, out, depth + 1);
            } else if is_video(&p) {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    for p in paths {
        if p.is_dir() {
            walk(p, &mut out, 0);
        } else if p.is_file() && is_video(p) {
            out.push(p.clone());
        }
    }
    out.sort();
    out.dedup();
    out
}

/// "01h02m03s" style label; adds milliseconds when the interval is not a whole number of seconds.
pub fn time_label(secs: f64, with_ms: bool) -> String {
    let total_ms = (secs * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let s = total_ms / 1000;
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    let mut out = if h > 0 { format!("t{h}h{m:02}m{sec:02}s") } else { format!("t{m:02}m{sec:02}s") };
    if with_ms {
        out.push_str(&format!("{ms:03}ms"));
    }
    out
}

// ---------------------------------------------------------------- ffmpeg discovery and probing

#[derive(Clone, Debug)]
pub struct Tools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    /// ffmpeg 5.1 renamed -vsync to -fps_mode; older builds (Ubuntu 22.04 has 4.4) only know -vsync.
    pub fps_flag: &'static str,
}

/// Reads "ffmpeg version 6.1.1-3ubuntu5 ..." as (6, 1). Git builds ("N-12345-g...") count as new.
pub fn parse_version(text: &str) -> Option<(u32, u32)> {
    let rest = text.lines().next()?.split("version").nth(1)?.trim();
    let first = rest.split_whitespace().next()?;
    if first.starts_with('N') || first.starts_with('n') && first.contains("-g") {
        return Some((99, 0));
    }
    let digits: String = first.trim_start_matches('n').chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    let mut it = digits.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next().and_then(|m| m.parse().ok()).unwrap_or(0);
    Some((major, minor))
}

fn fps_flag_for(ffmpeg: &Path) -> &'static str {
    let out = command(ffmpeg).arg("-version").stdin(Stdio::null()).output().ok();
    let text = out.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    match parse_version(&text) {
        Some((major, minor)) if major < 5 || (major == 5 && minor < 1) => "-vsync",
        _ => "-fps_mode",
    }
}

fn command(p: &Path) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(p);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flash from the GUI
    }
    c
}

fn works(p: &Path) -> bool {
    command(p).arg("-version").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

/// Looks next to this program first (a portable ffmpeg you dropped in), then on PATH.
pub fn find_tools() -> Result<Tools, String> {
    let exe_name = |n: &str| if cfg!(windows) { format!("{n}.exe") } else { n.to_string() };
    let mut candidates: Vec<(PathBuf, PathBuf)> = Vec::new();
    if let Ok(me) = std::env::current_exe() {
        if let Some(dir) = me.parent() {
            candidates.push((dir.join(exe_name("ffmpeg")), dir.join(exe_name("ffprobe"))));
            candidates.push((dir.join("ffmpeg").join("bin").join(exe_name("ffmpeg")), dir.join("ffmpeg").join("bin").join(exe_name("ffprobe"))));
        }
    }
    candidates.push((PathBuf::from("ffmpeg"), PathBuf::from("ffprobe")));
    for (ff, fp) in candidates {
        if works(&ff) {
            let probe = if works(&fp) { fp } else { PathBuf::from("ffprobe") };
            let fps_flag = fps_flag_for(&ff);
            return Ok(Tools { ffmpeg: ff, ffprobe: probe, fps_flag });
        }
    }
    Err(if cfg!(windows) {
        "ffmpeg was not found. Install it (winget install Gyan.FFmpeg) or put ffmpeg.exe and ffprobe.exe next to this program, then restart.".into()
    } else {
        "ffmpeg was not found. Install it (for example: sudo apt install ffmpeg) and restart.".into()
    })
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Probe {
    pub duration: Option<f64>,
    pub has_video: bool,
    pub has_audio: bool,
}

pub fn parse_probe(json: &str) -> Probe {
    let v: Value = serde_json::from_str(json).unwrap_or(Value::Null);
    let mut p = Probe::default();
    p.duration = v["format"]["duration"].as_str().and_then(|s| s.parse::<f64>().ok()).filter(|d| d.is_finite() && *d > 0.0);
    if let Some(streams) = v["streams"].as_array() {
        for s in streams {
            match s["codec_type"].as_str() {
                Some("video") => {
                    // cover art in an audio file shows up as a video stream; ignore it
                    if s["disposition"]["attached_pic"].as_i64().unwrap_or(0) == 0 {
                        p.has_video = true;
                    }
                }
                Some("audio") => p.has_audio = true,
                _ => {}
            }
        }
    }
    p
}

pub fn probe(tools: &Tools, input: &Path) -> Probe {
    let out = command(&tools.ffprobe)
        .args(["-v", "error", "-show_entries", "format=duration:stream=codec_type:stream_disposition=attached_pic", "-of", "json"])
        .arg(input)
        .stdin(Stdio::null())
        .output();
    match out {
        Ok(o) => parse_probe(&String::from_utf8_lossy(&o.stdout)),
        Err(_) => Probe::default(),
    }
}

pub fn has_encoder(tools: &Tools, name: &str) -> bool {
    command(&tools.ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .stdin(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().any(|l| l.split_whitespace().nth(1) == Some(name)))
        .unwrap_or(false)
}

// ---------------------------------------------------------------- argument builders

fn s(x: &str) -> String {
    x.to_string()
}

fn common(overwrite: bool) -> Vec<String> {
    vec![s("-nostdin"), s("-hide_banner"), s("-loglevel"), s("error"), s("-progress"), s("pipe:1"), s("-nostats"), s(if overwrite { "-y" } else { "-n" })]
}

pub fn compress_args(input: &Path, output: &Path, o: &CompressOpts, has_audio: bool) -> Vec<String> {
    let mut a = common(true); // the caller decides about overwriting; the temp file is always ours
    a.extend([s("-i"), input.to_string_lossy().into_owned(), s("-map"), s("0:v:0")]);
    if has_audio {
        a.extend([s("-map"), s("0:a:0")]);
    }
    a.extend([s("-c:v"), o.codec.lib().into(), s("-crf"), o.crf.to_string(), s("-preset"), o.preset.clone(), s("-pix_fmt"), s("yuv420p")]);
    if o.codec == Codec::H265 {
        // Without this tag QuickTime and iOS refuse to play HEVC in an MP4.
        a.extend([s("-tag:v"), s("hvc1")]);
    }
    if let Some(w) = o.max_width {
        a.extend([s("-vf"), format!("scale='min({w},iw)':-2")]);
    }
    if has_audio {
        a.extend([s("-c:a"), s("aac"), s("-b:a"), format!("{}k", o.audio_kbps)]);
    }
    a.extend([s("-movflags"), s("+faststart"), s("-sn"), s("-dn"), output.to_string_lossy().into_owned()]);
    a
}

pub fn audio_args(input: &Path, output: &Path, o: &AudioOpts) -> Vec<String> {
    let mut a = common(true);
    a.extend([s("-i"), input.to_string_lossy().into_owned(), s("-vn"), s("-map"), s("0:a:0"), s("-c:a"), o.format.encoder().into()]);
    if o.format != AudioFormat::Wav {
        a.extend([s("-b:a"), format!("{}k", o.kbps)]);
    }
    if o.mono {
        a.extend([s("-ac"), s("1")]);
    }
    if o.format == AudioFormat::Aac {
        a.extend([s("-f"), s("adts")]);
    }
    a.push(output.to_string_lossy().into_owned());
    a
}

/// `pattern` is the output file pattern, for example `/out/clip_frames/frame_%08d.jpg`.
pub fn frames_args(input: &Path, pattern: &Path, o: &FramesOpts, fps_flag: &str) -> Vec<String> {
    let mut a = common(true);
    if o.start > 0.0 {
        a.extend([s("-ss"), format!("{}", o.start)]);
    }
    a.extend([s("-i"), input.to_string_lossy().into_owned(), s("-an"), s("-sn")]);
    // Pick the first frame at or after every multiple of the interval. (The simpler fps=1/N filter
    // hands back the frame nearest the middle of each period, so "10 s" was really about 5 s, and
    // the file names were wrong.) Each frame is numbered by its own timestamp in milliseconds.
    let mut vf = format!("select='gte(t+0.001,selected_n*{})'", fmt_num(o.interval));
    if o.dedupe {
        vf.push_str(",mpdecimate");
    }
    vf.push_str(&format!(",scale='min({},iw)':-2", o.max_width));
    a.extend([s("-vf"), vf, s(fps_flag), s("passthrough"), s("-enc_time_base"), s("1/1000"), s("-frame_pts"), s("1")]);
    if o.format == ImageFormat::Jpg {
        a.extend([s("-q:v"), o.quality.to_string()]);
    }
    a.push(pattern.to_string_lossy().into_owned());
    a
}

fn fmt_num(x: f64) -> String {
    if (x - x.round()).abs() < 1e-9 {
        format!("{}", x.round() as i64)
    } else {
        format!("{x}")
    }
}

// ---------------------------------------------------------------- running ffmpeg

pub type Cancel = Arc<AtomicBool>;

/// Runs ffmpeg, reports progress as 0.0 to 1.0, and kills it when `cancel` is set.
pub fn run_ffmpeg(tools: &Tools, args: &[String], duration: Option<f64>, cancel: &Cancel, on_progress: &mut dyn FnMut(f32)) -> Result<(), String> {
    let mut child = command(&tools.ffmpeg)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start ffmpeg: {e}"))?;
    let stderr_text = Arc::new(Mutex::new(String::new()));
    let err_thread = child.stderr.take().map(|mut se| {
        let buf = stderr_text.clone();
        std::thread::spawn(move || {
            let mut t = String::new();
            let _ = se.read_to_string(&mut t);
            *buf.lock().unwrap() = t;
        })
    });
    // Kill promptly even while ffmpeg is quiet: a watcher thread owns the cancel check.
    let stdout = child.stdout.take().unwrap();
    let done = Arc::new(AtomicBool::new(false));
    let child = Arc::new(Mutex::new(child));
    let watcher = {
        let (cancel, done, child) = (cancel.clone(), done.clone(), child.clone());
        std::thread::spawn(move || {
            while !done.load(Ordering::Relaxed) {
                if cancel.load(Ordering::Relaxed) {
                    let _ = child.lock().unwrap().kill();
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        })
    };
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if let Some(us) = line.strip_prefix("out_time_us=").or_else(|| line.strip_prefix("out_time_ms=")) {
            if let (Ok(us), Some(d)) = (us.trim().parse::<f64>(), duration) {
                on_progress(((us / 1e6) / d).clamp(0.0, 1.0) as f32);
            }
        }
    }
    done.store(true, Ordering::Relaxed);
    let _ = watcher.join();
    let status = child.lock().unwrap().wait().map_err(|e| e.to_string())?;
    if let Some(t) = err_thread {
        let _ = t.join();
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("Cancelled".into());
    }
    if !status.success() {
        let text = stderr_text.lock().unwrap().clone();
        let tail: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let tail = tail[tail.len().saturating_sub(3)..].join(" | ");
        return Err(if tail.is_empty() { format!("ffmpeg stopped with {status}") } else { tail });
    }
    Ok(())
}

// ---------------------------------------------------------------- one file, one job

pub struct Outcome {
    pub message: String,
}

fn stem(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "video".into())
}

fn out_dir_for(input: &Path, out_dir: &Option<PathBuf>) -> PathBuf {
    match out_dir {
        Some(d) => d.clone(),
        None => input.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from(".")),
    }
}

pub fn process_file(tools: &Tools, job: &Job, input: &Path, out_dir: &Option<PathBuf>, cancel: &Cancel, on_progress: &mut dyn FnMut(f32)) -> Result<Outcome, String> {
    let pr = probe(tools, input);
    if !pr.has_video && !matches!(job, Job::Audio(_)) {
        return Err("no video stream found (is this a video file?)".into());
    }
    let dir = out_dir_for(input, out_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    match job {
        Job::Compress(o) => compress_one(tools, o, input, &dir, &pr, cancel, on_progress),
        Job::Audio(o) => audio_one(tools, o, input, &dir, &pr, cancel, on_progress),
        Job::Frames(o) => frames_one(tools, o, input, &dir, &pr, cancel, on_progress),
    }
}

fn compress_one(tools: &Tools, o: &CompressOpts, input: &Path, dir: &Path, pr: &Probe, cancel: &Cancel, on_progress: &mut dyn FnMut(f32)) -> Result<Outcome, String> {
    let name = stem(input);
    let final_path = if o.in_place { dir.join(format!("{name}.mp4")) } else { dir.join(format!("{name}{}.mp4", o.suffix)) };
    if !o.in_place && final_path.exists() && !o.overwrite {
        return Ok(Outcome { message: format!("skipped, {} already exists", final_path.file_name().unwrap().to_string_lossy()) });
    }
    if final_path == input && !o.in_place {
        return Err("output would overwrite the input; choose a different suffix".into());
    }
    let tmp = dir.join(format!("{name}.vet-tmp.mp4"));
    let _ = std::fs::remove_file(&tmp);
    let before = std::fs::metadata(input).map(|m| m.len()).unwrap_or(0);
    let args = compress_args(input, &tmp, o, pr.has_audio);
    let res = run_ffmpeg(tools, &args, pr.duration, cancel, on_progress);
    if let Err(e) = res {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    let after = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
    if after == 0 {
        let _ = std::fs::remove_file(&tmp);
        return Err("ffmpeg produced an empty file".into());
    }
    if o.in_place {
        if after >= before {
            let _ = std::fs::remove_file(&tmp);
            return Ok(Outcome { message: format!("kept the original ({} would not have been smaller)", human_size(after)) });
        }
        // Verify the new file opens before the original goes.
        if probe(tools, &tmp).duration.is_none() {
            let _ = std::fs::remove_file(&tmp);
            return Err("the new file did not verify; original kept".into());
        }
        let same_file = final_path == input;
        if !same_file {
            std::fs::remove_file(input).map_err(|e| format!("could not remove the original: {e}"))?;
        }
        std::fs::rename(&tmp, &final_path).or_else(|_| {
            std::fs::copy(&tmp, &final_path).map(|_| ()).and_then(|_| std::fs::remove_file(&tmp))
        }).map_err(|e| format!("could not place the new file: {e}"))?;
    } else {
        if final_path.exists() {
            let _ = std::fs::remove_file(&final_path);
        }
        std::fs::rename(&tmp, &final_path).map_err(|e| format!("could not place the new file: {e}"))?;
    }
    let pct = if before > 0 { 100.0 - (after as f64 * 100.0 / before as f64) } else { 0.0 };
    Ok(Outcome { message: format!("{} to {} ({pct:.0}% smaller){}", human_size(before), human_size(after), if o.in_place { ", replaced the original" } else { "" }) })
}

fn audio_one(tools: &Tools, o: &AudioOpts, input: &Path, dir: &Path, pr: &Probe, cancel: &Cancel, on_progress: &mut dyn FnMut(f32)) -> Result<Outcome, String> {
    if !pr.has_audio {
        return Err("this file has no audio track".into());
    }
    let out = dir.join(format!("{}.{}", stem(input), o.format.ext()));
    if out.exists() && !o.overwrite {
        return Ok(Outcome { message: format!("skipped, {} already exists", out.file_name().unwrap().to_string_lossy()) });
    }
    if out == input {
        return Err("output would overwrite the input".into());
    }
    if !has_encoder(tools, o.format.encoder()) {
        return Err(format!("this ffmpeg has no {} encoder; pick another format", o.format.encoder()));
    }
    let tmp = dir.join(format!("{}.vet-tmp.{}", stem(input), o.format.ext()));
    let _ = std::fs::remove_file(&tmp);
    let args = audio_args(input, &tmp, o);
    if let Err(e) = run_ffmpeg(tools, &args, pr.duration, cancel, on_progress) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    let size = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
    if size == 0 {
        let _ = std::fs::remove_file(&tmp);
        return Err("ffmpeg produced an empty file".into());
    }
    if out.exists() {
        let _ = std::fs::remove_file(&out);
    }
    std::fs::rename(&tmp, &out).map_err(|e| e.to_string())?;
    Ok(Outcome { message: format!("{} ({})", out.file_name().unwrap().to_string_lossy(), human_size(size)) })
}

fn frames_one(tools: &Tools, o: &FramesOpts, input: &Path, dir: &Path, pr: &Probe, cancel: &Cancel, on_progress: &mut dyn FnMut(f32)) -> Result<Outcome, String> {
    if !(o.interval > 0.0) {
        return Err("interval must be above zero".into());
    }
    let name = stem(input);
    let fdir = dir.join(format!("{name}_frames"));
    std::fs::create_dir_all(&fdir).map_err(|e| e.to_string())?;
    let ext = o.format.ext();
    let pattern = fdir.join(format!("vet_%08d.{ext}"));
    let args = frames_args(input, &pattern, o, tools.fps_flag);
    let shown_duration = pr.duration.map(|d| (d - o.start).max(0.1));
    let res = run_ffmpeg(tools, &args, shown_duration, cancel, on_progress);
    let produced = list_temp_frames(&fdir, ext);
    if let Err(e) = res {
        for (_, p) in &produced {
            let _ = std::fs::remove_file(p);
        }
        return Err(e);
    }
    let with_ms = (o.interval - o.interval.round()).abs() > 1e-9 || (o.start - o.start.round()).abs() > 1e-9;
    let (mut kept, mut skipped) = (0usize, 0usize);
    for (n, p) in produced {
        // n is the frame's own timestamp in milliseconds (see frames_args)
        let t = o.start + (n as f64) / 1000.0;
        let t = if with_ms { t } else { t.round() };
        let target = fdir.join(format!("{name}_{}.{ext}", time_label(t, with_ms)));
        if target.exists() && !o.overwrite {
            let _ = std::fs::remove_file(&p);
            skipped += 1;
            continue;
        }
        let _ = std::fs::remove_file(&target);
        std::fs::rename(&p, &target).map_err(|e| e.to_string())?;
        kept += 1;
    }
    if kept == 0 && skipped == 0 {
        return Err("no frames came out (is the start time past the end of the video?)".into());
    }
    let extra = if skipped > 0 { format!(", {skipped} already there") } else { String::new() };
    Ok(Outcome { message: format!("{kept} frames in {}/{extra}", fdir.file_name().unwrap().to_string_lossy()) })
}

/// Temp frame files `vet_00010000.jpg` as (10000, path), sorted. The number is milliseconds.
fn list_temp_frames(dir: &Path, ext: &str) -> Vec<(u64, PathBuf)> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            let Some(fname) = p.file_name().and_then(|f| f.to_str()) else { continue };
            if let Some(num) = fname.strip_prefix("vet_").and_then(|r| r.strip_suffix(&format!(".{ext}"))) {
                if let Ok(n) = num.parse::<u64>() {
                    v.push((n, p.clone()));
                }
            }
        }
    }
    v.sort();
    v
}

// ---------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(5 * 1024 * 1024 * 1024), "5.0 GB");
    }

    #[test]
    fn labels() {
        assert_eq!(time_label(0.0, false), "t00m00s");
        assert_eq!(time_label(125.0, false), "t02m05s");
        assert_eq!(time_label(3725.0, false), "t1h02m05s");
        assert_eq!(time_label(1.5, true), "t00m01s500ms");
    }

    #[test]
    fn compress_hevc_has_hvc1_and_even_scale() {
        let o = CompressOpts { max_width: Some(1280), ..Default::default() };
        let a = compress_args(Path::new("in.mov"), Path::new("out.mp4"), &o, true);
        let j = a.join(" ");
        assert!(j.contains("-tag:v hvc1"));
        assert!(j.contains("-pix_fmt yuv420p"));
        assert!(j.contains("scale='min(1280,iw)':-2"));
        assert!(j.contains("-map 0:a:0"));
        assert!(j.contains("-movflags +faststart"));
        assert!(j.contains("-nostdin"));
    }

    #[test]
    fn compress_h264_no_tag_and_no_audio_flags_without_audio() {
        let o = CompressOpts { codec: Codec::H264, ..Default::default() };
        let j = compress_args(Path::new("in.mp4"), Path::new("out.mp4"), &o, false).join(" ");
        assert!(!j.contains("hvc1"));
        assert!(!j.contains("-c:a"));
        assert!(!j.contains("-map 0:a"));
        assert!(j.contains("libx264"));
    }

    #[test]
    fn audio_formats() {
        let j = audio_args(Path::new("a.mp4"), Path::new("a.wav"), &AudioOpts { format: AudioFormat::Wav, ..Default::default() }).join(" ");
        assert!(j.contains("pcm_s16le") && !j.contains("-b:a"));
        let j = audio_args(Path::new("a.mp4"), Path::new("a.aac"), &AudioOpts { format: AudioFormat::Aac, mono: true, ..Default::default() }).join(" ");
        assert!(j.contains("-f adts") && j.contains("-ac 1") && j.contains("-b:a 96k"));
        assert_eq!(AudioFormat::parse("MP3"), Some(AudioFormat::Mp3));
        assert_eq!(AudioFormat::parse("flac"), None);
    }

    #[test]
    fn frames_filter() {
        let o = FramesOpts { interval: 0.5, dedupe: true, start: 3.0, ..Default::default() };
        let j = frames_args(Path::new("a.mp4"), Path::new("x/vet_%08d.jpg"), &o, "-fps_mode").join(" ");
        assert!(j.contains("select='gte(t+0.001,selected_n*0.5)',mpdecimate,scale="));
        assert!(j.contains("-ss 3"));
        assert!(j.contains("-frame_pts 1"));
        let j = frames_args(Path::new("a.mp4"), Path::new("x/vet_%08d.jpg"), &FramesOpts::default(), "-vsync").join(" ");
        assert!(j.contains("-vsync passthrough"));
        assert!(j.contains("selected_n*10)',scale="));
    }

    #[test]
    fn version_parsing() {
        assert_eq!(parse_version("ffmpeg version 4.4.2-0ubuntu0.22.04.1 Copyright (c) 2000-2021"), Some((4, 4)));
        assert_eq!(parse_version("ffmpeg version 6.1.1-3ubuntu5 Copyright"), Some((6, 1)));
        assert_eq!(parse_version("ffmpeg version 7.1-full_build-www.gyan.dev Copyright"), Some((7, 1)));
        assert_eq!(parse_version("ffmpeg version N-117000-gabcdef Copyright"), Some((99, 0)));
        assert_eq!(parse_version("nonsense"), None);
    }

    #[test]
    fn probe_parsing() {
        let p = parse_probe(r#"{"streams":[{"codec_type":"video","disposition":{"attached_pic":0}},{"codec_type":"audio"}],"format":{"duration":"12.5"}}"#);
        assert_eq!(p, Probe { duration: Some(12.5), has_video: true, has_audio: true });
        let p = parse_probe(r#"{"streams":[{"codec_type":"video","disposition":{"attached_pic":1}},{"codec_type":"audio"}],"format":{}}"#);
        assert!(!p.has_video && p.has_audio && p.duration.is_none());
        assert_eq!(parse_probe("garbage"), Probe::default());
    }

    #[test]
    fn collects_recursively_and_ignores_others() {
        let d = std::env::temp_dir().join(format!("vet-collect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("sub")).unwrap();
        for f in ["a.mp4", "b.TXT", "sub/c.MOV", "sub/d.png"] {
            std::fs::write(d.join(f), b"x").unwrap();
        }
        let got = collect_inputs(&[d.clone()]);
        assert_eq!(got.len(), 2);
        assert!(got.iter().any(|p| p.ends_with("a.mp4")) && got.iter().any(|p| p.ends_with("c.MOV")));
        let _ = std::fs::remove_dir_all(&d);
    }
}
