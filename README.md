# Toolkits

Small desktop tools, mostly built in Rust with egui. Each tool lives in its own folder, is built automatically by GitHub Actions on every push, and updates itself from this repo's Releases.

| Tool | What it does | Download |
| --- | --- | --- |
| [PianoCraft](pianocraft/) | Piano practice companion: Russian School course with Nahre Sol's approach, sheet + falling notes, scales in every mode, adaptive difficulty, book viewer with annotations | [Releases](https://github.com/pink-bourbon-estate/toolkits/releases) (`pianocraft-v0.1.N`) |
| [ChessCraft](chesscraft/) | Chess opening trainer for the Modern Defence (Black) and the London System (White): a full Stockfish-generated and move-checked line tree, Explore/Learn/Drill modes, and honest spaced repetition | [Releases](https://github.com/pink-bourbon-estate/toolkits/releases) (`chesscraft-v0.1.N`) |
| [YouTube Transcripts](youtube-transcripts/) | Python script: download the captions of a channel, playlist or video as Obsidian-style notes and combine them into master files for analysis | [`get_transcripts.py`](youtube-transcripts/get_transcripts.py) |
| [YouTube Frames](youtube-frames/) | Python script: capture a frame from a YouTube video every mm:ss and save them as compressed 960x540 JPEGs | [`youtube_frames.py`](youtube-frames/youtube_frames.py) |

## Privacy and security
- No accounts, telemetry or tracking. The only network use is the optional update check (anonymous download from github.com), plus links you click yourself.
- Your data (progress, notes, imported music) stays in your user folder and is never committed. `.gitignore` excludes PDFs, music files, app data and key files.
- Every push runs [gitleaks](https://github.com/gitleaks/gitleaks) (`.gitleaks.toml`) before anything is built. If it finds an API key, token, password, private key or personal email address, the build fails and nothing is released.
- Updates are verified against SHA-256 checksums published with each release before they replace the program.

## Licence
MIT – see `LICENSE-MIT`. Third-party font subset: see `pianocraft/assets/DEJAVU-LICENSE.txt`.
