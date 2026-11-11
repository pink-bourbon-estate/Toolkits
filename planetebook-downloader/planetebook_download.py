#!/usr/bin/env python3
"""
Download every book in a fixed list from planetebook.com in EPUB format.

Version 1.1  (1.1 adds explanatory comments, tidies the wording and shows the
              version number at start-up; it downloads exactly as 1.0 did)

Usage:
    python planetebook_download.py [destination_folder]

If no destination folder is given, files are saved into a "PlanetEbook"
folder created next to this script. Already-downloaded files (matching
name and non-zero size) are skipped, so it's safe to re-run.

HOW THE SCRIPT WORKS (a map for newcomers)
    1. SLUGS is a list of book names, written the way the website writes them
       in its web addresses (for example "pride-and-prejudice").
    2. For each name, the script builds the download address, fetches the file,
       and saves it as <name>.epub in the destination folder.
    3. At the end it prints a summary: downloaded, already had, and failed.

  Vocabulary used in the comments:
    - function   : a named, reusable block of code, written as "def name(...):"
    - list       : an ordered collection, such as ["a", "b", "c"]
    - dictionary : a collection of labelled values, such as {"key": "value"}
    - slug       : the short, lower-case name of an item as it appears in a web address
    - EPUB       : a common e-book file format that most e-reader apps can open
"""

# =============================================================================
# STEP 1: IMPORTS
# =============================================================================
# An "import" loads a ready-made toolkit that comes with Python (the "standard
# library"), so we do not have to write these common jobs ourselves.
import os               # Work with files and folders: create folders, check sizes, rename files.
import sys              # Read what was typed after the script name (the optional destination folder).
import time             # Pause between downloads (time.sleep).
import urllib.request   # Fetch files from the internet.
import urllib.error     # Web-related error types. Not used directly below, because errors are
                        # caught with the general "Exception" in STEP 5, but it is the module
                        # you would use to catch web errors more specifically.

# The version number of this script. Shown when the script starts.
__version__ = "1.1"


# =============================================================================
# STEP 2: THE WEB ADDRESS PATTERN AND THE LIST OF BOOKS
# =============================================================================
# BASE is the web address pattern. The "{slug}" part is a placeholder: for each
# book, the script swaps it for that book's name (see STEP 5).
BASE = "https://www.planetebook.com/free-ebooks/{slug}.epub"

# SLUGS is the list of books to download (82 of them). Each entry is the
# book's name exactly as the website writes it in its web addresses.
# To add or remove a book, add or remove a name here. Names are separated by
# commas and wrapped in quotation marks.
SLUGS = [
    "the-adventures-of-huckleberry-finn", "the-adventures-of-tom-sawyer", "aesops-fables",
    "agnes-grey", "alices-adventures-in-wonderland", "andersens-fairy-tales", "anna-karenina",
    "anne-of-green-gables", "around-the-world-in-80-days", "beyond-good-and-evil", "bleak-house",
    "the-brothers-karamazov", "a-christmas-carol", "crime-and-punishment", "david-copperfield",
    "down-and-out-in-paris-and-london", "dracula", "dubliners", "emma", "erewhon",
    "for-the-term-of-his-natural-life", "frankenstein", "great-expectations", "the-great-gatsby",
    "grimms-fairy-tales", "gullivers-travels", "heart-of-darkness", "the-hound-of-the-baskervilles",
    "the-idiot", "the-iliad", "the-island-of-doctor-moreau", "jane-eyre", "the-jungle-book",
    "kidnapped", "lady-chatterlys-lover", "the-last-of-the-mohicans", "the-legend-of-sleepy-hollow",
    "les-miserables", "little-women", "madame-bovary", "the-merry-adventures-of-robin-hood",
    "the-metamorphosis", "middlemarch", "moby-dick", "1984", "northanger-abbey",
    "nostromo-a-tale-of-the-seaboard", "notes-from-the-underground", "the-odyssey",
    "of-human-bondage", "oliver-twist", "paradise-lost", "persuasion", "the-picture-of-dorian-gray",
    "pollyanna", "the-portrait-of-a-lady", "a-portrait-of-the-artist-as-a-young-man",
    "pride-and-prejudice", "the-prince", "robinson-crusoe", "the-scarlet-pimpernel",
    "sense-and-sensibility", "sons-and-lovers", "the-strange-case-of-dr-jekyll", "swanns-way",
    "a-tale-of-two-cities", "the-tales-of-mother-goose", "tarzan-of-the-apes", "tender-is-the-night",
    "tess-of-the-durbervilles", "the-thirty-nine-steps", "the-three-musketeers", "the-time-machine",
    "treasure-island", "the-trial", "ulysses", "utopia", "vanity-fair", "war-and-peace",
    "within-a-budding-grove", "women-in-love", "wuthering-heights",
]


# =============================================================================
# STEP 3: REQUEST HEADERS
# =============================================================================
# Every web request carries "headers": small labelled details about who is
# asking. The "User-Agent" header normally names the browser. Some websites
# refuse requests that do not include one, so this sets a standard
# browser-style value (here, Chrome on Windows).
HEADERS = {
    "User-Agent": (
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
        "(KHTML, like Gecko) Chrome/124.0 Safari/537.36"
    )
}


# =============================================================================
# STEP 4: THE MAIN FUNCTION - SETTING UP
# =============================================================================
def main():
    """Download every book in SLUGS, then print a summary."""
    print(f"PlanetEbook downloader (version {__version__})")

    # Decide where to save the books.
    #   sys.argv is the list of words typed on the command line. Item 0 is the
    #   script name, so item 1 (if present) is the destination folder.
    #   If nothing was typed, use a folder called "PlanetEbook" next to this script
    #   (__file__ is this script's path; abspath makes it a full path; dirname
    #   gives the folder it sits in).
    dest = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        os.path.dirname(os.path.abspath(__file__)), "PlanetEbook"
    )

    # Create the folder. exist_ok=True means "do not report an error if the
    # folder is already there".
    os.makedirs(dest, exist_ok=True)

    # Three empty lists, used as a running tally:
    #   ok      - books downloaded in this run
    #   failed  - books that could not be downloaded
    #   skipped - books that were already in the folder
    ok, failed, skipped = [], [], []

    # =========================================================================
    # STEP 5: WORKING THROUGH THE LIST, ONE BOOK AT A TIME
    # =========================================================================
    # enumerate(SLUGS, 1) gives each book together with a counter that starts
    # at 1, so we can show progress such as "[3/82]".
    for i, slug in enumerate(SLUGS, 1):

        # The final file name, for example ".../PlanetEbook/dracula.epub".
        out_path = os.path.join(dest, f"{slug}.epub")

        # --- Skip books we already have ---
        # If the file exists and is bigger than 1024 bytes (about 1 KB), assume
        # an earlier run saved it properly and move on. A file smaller than that
        # is probably an error page, so it gets downloaded again.
        if os.path.exists(out_path) and os.path.getsize(out_path) > 1024:
            print(f"[{i}/{len(SLUGS)}] SKIP  {slug} (already downloaded)")
            skipped.append(slug)
            continue    # "continue" jumps straight to the next book in the loop

        # Put this book's name into the address pattern from STEP 2.
        url = BASE.format(slug=slug)
        # Create the request, attaching the headers from STEP 3.
        req = urllib.request.Request(url, headers=HEADERS)

        # --- Save to a temporary name first ---
        # The download is written to "<name>.epub.part". Only when it has
        # finished and passed a size check is it renamed to "<name>.epub".
        # That way an interrupted download (for example, the connection drops)
        # never leaves a half-finished file that looks like a real book.
        tmp_path = out_path + ".part"

        # "try / except" means: attempt this, and if anything goes wrong, run
        # the "except" part instead of stopping the whole script.
        try:
            # Open the connection (waiting at most 30 seconds) and open the
            # temporary file for writing in binary mode ("wb"). "with" closes
            # both automatically when the block ends.
            with urllib.request.urlopen(req, timeout=30) as resp, open(tmp_path, "wb") as f:
                f.write(resp.read())

            # How many bytes did we receive?
            size = os.path.getsize(tmp_path)

            # A real book is far larger than 1 KB. A very small response is
            # most likely an error page from the website, so treat it as a
            # failure. "raise" signals an error, which jumps to "except" below.
            if size < 1024:
                raise ValueError(f"suspiciously small response ({size} bytes)")

            # Success: rename the temporary file to its final name.
            os.replace(tmp_path, out_path)
            # {size:,} shows the number with thousands separators, e.g. 1,234,567.
            print(f"[{i}/{len(SLUGS)}] OK    {slug} ({size:,} bytes)")
            ok.append(slug)

        except Exception as e:
            # Something went wrong (for example a network problem or an
            # unknown address). Remove the unfinished temporary file, report
            # the problem, and carry on with the next book.
            if os.path.exists(tmp_path):
                os.remove(tmp_path)
            print(f"[{i}/{len(SLUGS)}] FAIL  {slug}: {e}")
            failed.append(slug)

        # --- Short pause between downloads ---
        # Wait 0.3 seconds before the next request. This avoids sending a burst
        # of requests and keeps the load on the website small.
        time.sleep(0.3)

    # =========================================================================
    # STEP 6: THE SUMMARY
    # =========================================================================
    # len(list) is the number of items in the list.
    print("\n--- Summary ---")
    print(f"Downloaded: {len(ok)}")
    print(f"Already had: {len(skipped)}")
    print(f"Failed: {len(failed)}")
    if failed:
        # ", ".join(list) turns ["a", "b"] into "a, b".
        print("Failed slugs:", ", ".join(failed))
    print(f"\nFiles saved to: {dest}")


# This line means: only run main() when the script is started directly (for
# example "python planetebook_download.py"), not when another script imports it.
if __name__ == "__main__":
    main()
