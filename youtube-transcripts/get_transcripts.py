"""
Download every video transcript from a YouTube channel as Obsidian-style notes,
then combine them into a few big "master" files that are easy to upload and analyse.

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

WHERE THE RESULTS GO
  <Channel Name> Transcripts/             one .md note per video (Obsidian layout)
  <Channel Name> Transcripts/Combined/    the master files, split into numbered parts
"""

import argparse
import datetime
import html
import json
import re
import subprocess
import sys
from pathlib import Path
from xml.sax.saxutils import escape as xml_escape

SCRIPT_DIR = Path(__file__).resolve().parent

# If YouTube says "Sign in to confirm you're not a bot", set this to "chrome",
# "firefox", "edge" or "safari" (whichever browser you're signed in to YouTube with).
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
    """Import a module, installing it with pip first if it's missing."""
    try:
        return __import__(module)
    except ImportError:
        print(f"Installing {pip_name or module}...")
        subprocess.check_call([sys.executable, "-m", "pip", "install", "--upgrade", pip_name or module])
        return __import__(module)


def safe_filename(name):
    name = re.sub(r'[\\/:*?"<>|#^\[\]]', "", name).strip().rstrip(".")
    return name[:150] or "untitled"


# ----------------------------------------------------------------------------
# 1. Download captions + video details (description, chapters, date)
# ----------------------------------------------------------------------------
def channel_url(url):
    url = url.strip()
    if url.startswith("@"):
        url = "https://www.youtube.com/" + url
    if re.search(r"youtube\.com/(@[^/?]+|channel/[^/?]+|c/[^/?]+|user/[^/?]+)/?$", url):
        url = url.rstrip("/") + "/videos"
    return url


def source_kind(url):
    """'video', 'playlist' or 'channel', judged from the address."""
    if re.search(r"(youtu\.be/|[?&]v=|/shorts/|/live/)", url):
        return "video"      # includes a video link copied from inside a playlist
    if re.search(r"[?&]list=|/playlist", url):
        return "playlist"
    return "channel"


def needs_login(err):
    text = str(err).lower()
    return any(k in text for k in ("does not exist", "private", "sign in", "log in", "login"))


def ask_for_browser():
    """Private playlists are hidden unless the script borrows a browser's YouTube sign-in."""
    global COOKIES_FROM_BROWSER
    print("\nYouTube won't show that address to the script. This usually means the playlist is PRIVATE\n"
          "(or the link is cut short; a full playlist link has about 34 characters after list=).\n\n"
          "Easiest fix: on YouTube open the playlist, click Edit, set Visibility to 'Unlisted',\n"
          "then run the script again. No sign-in needed.\n\n"
          "Or let the script borrow your browser's YouTube sign-in. Close that browser first, then\n"
          "type its name: firefox (most reliable), chrome, edge, brave, opera, vivaldi or safari.")
    browser = input("\nBrowser name (or press Enter to stop): ").strip().lower()
    if not browser:
        return False
    COOKIES_FROM_BROWSER = browser
    return True


def list_source(yt_dlp, url):
    """Return (kind, folder name, [video ids]) using a quick listing (no downloads)."""
    kind = source_kind(url)
    opts = {"extract_flat": "in_playlist", "quiet": True, "skip_download": True,
            "sleep_interval_requests": 1, "ignore_no_formats_error": True,
            "noplaylist": kind == "video"}
    if COOKIES_FROM_BROWSER:
        opts["cookiesfrombrowser"] = (COOKIES_FROM_BROWSER,)
    print({"video": "Looking up the video...", "playlist": "Listing the playlist's videos...",
           "channel": "Listing the channel's videos..."}[kind])
    with yt_dlp.YoutubeDL(opts) as ydl:
        info = ydl.extract_info(url, download=False)
    if not info:
        raise SystemExit("Couldn't read that address. Check the link and try again.")
    channel = info.get("channel") or info.get("uploader") or ""
    entries = info.get("entries")
    if entries is None:  # a single video: it goes in its channel's folder
        ids = [info["id"]] if len(info.get("id") or "") == 11 else []
        return "video", channel or "Videos", ids
    ids = [e["id"] for e in entries if e and e.get("id") and len(e["id"]) == 11]
    if kind == "playlist":
        name = info.get("title") or channel or "Playlist"
    else:
        name = channel or re.sub(r"\s*-\s*Videos$", "", info.get("title") or "") or "Channel"
    return kind, name, ids


def caption_ids(raw):
    ids = set()
    for f in raw.glob("*.vtt"):
        m = re.search(r"\[([\w-]{11})\]\.[\w-]+\.vtt$", f.name)
        if m:
            ids.add(m.group(1))
    return ids


def download(folder, ids):
    yt_dlp = ensure("yt_dlp", "yt-dlp")
    raw = folder / "_raw_vtt"
    raw.mkdir(parents=True, exist_ok=True)

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
            "outtmpl": str(raw / "%(upload_date)s - %(title).120B [%(id)s].%(ext)s"),
            "ignoreerrors": True,
            "sleep_interval_subtitles": 2,      # be gentle so YouTube doesn't block you
            "sleep_interval_requests": 1,
        }
        if COOKIES_FROM_BROWSER:
            opts["cookiesfrombrowser"] = (COOKIES_FROM_BROWSER,)
        with yt_dlp.YoutubeDL(opts) as ydl:
            ydl.download([f"https://www.youtube.com/watch?v={i}" for i in todo])

    missing = [i for i in ids if i not in caption_ids(raw)]
    if missing:
        (folder / "_no_captions.txt").write_text(
            "\n".join(f"https://www.youtube.com/watch?v={i}" for i in missing) + "\n", encoding="utf-8")
        print(f"\n{len(missing)} videos have no captions (or failed). "
              f"Listed in _no_captions.txt. Re-run to retry them.")


# ----------------------------------------------------------------------------
# 2. Turn a .vtt caption file into timed segments of new text
# ----------------------------------------------------------------------------
CUE = re.compile(r"^(\d+):(\d{2}):(\d{2})\.(\d{3}) --> (\d+):(\d{2}):(\d{2})\.(\d{3})")
TAG = re.compile(r"<[^>]+>")


def secs(h, m, s, ms):
    return int(h) * 3600 + int(m) * 60 + int(s) + int(ms) / 1000


def vtt_segments(vtt):
    """Return [(start, end, text)] with YouTube's rolling duplicate lines removed."""
    segs, last = [], ""
    blocks = re.split(r"\n{2,}", vtt.replace("\r", ""))
    for block in blocks:
        lines = block.strip("\n").split("\n")
        m = next((CUE.match(l) for l in lines if CUE.match(l)), None)
        if not m:
            continue
        start, end = secs(*m.groups()[:4]), secs(*m.groups()[4:])
        text_lines = lines[[i for i, l in enumerate(lines) if CUE.match(l)][0] + 1:]
        text_lines = [html.unescape(TAG.sub("", l)).strip() for l in text_lines]
        text_lines = [l for l in text_lines if l]
        # auto captions repeat the previous line at the top of each cue: drop it
        while text_lines and text_lines[0] == last:
            text_lines.pop(0)
        if not text_lines:
            continue
        text = " ".join(text_lines)
        text = re.sub(r"^>>\s*", "", text).replace(" >> ", " ")
        if text:
            segs.append([start, end, text])
        last = text_lines[-1]
    return segs


# ----------------------------------------------------------------------------
# 3. Build an Obsidian Web Clipper style note for each video
# ----------------------------------------------------------------------------
def ts(t):
    t = int(t)
    h, m, s = t // 3600, t % 3600 // 60, t % 60
    return f"{h}:{m:02d}:{s:02d}" if h else f"{m}:{s:02d}"


def md_escape(text):
    return text.replace("[", "\\[").replace("]", "\\]").replace("_", "\\_")


def build_paragraphs(segs, chapters):
    """Group segments into paragraphs, each tagged with its start time and chapter."""
    starts = [c["start_time"] for c in chapters]
    chap_idx = 0 if starts and starts[0] <= (segs[0][0] if segs else 0) + 1 else -1
    paras, cur = [], None
    for i, (start, end, text) in enumerate(segs):
        # move to the next chapter once we've passed its start at a sentence boundary
        new_chapter = False
        while chap_idx + 1 < len(starts) and start >= starts[chap_idx + 1] and (
                cur is None or re.search(r"[.!?]\s*$", cur["text"])):
            chap_idx += 1
            new_chapter = True
        prev_end = segs[i - 1][1] if i else 0
        sentence_done = cur is not None and re.search(r"[.!?]\s*$", cur["text"])
        if (cur is None or new_chapter or
                (sentence_done and (len(cur["text"].split()) >= PARA_WORDS
                                    or start - prev_end >= PAUSE_SECONDS))):
            cur = {"t": start, "text": text, "chapter": chap_idx, "new_chapter": new_chapter or i == 0}
            paras.append(cur)
        else:
            cur["text"] += " " + text
    return paras


def yaml_str(s):
    return '"' + (s or "").replace("\\", "\\\\").replace('"', '\\"') + '"'


def make_note(info, segs):
    vid = info["id"]
    title = info.get("title", vid)
    channel = info.get("channel") or info.get("uploader") or ""
    d = info.get("upload_date", "")
    published = f"{d[:4]}-{d[4:6]}-{d[6:]}" if len(d) == 8 else ""
    desc = (info.get("description") or "").strip()
    short_desc = re.sub(r"\s+", " ", desc)[:200].rstrip()
    chapters = info.get("chapters") or []

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
        if p["new_chapter"] and p["chapter"] >= 0:
            out += [f"### {chapters[p['chapter']]['title']}", ""]
        out += [f"**{ts(p['t'])}** · {md_escape(p['text'])}", ""]
    return "\n".join(out).rstrip() + "\n"


def build_all(folder):
    """Rebuild every note in `folder` from its _raw_vtt captions (local, no network)."""
    raw = folder / "_raw_vtt"
    best = {}  # one caption file per video, preferring creator-made English
    for f in raw.glob("*.vtt"):
        m = re.search(r"\[([\w-]{11})\]\.([\w-]+)\.vtt$", f.name)
        if not m:
            continue
        vid, lang = m.groups()
        rank = {"en": 0, "en-US": 1, "en-GB": 2, "en-orig": 3}.get(lang, 9)
        if vid not in best or rank < best[vid][0]:
            best[vid] = (rank, f)

    made, used = 0, set()
    for vid, (_, f) in sorted(best.items()):
        stem = f.name.split(f" [{vid}]")[0]
        info_file = raw / f"{stem} [{vid}].info.json"
        if info_file.exists():
            info = json.loads(info_file.read_text(encoding="utf-8"))
        else:  # captions without details: still make a basic note
            date, _, title = stem.partition(" - ")
            info = {"id": vid, "title": title, "upload_date": date}
        segs = vtt_segments(f.read_text(encoding="utf-8", errors="ignore"))
        if not segs:
            continue
        name = safe_filename(info.get("title", vid))
        if name.lower() in used:
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
        m = re.search(rf'^{key}:\s*"?(.*?)"?\s*$', head, re.M)
        return re.sub(r'\\(["\\])', r"\1", m.group(1)) if m else ""

    for f in sorted(folder.glob("*.md")):
        if f.name.startswith("_"):
            continue
        text = f.read_text(encoding="utf-8", errors="ignore")
        if "\n## Transcript" not in text:
            continue
        head, body = text.split("\n## Transcript", 1)
        title = field(head, "title") or f.stem
        desc = ""
        if INCLUDE_DESCRIPTION:
            desc = re.split(r"^!\[\]\(.*?\)\s*$", head, maxsplit=1, flags=re.M)[-1].strip()
        body = re.sub(r"^### ", "## ", body.strip(), flags=re.M)
        body = body.replace("\\[", "[").replace("\\]", "]").replace("\\_", "_")
        vids.append({"title": title, "url": field(head, "source"),
                     "date": field(head, "published"), "desc": desc, "body": body})
        titles.add(title)

    # notes from the very first version of the script (plain .txt files)
    for f in sorted(folder.glob("*.txt")):
        if f.name.startswith("_"):
            continue
        m = re.match(r"Title: (.*)\nURL: (.*)\nDate: (.*)\n\n(.*)", f.read_text(encoding="utf-8", errors="ignore"), re.S)
        if m and m.group(1) not in titles:
            vids.append({"title": m.group(1), "url": m.group(2), "date": m.group(3),
                         "desc": "", "body": m.group(4).strip()})

    for v in vids:
        v["words"] = len(v["body"].split())
    vids.sort(key=lambda v: (v["date"] or "9999", v["title"].lower()))
    return vids


def part_markdown(channel, vids, first_no, total_videos, part_no, n_parts):
    d0 = vids[0]["date"] or "undated"
    d1 = vids[-1]["date"] or "undated"
    lines = [f"# {channel} transcripts" + (f" - Part {part_no} of {n_parts}" if n_parts > 1 else ""),
             "",
             f"Videos {first_no}-{first_no + len(vids) - 1} of {total_videos}, oldest to newest "
             f"({d0} to {d1}). {sum(v['words'] for v in vids):,} words.",
             "", "## Contents", ""]
    for i, v in enumerate(vids, first_no):
        lines.append(f"{i}. {v['title']} ({v['date'] or 'undated'})")
    lines += ["", "---", ""]
    for i, v in enumerate(vids, first_no):
        lines += [f"# Video {i}: {v['title']}", "",
                  f"Published {v['date'] or 'unknown'} · {v['url']}", ""]
        if v["desc"]:
            lines += [v["desc"], ""]
        lines += [v["body"], "", "---", ""]
    return "\n".join(lines)


def write_pdf(md_text, pdf_path, title):
    ensure("reportlab")
    from reportlab.lib.pagesizes import A4
    from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
    from reportlab.lib.units import mm
    from reportlab.platypus import HRFlowable, Paragraph, SimpleDocTemplate, Spacer

    ss = getSampleStyleSheet()
    styles = {
        1: ParagraphStyle("h1", parent=ss["Heading1"], fontSize=15, leading=18, spaceBefore=10),
        2: ParagraphStyle("h2", parent=ss["Heading2"], fontSize=12, leading=15, spaceBefore=8),
        0: ParagraphStyle("b", parent=ss["BodyText"], fontSize=9.5, leading=13, spaceAfter=5),
    }

    def clean(t):  # the built-in PDF fonts only cover Western characters
        t = t.encode("cp1252", "replace").decode("cp1252")
        return re.sub(r"\*\*(.+?)\*\*", r"<b>\1</b>", xml_escape(t))

    story = []
    for line in md_text.split("\n"):
        s = line.strip()
        if not s:
            continue
        if s == "---":
            story += [Spacer(1, 4), HRFlowable(width="100%", color="#bbbbbb"), Spacer(1, 4)]
            continue
        level = 1 if s.startswith("# ") else 2 if s.startswith("## ") else 0
        story.append(Paragraph(clean(s[level + 1:] if level else s), styles[level]))

    def footer(canvas, doc):
        canvas.setFont("Helvetica", 8)
        canvas.drawCentredString(A4[0] / 2, 10 * mm, f"{title} - page {doc.page}")

    SimpleDocTemplate(str(pdf_path), pagesize=A4, title=title, leftMargin=18 * mm,
                      rightMargin=18 * mm, topMargin=16 * mm, bottomMargin=18 * mm
                      ).build(story, onFirstPage=footer, onLaterPages=footer)


def combine(folder, words_per_part, make_pdf):
    channel = re.sub(r"\s*Transcripts$", "", folder.name, flags=re.I).strip() or folder.name
    vids = load_videos(folder)
    if not vids:
        print(f"  No notes found in '{folder.name}', skipping.")
        return

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

    first_no = 1
    for n, part in enumerate(parts, 1):
        if len(parts) == 1:
            stem = f"{channel} - All Transcripts"
        else:
            d0, d1 = part[0]["date"] or "undated", part[-1]["date"] or "undated"
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
# Main
# ----------------------------------------------------------------------------
def find_folders():
    return sorted(p for p in SCRIPT_DIR.iterdir()
                  if p.is_dir() and (p.name.lower().endswith(" transcripts") or p.name.lower() == "transcripts"))


def resolve_folder(text):
    p = Path(text)
    if not p.is_absolute() and not p.exists():
        p = SCRIPT_DIR / text
    return p


def main():
    ap = argparse.ArgumentParser(description="Download YouTube channel transcripts and combine them.")
    ap.add_argument("target", nargs="?", help="a channel URL to download, or an existing transcripts folder to combine")
    ap.add_argument("--pdf", action="store_true", help="also write PDF versions of the master files")
    ap.add_argument("--words", type=int, default=DEFAULT_WORDS_PER_PART,
                    help=f"max words per master file (default {DEFAULT_WORDS_PER_PART}; 0 = a single file)")
    ap.add_argument("--no-combine", action="store_true", help="download only, skip combining")
    args = ap.parse_args()

    target = args.target or ""
    if not target:
        print("What would you like to do?\n"
              "  1) Download a channel, playlist or single video, then combine\n"
              "  2) Combine folders I've already downloaded (no download)")
        choice = ""
        while choice not in ("1", "2"):
            choice = input("Type 1 or 2: ").strip()
        if choice == "1":
            while not re.match(r"^(https?://|@)", target):
                target = input("Paste the YouTube address (channel, playlist or video): ").strip().strip('"')
        else:
            target = input("Paste the folder path\n"
                           "(or press Enter to combine every '... Transcripts' folder next to this script): "
                           ).strip().strip('"')
    if re.match(r"^(https?://|@)", target):
        yt_dlp = ensure("yt_dlp", "yt-dlp")
        url = channel_url(target)
        try:
            kind, name, ids = list_source(yt_dlp, url)
        except Exception as e:
            if COOKIES_FROM_BROWSER or not needs_login(e):
                print(f"\nCouldn't read that address: {e}")
                return
            if not ask_for_browser():
                return
            try:
                kind, name, ids = list_source(yt_dlp, url)
            except Exception as e2:
                print(f"\nStill couldn't read it: {e2}\n"
                      "Check the full link is copied, or make the playlist 'Unlisted' and try again.")
                return
        if not ids:
            print("No videos found at that address.")
            return
        folder = SCRIPT_DIR / f"{safe_filename(name)} Transcripts"
        folder.mkdir(exist_ok=True)
        print(f"{kind.capitalize()}: {name}\nFolder:  {folder}\n")
        if kind == "video":
            print("(A single video is added to its channel's folder. Use the playlist "
                  "address instead to download a whole playlist.)\n")
        try:
            download(folder, ids)
        except KeyboardInterrupt:
            print("\nStopped. Run the same command again to carry on where it left off.")
            return
        folders = [folder]
    elif target:
        folders = [resolve_folder(target)]
    else:
        folders = find_folders()
        if not folders:
            ap.print_help()
            print(f"\nNo '... Transcripts' folders found next to the script ({SCRIPT_DIR}).")
            return

    for folder in folders:
        if not folder.is_dir():
            print(f"Folder not found: {folder}")
            continue
        print(f"\n{folder.name}")
        if any((folder / "_raw_vtt").glob("*.vtt")):
            print(f"  Building notes from the downloaded captions: {build_all(folder)} notes")
        if not (args.no_combine and re.match(r"^(https?://|@)", target)):
            combine(folder, args.words, args.pdf)

    print("\nDone.")


if __name__ == "__main__":
    main()
