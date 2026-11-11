#!/usr/bin/env python3
"""
Capture a frame from a YouTube video at a fixed interval, resized to 960x540
and saved as compressed JPEGs.

Version 1.1  (1.1 only adds explanatory comments and shows the version number
              when it starts; it behaves exactly like 1.0)

The script asks you for:
    1. the YouTube URL
    2. the folder to save frames in
    3. how often to capture a frame (mm:ss)
and shows a summary for confirmation before it does anything.

Requirements:
    pip install yt-dlp
    ffmpeg must be installed and on your PATH
        (macOS: brew install ffmpeg | Windows: winget install ffmpeg | Linux: apt install ffmpeg)

HOW THE SCRIPT WORKS (a map for newcomers)
    1. ask_url / ask_folder / ask_interval  - collect your answers, re-asking if
                                              an answer is not usable
    2. get_stream_info                      - use yt-dlp to find a direct link to
                                              the video stream
    3. main                                 - runs ffmpeg (a free video tool) to
                                              pull out one picture every N seconds,
                                              then renames each file with its
                                              position in the video

  Vocabulary used in the comments:
    - function : a named, reusable block of code, written as "def name(...):"
    - string   : a piece of text, such as "hello"
    - list     : an ordered collection, such as [1, 2, 3]
    - PATH     : the list of folders your computer searches to find programs
"""

# ----------------------------------------------------------------------------
# Imports: ready-made toolkits that come with Python (the "standard library").
# ----------------------------------------------------------------------------
import shutil        # file and program helpers (used to check ffmpeg is installed)
import subprocess    # runs other programs (used here to run ffmpeg)
import sys           # information about the running Python itself (used to exit)
from pathlib import Path    # a tidy way to work with file and folder paths

# The version number of this script. Shown when the script starts.
__version__ = "1.1"

# ---- Settings you can tweak -------------------------------------------------
# Names in CAPITAL LETTERS are a Python convention for values that are meant to
# stay fixed while the script runs. Change the numbers below to suit you.
WIDTH = 960            # width of each saved picture, in pixels
HEIGHT = 540           # height of each saved picture, in pixels
JPEG_QUALITY = 70      # 1-100, lower = smaller files
CROP_TO_FILL = False   # False = keep aspect ratio with black bars, True = crop to fill
# -----------------------------------------------------------------------------


def ask_url() -> str:
    """Ask for the video address until it looks like a YouTube link."""
    # "-> str" is a hint that this function gives back a piece of text.
    while True:    # repeat until we reach a "return"
        # input() shows a question and waits for the person to type an answer.
        # .strip() removes spaces from both ends of what was typed.
        url = input("1) YouTube URL: ").strip()
        if "youtube.com" in url or "youtu.be" in url:
            return url    # a good answer: hand it back and stop asking
        print("   That doesn't look like a YouTube URL, try again.")


def ask_folder() -> Path:
    """Ask which folder to save the pictures in."""
    while True:
        # Remove spaces and any quote marks (Windows adds quotes when you copy a path).
        raw = input("2) Folder to save frames in: ").strip().strip('"').strip("'")
        if not raw:    # an empty answer counts as False in Python
            print("   Please enter a folder path.")
            continue    # jump back to the top of the loop and ask again
        # Path(...) turns the text into a path object; expanduser() turns a
        # leading "~" into the person's home folder.
        return Path(raw).expanduser()


def parse_mmss(text: str):
    """Parse 'mm:ss' (also accepts 'ss' or 'hh:mm:ss'). Returns seconds or None."""
    # Split "01:30" at the colons into ["01", "30"].
    parts = text.strip().split(":")
    if not 1 <= len(parts) <= 3:    # we accept one, two or three parts only
        return None                 # None means "no valid answer"
    try:
        # Turn each part from text into a whole number: ["01", "30"] -> [1, 30].
        nums = [int(p) for p in parts]
    except ValueError:
        # ValueError means a part was not a number (for example "ab:cd").
        return None
    if any(n < 0 for n in nums):    # negative times make no sense
        return None
    # Combine the parts into seconds. Each step multiplies by 60, because there
    # are 60 seconds in a minute and 60 minutes in an hour:
    #   [1, 30]    -> (0 * 60 + 1) * 60 + 30            = 90 seconds
    #   [1, 2, 3]  -> ((0 * 60 + 1) * 60 + 2) * 60 + 3  = 3,723 seconds
    total = 0
    for n in nums:
        total = total * 60 + n
    return total


def ask_interval() -> int:
    """Ask how often to capture a frame; returns the gap in seconds."""
    while True:
        raw = input("3) Capture a frame every (mm:ss, e.g. 00:10 or 01:30): ")
        seconds = parse_mmss(raw)
        # "seconds and seconds > 0" rejects both None (invalid) and 0 (no gap).
        if seconds and seconds > 0:
            return seconds
        print("   Please enter a time like 00:10 or 01:30 (must be more than 0).")


def get_stream_info(url: str):
    """Resolve a direct video-only stream URL (<=1080p) using yt-dlp."""
    # yt-dlp is not built into Python, so we import it here and give a clear
    # message if it has not been installed yet.
    try:
        import yt_dlp
    except ImportError:
        sys.exit("yt-dlp is not installed. Run: pip install yt-dlp")

    # A dictionary of settings for yt-dlp: each "key": value pair switches one
    # behaviour on or off.
    opts = {
        "quiet": True,           # keep yt-dlp's own messages short
        "no_warnings": True,     # hide non-critical warnings
        "noplaylist": True,      # if the link is inside a playlist, use only this video
        "format": "bv*[height<=1080]/bv*/b",  # best video up to 1080p, no audio needed
    }
    # "with" opens something, uses it, then closes it again automatically.
    with yt_dlp.YoutubeDL(opts) as ydl:
        info = ydl.extract_info(url, download=False)    # look up details, no download
    # Give back three things: the direct stream link, the title and the length
    # in seconds. info.get("title", "video") uses "video" if no title is found.
    return info["url"], info.get("title", "video"), info.get("duration")


def quality_to_qv(quality: int) -> int:
    """Map 1-100 JPEG quality (higher = better) to ffmpeg's -q:v 2-31 (lower = better)."""
    # ffmpeg measures JPEG quality on its own scale, where a LOWER number is
    # BETTER, so we convert from the familiar 1-100 scale.
    # First clamp the value into the range 1-100 (min/max stop it going outside).
    quality = max(1, min(100, quality))
    return round(31 - (quality / 100) * 29)


def fmt_mmss(seconds: int) -> str:
    """Format seconds for display: 90 -> '01:30', 3723 -> '1:02:03'."""
    # divmod(a, b) gives the whole-number answer and the remainder together.
    h, rem = divmod(seconds, 3600)    # hours, and what is left over
    m, s = divmod(rem, 60)            # minutes and seconds from the leftover
    # {m:02d} means "show as a whole number, padded with a zero to 2 digits".
    return f"{h:d}:{m:02d}:{s:02d}" if h else f"{m:02d}:{s:02d}"


def fmt_name_time(seconds: int) -> str:
    """Format seconds for use in a file name: 90 -> '00h01m30s'."""
    h, rem = divmod(seconds, 3600)
    m, s = divmod(rem, 60)
    return f"{h:02d}h{m:02d}m{s:02d}s"


def main():
    """Run the whole program: ask, confirm, extract the frames, rename them."""
    print(f"=== YouTube Frame Capture (version {__version__}) ===\n")
    url = ask_url()
    out_dir = ask_folder()
    interval = ask_interval()

    # Confirmation before executing: show what will happen and let the person back out.
    print("\n--- Summary ---")
    print(f"URL:        {url}")
    print(f"Save to:    {out_dir.resolve()}")    # .resolve() shows the full path
    print(f"Interval:   every {fmt_mmss(interval)} (mm:ss)")
    print(f"Frame size: {WIDTH}x{HEIGHT} JPEG, quality {JPEG_QUALITY}")
    # Pressing Enter alone counts as "yes"; only "n" or "no" cancels.
    if input("\nStart capture? [Y/n]: ").strip().lower() in ("n", "no"):
        print("Cancelled.")
        return    # leave main() early, which ends the program

    # shutil.which() looks for a program on the PATH and gives nothing back if
    # it is missing, so this checks that ffmpeg is installed before we start.
    if not shutil.which("ffmpeg"):
        sys.exit("ffmpeg not found. Install it and make sure it's on your PATH.")

    # Create the output folder (and any missing parent folders) if needed.
    out_dir.mkdir(parents=True, exist_ok=True)

    print("\nResolving video stream...")
    stream_url, title, duration = get_stream_info(url)
    print(f"Title: {title}")
    if duration:
        # // is whole-number division. +1 because a picture is taken at 0:00 too.
        print(f"Duration: {fmt_mmss(int(duration))} -> approx. {int(duration) // interval + 1} frames")

    # Build the ffmpeg "video filter" text that resizes each picture.
    if CROP_TO_FILL:
        # "increase" scales until the picture covers the frame, then crop trims the extra.
        scale = f"scale={WIDTH}:{HEIGHT}:force_original_aspect_ratio=increase,crop={WIDTH}:{HEIGHT}"
    else:
        # "decrease" scales until the picture fits inside the frame, then pad
        # fills the leftover space with black bars, centred.
        scale = (f"scale={WIDTH}:{HEIGHT}:force_original_aspect_ratio=decrease,"
                 f"pad={WIDTH}:{HEIGHT}:(ow-iw)/2:(oh-ih)/2")
    # "fps=1/N" means "one frame every N seconds".
    vf = f"fps=1/{interval},{scale}"

    # The ffmpeg command, written as a list of separate words:
    #   -hide_banner / -loglevel error / -stats : keep the screen output tidy
    #   -y            : overwrite files with the same name without asking
    #   -i            : the input (the video stream link)
    #   -an           : ignore the audio
    #   -vf           : apply the filter built above
    #   -q:v          : JPEG quality
    # The last item is the output name; %06d becomes 000001, 000002, ...
    cmd = [
        "ffmpeg", "-hide_banner", "-loglevel", "error", "-stats", "-y",
        "-i", stream_url,
        "-an", "-vf", vf,
        "-q:v", str(quality_to_qv(JPEG_QUALITY)),
        str(out_dir / "tmp_%06d.jpg"),
    ]

    print("Extracting frames (this reads through the whole video, so it may take a while)...")
    # subprocess.run() runs the command and waits. A return code of 0 means
    # success; anything else means ffmpeg reported a problem.
    if subprocess.run(cmd).returncode != 0:
        sys.exit("ffmpeg failed.")

    # Rename frames to include their timestamp in the video.
    # ffmpeg saved them as tmp_000001.jpg, tmp_000002.jpg, ... in order.
    frames = sorted(out_dir.glob("tmp_*.jpg"))
    for idx, f in enumerate(frames):    # enumerate counts the items, from 0
        # Frame number idx is idx * interval seconds into the video.
        f.rename(out_dir / f"frame_{idx + 1:05d}_{fmt_name_time(idx * interval)}.jpg")

    # Add up the file sizes (in bytes), then divide by 1024 to get kilobytes.
    total_kb = sum(f.stat().st_size for f in out_dir.glob("frame_*.jpg")) / 1024
    print(f"\nDone. Saved {len(frames)} frames ({total_kb:.0f} KB total) to {out_dir.resolve()}")


# This line means: only run main() when the script is started directly (for
# example "py youtube_frames.py"), not when another script imports this file.
if __name__ == "__main__":
    main()
