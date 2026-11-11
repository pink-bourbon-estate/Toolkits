"""
Download every video transcript from a YouTube channel as Obsidian-style notes,
then combine them into a few big "master" files that are easy to upload and analyse.

Version 1.1  (1.1 only adds explanatory comments and a --version option; it
              behaves exactly like 1.0)

HOW TO USE (run it from the folder this script lives in)

  py get_transcripts.py https://www.youtube.com/@ChannelName/videos
      Downloads the channel into a folder called "<Channel Name> Transcripts"
      (next to this script), then combines the notes. Safe to stop and re-run.

  py get_transcripts.py https://www.youtube.com/playlist?list=PLxxxxxxxx
      Downloads a playlist into "<Playlist Title> Transcripts".

  py get_transcripts.py https://www.youtube.com/watch?v=xxxxxxxxxxx
      Downloads one video into its channel's folder (a video link copied from
      inside a playlist counts as a single video; use the playlist link for the list).

  py get_transcripts.py "Channel Name Transcripts"
      Combines a folder you already downloaded. Nothing is downloaded again.

  py get_transcripts.py
      Asks whether to download a channel (then asks for its address) or just
      combine existing folders (then asks for the folder path; press Enter to
      combine EVERY "... Transcripts" folder sitting next to this script).

OPTIONS
  --pdf          also write each master file as a PDF
  --words N      max words per master file (default 60000). 0 = one single file.
  --no-combine   with a channel URL: download and build notes, skip combining
  --version      show the version number and exit

WHERE THE RESULTS GO
  <Channel Name> Transcripts/             one .md note per video (Obsidian layout)
  <Channel Name> Transcripts/Combined/    the master files, split into numbered parts

HOW THE SCRIPT IS ORGANISED (a map for newcomers)
  The work happens in four stages. Each stage is a group of functions below:

    1. Download   - ask YouTube for each video's captions (uses the yt-dlp library)
    2. Parse      - turn a raw caption file (.vtt) into a list of timed text pieces
    3. Build      - turn those pieces into one tidy Markdown note per video
    4. Combine    - join all the notes into a few large "master" files

  The main() function at the very bottom decides which stages to run, based on
  what you typed. Reading the script from the bottom (main) upwards is a good
  way to follow what happens.

  Vocabulary used in the comments:
    - function   : a named, reusable block of code, written as "def name(...):"
    - string     : a piece of text, such as "hello"
    - list       : an ordered collection, such as [1, 2, 3]
    - dictionary : a collection of labelled values, such as {"title": "My video"}
    - regex      : a "regular expression", a compact pattern used to find or
                   change text (for example, "any run of digits")
"""

# ----------------------------------------------------------------------------
# Imports: ready-made toolkits that come with Python (the "standard library").
# ----------------------------------------------------------------------------
import argparse      # reads options typed after the script name (like --pdf)
import datetime      # works with dates (used to stamp the "created" date)
import html          # converts web text such as "&amp;" back into "&"
import json          # reads and writes JSON files (a common data format)
import re            # "regular expressions": pattern matching for text
import subprocess    # runs other programs (used here to run pip)
import sys           # information about the running Python itself
from pathlib import Path                         # a tidy way to work with file paths
from xml.sax.saxutils import escape as xml_escape  # makes text safe for the PDF builder

# The version number of this script. Shown by "--version".
__version__ = "1.1"

# The folder this script lives in. Output folders are created next to it.
# __file__ is the path of this script; .resolve() makes it a full path;
# .parent means "the folder containing it".
SCRIPT_DIR = Path(__file__).resolve().parent

# ----------------------------------------------------------------------------
# Settings you can change. Names in CAPITAL LETTERS are a Python convention for
# values that are meant to stay fixed while the script runs ("constants").
# ----------------------------------------------------------------------------

# If YouTube says "Sign in to confirm you're not a bot", set this to "chrome",
# "firefox", "edge" or "safari" (whichever browser you're signed in to YouTube with).
# None means "not set" (Python's way of saying "nothing here").
COOKIES_FROM_BROWSER = None

# Paragraph sizing: start a new paragraph at a sentence end once a paragraph
# has this many words, or whenever the speaker pauses for this many seconds.
PARA_WORDS = 70
PAUSE_SECONDS = 2.0

# Master files are split so each part stays a comfortable size to analyse in one go.
DEFAULT_WORDS_PER_PART = 60000

# The video description is mostly sponsor links, so it is left out of the master
# files. Set to True if you want it included.
INCLUDE_DESCRIPTION = False


def ensure(module, pip_name=None):
    """Import a module, installing it with pip first if it's missing.

    A "module" is a toolkit of code written by someone else. Some (like yt_dlp)
    are not built into Python, so we install them the first time they are needed.
    """
    try:
        # Try to load the module. If it is already installed this just works.
        return __import__(module)
    except ImportError:
        # ImportError means "not installed yet", so install it, then load it.
        # "pip_name or module" means: use pip_name if given, otherwise module.
        print(f"Installing {pip_name or module}...")
        subprocess.check_call([sys.executable, "-m", "pip", "install", "--upgrade", pip_name or module])
        return __import__(module)


def safe_filename(name):
    """Remove characters that Windows, Mac or Linux do not allow in file names."""
    # re.sub("pattern", "replacement", text) swaps every match of the pattern.
    # The pattern lists the forbidden characters; replacing them with "" deletes them.
    name = re.sub(r'[\\/:*?"<>|#^\[\]]', "", name).strip().rstrip(".")
    # name[:150] keeps at most the first 150 characters (very long names can fail).
    # "or 'untitled'" is used if the name ended up empty.
    return name[:150] or "untitled"


# ----------------------------------------------------------------------------
# 1. Download captions + video details (description, chapters, date)
# ----------------------------------------------------------------------------
def channel_url(url):
    """Tidy a pasted address so yt-dlp knows to list a channel's videos."""
    url = url.strip()
    # A bare handle such as "@ChannelName" becomes a full web address.
    if url.startswith("@"):
        url = "https://www.youtube.com/" + url
    # If the address ends at the channel's main page (no /videos on the end),
    # add "/videos" so we get the list of uploads rather than the home page.
    # [^/?]+ means "one or more characters that are not a slash or question mark".
    if re.search(r"youtube\.com/(@[^/?]+|channel/[^/?]+|c/[^/?]+|user/[^/?]+)/?$", url):
        url = url.rstrip("/") + "/videos"
    return url


def source_kind(url):
    """'video', 'playlist' or 'channel', judged from the address."""
    # Check for a video first, because a video link can also contain "list=".
    if re.search(r"(youtu\.be/|[?&]v=|/shorts/|/live/)", url):
        return "video"      # includes a video link copied from inside a playlist
    if re.search(r"[?&]list=|/playlist", url):
        return "playlist"
    # Anything else is treated as a channel.
    return "channel"


def needs_login(err):
    """True if an error message suggests the page is private or needs a sign-in."""
    text = str(err).lower()   # lower-case it so "Private" and "private" both match
    # any(...) is True if at least one of the phrases appears in the message.
    return any(k in text for k in ("does not exist", "private", "sign in", "log in", "login"))


def ask_for_browser():
    """Private playlists are hidden unless the script borrows a browser's YouTube sign-in."""
    # "global" lets this function change the setting defined at the top of the file.
    global COOKIES_FROM_BROWSER
    print("\nYouTube won't show that address to the script. This usually means the playlist is PRIVATE\n"
          "(or the link is cut short; a full playlist link has about 34 characters after list=).\n\n"
          "Easiest fix: on YouTube open the playlist, click Edit, set Visibility to 'Unlisted',\n"
          "then run the script again. No sign-in needed.\n\n"
          "Or let the script borrow your browser's YouTube sign-in. Close that browser first, then\n"
          "type its name: firefox (most reliable), chrome, edge, brave, opera, vivaldi or safari.")
    # input() shows a question and waits for the person to type an answer.
    browser = input("\nBrowser name (or press Enter to stop): ").strip().lower()
    if not browser:
        return False     # nothing typed: the person chose to stop
    COOKIES_FROM_BROWSER = browser
    return True


def list_source(yt_dlp, url):
    """Return (kind, folder name, [video ids]) using a quick listing (no downloads)."""
    kind = source_kind(url)

    # A dictionary of settings for yt-dlp. Each "key": value pair switches
    # one behaviour on or off.
    opts = {"extract_flat": "in_playlist",      # list the videos without opening each one
            "quiet": True,                      # keep yt-dlp's own messages short
            "skip_download": True,              # never download the video itself
            "sleep_interval_requests": 1,       # pause 1 second between requests
            "ignore_no_formats_error": True,    # we do not need video formats, only names
            "noplaylist": kind == "video"}      # for a single video, ignore its playlist
    if COOKIES_FROM_BROWSER:
        # A tuple (a fixed list) holding the browser name; lets yt-dlp reuse its sign-in.
        opts["cookiesfrombrowser"] = (COOKIES_FROM_BROWSER,)

    # Show a friendly message that matches what we are doing.
    print({"video": "Looking up the video...", "playlist": "Listing the playlist's videos...",
           "channel": "Listing the channel's videos..."}[kind])

    # "with" opens something, uses it, and closes it again automatically.
    with yt_dlp.YoutubeDL(opts) as ydl:
        info = ydl.extract_info(url, download=False)   # ask YouTube for details
    if not info:
        raise SystemExit("Couldn't read that address. Check the link and try again.")

    # info.get("key") returns the value, or None if it is missing (no crash).
    channel = info.get("channel") or info.get("uploader") or ""
    entries = info.get("entries")
    if entries is None:
        # No list of entries means this address is one single video.
        # A YouTube video id is always exactly 11 characters long.
        ids = [info["id"]] if len(info.get("id") or "") == 11 else []
        return "video", channel or "Videos", ids

    # Keep only entries that have a proper 11-character video id.
    ids = [e["id"] for e in entries if e and e.get("id") and len(e["id"]) == 11]
    if kind == "playlist":
        name = info.get("title") or channel or "Playlist"
    else:
        # Channel pages are titled "<Name> - Videos"; remove the " - Videos" ending.
        name = channel or re.sub(r"\s*-\s*Videos$", "", info.get("title") or "") or "Channel"
    return kind, name, ids


def caption_ids(raw):
    """Return the set of video ids that already have a caption file in `raw`."""
    ids = set()    # a set holds each value once, with no duplicates
    for f in raw.glob("*.vtt"):          # every file ending in .vtt in the folder
        # File names look like: "2026 - Title [VIDEOID].en-orig.vtt".
        # The pattern finds the 11 characters inside the square brackets.
        m = re.search(r"\[([\w-]{11})\]\.[\w-]+\.vtt$", f.name)
        if m:
            ids.add(m.group(1))          # group(1) is the part inside the ( )
    return ids


def download(folder, ids):
    """Download captions and video details for every id not already saved."""
    yt_dlp = ensure("yt_dlp", "yt-dlp")
    raw = folder / "_raw_vtt"                    # "/" joins path parts together
    raw.mkdir(parents=True, exist_ok=True)       # create the folder if it is missing

    # Work out which videos still need downloading (this is what lets you
    # stop the script and run it again later without starting over).
    todo = [i for i in ids if i not in caption_ids(raw)]
    print(f"{len(ids)} video(s) found, {len(ids) - len(todo)} already downloaded, "
          f"{len(todo)} to go.\n")
    if todo:
        opts = {
            "skip_download": True,              # captions only, no video
            "writesubtitles": True,             # creator-uploaded captions if they exist
            "writeautomaticsub": True,          # otherwise YouTube's auto captions
            # "en-orig" is YouTube's original English auto-caption track. Asking for
            # "en" as well doubles the requests and triggers "429 Too Many Requests".
            "subtitleslangs": ["en-orig"],
            "subtitlesformat": "vtt",
            "writeinfojson": True,              # description, chapters, upload date
            "clean_infojson": True,
            # How to name each saved file: upload date, title (max 120 characters), id.
            "outtmpl": str(raw / "%(upload_date)s - %(title).120B [%(id)s].%(ext)s"),
            "ignoreerrors": True,               # skip a failing video instead of stopping
            "sleep_interval_subtitles": 2,      # be gentle so YouTube doesn't block you
            "sleep_interval_requests": 1,
        }
        if COOKIES_FROM_BROWSER:
            opts["cookiesfrombrowser"] = (COOKIES_FROM_BROWSER,)
        with yt_dlp.YoutubeDL(opts) as ydl:
            # Turn each id into a full web address, then download them all.
            ydl.download([f"https://www.youtube.com/watch?v={i}" for i in todo])

    # After downloading, note any videos that still have no captions.
    missing = [i for i in ids if i not in caption_ids(raw)]
    if missing:
        (folder / "_no_captions.txt").write_text(
            "\n".join(f"https://www.youtube.com/watch?v={i}" for i in missing) + "\n", encoding="utf-8")
        print(f"\n{len(missing)} videos have no captions (or failed). "
              f"Listed in _no_captions.txt. Re-run to retry them.")


# ----------------------------------------------------------------------------
# 2. Turn a .vtt caption file into timed segments of new text
# ----------------------------------------------------------------------------
# A .vtt (WebVTT) file is plain text. Each caption looks roughly like:
#     00:00:02.270 --> 00:00:04.070
#     You just need to master the basic
# "CUE" matches the timing line (start --> end). Each (...) captures a number:
# hours, minutes, seconds, milliseconds for the start, then the same for the end.
# re.compile prepares the pattern once so it can be reused quickly.
CUE = re.compile(r"^(\d+):(\d{2}):(\d{2})\.(\d{3}) --> (\d+):(\d{2}):(\d{2})\.(\d{3})")
# "TAG" matches anything wrapped in < >, such as the <c> word-timing marks YouTube adds.
TAG = re.compile(r"<[^>]+>")


def secs(h, m, s, ms):
    """Convert hours, minutes, seconds and milliseconds into total seconds."""
    return int(h) * 3600 + int(m) * 60 + int(s) + int(ms) / 1000


def vtt_segments(vtt):
    """Return [(start, end, text)] with YouTube's rolling duplicate lines removed.

    YouTube's automatic captions "roll": each new caption repeats the previous
    line and adds a new one. This function keeps only the new text.
    """
    segs, last = [], ""    # segs: the results so far; last: the previous caption line

    # Captions are separated by blank lines, so split the file on those.
    blocks = re.split(r"\n{2,}", vtt.replace("\r", ""))
    for block in blocks:
        lines = block.strip("\n").split("\n")
        # Find the timing line in this block; skip blocks that do not have one.
        m = next((CUE.match(l) for l in lines if CUE.match(l)), None)
        if not m:
            continue    # "continue" jumps straight to the next block

        # m.groups() holds the 8 numbers; the first 4 are the start, the rest the end.
        start, end = secs(*m.groups()[:4]), secs(*m.groups()[4:])

        # Everything after the timing line is the caption text.
        text_lines = lines[[i for i, l in enumerate(lines) if CUE.match(l)][0] + 1:]
        # Remove the <...> marks, convert "&amp;"-style codes, and trim spaces.
        text_lines = [html.unescape(TAG.sub("", l)).strip() for l in text_lines]
        text_lines = [l for l in text_lines if l]    # drop empty lines

        # Auto captions repeat the previous line at the top of each cue: drop it.
        while text_lines and text_lines[0] == last:
            text_lines.pop(0)
        if not text_lines:
            continue

        text = " ".join(text_lines)
        # Remove ">>" speaker-change marks.
        text = re.sub(r"^>>\s*", "", text).replace(" >> ", " ")
        if text:
            segs.append([start, end, text])
        last = text_lines[-1]
    return segs


# ----------------------------------------------------------------------------
# 3. Build an Obsidian Web Clipper style note for each video
# ----------------------------------------------------------------------------
def ts(t):
    """Format seconds as a timestamp: 75 -> '1:15', 3725 -> '1:02:05'."""
    t = int(t)
    # // is whole-number division and % is the remainder after dividing.
    h, m, s = t // 3600, t % 3600 // 60, t % 60
    # {m:02d} means "show as a whole number, padded with a zero to 2 digits".
    return f"{h}:{m:02d}:{s:02d}" if h else f"{m}:{s:02d}"


def md_escape(text):
    """Put a backslash before characters that Markdown would treat as formatting."""
    return text.replace("[", "\\[").replace("]", "\\]").replace("_", "\\_")


def build_paragraphs(segs, chapters):
    """Group segments into paragraphs, each tagged with its start time and chapter.

    Captions arrive in tiny pieces, which are hard to read. This joins them into
    paragraphs, starting a new one when a chapter begins, when the speaker
    pauses, or when the paragraph has grown long enough.
    """
    starts = [c["start_time"] for c in chapters]    # the start time of each chapter
    # chap_idx is the number of the chapter we are currently inside (-1 = none yet).
    chap_idx = 0 if starts and starts[0] <= (segs[0][0] if segs else 0) + 1 else -1
    paras, cur = [], None    # paras: finished paragraphs; cur: the paragraph being built

    for i, (start, end, text) in enumerate(segs):
        # Move to the next chapter once we've passed its start, but only at the
        # end of a sentence so a heading never lands in the middle of one.
        new_chapter = False
        while chap_idx + 1 < len(starts) and start >= starts[chap_idx + 1] and (
                cur is None or re.search(r"[.!?]\s*$", cur["text"])):
            chap_idx += 1
            new_chapter = True

        prev_end = segs[i - 1][1] if i else 0           # when the previous piece ended
        # Does the current paragraph end with . ! or ? (a finished sentence)?
        sentence_done = cur is not None and re.search(r"[.!?]\s*$", cur["text"])

        # Start a new paragraph when: there is none yet, a chapter just began, or
        # a sentence has finished AND the paragraph is long OR the speaker paused.
        if (cur is None or new_chapter or
                (sentence_done and (len(cur["text"].split()) >= PARA_WORDS
                                    or start - prev_end >= PAUSE_SECONDS))):
            cur = {"t": start, "text": text, "chapter": chap_idx, "new_chapter": new_chapter or i == 0}
            paras.append(cur)
        else:
            # Otherwise keep adding to the current paragraph.
            cur["text"] += " " + text
    return paras


def yaml_str(s):
    """Wrap text in quotes for the note's header block, escaping quotes inside it."""
    return '"' + (s or "").replace("\\", "\\\\").replace('"', '\\"') + '"'


def make_note(info, segs):
    """Build the full text of one Markdown note.

    `info` is a dictionary of video details (title, date, chapters...) and
    `segs` is the list of caption pieces from vtt_segments().
    """
    vid = info["id"]
    title = info.get("title", vid)
    channel = info.get("channel") or info.get("uploader") or ""
    d = info.get("upload_date", "")                  # looks like "20261009"
    # Turn "20261009" into "2026-10-09" (only if it has exactly 8 characters).
    published = f"{d[:4]}-{d[4:6]}-{d[6:]}" if len(d) == 8 else ""
    desc = (info.get("description") or "").strip()
    # A one-line, 200-character version of the description for the header block.
    short_desc = re.sub(r"\s+", " ", desc)[:200].rstrip()
    chapters = info.get("chapters") or []

    # `out` collects the note one line at a time; we join the lines at the end.
    # The block between the two "---" lines is "front matter": details that
    # note-taking apps such as Obsidian read as properties.
    out = [
        "---",
        f"title: {yaml_str(title)}",
        f'source: "https://www.youtube.com/watch?v={vid}"',
        "author:",
        f'  - "[[{channel}]]"',
        f"published: {published}",
        f"created: {datetime.date.today().isoformat()}",
        f"description: {yaml_str(short_desc)}",
        "tags:",
        '  - "clippings"',
        "---",
        f"![](https://www.youtube.com/watch?v={vid})",
        "",
    ]
    if desc:
        out.append("  \n".join(desc.split("\n")))
        out.append("")
    out += ["## Transcript", ""]

    for p in build_paragraphs(segs, chapters):
        # When a paragraph starts a new chapter, print the chapter name as a heading.
        if p["new_chapter"] and p["chapter"] >= 0:
            out += [f"### {chapters[p['chapter']]['title']}", ""]
        # Each paragraph begins with its timestamp in bold, e.g. **1:15** · text
        out += [f"**{ts(p['t'])}** · {md_escape(p['text'])}", ""]
    return "\n".join(out).rstrip() + "\n"


def build_all(folder):
    """Rebuild every note in `folder` from its _raw_vtt captions (local, no network)."""
    raw = folder / "_raw_vtt"
    best = {}  # one caption file per video, preferring creator-made English
    for f in raw.glob("*.vtt"):
        # Pull the video id and the language code out of the file name.
        m = re.search(r"\[([\w-]{11})\]\.([\w-]+)\.vtt$", f.name)
        if not m:
            continue
        vid, lang = m.groups()
        # A lower rank is preferred. Unknown languages get 9 (the least preferred).
        rank = {"en": 0, "en-US": 1, "en-GB": 2, "en-orig": 3}.get(lang, 9)
        if vid not in best or rank < best[vid][0]:
            best[vid] = (rank, f)

    made, used = 0, set()    # made: notes written; used: file names already taken
    for vid, (_, f) in sorted(best.items()):    # "_" means "a value we do not need"
        stem = f.name.split(f" [{vid}]")[0]     # the file name before " [VIDEOID]"
        info_file = raw / f"{stem} [{vid}].info.json"
        if info_file.exists():
            # The .info.json file holds the title, description, chapters and so on.
            info = json.loads(info_file.read_text(encoding="utf-8"))
        else:  # captions without details: still make a basic note
            date, _, title = stem.partition(" - ")
            info = {"id": vid, "title": title, "upload_date": date}
        segs = vtt_segments(f.read_text(encoding="utf-8", errors="ignore"))
        if not segs:
            continue    # an empty caption file: nothing to write
        name = safe_filename(info.get("title", vid))
        if name.lower() in used:
            # Two videos with the same title: add the id so neither is overwritten.
            name = f"{name} ({vid})"
        used.add(name.lower())
        (folder / f"{name}.md").write_text(make_note(info, segs), encoding="utf-8")
        made += 1
    return made


# ----------------------------------------------------------------------------
# 4. Combine the notes into numbered master files (.md, optionally .pdf)
# ----------------------------------------------------------------------------
def load_videos(folder):
    """Read every note in the folder back in: title, date, link and transcript text."""
    vids, titles = [], set()

    def field(head, key):
        """Find a line such as 'title: "My video"' in the header and return its value."""
        # re.M lets ^ and $ match at the start and end of each line.
        m = re.search(rf'^{key}:\s*"?(.*?)"?\s*$', head, re.M)
        # Undo the backslashes that yaml_str() added in front of quotes.
        return re.sub(r'\\(["\\])', r"\1", m.group(1)) if m else ""

    for f in sorted(folder.glob("*.md")):
        if f.name.startswith("_"):
            continue    # skip helper files whose names start with an underscore
        text = f.read_text(encoding="utf-8", errors="ignore")
        if "\n## Transcript" not in text:
            continue    # not one of our notes
        # Split the note into the header part and the transcript part.
        head, body = text.split("\n## Transcript", 1)
        title = field(head, "title") or f.stem
        desc = ""
        if INCLUDE_DESCRIPTION:
            desc = re.split(r"^!\[\]\(.*?\)\s*$", head, maxsplit=1, flags=re.M)[-1].strip()
        # Chapter headings become "##" so they sit under each video's "#" title.
        body = re.sub(r"^### ", "## ", body.strip(), flags=re.M)
        # Remove the protective backslashes added by md_escape().
        body = body.replace("\\[", "[").replace("\\]", "]").replace("\\_", "_")
        vids.append({"title": title, "url": field(head, "source"),
                     "date": field(head, "published"), "desc": desc, "body": body})
        titles.add(title)

    # Notes from the very first version of the script were plain .txt files.
    # Read those too, unless the same video was already found as a .md note.
    for f in sorted(folder.glob("*.txt")):
        if f.name.startswith("_"):
            continue
        m = re.match(r"Title: (.*)\nURL: (.*)\nDate: (.*)\n\n(.*)", f.read_text(encoding="utf-8", errors="ignore"), re.S)
        if m and m.group(1) not in titles:
            vids.append({"title": m.group(1), "url": m.group(2), "date": m.group(3),
                         "desc": "", "body": m.group(4).strip()})

    # Count the words in each video (needed to decide where to split the parts).
    for v in vids:
        v["words"] = len(v["body"].split())
    # Sort oldest first. Videos with no date go last ("9999" sorts after any year).
    vids.sort(key=lambda v: (v["date"] or "9999", v["title"].lower()))
    return vids


def part_markdown(channel, vids, first_no, total_videos, part_no, n_parts):
    """Build the text of one master file (one 'part') from a list of videos."""
    d0 = vids[0]["date"] or "undated"     # date of the first video in this part
    d1 = vids[-1]["date"] or "undated"    # date of the last video in this part
    # The top of the file: a title, a one-line summary, then a contents list.
    lines = [f"# {channel} transcripts" + (f" - Part {part_no} of {n_parts}" if n_parts > 1 else ""),
             "",
             f"Videos {first_no}-{first_no + len(vids) - 1} of {total_videos}, oldest to newest "
             f"({d0} to {d1}). {sum(v['words'] for v in vids):,} words.",
             "", "## Contents", ""]
    # enumerate(list, start) counts the items for us, beginning at `start`.
    for i, v in enumerate(vids, first_no):
        lines.append(f"{i}. {v['title']} ({v['date'] or 'undated'})")
    lines += ["", "---", ""]
    # Then every video in full: its title, date and link, and its transcript.
    for i, v in enumerate(vids, first_no):
        lines += [f"# Video {i}: {v['title']}", "",
                  f"Published {v['date'] or 'unknown'} · {v['url']}", ""]
        if v["desc"]:
            lines += [v["desc"], ""]
        lines += [v["body"], "", "---", ""]
    return "\n".join(lines)


def write_pdf(md_text, pdf_path, title):
    """Turn Markdown text into a simple PDF using the reportlab library."""
    ensure("reportlab")
    # These imports are inside the function so reportlab is only needed when a
    # PDF is actually requested.
    from reportlab.lib.pagesizes import A4
    from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
    from reportlab.lib.units import mm
    from reportlab.platypus import HRFlowable, Paragraph, SimpleDocTemplate, Spacer

    # Three text styles: 1 = main heading, 2 = sub-heading, 0 = normal text.
    ss = getSampleStyleSheet()
    styles = {
        1: ParagraphStyle("h1", parent=ss["Heading1"], fontSize=15, leading=18, spaceBefore=10),
        2: ParagraphStyle("h2", parent=ss["Heading2"], fontSize=12, leading=15, spaceBefore=8),
        0: ParagraphStyle("b", parent=ss["BodyText"], fontSize=9.5, leading=13, spaceAfter=5),
    }

    def clean(t):  # the built-in PDF fonts only cover Western characters
        # Replace characters the font cannot show with "?", then make the text
        # safe for the PDF builder and turn **bold** markers into bold tags.
        t = t.encode("cp1252", "replace").decode("cp1252")
        return re.sub(r"\*\*(.+?)\*\*", r"<b>\1</b>", xml_escape(t))

    # The "story" is the list of items that make up the PDF, top to bottom.
    story = []
    for line in md_text.split("\n"):
        s = line.strip()
        if not s:
            continue    # ignore blank lines
        if s == "---":
            # A line of three dashes becomes a thin grey divider.
            story += [Spacer(1, 4), HRFlowable(width="100%", color="#bbbbbb"), Spacer(1, 4)]
            continue
        # Work out the heading level from the leading "# " or "## ".
        level = 1 if s.startswith("# ") else 2 if s.startswith("## ") else 0
        story.append(Paragraph(clean(s[level + 1:] if level else s), styles[level]))

    def footer(canvas, doc):
        """Draw the page number at the bottom of every page."""
        canvas.setFont("Helvetica", 8)
        canvas.drawCentredString(A4[0] / 2, 10 * mm, f"{title} - page {doc.page}")

    SimpleDocTemplate(str(pdf_path), pagesize=A4, title=title, leftMargin=18 * mm,
                      rightMargin=18 * mm, topMargin=16 * mm, bottomMargin=18 * mm
                      ).build(story, onFirstPage=footer, onLaterPages=footer)


def combine(folder, words_per_part, make_pdf):
    """Join all notes in `folder` into numbered master files inside 'Combined'."""
    # The channel name is the folder name without the word "Transcripts".
    channel = re.sub(r"\s*Transcripts$", "", folder.name, flags=re.I).strip() or folder.name
    vids = load_videos(folder)
    if not vids:
        print(f"  No notes found in '{folder.name}', skipping.")
        return

    # Split the videos into parts. We keep adding videos to the current part
    # until the next one would push it over the word limit, then start a new part.
    # A video is never cut in half. (A limit of 0 means "no limit", so one part.)
    parts, cur, cur_words = [], [], 0
    for v in vids:
        if words_per_part and cur and cur_words + v["words"] > words_per_part:
            parts.append(cur)
            cur, cur_words = [], 0
        cur.append(v)
        cur_words += v["words"]
    parts.append(cur)

    out_dir = folder / "Combined"
    out_dir.mkdir(exist_ok=True)
    for old in list(out_dir.glob("*.md")) + list(out_dir.glob("*.pdf")):
        old.unlink()  # clear parts from an earlier run with different settings

    first_no = 1    # the running video number, carried from one part to the next
    for n, part in enumerate(parts, 1):
        if len(parts) == 1:
            stem = f"{channel} - All Transcripts"
        else:
            d0, d1 = part[0]["date"] or "undated", part[-1]["date"] or "undated"
            # {n:02d} pads the number to 2 digits: 1 becomes 01.
            stem = f"{channel} - Part {n:02d} of {len(parts):02d} ({d0} to {d1})"
        stem = safe_filename(stem)
        md = part_markdown(channel, part, first_no, len(vids), n, len(parts))
        (out_dir / f"{stem}.md").write_text(md, encoding="utf-8")
        if make_pdf:
            write_pdf(md, out_dir / f"{stem}.pdf", stem)
        first_no += len(part)

    total = sum(v["words"] for v in vids)
    print(f"  {len(vids)} videos, {total:,} words -> {len(parts)} master file(s) in:\n  {out_dir}")


# ----------------------------------------------------------------------------
# Main: reads what you typed and runs the stages above in the right order
# ----------------------------------------------------------------------------
def find_folders():
    """Find every '... Transcripts' folder sitting next to this script."""
    return sorted(p for p in SCRIPT_DIR.iterdir()
                  if p.is_dir() and (p.name.lower().endswith(" transcripts") or p.name.lower() == "transcripts"))


def resolve_folder(text):
    """Turn typed text into a folder path, also trying the script's own folder."""
    p = Path(text)
    if not p.is_absolute() and not p.exists():
        p = SCRIPT_DIR / text
    return p


def main():
    # argparse reads the words typed after the script name and turns them into
    # named values we can use (args.pdf, args.words, and so on).
    ap = argparse.ArgumentParser(description="Download YouTube channel transcripts and combine them.")
    ap.add_argument("target", nargs="?", help="a channel URL to download, or an existing transcripts folder to combine")
    ap.add_argument("--pdf", action="store_true", help="also write PDF versions of the master files")
    ap.add_argument("--words", type=int, default=DEFAULT_WORDS_PER_PART,
                    help=f"max words per master file (default {DEFAULT_WORDS_PER_PART}; 0 = a single file)")
    ap.add_argument("--no-combine", action="store_true", help="download only, skip combining")
    ap.add_argument("--version", action="version", version=f"get_transcripts {__version__}")
    args = ap.parse_args()

    target = args.target or ""

    # If nothing was typed after the script name, show a simple menu.
    if not target:
        print("What would you like to do?\n"
              "  1) Download a channel, playlist or single video, then combine\n"
              "  2) Combine folders I've already downloaded (no download)")
        choice = ""
        while choice not in ("1", "2"):      # keep asking until the answer is 1 or 2
            choice = input("Type 1 or 2: ").strip()
        if choice == "1":
            # Keep asking until the text looks like a web address or an @handle.
            while not re.match(r"^(https?://|@)", target):
                target = input("Paste the YouTube address (channel, playlist or video): ").strip().strip('"')
        else:
            target = input("Paste the folder path\n"
                           "(or press Enter to combine every '... Transcripts' folder next to this script): "
                           ).strip().strip('"')

    if re.match(r"^(https?://|@)", target):
        # --- Download path: the target is a web address ---
        yt_dlp = ensure("yt_dlp", "yt-dlp")
        url = channel_url(target)
        try:
            kind, name, ids = list_source(yt_dlp, url)
        except Exception as e:
            # Something went wrong reading the address. If it looks like a
            # private page and no sign-in is set up, offer to use the browser's.
            if COOKIES_FROM_BROWSER or not needs_login(e):
                print(f"\nCouldn't read that address: {e}")
                return
            if not ask_for_browser():
                return
            try:
                kind, name, ids = list_source(yt_dlp, url)    # try once more
            except Exception as e2:
                print(f"\nStill couldn't read it: {e2}\n"
                      "Check the full link is copied, or make the playlist 'Unlisted' and try again.")
                return
        if not ids:
            print("No videos found at that address.")
            return
        # Create the output folder next to the script, named after the channel
        # (or playlist), for example "Channel Name Transcripts".
        folder = SCRIPT_DIR / f"{safe_filename(name)} Transcripts"
        folder.mkdir(exist_ok=True)
        print(f"{kind.capitalize()}: {name}\nFolder:  {folder}\n")
        if kind == "video":
            print("(A single video is added to its channel's folder. Use the playlist "
                  "address instead to download a whole playlist.)\n")
        try:
            download(folder, ids)
        except KeyboardInterrupt:
            # KeyboardInterrupt happens when you press Ctrl+C to stop the script.
            print("\nStopped. Run the same command again to carry on where it left off.")
            return
        folders = [folder]
    elif target:
        # --- Combine-only path: the target is a folder name or path ---
        folders = [resolve_folder(target)]
    else:
        # --- Nothing typed: combine every "... Transcripts" folder we can find ---
        folders = find_folders()
        if not folders:
            ap.print_help()
            print(f"\nNo '... Transcripts' folders found next to the script ({SCRIPT_DIR}).")
            return

    # Whichever path we took, finish by building notes and combining each folder.
    for folder in folders:
        if not folder.is_dir():
            print(f"Folder not found: {folder}")
            continue
        print(f"\n{folder.name}")
        # If the folder still holds raw caption files, (re)build the notes from them.
        if any((folder / "_raw_vtt").glob("*.vtt")):
            print(f"  Building notes from the downloaded captions: {build_all(folder)} notes")
        if not (args.no_combine and re.match(r"^(https?://|@)", target)):
            combine(folder, args.words, args.pdf)

    print("\nDone.")


# This line means: only run main() when the script is started directly (for
# example "py get_transcripts.py"), not when another script imports this file.
if __name__ == "__main__":
    main()
