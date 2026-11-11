# YouTube Frames

A single Python script that captures a still frame from a YouTube video at a fixed interval and saves them as small, compressed JPEGs (default 960x540, quality 70). Handy for turning a lecture or tutorial into a set of slides or study images.

## Use
```
py youtube_frames.py
```
It asks for the video URL, the folder to save into, and how often to capture a frame (`mm:ss`, e.g. `00:10` or `01:30`), then shows a summary and waits for confirmation before doing anything.

Frames are named with their position in the video, e.g. `frame_00003_00h00m20s.jpg`.

## Requirements
- Python 3
- `pip install yt-dlp`
- `ffmpeg` on your PATH (macOS: `brew install ffmpeg`, Windows: `winget install ffmpeg`, Linux: `apt install ffmpeg`)

## Settings
At the top of the script: `WIDTH`, `HEIGHT`, `JPEG_QUALITY`, and `CROP_TO_FILL` (black bars vs. cropping to fill the frame).

## Notes
- Only the video stream is read (no audio) and nothing is kept except the JPEGs.
- It reads through the whole video, so long videos take a while.
- Respect YouTube's terms and creators' rights; use frames for personal study.

## Version
Current version: **1.1** (shown when the script starts).

| Version | Changes |
| --- | --- |
| 1.1 | Added beginner-friendly comments throughout the script and a "how it works" guide at the top. The version number now appears in the start-up banner. No change to behaviour. |
| 1.0 | First release: capture a frame at a fixed interval and save compressed JPEGs. |
