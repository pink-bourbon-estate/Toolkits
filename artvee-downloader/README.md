# Artvee Offline Downloader

A Python script that downloads public-domain artwork from [Artvee](https://artvee.com/) into a local folder for **personal offline study and reference**. It is a wrapper around the open-source [`artvee-scraper`](https://github.com/zduclos/artvee-scraper) library (MIT), adding fixes for site changes, a faster resume, resizing, and a log.

## Install
```
pip install artvee-scraper Pillow
pip install opencv-python numpy      # only if you use --format exr
```

## Use
```
python artvee_offline_downloader.py --list-categories          # show the categories the site offers, then exit
python artvee_offline_downloader.py                            # everything, 2 threads, 1920px JPGs
python artvee_offline_downloader.py -c landscape -c still-life # chosen categories only
python artvee_offline_downloader.py -o art_reference           # choose the output folder
python artvee_offline_downloader.py --help                     # all options
```

## What you get
- `<output>/<category>/<artist>/<title>.jpg`, shrunk to `--max-width` (default 1920) and never enlarged
- a `.json` details file beside each image (title, artist, date, source link)
- `_download_log.csv` listing every success and failure
- resumable runs: works you already have are skipped before anything is downloaded

## Please be considerate
A full-site run touches well over 250,000 works. Keep `--worker-threads` low (2-3), run it once and let resume handle interruptions. Check Artvee's Terms of Service before a large run. Use the images for personal, non-commercial study only and do not redistribute them.

## Version
Current version: **5.1**

| Version | Changes |
| --- | --- |
| 5.1 | Comments and wording only: beginner-friendly guide at the top, tidier comments, version number defined in one place. No change to behaviour. |
| 5 | Fast resume (skip finished works before downloading) and one progress line per page. |
| 4 | Writes the `.json` details files as UTF-8 so titles with accents no longer fail on Windows. |
| 3 | Fixed page counting, 404 handling for out-of-range pages, and discovers category names from the live site. |
