#!/usr/bin/env python3
"""
artvee_offline_downloader.py

Downloads public-domain artwork from Artvee.com to a local folder for
personal, offline study/reference. Not for redistribution or commercial use.

This is a wrapper around the open-source `artvee-scraper` library (MIT), which
handles Artvee's HTML parsing:  https://github.com/zduclos/artvee-scraper

WHAT v5.1 CHANGES (versus v5):
  Comments and wording only: a beginner-friendly guide near the top, a few
  comments tidied, and the version number now comes from one place
  (__version__). Downloads work exactly as they did in v5.

WHAT v5 ADDED (versus v4):
  Fast resume. Works you already have are filtered out BEFORE any image is
  downloaded, and one progress line is printed per page, so a long resume
  never looks frozen.

WHAT v4 FIXES (versus v3):
  4. The "'charmap' codec can't encode character" failures (roughly 1 in 20).
     These were NOT download failures — the JPG was already safely on disk.
     What failed was writing the .json sidecar beside it: Path.write_text()
     with no encoding argument uses the OS default, which on Windows is
     cp1252, and cp1252 cannot represent ō / č / ł / ‘ ’ / zero-width spaces,
     so any title containing one raised UnicodeEncodeError. v4 writes that
     file as UTF-8 explicitly. Console/redirected output is forced to UTF-8
     too, so a stray character can't kill a long unattended run.

     Because already_done() requires BOTH the image and its .json, every one
     of those affected works is simply re-fetched on the next run and repaired
     automatically — no manual cleanup needed.

WHAT v3 FIXED (versus v1/v2):
  1. get_page_count() comma bug — the library reads a "…of 10,184 results" line
     and calls int() on it, which throws on the comma. v3 strips separators.
  2. Out-of-range pages return HTTP 404, not an empty page. v2's page-probe
     assumed "empty list" and so crashed with HTTPError on every category.
     v3 treats a 404 as "past the last page", which is what it actually means.
  3. Artvee has renamed/merged some categories (the site's Browse menu now
     shows "Plants & Animals", and there's no separate Animals / Botanical /
     Asian Art). The library's hardcoded slugs for those return a valid-but-
     empty page, so they silently yielded nothing. v3 discovers the real
     category slugs from Artvee's own navigation at runtime, so it stays
     correct even if they rename things again.

Other features:
  - Organized output: <output>/<category>/<artist>/<title>.jpg
  - Downscales to --max-width (default 1920), re-encodes as JPG (default) or EXR
  - A JSON sidecar per image (title/artist/date/source URL) for reference
  - Resumable: already-downloaded works are skipped on re-run
  - CSV log of every success/failure

Install (always required):
    pip install artvee-scraper Pillow

Install (only if you use --format exr):
    pip install opencv-python numpy

Usage:
    # See what categories the site actually offers right now, then exit:
    python artvee_offline_downloader.py --list-categories

    # Full site, 2 polite threads, 1920px JPGs (all defaults):
    python artvee_offline_downloader.py

    # Specific categories only:
    python artvee_offline_downloader.py -c landscape -c still-life

    # Custom output folder:
    python artvee_offline_downloader.py -o art_reference

Be considerate: a full-site run touches every image on Artvee (well over
250,000 works). Keep --worker-threads low (2-3), run it once, and let the
resumability handle interruptions rather than re-running from scratch. Check
Artvee's Terms of Service before a full-site scrape; this is intended for
personal, non-commercial, non-distributed study use only.
"""

# =============================================================================
# HOW THE SCRIPT IS ORGANISED (a map for newcomers)
# =============================================================================
# The script is a wrapper around a library called artvee-scraper. A "library"
# is a toolkit of code written by someone else. This script uses the library to
# do the browsing, and adds fixes and extras of its own. The parts below follow
# the order of the file:
#
#   PART 1  Imports        - load the toolkits the script needs
#   PART 2  Settings       - version number, shared counters, and a small
#                            "stand-in" class for website categories
#   PART 3  Repairs        - replacement versions of three library methods that
#                            had problems (category list, page counting, and
#                            skipping works that were already downloaded)
#   PART 4  File paths     - decide where each image and its details file are saved
#   PART 5  Images         - shrink an image and save it as a JPG (or EXR)
#   PART 6  Callback       - the function that runs once per artwork: it saves
#                            the image, writes the details file, and logs the result
#   PART 7  Main program   - read your options, set everything up, and start
#
# Vocabulary used in the comments:
#   - function     : a named, reusable block of code, written as "def name(...):"
#   - class        : a template for creating objects that bundle data with behaviour
#   - thread       : one of several tasks running at the same time
#   - lock         : a guard that lets only one thread change shared data at a time
#   - callback     : a function you hand to a library so the library can call it later
#   - slug         : the short, lower-case name of an item as it appears in a web address
#   - sidecar file : a small extra file (here, .json) saved next to an image, holding its details
#   - HTTP 404     : the standard "page not found" reply from a website

# =============================================================================
# PART 1: IMPORTING THE TOOLBOXES (MODULES)
# =============================================================================
# Python comes with many built-in toolboxes for specific jobs so we don't have
# to write everything from scratch.

import argparse           # Parses command-line instructions (like typing '-c landscape' or '--max-width 1920' in the terminal)
import csv                # Handles reading and writing spreadsheet-style .csv log files
import io                 # Wraps raw downloaded computer bytes into a pretend "file" so Pillow can open it in memory
import json               # Reads and writes .json metadata files (saving the artwork title and artist details)
import math               # Handles math calculations (we use it here to divide item counts into pages)
import os                 # Lets us talk to the computer's operating system (to set environment variables)
import re                 # 'Regular Expressions': advanced text-searching to clean up messy filenames and URLs
import sys                # Controls the core Python system, letting us print errors and stop the script entirely
import threading          # Allows 'locks' so multiple download threads don't trip over each other when writing to the log
from pathlib import Path  # A modern, smart way to handle file and folder paths across Windows, Mac, and Linux

# OpenCV only allows reading/writing EXR files if this environment variable is set to "1" *before* cv2 is imported,
# so it's set here unconditionally — harmless if --format exr is never used and cv2 never actually gets imported.
os.environ.setdefault("OPENCV_IO_ENABLE_OPENEXR", "1")


# -----------------------------------------------------------------------------
# THE WINDOWS ENCODING FIX
# -----------------------------------------------------------------------------
# Artwork titles routinely contain characters outside Windows' default cp1252 codepage (ō, č, ł, ‘ ’, zero-width
# spaces, CJK). Printing those to a cp1252 stream raises UnicodeEncodeError — which matters most when you redirect
# output to a file (e.g. "> run.log"), since redirected output does NOT use the Unicode-aware console writer.
# Switching both streams to UTF-8 with errors="replace" means an odd character can never kill a long overnight run.
for _stream in (sys.stdout, sys.stderr):          # apply the same treatment to normal output and error output
    try:
        _stream.reconfigure(encoding="utf-8", errors="replace")   # available on Python 3.7+; replaces anything unencodable
    except Exception:                              # older Python, or a stream that doesn't support reconfigure
        pass                                        # not fatal — just carry on with whatever encoding was already in use


# -----------------------------------------------------------------------------
# LOADING EXTERNAL LIBRARIES
# -----------------------------------------------------------------------------
# These libraries don't come with Python. We use 'try / except' blocks so that
# if the user forgot to install them, the script explains how to fix it instead
# of just stopping with an unhelpful error message.

try:
    from artvee_scraper.scraper import ArtveeScraper       # the class that does the actual crawling/downloading of Artvee's site
    from artvee_scraper.artwork import CategoryType         # the library's hardcoded enum of category slugs (now partly out of date)
    from artvee_scraper.artvee_client import ArtveeClient   # the low-level HTTP/HTML client; we patch one of its methods below
except ImportError:
    # If the user doesn't have the scraper installed, print a friendly error message to the system error stream:
    print(
        "Missing dependency. Install it first with:\n"
        "    pip install artvee-scraper\n",
        file=sys.stderr,
    )
    sys.exit(1)                                          # Stop the script entirely with an error code (1)

try:
    from PIL import Image as PILImage   # Pillow's Image module, used for resizing and re-encoding every downloaded image
except ImportError:
    # If the user doesn't have the image editor installed, tell them how to get it:
    print(
        "Missing dependency. Install it first with:\n"
        "    pip install Pillow\n",
        file=sys.stderr,
    )
    sys.exit(1)                                          # Stop the script entirely

import requests                        # used to catch requests.exceptions.HTTPError (specifically 404s) during page probing
from bs4 import BeautifulSoup          # used to parse Artvee's HTML for the item count and the category navigation links


# =============================================================================
# PART 2: GLOBAL VARIABLES & DATA MODELS
# =============================================================================

__version__ = "5.1"   # the version number of this script; change it here and it updates everywhere
SCRIPT_VERSION = f"artvee_offline_downloader v{__version__} — fast resume + per-page progress"   # printed at startup so you can confirm which copy is running

# When downloading 10 images at the same time, we need a 'Lock'.
# This prevents two threads from writing "downloaded!" to our stats at the exact same millisecond and corrupting the number.
_lock = threading.Lock()                                  # guards the shared _stats dict below from race conditions across worker threads
_stats = {"downloaded": 0, "skipped": 0, "failed": 0}      # running totals, updated inside the callback and printed once at the end


# -----------------------------------------------------------------------------
# FIXING BROKEN CATEGORIES
# -----------------------------------------------------------------------------
# The artvee-scraper library has a hardcoded list of categories built-in, but
# the real website changed them! This class is a "stand-in" that perfectly
# mimics the library's objects but allows us to use live, updated categories.
class SiteCategory:
    """A stand-in for the library's CategoryType enum, holding a category slug
    discovered from Artvee's live navigation rather than hardcoded at build time.

    The library only ever uses a category in two ways: str(category) to build a
    URL, and category.value to label the artwork. This class supplies both, so it
    drops straight into the library with no other changes."""

    def __init__(self, slug: str):
        self.value = slug                                  # the URL slug, e.g. "landscape" — also used as the artwork's category label
        self.name = slug.upper().replace("-", "_")          # an enum-style uppercase name, e.g. "STILL_LIFE", for display purposes only

    def __str__(self) -> str:
        return self.value                                   # the library f-strings the category straight into a URL, so this must be the slug

    def __repr__(self) -> str:
        return f"SiteCategory({self.value!r})"              # nicer output if one of these ever gets printed during debugging


# =============================================================================
# PART 3: REPAIRING AND PATCHING THE SCRAPER
# =============================================================================

def discover_categories(client: ArtveeClient):
    """Fetch Artvee's homepage and pull the real category slugs out of its own
    navigation links (they look like https://artvee.com/c/<slug>/).

    This exists because Artvee has renamed and merged categories since the
    library was published — 'animals', 'botanical' and 'asian-art' no longer
    resolve to real listings, and the site now shows a combined 'Plants &
    Animals' instead. Reading the slugs off the live site keeps this working
    regardless of what they're called today."""

    try:
        resp = client._session.get("https://artvee.com/", timeout=client._timeout)   # reuse the library's own configured session (headers/retries)
        resp.raise_for_status()                                                       # raise if the homepage itself failed to load
        soup = BeautifulSoup(resp.content, "html.parser")                             # parse the returned HTML

        slugs = []                                                                     # collected slugs, kept in the order the site lists them
        for a in soup.find_all("a", href=True):                                        # walk every link on the page
            # Match links that look exactly like category folders (e.g. /c/landscape/):
            m = re.match(r"^https?://(?:www\.)?artvee\.com/c/([^/?#]+)/?$", a["href"])
            if m:                                                                       # if this link is a category archive...
                slug = m.group(1)                                                       # ...pull the slug out of the URL
                if slug not in slugs:                                                   # skip duplicates (the same category appears in several menus)
                    slugs.append(slug)                                                  # keep it, preserving the site's own ordering

        return slugs                                                                    # hand back whatever we found (possibly an empty list)
    except Exception as e:                                                              # any network/parse problem shouldn't be fatal...
        print(f"[cats]    Category discovery failed ({e}); falling back to the library's built-in list.", flush=True)
        return []                                                                        # ...we just fall back to the hardcoded list instead


# A reference to the library's ORIGINAL, unfiltered get_metadata, captured before we replace it further down.
# The page-counting probe below must use this rather than the filtered version, otherwise a page whose images
# you have already downloaded would come back empty and the probe would wrongly conclude it hit the last page.
_ORIGINAL_GET_METADATA = ArtveeClient.get_metadata


def _page_has_items(client: ArtveeClient, category, page: int) -> bool:
    """Return True if the given page of a category has any artwork on it.

    Artvee returns HTTP 404 for a page number past the end of a category, and the
    library's get_metadata() calls raise_for_status(), so that surfaces as an
    HTTPError rather than an empty list. Treating a 404 as 'no items here' is what
    makes the page-count search below terminate correctly instead of crashing."""

    try:
        return bool(_ORIGINAL_GET_METADATA(client, category, page))   # deliberately UNFILTERED — see the note above
    except requests.exceptions.HTTPError as e:                # an HTTP error status came back
        status = getattr(e.response, "status_code", None)     # find out which status it was
        if status == 404:                                      # 404 specifically means "this page number is past the end"
            return False                                       # so report it as having no items, ending the search cleanly
        raise                                                  # anything else (500, 403, …) is a real problem — let it propagate


def _patched_get_page_count(self, category) -> int:
    """Replacement for ArtveeClient.get_page_count().

    Strategy 1 (fast): read the "Showing 1-70 of 10,184 results" line off page 1
    and strip the thousands separators before parsing. The library fails here
    because it calls int("10,184") directly.

    Strategy 2 (fallback): if that line is missing or unparseable, find the last
    real page by doubling the page number until a 404, then binary-searching the
    boundary. Slower (a couple dozen extra requests) but essentially unbreakable."""

    print(f"[pages]   Working out how many pages '{category}' has...", flush=True)   # progress feedback, since this can take a few seconds

    # --- Strategy 1: parse the item count off page 1 -----------------------------
    try:
        url = f"https://artvee.com/c/{category}/page/1/?per_page={self._ITEMS_PER_PAGE}"   # same URL shape the library itself uses
        resp = self._session.get(url, timeout=self._timeout)                                # fetch page 1 of this category
        resp.raise_for_status()                                                             # raise if the category page itself 404s or errors
        soup = BeautifulSoup(resp.content, "html.parser")                                   # parse the HTML

        result_el = soup.find("p", class_="woocommerce-result-count")                       # the "Showing X-Y of Z results" paragraph, if present
        if result_el:                                                                        # it isn't present on every category page
            numbers = re.findall(r"[\d][\d,]*", result_el.get_text())                        # pull out every number-ish token, commas included
            if numbers:                                                                       # the total is the largest of them (e.g. 10,184 in "1-70 of 10,184")
                total_items = max(int(n.replace(",", "")) for n in numbers)                   # strip commas THEN convert — this is the actual v1 bugfix
                pages = math.ceil(total_items / self._ITEMS_PER_PAGE)                          # convert an item count into a page count
                if pages > 0:                                                                  # sanity check before trusting it
                    print(f"[pages]   '{category}': {pages} pages ({total_items:,} items). Starting downloads.", flush=True)
                    return pages                                                               # fast path succeeded; no probing needed
    except requests.exceptions.HTTPError:                                                      # the category page itself doesn't exist (renamed/removed)
        print(f"[pages]   '{category}': category page not found — skipping.", flush=True)      # say so plainly rather than failing the whole run
        return 0                                                                                # 0 pages means the library's loop simply does nothing
    except Exception:                                                                           # anything else — fall through to the probe below
        pass                                                                                    # deliberately silent; Strategy 2 will report what it finds

    # --- Strategy 2: probe for the last real page --------------------------------
    if not _page_has_items(self, category, 1):     # does page 1 have any artwork at all?
        print(f"[pages]   WARNING: '{category}' has no items on page 1 — skipping this category.", flush=True)
        return 0                                     # nothing to scrape here

    lo = 1                                           # highest page number CONFIRMED to have items
    hi = 2                                           # next page number to test
    hard_cap = 8000                                  # safety ceiling: 8000 pages * 70 items/page = 560,000 items, far above any real category

    while hi <= hard_cap and _page_has_items(self, category, hi):   # keep doubling while the far page still has items
        lo = hi                                       # that page had items, so raise the known-good floor up to it
        hi *= 2                                        # and double the search distance again

    if hi > hard_cap:                                  # doubling ran past the safety ceiling without finding the end
        print(f"[pages]   '{category}': hit the {hard_cap}-page safety cap.", flush=True)
        return hard_cap                                # cap it rather than looping (near-)forever

    # This binary search efficiently narrows down the exact page where items end and 404s begin.
    while lo + 1 < hi:                                 # binary-search the exact boundary between "has items" (lo) and "empty" (hi)
        mid = (lo + hi) // 2                           # try the midpoint page
        if _page_has_items(self, category, mid):       # if it has items...
            lo = mid                                    # ...that becomes the new known-good floor
        else:
            hi = mid                                    # ...otherwise that becomes the new known-empty ceiling

    print(f"[pages]   '{category}': {lo} pages (~{lo * 70:,} items). Starting downloads.", flush=True)   # report before real scraping begins
    return lo                                          # lo now sits exactly on the last page that actually has items

# MONKEY-PATCHING: replacing a method inside a library with our own corrected version, without editing
# the library's files. Python lets us do this by assigning a new function to the class at run time.
ArtveeClient.get_page_count = _patched_get_page_count   # install the fix by replacing the broken method on the class itself


# --- Skip-before-download filter (the big speed win when resuming) ----------
#
# The library's worker downloads the full image FIRST and only then hands it to our
# callback — so a callback that says "I already have this, skip it" has already paid
# the full network cost. On a resumed run that means re-downloading everything you
# already had, silently (skips print nothing), which looks exactly like the program
# having hung.
#
# This wraps get_metadata() so that already-downloaded works are dropped from the
# page's work list BEFORE the download pool ever sees them. Resuming now costs one
# lightweight listing request per page instead of 70 full image downloads.
_BASE_DIR = None    # set by main() once the output directory is known
_FMT = "jpg"        # set by main() to whichever image format this run is writing


def _patched_get_metadata(self, category, page):
    tuples = _ORIGINAL_GET_METADATA(self, category, page)   # ask the library for this page's artwork listing as normal

    if _BASE_DIR is None:            # configuration hasn't been applied yet (e.g. during the page-count probe)
        return tuples                 # so hand back the unfiltered list untouched

    kept = []                         # works we still need to download
    have = 0                          # count of works already on disk, for the progress line

    for artwork_metadata, img_metadata in tuples:                 # examine each work the page listed
        image_path, meta_path = compute_paths(                     # work out where it WOULD be saved...
            _BASE_DIR, artwork_metadata.category, artwork_metadata.artist,
            artwork_metadata.title, artwork_metadata.resource, _FMT,
        )                                                          # ...without creating any folders
        if already_done(image_path, meta_path):                    # both the image and its sidecar are already present
            have += 1                                              # count it...
            with _lock:                                            # ...lock the shared stats...
                _stats["skipped"] += 1                             # ...and record it as skipped, having spent no bandwidth at all
        else:
            kept.append((artwork_metadata, img_metadata))          # otherwise it still needs fetching

    print(                                                          # one progress line per page, so a long resume never looks frozen
        f"[page]    {category} p{page}: {len(tuples)} listed — {have} already have, {len(kept)} to fetch",
        flush=True,
    )
    return kept                                                     # only the outstanding works reach the download pool

# Apply the patch, so works we already have are skipped before any image is downloaded.
ArtveeClient.get_metadata = _patched_get_metadata   # install the filter


# =============================================================================
# PART 4: FILE PATHS & FOLDER MANAGEMENT
# =============================================================================

def sanitize(name: str) -> str:
    """Make a string safe to use as a file/folder name."""
    name = name.strip() or "unknown"                       # trim whitespace; fall back to the literal word "unknown" if the string was empty
    name = re.sub(r'[<>:"/\\|?*\x00-\x1f]', "_", name)     # replace characters illegal in Windows/macOS/Linux filenames with an underscore
    name = re.sub(r"\s+", " ", name)                       # collapse any run of consecutive whitespace down to a single space
    return name[:150]                                      # truncate long titles so the full path stays under OS path-length limits


def compute_paths(base_dir: Path, category, artist, title, resource, fmt: str, make_dirs: bool = False):
    """Work out where a given artwork's image and metadata files belong.

    Kept separate from build_paths() so the pre-download filter can ask 'do I already
    have this?' WITHOUT creating empty folders for works it is about to skip."""

    # We build the folder structure like this: /Downloads/Category/Artist Name/
    category_dir = base_dir / sanitize(category or "uncategorized")    # e.g. artvee_downloads/Landscape
    artist_dir = category_dir / sanitize(artist or "Unknown Artist")   # e.g. artvee_downloads/Landscape/Vincent van Gogh
    if make_dirs:                                                      # only actually create the folders when we're about to write into them
        artist_dir.mkdir(parents=True, exist_ok=True)                  # create that folder tree now if it doesn't already exist

    stem = sanitize(f"{title} ({resource})")   # filename base, e.g. "The Starry Night (the-starry-night)"
    image_path = artist_dir / f"{stem}.{fmt}"  # full path to the image file, using the chosen output format's extension
    meta_path = artist_dir / f"{stem}.json"    # full path to this image's metadata sidecar file
    return image_path, meta_path               # hand both paths back to whoever called this function


def build_paths(base_dir: Path, artwork, fmt: str):
    return compute_paths(                                  # same calculation as above...
        base_dir, artwork.category, artwork.artist,        # ...reading the fields off a fully-built Artwork object...
        artwork.title, artwork.resource, fmt,
        make_dirs=True,                                    # ...and creating the folders, since the caller is about to save files
    )


def already_done(image_path: Path, meta_path: Path) -> bool:
    return image_path.exists() and meta_path.exists()   # only treat it as "already downloaded" if BOTH the image and its metadata exist


# =============================================================================
# PART 5: IMAGE PROCESSING (RESIZING & SAVING)
# =============================================================================

def load_and_resize(raw_bytes: bytes, max_width: int):
    """Open raw image bytes, convert to RGB, and downscale if wider than
    max_width. Never upscales. Returns a PIL Image."""
    img = PILImage.open(io.BytesIO(raw_bytes))    # wrap the raw downloaded bytes in a file-like object so Pillow can decode them
    img.load()                                    # force Pillow to fully decode the pixel data now, surfacing corrupt-file errors early
    if img.mode not in ("RGB",):                  # check whether the image is already plain RGB (as opposed to CMYK, grayscale, RGBA, etc.)
        img = img.convert("RGB")                  # convert to RGB so the JPEG/EXR saving functions below behave predictably

    if max_width and img.width > max_width:                           # only resize if a limit was given AND the image is currently wider than it
        new_height = round(img.height * (max_width / img.width))      # scale the height by the same ratio as the width, to preserve proportions
        img = img.resize((max_width, new_height), PILImage.LANCZOS)   # actually perform the resize, using a high-quality downscaling filter

    return img   # hand back the (possibly resized) image, ready to be saved to disk


def save_as_jpg(img, out_path: Path, quality: int):
    img.save(out_path, "JPEG", quality=quality, optimize=True)   # write the image to disk as a compressed JPEG at the requested quality level


def save_as_exr(img, out_path: Path):
    """Save a PIL RGB image as a 16-bit half-float EXR using OpenCV."""
    try:
        import cv2            # OpenCV, imported lazily here so it's only required at all when --format exr is actually used
        import numpy as np    # used to convert the PIL image into the numeric array format cv2's imwrite expects
    except ImportError as e:
        raise RuntimeError(                                    # re-raise as a clearer, more actionable message for the user...
            "EXR output requires extra packages. Install them with:\n"
            "    pip install opencv-python numpy"
        ) from e                                                # ...while keeping the original ImportError attached for debugging

    arr = np.asarray(img, dtype=np.float32) / 255.0   # convert 0-255 integer pixel values into 0.0-1.0 floats, as an HxWx3 array
    bgr = arr[:, :, ::-1]                              # reverse the last axis to swap RGB -> BGR, the channel order cv2 expects

    ok = cv2.imwrite(
        str(out_path),                                       # destination file path (cv2 wants a plain string, not a pathlib.Path)
        bgr,                                                  # the pixel data to write
        [cv2.IMWRITE_EXR_TYPE, cv2.IMWRITE_EXR_TYPE_HALF],    # tells cv2 to encode as half-float (16-bit) rather than full 32-bit float
    )
    if not ok:                                           # cv2.imwrite returns False on failure instead of raising an exception itself
        raise RuntimeError(                              # so we raise our own exception here, with a hint at the likely cause
            "cv2.imwrite failed to write EXR. Make sure OPENCV_IO_ENABLE_OPENEXR=1 "
            "is set (this script sets it automatically before importing cv2) and "
            "that your opencv-python build supports the EXR codec."
        )


# =============================================================================
# PART 6: THE DOWNLOAD CALLBACK (THE CORE ACTION)
# =============================================================================

def make_callback(base_dir: Path, csv_writer, csv_lock, max_width: int, fmt: str, jpg_quality: int):
    # This is the function that the scraper will call every single time it finishes downloading an image.
    def on_event(artwork, exception):                        # ArtveeScraper calls this once per artwork, from a worker thread
        if exception is not None:                             # the library hit an error scraping or downloading this particular artwork
            with _lock:                                        # acquire the stats lock so the increment below is thread-safe
                _stats["failed"] += 1                          # count this artwork as a failure
            with csv_lock:                                     # acquire the log-file lock so concurrent threads don't interleave writes
                csv_writer.writerow(
                    [
                        getattr(artwork, "resource", ""),      # unique id, if the library managed to populate it despite the error
                        getattr(artwork, "title", ""),         # title, if known
                        getattr(artwork, "artist", ""),        # artist, if known
                        getattr(artwork, "url", ""),           # source URL, if known
                        "FAILED",                              # status column
                        str(exception),                        # the actual error text, for troubleshooting later
                    ]
                )
            print(f"[FAILED]  {getattr(artwork, 'title', '?')}: {exception}")   # also echo the failure to the console in real time
            return                                              # nothing more to do for an artwork that failed to scrape at all

        image_path, meta_path = build_paths(base_dir, artwork, fmt)   # work out where this artwork's image + metadata file should live

        if already_done(image_path, meta_path):    # if both files already exist from a previous run of this script...
            with _lock:                              # ...lock the stats dict...
                _stats["skipped"] += 1               # ...and count it as skipped, without re-downloading or re-processing anything
            return                                    # move on without touching the network or disk further for this artwork

        try:
            if artwork.image and artwork.image.raw:                   # only proceed if the library actually returned image bytes
                img = load_and_resize(artwork.image.raw, max_width)   # decode the raw bytes and downscale them if needed
                if fmt == "jpg":                                      # branch on the user's chosen output format
                    save_as_jpg(img, image_path, jpg_quality)         # write it out as a compressed JPEG
                elif fmt == "exr":
                    save_as_exr(img, image_path)                      # write it out as a half-float EXR instead
                else:
                    raise ValueError(f"Unknown format: {fmt}")        # defensive check; argparse's choices= should make this unreachable

            # Collect the metadata to save alongside the image for offline study.
            meta = {
                "title": artwork.title,                # the artwork's title, as scraped from Artvee
                "artist": artwork.artist,               # the artist's name
                "date": artwork.date,                   # completion year/date range, if known
                "origin": artwork.origin,               # artist nationality/lifespan, if known
                "category": artwork.category,           # which Artvee category this artwork came from
                "source_url": artwork.url,              # the original Artvee page, kept for citation/reference later
                "resource_id": artwork.resource,        # Artvee's unique identifier for this artwork
                "max_width": max_width,                 # records what resize setting was used, for your own future reference
                "format": fmt,                          # records what output format was used
                "note": "Downloaded and downscaled for personal offline study/reference "
                        "only. Not for redistribution or commercial use.",   # reminder of intended use, baked into every sidecar file
            }
            meta_path.write_text(                                        # write that metadata out as a nicely formatted JSON file
                json.dumps(meta, indent=2, ensure_ascii=False),           # ensure_ascii=False keeps accents/CJK readable instead of escaping them
                encoding="utf-8",                                         # REQUIRED: without this, Windows defaults to cp1252 ('charmap'), which cannot
            )                                                             # represent characters like ō, č, ł or a zero-width space, and raises an error

            with _lock:                                   # lock the stats dict again...
                _stats["downloaded"] += 1                 # ...and count this as a successful download
            with csv_lock:                                # lock the log file again...
                csv_writer.writerow(
                    [artwork.resource, artwork.title, artwork.artist, artwork.url, "OK", ""]   # ...and record a success row
                )
            print(f"[OK]      {artwork.artist} — {artwork.title}")   # echo the success to the console in real time
        except Exception as e:  # noqa: BLE001                        # catch anything that went wrong while resizing/saving/writing to disk
            with _lock:                                   # lock the stats dict...
                _stats["failed"] += 1                     # ...and count this as a failure instead of a success
            with csv_lock:                                # lock the log file...
                csv_writer.writerow(
                    [artwork.resource, artwork.title, artwork.artist, artwork.url, "FAILED", str(e)]   # ...and record the failure + reason
                )
            print(f"[FAILED]  {artwork.title}: {e}")      # echo the failure to the console in real time

    return on_event   # hand the fully configured callback function back to main(), to be registered with the scraper


# =============================================================================
# PART 7: STARTUP CONFIGURATION & THE MAIN PROGRAM
# =============================================================================

def parse_args():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)   # set up the parser; --help shows this file's docstring

    p.add_argument(
        "-o", "--output", default="./artvee_downloads",          # short and long flag spellings, with a default output folder
        help="Output directory (default: ./artvee_downloads)",   # text shown in --help
    )                                                             # defines where all downloaded images/metadata/logs get written

    p.add_argument(
        "-c", "--category", action="append", dest="categories",   # "append" lets -c be passed multiple times to select several categories
        metavar="SLUG",                                            # shown in --help as -c SLUG; free-form now, since the site's list can change
        help="Category slug to scrape (repeatable), e.g. -c landscape -c posters. "
             "Default: every category found on the site. Run --list-categories to see them.",   # text shown in --help
    )                                                              # omitting this flag entirely means "every discovered category"

    p.add_argument(
        "--list-categories", action="store_true",                 # a flag that takes no value; just present or absent
        help="Print the category slugs currently available on Artvee, then exit without downloading anything.",
    )                                                              # useful for checking what's scrapeable before committing to a long run

    p.add_argument(
        "--no-discover", action="store_true",                     # a flag that takes no value; just present or absent
        help="Skip live category discovery and use the artvee-scraper library's built-in (possibly outdated) category list instead.",
    )                                                              # an escape hatch in case discovery ever misbehaves

    p.add_argument(
        "-t", "--worker-threads", type=int, default=2,            # how many artworks to process concurrently; default kept low and polite
        help="Concurrent worker threads, 1-10 (default: 2). Keep this low "
             "to be polite to Artvee's servers, especially for a full-site run.",   # text shown in --help
    )                                                              # higher = faster scraping but more load placed on Artvee's servers

    p.add_argument(
        "--max-width", type=int, default=1920,                    # pixel-width ceiling used by load_and_resize(); 1920 by default
        help="Resize so image width never exceeds this many pixels (default: 1920). "
             "Aspect ratio is preserved; narrower images are left as-is (never upscaled). "
             "Pass 0 to disable resizing.",                        # text shown in --help
    )                                                              # this is what actually shrinks file sizes versus the raw download

    p.add_argument(
        "--format", choices=["jpg", "exr"], default="jpg",        # restricts to the two formats this script can write; jpg is the default
        help="Output image format after resizing (default: jpg). 'jpg' = standard "
             "compressed JPEG, much smaller and recommended. 'exr' = 16-bit half-float "
             "OpenEXR, usually LARGER for this material; only use if a downstream tool "
             "requires .exr. Requires 'pip install opencv-python numpy'.",   # text shown in --help
    )                                                              # determines whether save_as_jpg() or save_as_exr() is used for every image

    p.add_argument(
        "--jpg-quality", type=int, default=88,                    # JPEG quality setting, 1-100; only meaningful when --format jpg is used
        help="JPEG quality 1-100, used only with --format jpg (default: 88).",   # text shown in --help
    )                                                              # ignored entirely when --format exr is selected

    return p.parse_args()   # actually read sys.argv and return the populated Namespace of all the options above


def resolve_categories(args, client):
    """Work out the final list of categories to scrape, as SiteCategory objects."""

    builtin = [c.value for c in CategoryType]     # the library's hardcoded slugs, e.g. ["abstract", "figurative", …]

    if args.no_discover:                           # the user explicitly asked to skip live discovery
        slugs = builtin                             # so just use the library's built-in list, outdated or not
        print(f"[cats]    Using the library's built-in category list ({len(slugs)} categories).", flush=True)
    else:
        discovered = discover_categories(client)    # read the real slugs off Artvee's own navigation links
        if discovered:                               # discovery worked and found at least one category
            slugs = discovered                        # prefer the live list, since it reflects any renames/merges
            print(f"[cats]    Discovered {len(slugs)} categories from artvee.com: {', '.join(slugs)}", flush=True)
        else:                                         # discovery came back empty (blocked, layout changed, offline, …)
            slugs = builtin                            # fall back to the library's built-in list so the run can still proceed
            print(f"[cats]    Discovery found nothing; falling back to the built-in list ({len(slugs)} categories).", flush=True)

    if args.categories:                                        # the user restricted the run with one or more -c flags
        wanted = [c.strip().lower() for c in args.categories]   # normalize what they typed
        unknown = [w for w in wanted if w not in slugs]          # spot any that don't match a known category slug
        if unknown:                                              # warn about typos rather than silently scraping nothing
            print(f"[cats]    WARNING: these don't match any known category and may not work: {', '.join(unknown)}", flush=True)
        slugs = wanted                                            # honour exactly what the user asked for, known or not

    return [SiteCategory(s) for s in slugs]   # wrap each slug in the shim object the library can consume


def main():
    args = parse_args()                                    # parse all command-line flags into an object with .output, .categories, etc.

    print(f"=== {SCRIPT_VERSION} ===")                      # version banner, so it's obvious at a glance which copy of this script is running

    client = ArtveeClient()                                 # build one HTTP client up front — used for discovery, then handed to the scraper
    categories = resolve_categories(args, client)            # decide which categories this run will cover

    if args.list_categories:                                 # the user only wanted to see the category list...
        print("\nCategories currently available:")            # ...so print it...
        for c in categories:                                  # one per line
            print(f"  {c.value}")                             # showing the exact slug you'd pass to -c
        print("\n(Pass one or more with -c, e.g. -c landscape -c posters.)")
        return                                                # ...and exit without downloading anything

    base_dir = Path(args.output).expanduser().resolve()      # turn the (possibly relative, possibly "~"-containing) output path into an absolute one
    base_dir.mkdir(parents=True, exist_ok=True)               # create the output folder, and any missing parent folders, if it doesn't exist yet

    global _BASE_DIR, _FMT                        # hand the output location and format to the skip-before-download filter
    _BASE_DIR = base_dir                          # ...so it can work out whether each listed work is already on disk...
    _FMT = args.format                            # ...using the same file extension this run is writing

    max_width = max(0, args.max_width)             # clamp a negative value to 0 (0 means "resizing disabled") so a typo can't misbehave
    jpg_quality = max(1, min(100, args.jpg_quality))   # clamp quality into the 1-100 range that JPEG encoding actually accepts

    log_path = base_dir / "_download_log.csv"      # the CSV log always lives at the top level of the output folder
    log_is_new = not log_path.exists()             # remember whether this is a brand-new log, so we know whether to write a header row
    csv_lock = threading.Lock()                    # a lock so multiple worker threads can safely append to the same CSV file at once

    with open(log_path, "a", newline="", encoding="utf-8") as f:   # open the log in append mode, so re-runs add to history rather than erasing it
        writer = csv.writer(f)                      # wrap the open file in a CSV writer object
        if log_is_new:                              # only write the header row the very first time this log file is created
            writer.writerow(["resource_id", "title", "artist", "source_url", "status", "error"])   # the column headers

        threads = max(1, min(10, args.worker_threads))   # clamp thread count into the library's allowed 1-10 range

        scraper = ArtveeScraper(                     # construct the scraper...
            artvee_client=client,                     # ...reusing the same HTTP client we already used for discovery
            worker_threads=threads,                   # ...with the chosen concurrency
            categories=tuple(categories),             # ...over exactly the categories resolved above
        )
        scraper.register_listener(
            make_callback(base_dir, writer, csv_lock, max_width, args.format, jpg_quality)   # build and attach our per-artwork callback
        )

        print(f"\nStarting scrape into: {base_dir}")                                              # tell the user where files will land
        print(f"Categories: {', '.join(c.value for c in categories)}")                            # tell the user exactly what's being scraped
        print(f"Worker threads: {threads}")                                                        # tell the user the concurrency level in effect
        print(f"Max width: {max_width if max_width else 'disabled (original size)'}")              # tell the user the resize setting in effect
        print(f"Format: {args.format}" + (f" (quality {jpg_quality})" if args.format == "jpg" else ""))   # tell the user the output format/quality
        print("Personal/offline study use only — no redistribution or commercial use.\n")          # reminder of intended use, printed every run

        scraper.start()                              # kick off the scraper's background "boss" thread, which begins dispatching work to workers
        try:
            scraper.join()                            # block here until every category/artwork has been processed (or until Ctrl+C below)
        except KeyboardInterrupt:                      # the user pressed Ctrl+C while the scrape was still running
            print("\nInterrupted — shutting down workers (finishing in-flight downloads)...")   # let them know we're stopping gracefully, not abruptly
            scraper.shutdown()                          # tell the scraper to stop handing out any new work to its threads
            scraper.join()                              # wait for whatever downloads were already in flight to finish before exiting

    print("\nDone.")                                        # final summary header, printed once the whole run (or an interrupted run) has ended
    print(f"  Downloaded: {_stats['downloaded']}")          # how many new images were saved during this run
    print(f"  Skipped (already had): {_stats['skipped']}")  # how many were already present from before and left untouched
    print(f"  Failed: {_stats['failed']}")                  # how many hit an error along the way (see the CSV log for the specific reasons)
    print(f"  Log: {log_path}")                             # remind the user exactly where the detailed CSV log file lives


if __name__ == "__main__":   # this block only runs when the file is executed directly (e.g. `python artvee_offline_downloader.py`),
    main()                    # ...not when it's imported as a module from somewhere else — so main() is the actual entry point