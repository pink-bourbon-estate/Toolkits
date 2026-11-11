# PlanetEbook Downloader

A single Python script that downloads a fixed list of 82 classic books from [planetebook.com](https://www.planetebook.com/) as EPUB files, so you do not have to click "Download" on each page.

## Use
```
python planetebook_download.py                    # saves into a "PlanetEbook" folder next to the script
python planetebook_download.py "path/to/folder"   # saves into a folder of your choice
```

- Uses only Python's standard library; nothing to install.
- Safe to stop and re-run: books already downloaded are skipped.
- Each download is saved as `<name>.epub.part` first and renamed only when complete, so an interrupted run never leaves a half-finished book.
- A short pause between downloads keeps the load on the website small.
- A summary at the end shows how many were downloaded, skipped and failed.

To change the list, edit `SLUGS` near the top of the script. Each entry is the book's name as it appears in the site's web addresses.

## Notes
- The books are the ones the website offers as free downloads. Copyright rules differ between countries, so check that a title is free to use where you live.
- Be considerate of the website: run it once and let the skip logic handle re-runs.

## Version
Current version: **1.1**

| Version | Changes |
| --- | --- |
| 1.1 | Added beginner-friendly comments and a "how it works" guide, tidied the wording, and show the version at start-up. Downloads exactly as 1.0 did. |
| 1.0 | First release: download the list as EPUB, skip existing files, summary report. |
