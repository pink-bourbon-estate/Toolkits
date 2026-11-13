# Video Editing Toolkit

Three small jobs that come up all the time, in one program with a window and a command line:

- **Compress** videos to a fraction of their size (H.265 or H.264, MP4 out).
- **Extract audio** as mp3, m4a, aac, opus or wav, for transcripts, podcasts or listening.
- **Extract frames**: a still every N seconds, named by time (`lecture_t00m10s.jpg`), for notes, slides and reference.

It is a thin, careful wrapper around [ffmpeg](https://ffmpeg.org). Written in Rust with egui, like the other tools in this repo. Local-first: your videos never leave your computer. The only network use is the optional update check.

This replaces the three separate Python scripts (`compress_video.py`, `extract_audio.py`, `extract_frames.py`).

## Install (Windows)
1. Install ffmpeg once: `winget install Gyan.FFmpeg` (or put `ffmpeg.exe` and `ffprobe.exe` next to this program). Version 5.1 or newer.
2. Download `video-editing-toolkit.exe` from the newest **Video Editing Toolkit v0.1.x** release on the [Releases page](https://github.com/pink-bourbon-estate/toolkits/releases).
3. Run it. If SmartScreen warns about an unknown app, click **More info, Run anyway** (it isn't code-signed).
4. Click **Install Video Editing Toolkit** on the bottom bar. It copies itself to `%LOCALAPPDATA%\Programs\VideoEditingToolkit` (no admin rights) and adds a Start menu entry and a **Settings, Apps** entry.

Linux: download `video-editing-toolkit-linux-x86_64`, `chmod +x` it, install ffmpeg with your package manager, and run it (`--install` puts it in `~/.local/bin` with a menu entry).

## Using the window
Drag videos or whole folders onto it (folders are searched recursively), pick a tab, adjust the settings and press the button. Results are saved next to each video unless you choose a folder. A progress bar shows each file, and **Cancel** stops cleanly and removes half-written files.

## Command line
```
video-editing-toolkit compress  <files or folders> [--crf 28] [--preset medium] [--codec h265|h264]
                                [--max-width 1280] [--audio-bitrate 128] [--suffix _compressed]
                                [--in-place] [--overwrite] [-o OUTPUT_DIR]
video-editing-toolkit audio     <files or folders> [--format mp3|m4a|aac|opus|wav] [--bitrate 96] [--mono]
video-editing-toolkit frames    <files or folders> [--interval 10] [--start 0] [--max-width 1920]
                                [--quality 5] [--format jpg|png] [--dedupe]
```
The exit code is 0 only if every file worked, so it is safe to use in scripts. Other flags: `--install`, `--uninstall`, `--update`, `--version`.

## What changed from the Python scripts
- **Frame times are now correct.** The old `fps=1/N` approach returns the frame nearest the *middle* of each period, so "every 10 s" really gave frames at about 5, 15 and 25 s, and the file names were wrong. This version takes the first frame at or after each multiple of the interval, and names each file from that frame's own timestamp.
- HEVC is tagged `hvc1`, so the result plays in QuickTime, iPhone and Safari.
- Output is forced to `yuv420p`, which every player accepts (10-bit or 4:4:4 sources no longer make files that will not play).
- Only the first video stream and first audio stream are kept, so files with extra data tracks no longer fail.
- Videos with no audio compress fine, and audio extraction says clearly that there is none.
- **Replace original** writes to a temporary file, checks it opens, and only swaps it in when it is smaller. Otherwise your original is untouched.
- Optional: skip near-identical frames (`--dedupe`), start part-way in (`--start`), mono audio, opus and wav output, resize without ever scaling up.
- A live progress bar, a Cancel button, and ffmpeg's own error message when something fails.

## Updates
Every change pushed to `video-editing-toolkit/` is built by GitHub Actions and published as release `video-editing-toolkit-v0.1.N` with SHA-256 checksums. **Check for updates** downloads the new build, verifies the checksum, replaces itself and restarts. Nothing about you is sent.

## Build from source
```
cd video-editing-toolkit
cargo test
cargo build --release
```
