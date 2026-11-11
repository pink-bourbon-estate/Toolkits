# YouTube Transcripts

A single Python script that downloads the captions for a YouTube channel, playlist or single video, turns them into clean Obsidian-style Markdown notes (front matter, chapter headings, timestamped paragraphs), and combines them into a few large "master" files that are easy to upload and analyse.

## Use
Needs Python 3. The script installs `yt-dlp` (and `reportlab` for PDFs) itself if missing. Run it from the folder it lives in:

```
py get_transcripts.py https://www.youtube.com/@ChannelName/videos     # whole channel
py get_transcripts.py https://www.youtube.com/playlist?list=PL...     # playlist
py get_transcripts.py https://www.youtube.com/watch?v=...             # one video
py get_transcripts.py "Channel Name Transcripts"                      # combine a folder you already have
py get_transcripts.py                                                 # asks what you want to do
```

Options: `--pdf` (also write PDFs), `--words N` (max words per master file, default 60000, `0` = one file), `--no-combine`.

## Output
- `<Name> Transcripts/` holds one `.md` note per video.
- `<Name> Transcripts/Combined/` holds the numbered master files, oldest to newest, each starting with a contents list.

## Notes
- Captions only; no video is downloaded. Safe to stop and re-run: finished videos are skipped.
- Videos without captions are listed in `_no_captions.txt`.
- Private playlists need a sign-in: make the playlist Unlisted, or let the script borrow a browser's YouTube sign-in when it asks (or set `COOKIES_FROM_BROWSER` at the top of the script).
- Respect YouTube's terms and creators' rights; use transcripts for personal study.
