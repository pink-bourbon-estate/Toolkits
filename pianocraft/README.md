# PianoCraft

A piano practice companion built on the Russian School approach: hear it first, hands separately, slowly, then together, transpose it, and don't move on until it's secure. When a step is too hard, PianoCraft makes the step smaller instead of letting you get stuck.

Written in Rust with egui, the same toolkit as EffectCraft and PdfCraft. Like those apps it's local-first: no accounts, no cloud, and your progress stays on your computer.

## Install (Windows)
1. Download `pianocraft.exe` from the newest **PianoCraft v0.1.x** release on the [Releases page](https://github.com/pink-bourbon-estate/toolkits/releases).
2. Run it. If SmartScreen warns about an unknown app, click **More info → Run anyway** (it isn't code-signed).
3. Click **Install** on the green bar (or Settings → *Install PianoCraft*). It copies itself to `%LOCALAPPDATA%\Programs\PianoCraft` (no admin rights), adds **PianoCraft to the Start menu** and to **Settings → Apps** (uninstall from there), then restarts from the installed copy. You can delete the downloaded file.

Command line: `PianoCraft.exe --install [--desktop]`, `--uninstall`, `--update`, `--version`, `--convert <folder or PDFs>` (convert PDFs without opening the window and print what was found).

## Updates
Every change pushed to `pianocraft/` is built by GitHub Actions and published as release `pianocraft-v0.1.N`, with SHA-256 checksums.
PianoCraft checks for a newer build when it starts (at most every 6 hours). When there is one, an **⬆ Update to v0.1.N** button appears at the top right. Click it, and PianoCraft downloads the new version, verifies the checksum, replaces itself and restarts. You can also check by hand in Settings, and turn automatic checks off there.
The only network requests are anonymous downloads from github.com. No account, no telemetry, nothing about you is sent.

## What's in it
One screen. On the left is a list of everything you can practise; on the right is whatever you opened. At the top is the path to it (for example *Russian School 1a › p.25 › 47. The Wind Blew on the Sea*), where you can click any part to go back, and **◀ ▶** for the previous and next exercise. **Settings** is the only other tab.

- **Course**: the Russian School of Piano Playing from the very start of 1a (singing by ear, notes and the stave) to the end of 1b, chapter by chapter, with Nahre Sol's approach alongside.
  - Each chapter lists the book's numbered pieces and a short plan: warm-ups, theory pages from *Elements of Music*, and an ear task.
  - **Practise** walks you through the Russian-School steps: listen and sing, count and clap, right hand, left hand, hands together, learn by heart, up to tempo, transpose. **Too hard** makes the step smaller and **Nailed it** moves you on. **I know this whole chapter** skips what you can already play.
- **Books & sheet music**: every PDF in your music folders, grouped by folder. Each opens as book › page › the exercises on that page.
  - Typeset sheet music (PDFs from MuseScore, Sibelius, Dorico or Finale) is read by PianoCraft itself in a second or two: notes, rhythms, beams, triplets, key and time changes, 8va lines, clef changes, and guitar scores (moved down an octave, with bass and tune split between the hands).
  - The exercises inside a book are found from their headings (e.g. *EXERCISE NO. 3B*, *5. Left Hand Swells*, *II: Michishirube*).
  - Scanned books need the free Audiveris program; PianoCraft runs it on a page by itself the first time you open a piece on it.
  - Once a page is playable, it stays playable.
  - Annotate any page with pen, highlighter, text and finger numbers. Your PDFs are never changed.
- **Playing along**: the book page sits beside the sheet music and falling notes.
  - The page turns by itself and the bar being played is highlighted. Click a bar on the page to jump there.
  - Tempo, metronome, count-in, repeat ×N, drag to loop bars, either hand alone, change key or **Easiest key**, and **Peek** for memory work.
  - **Difficulty slider**, like a rhythm game: 1 = just the tune on the main beats, 2 = the whole tune, 3 = + held bass notes, 4 = + bass line, 5 = tune with one harmony note, 6 = full right hand, 7 = everything. It works on every piece.
- **Keyboard**: 61, 76 or 88 keys, with zoom and a scroll strip, following the music. Names are on every key, including C♯/D♭.
- **Scales & technique**: the settings sit beside the music and change it straight away.
  - 16 scales and modes in 1–4 octaves.
  - Exercises: five-finger, scale, arpeggio (eight chord types), triad inversions, cadence, ii–V–I and other progressions, measured trills, left-hand swells, Chopin seeds, rhythm reading.
  - 7 patterns, 6 ways for the hands, 7 rhythms, accents, and chains through keys.
  - Fingering is automatic.
- **Note reading**: Nahre Sol's landmark method in four levels. A wrong answer explains which clef the note was in.
- **Repertoire**: 55 graded pieces built in (Grade 1–8, Mutopia Project, public domain / CC BY-SA, credits in `repertoire/LICENSES.md`), up to Clair de lune.
- **Your files**: drop MIDI or MusicXML onto the window. **Find music** searches free sources.

## Making scanned books playable (Audiveris)
Only needed for scans. Typeset PDFs are read by PianoCraft itself.
1. Install Audiveris, free from https://github.com/Audiveris/audiveris/releases. Use the Windows **.msi** installer under *Assets*, not the source code.
2. Open any piece in the book. PianoCraft makes its page playable in the background (about a minute) and switches to sound when it's done.
3. Old scans can have recognition mistakes, so compare with the page.

## Built-in pieces
- Au clair de la lune, Ode to Joy, Frère Jacques (canon) and Korobeiniki: the tunes are public domain and the left hands are simple arrangements.
- Minuet in G (Petzold, BWV Anh. 114, bars 1–16) and Musette in D (BWV Anh. 126): original notes, checked against your Anna Magdalena PDFs.


## Connecting your digital piano (optional)
Keyboards with a USB port just need a USB cable. Many digital pianos have round 5-pin MIDI sockets instead.
1. Get a **USB MIDI interface cable** (5-pin DIN to USB, about £10–25). A guitar or aux-to-USB lead won't work, because it carries audio, not MIDI.
2. Plug the cable's **IN** plug into the piano's **MIDI OUT**, and plug the USB end into the computer.
3. In Settings, click **Look for keyboards → Connect**.

Once connected, the keys you press light up green, and **Wait for me** pauses at each note until you play it.

## Troubleshooting
If PianoCraft ever doesn't open, look at `%APPDATA%\\PianoCraft\\startup.log`: the last line shows how far it got. It only records step names and times, and never leaves your PC.

## Your data
Progress and annotations (`progress.json`, `annotations.json`) and your converted and imported music (`library\`: a small index plus one file per piece, read only when you open it) live in `%APPDATA%\PianoCraft\`. If PianoCraft ever crashes, `startup.log` there says where.

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
- `vecscore`: the built-in reader for typeset PDFs. It reads the music-font symbols and lines directly (hayro), with no optical recognition.
- `ui/convert`: background conversion of pages, books and folders.
- `audio`: the synth, metronome and sample-accurate transport.
- `midi_in`: keyboard input.
- `curriculum`: the units, the practice steps, the adaptive steps and saved progress.
- `books`: where each Russian School piece is in your PDFs.
- `ui/pdfview`: the book viewer and annotations (hayro, as in PdfCraft), plus Audiveris.
- `ui/reading`: the note-reading trainer.
- `ui/app/workspace`: the one practice screen (browser, path, player, page following).
- `ui/`: the rest of the egui front end (app, falling notes and keyboard, staff).

License: MIT OR Apache-2.0.

The ♯/♭/♮ symbols use a 4 KB subset of the DejaVu Sans font (free licence, see `assets/DEJAVU-LICENSE.txt`).

