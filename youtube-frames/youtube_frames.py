#!/usr/bin/env python3
"""
Capture a frame from a YouTube video at a fixed interval, resized to 960x540
and saved as compressed JPEGs.

The script asks you for:
    1. the YouTube URL
    2. the folder to save frames in
    3. how often to capture a frame (mm:ss)
and shows a summary for confirmation before it does anything.

Requirements:
    pip install yt-dlp
    ffmpeg must be installed and on your PATH
        (macOS: brew install ffmpeg | Windows: winget install ffmpeg | Linux: apt install ffmpeg)
"""

import shutil
import subprocess
import sys
from pathlib import Path

# ---- Settings you can tweak -------------------------------------------------
WIDTH = 960
HEIGHT = 540
JPEG_QUALITY = 70      # 1-100, lower = smaller files
CROP_TO_FILL = False   # False = keep aspect ratio with black bars, True = crop to fill
# -----------------------------------------------------------------------------


def ask_url() -> str:
    while True:
        url = input("1) YouTube URL: ").strip()
        if "youtube.com" in url or "youtu.be" in url:
            return url
        print("   That doesn't look like a YouTube URL, try again.")


def ask_folder() -> Path:
    while True:
        raw = input("2) Folder to save frames in: ").strip().strip('"').strip("'")
        if not raw:
            print("   Please enter a folder path.")
            continue
        return Path(raw).expanduser()


def parse_mmss(text: str):
    """Parse 'mm:ss' (also accepts 'ss' or 'hh:mm:ss'). Returns seconds or None."""
    parts = text.strip().split(":")
    if not 1 <= len(parts) <= 3:
        return None
    try:
        nums = [int(p) for p in parts]
    except ValueError:
        return None
    if any(n < 0 for n in nums):
        return None
    total = 0
    for n in nums:
        total = total * 60 + n
    return total


def ask_interval() -> int:
    while True:
        raw = input("3) Capture a frame every (mm:ss, e.g. 00:10 or 01:30): ")
        seconds = parse_mmss(raw)
        if seconds and seconds > 0:
            return seconds
        print("   Please enter a time like 00:10 or 01:30 (must be more than 0).")


def get_stream_info(url: str):
    """Resolve a direct video-only stream URL (<=1080p) using yt-dlp."""
    try:
        import yt_dlp
    except ImportError:
        sys.exit("yt-dlp is not installed. Run: pip install yt-dlp")

    opts = {
        "quiet": True,
        "no_warnings": True,
        "noplaylist": True,
        "format": "bv*[height<=1080]/bv*/b",  # best video up to 1080p, no audio needed
    }
    with yt_dlp.YoutubeDL(opts) as ydl:
        info = ydl.extract_info(url, download=False)
    return info["url"], info.get("title", "video"), info.get("duration")


def quality_to_qv(quality: int) -> int:
    """Map 1-100 JPEG quality (higher = better) to ffmpeg's -q:v 2-31 (lower = better)."""
    quality = max(1, min(100, quality))
    return round(31 - (quality / 100) * 29)


def fmt_mmss(seconds: int) -> str:
    h, rem = divmod(seconds, 3600)
    m, s = divmod(rem, 60)
    return f"{h:d}:{m:02d}:{s:02d}" if h else f"{m:02d}:{s:02d}"


def fmt_name_time(seconds: int) -> str:
    h, rem = divmod(seconds, 3600)
    m, s = divmod(rem, 60)
    return f"{h:02d}h{m:02d}m{s:02d}s"


def main():
    print("=== YouTube Frame Capture ===\n")
    url = ask_url()
    out_dir = ask_folder()
    interval = ask_interval()

    # Confirmation before executing
    print("\n--- Summary ---")
    print(f"URL:        {url}")
    print(f"Save to:    {out_dir.resolve()}")
    print(f"Interval:   every {fmt_mmss(interval)} (mm:ss)")
    print(f"Frame size: {WIDTH}x{HEIGHT} JPEG, quality {JPEG_QUALITY}")
    if input("\nStart capture? [Y/n]: ").strip().lower() in ("n", "no"):
        print("Cancelled.")
        return

    if not shutil.which("ffmpeg"):
        sys.exit("ffmpeg not found. Install it and make sure it's on your PATH.")

    out_dir.mkdir(parents=True, exist_ok=True)

    print("\nResolving video stream...")
    stream_url, title, duration = get_stream_info(url)
    print(f"Title: {title}")
    if duration:
        print(f"Duration: {fmt_mmss(int(duration))} -> approx. {int(duration) // interval + 1} frames")

    if CROP_TO_FILL:
        scale = f"scale={WIDTH}:{HEIGHT}:force_original_aspect_ratio=increase,crop={WIDTH}:{HEIGHT}"
    else:
        scale = (f"scale={WIDTH}:{HEIGHT}:force_original_aspect_ratio=decrease,"
                 f"pad={WIDTH}:{HEIGHT}:(ow-iw)/2:(oh-ih)/2")
    vf = f"fps=1/{interval},{scale}"

    cmd = [
        "ffmpeg", "-hide_banner", "-loglevel", "error", "-stats", "-y",
        "-i", stream_url,
        "-an", "-vf", vf,
        "-q:v", str(quality_to_qv(JPEG_QUALITY)),
        str(out_dir / "tmp_%06d.jpg"),
    ]

    print("Extracting frames (this reads through the whole video, so it may take a while)...")
    if subprocess.run(cmd).returncode != 0:
        sys.exit("ffmpeg failed.")

    # Rename frames to include their timestamp in the video
    frames = sorted(out_dir.glob("tmp_*.jpg"))
    for idx, f in enumerate(frames):
        f.rename(out_dir / f"frame_{idx + 1:05d}_{fmt_name_time(idx * interval)}.jpg")

    total_kb = sum(f.stat().st_size for f in out_dir.glob("frame_*.jpg")) / 1024
    print(f"\nDone. Saved {len(frames)} frames ({total_kb:.0f} KB total) to {out_dir.resolve()}")


if __name__ == "__main__":
    main()
