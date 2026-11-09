# Toolkits

Small desktop tools, built in Rust with egui. Each tool lives in its own folder, is built automatically by GitHub Actions on every push, and updates itself from this repo's Releases.

| Tool | What it does | Download |
| --- | --- | --- |
| [PianoCraft](pianocraft/) | Piano practice companion: Russian School course with Nahre Sol's approach, sheet + falling notes, scales in every mode, adaptive difficulty, book viewer with annotations | [Releases](https://github.com/pink-bourbon-estate/toolkits/releases) (`pianocraft-v0.1.N`) |

## Privacy and security
- No accounts, telemetry or tracking. The only network use is the optional update check (anonymous download from github.com), plus links you click yourself.
- Your data (progress, notes, imported music) stays in your user folder and is never committed. `.gitignore` excludes PDFs, music files, app data and key files.
- Every push runs [gitleaks](https://github.com/gitleaks/gitleaks) (`.gitleaks.toml`) before anything is built. If it finds an API key, token, password, private key or personal email address, the build fails and nothing is released.
- Updates are verified against SHA-256 checksums published with each release before they replace the program.

## Licence
MIT – see `LICENSE-MIT`. Third-party font subset: see `pianocraft/assets/DEJAVU-LICENSE.txt`.
