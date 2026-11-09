# PianoCraft

A piano practice companion built on the Russian School approach: hear it first, hands separately, slowly, then together, transpose it, and don't move on until it's secure. When a step is too hard, PianoCraft makes the step smaller instead of letting you get stuck.

Written in Rust with egui, the same toolkit as EffectCraft and PdfCraft. Like those apps it's local-first: no accounts, no cloud, and your progress stays on your computer.

## Install (Windows)
1. Download `pianocraft.exe` from the newest **PianoCraft v0.1.x** release on the [Releases page](https://github.com/pink-bourbon-estate/toolkits/releases).
2. Run it. If SmartScreen warns about an unknown app, click **More info → Run anyway** (it isn't code-signed).
3. Click **Install** on the green bar (or Settings → *Install PianoCraft*). It copies itself to `%LOCALAPPDATA%\Programs\PianoCraft` (no admin rights), adds **PianoCraft to the Start menu** and to **Settings → Apps** (uninstall from there), then restarts from the installed copy. You can delete the downloaded file.

Command line: `PianoCraft.exe --install [--desktop]`, `--uninstall`, `--update`, `--version`.

## Updates
Every change pushed to `pianocraft/` is built by GitHub Actions and published as release `pianocraft-v0.1.N`, with SHA-256 checksums.
PianoCraft checks for a newer build when it starts (at most every 6 hours). When there is one, an **⬆ Update to v0.1.N** button appears at the top right. Click it, and PianoCraft downloads the new version, verifies the checksum, replaces itself and restarts. You can also check by hand in Settings, and turn automatic checks off there.
The only network requests are anonymous downloads from github.com. No account, no telemetry, nothing about you is sent.

## What's in it
- **Today**: your course, the Russian School of Piano Playing chapter by chapter from 1a chapter V (quavers) through the end of 1b, with Nahre Sol's approach alongside.
  - Each unit lists the book's own numbered pieces. **Practise** opens your copy at the right page and walks you through the Russian-School steps (listen and sing, count and clap, RH, LH, hands together, learn by heart, up to tempo, transpose) with a metronome and repetition counter. **Too hard** makes the step smaller; **Nailed it** moves you on.
  - Each unit also has simplified versions of Nahre Sol's exercises (circling scales, left-hand swells, turnaround chains, ii-V-I warm-ups, measured trills, Chopin seeds) with a button to her full version in your PDFs, plus a theory page from *Elements of Music* and an ear/creative task.
- **Books**: your PDFs inside the app. Annotate with pen, highlighter, text and finger-number stamps. Notes are saved per page and the PDFs are never changed.
- **Make this page playable**: runs the free Audiveris program to turn a page into notes. You can then hear it, loop it, slow it down, split the hands and see falling notes, with the page beside it.
- **Practice**: sheet music plus falling notes, tempo, metronome, count-in, repeat ×N, drag to loop bars, simplify either hand, change key or **Easiest key**, and **Peek** for memory work.
- **Keyboard**: 61, 76 or 88 keys. Zoom and the scroll strip move around it, and it follows the music. Names are on every key, including C♯/D♭ on the black keys.
- **Scales & technique**: 16 scales and modes (major, three minors, seven modes, pentachords, pentatonics, blues, whole tone, chromatic) in 1-4 octaves.
  - Exercise types: five-finger, scale, arpeggio (eight chord types), triad inversions, cadence, progressions (ii-V-I, I-IV-V-I, turnarounds), measured trills, LH swells, Chopin seeds and rhythm reading.
  - Patterns: straight, broken thirds, groups of 3 and 4, circling, turnaround and rising fragments.
  - Hands: either hand alone, together one or two octaves apart, a tenth apart, or contrary motion.
  - Seven rhythms, accents, crescendo-diminuendo, and key chains (fifths, fourths, whole tones, minor thirds, chromatic).
  - Fingering is automatic and matches the standard tables.
- **Note reading**: Nahre Sol's landmark method in four levels. Answer on screen or on your piano.
- **Library / Find music**: import MIDI and MusicXML, and search free sources.

## Making book pages playable (Audiveris)
1. Install Audiveris, free from https://github.com/Audiveris/audiveris/releases (Windows .msi).
2. In PianoCraft, press **Make playable** on any piece or page. It takes about a minute.
3. Old scans can have recognition mistakes, so compare with the page.

## Built-in pieces
- Au clair de la lune, Ode to Joy, Frère Jacques (canon) and Korobeiniki: the tunes are public domain and the left hands are simple arrangements.
- Minuet in G (Petzold, BWV Anh. 114, bars 1–16) and Musette in D (BWV Anh. 126): original notes, checked against your Anna Magdalena PDFs.

## PDFs
PDFs are pictures of music, so they can't be played directly. Convert them with **Audiveris** (free) or **MuseScore 4** (File → Import PDF) to MusicXML, then import that file.

## Connecting your digital piano (optional)
Keyboards with a USB port just need a USB cable. Many digital pianos have round 5-pin MIDI sockets instead.
1. Get a **USB MIDI interface cable** (5-pin DIN to USB, about £10–25). A guitar or aux-to-USB lead won't work, because it carries audio, not MIDI.
2. Plug the cable's **IN** plug into the piano's **MIDI OUT**, and plug the USB end into the computer.
3. In Settings, click **Look for keyboards → Connect**.

Once connected, the keys you press light up green, and **Wait for me** pauses at each note until you play it.

## Your data
Progress, annotations and imported songs live in `%APPDATA%\PianoCraft\`.

## Building from source
```
cargo run --release
```
This needs Rust 1.80 or later. On Linux it also needs `libasound2-dev`.

Typing in your own short piece is possible using the text notation described at the top of `src/notation.rs` (for example `C4:q E4:q G4:h | C5:w`). New pieces go in `src/builtin.rs`.

### Source layout
The engine is kept below the UI, the same layering as EffectCraft.
- `model`, `notation`, `transform` and `generate`: the music engine. This is where simplifying, transposing, difficulty rating and generated exercises live.
- `import`: MIDI and MusicXML import.
- `audio`: the synth, metronome and sample-accurate transport.
- `midi_in`: keyboard input.
- `curriculum`: the units, the practice steps, the adaptive steps and saved progress.
- `books`: where each Russian School piece is in your PDFs.
- `ui/pdfview`: the book viewer and annotations (hayro, as in PdfCraft), plus Audiveris.
- `ui/reading`: the note-reading trainer.
- `ui/`: the egui front end (app, falling notes and keyboard, staff).

License: MIT OR Apache-2.0.

The ♯/♭/♮ symbols use a 4 KB subset of the DejaVu Sans font (free licence, see `assets/DEJAVU-LICENSE.txt`).
