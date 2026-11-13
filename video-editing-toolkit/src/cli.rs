//! Command line: `video-editing-toolkit compress|audio|frames <files or folders>`.

use crate::engine::{self, AudioFormat, AudioOpts, Codec, CompressOpts, FramesOpts, ImageFormat, Job};
use crate::runner::{self, Event};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc};

#[derive(Parser)]
#[command(name = "video-editing-toolkit", about = "Compress video, extract audio and grab frames with ffmpeg. Run with no arguments for the window.", version)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Args)]
pub struct Common {
    /// Video files or folders (folders are searched recursively)
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,
    /// Write results here instead of next to each video
    #[arg(short = 'o', long)]
    pub output_dir: Option<PathBuf>,
    /// Replace files that already exist
    #[arg(long)]
    pub overwrite: bool,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum CodecArg {
    H265,
    H264,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ImgArg {
    Jpg,
    Png,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// Shrink videos (H.265 by default, MP4 out)
    Compress {
        #[command(flatten)]
        common: Common,
        /// Quality: lower is better and bigger (18 to 35 is the useful range)
        #[arg(long, default_value_t = 28)]
        crf: u32,
        #[arg(long, default_value = "medium", value_parser = engine::PRESETS.to_vec())]
        preset: String,
        #[arg(long, value_enum, default_value = "h265")]
        codec: CodecArg,
        /// Audio bitrate in kbit/s
        #[arg(long, default_value_t = 128)]
        audio_bitrate: u32,
        /// Scale down to at most this width (never scales up)
        #[arg(long)]
        max_width: Option<u32>,
        #[arg(long, default_value = "_compressed")]
        suffix: String,
        /// Replace the original, only when the result is smaller
        #[arg(long)]
        in_place: bool,
    },
    /// Pull the audio track out (mp3, m4a, aac, opus or wav)
    Audio {
        #[command(flatten)]
        common: Common,
        #[arg(long, default_value = "mp3")]
        format: String,
        /// Bitrate in kbit/s (ignored for wav)
        #[arg(long, default_value_t = 96)]
        bitrate: u32,
        #[arg(long)]
        mono: bool,
    },
    /// Save a still every N seconds, named by timestamp
    Frames {
        #[command(flatten)]
        common: Common,
        /// Seconds between frames
        #[arg(long, default_value_t = 10.0)]
        interval: f64,
        #[arg(long, default_value_t = 1920)]
        max_width: u32,
        /// JPEG quality, 1 best to 31 worst
        #[arg(long, default_value_t = 5)]
        quality: u32,
        #[arg(long, value_enum, default_value = "jpg")]
        format: ImgArg,
        /// Start this many seconds in
        #[arg(long, default_value_t = 0.0)]
        start: f64,
        /// Skip frames that look the same as the last one kept
        #[arg(long)]
        dedupe: bool,
    },
}

pub fn run(cli: Cli) -> i32 {
    let (common, job) = match cli.cmd {
        Cmd::Compress { common, crf, preset, codec, audio_bitrate, max_width, suffix, in_place } => {
            let o = CompressOpts { crf, preset, codec: match codec { CodecArg::H265 => Codec::H265, CodecArg::H264 => Codec::H264 }, audio_kbps: audio_bitrate, max_width, suffix, in_place, overwrite: common.overwrite };
            (common, Job::Compress(o))
        }
        Cmd::Audio { common, format, bitrate, mono } => {
            let Some(f) = AudioFormat::parse(&format) else {
                eprintln!("Unknown audio format '{format}'. Use mp3, m4a, aac, opus or wav.");
                return 2;
            };
            let o = AudioOpts { format: f, kbps: bitrate, mono, overwrite: common.overwrite };
            (common, Job::Audio(o))
        }
        Cmd::Frames { common, interval, max_width, quality, format, start, dedupe } => {
            if !(interval > 0.0) {
                eprintln!("--interval must be above zero.");
                return 2;
            }
            let o = FramesOpts { interval, max_width, quality, format: match format { ImgArg::Jpg => ImageFormat::Jpg, ImgArg::Png => ImageFormat::Png }, start, dedupe, overwrite: common.overwrite };
            (common, Job::Frames(o))
        }
    };
    let tools = match engine::find_tools() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let inputs = engine::collect_inputs(&common.inputs);
    if inputs.is_empty() {
        eprintln!("No video files found ({}).", engine::VIDEO_EXTS.join(", "));
        return 1;
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let out_dir = common.output_dir.clone();
    let worker = {
        let cancel = cancel.clone();
        std::thread::spawn(move || runner::run_batch(tools, job, inputs, out_dir, cancel, tx))
    };
    let tty = std::io::stderr().is_terminal();
    let mut code = 0;
    for ev in rx {
        match ev {
            Event::FileStart { index, total, name } => eprintln!("[{index}/{total}] {name}"),
            Event::Progress(p) => {
                if tty {
                    eprint!("\r    {:>3.0}%", p * 100.0);
                }
            }
            Event::FileDone { ok, message, .. } => {
                if tty {
                    eprint!("\r");
                }
                println!("    {} {message}", if ok { "ok:" } else { "FAILED:" });
                if !ok {
                    code = 1;
                }
            }
            Event::Finished { done, failed, cancelled } => {
                println!("{done} done, {failed} failed{}.", if cancelled { ", cancelled" } else { "" });
                if failed > 0 {
                    code = 1;
                }
            }
        }
    }
    let _ = worker.join();
    code
}
