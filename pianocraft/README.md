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
- **Today**: your course, the Russian School of Piano Playing chapter by chapter from 1a chapter V (quavers) through the end of 1b, with Nahre Sol's approach alongside.
  - Each unit lists the book's own numbered pieces. **Practise** opens your copy at the right page and walks you through the Russian-School steps (listen and sing, count and clap, RH, LH, hands together, learn by heart, up to tempo, transpose) with a metronome and repetition counter. **Too hard** makes the step smaller; **Nailed it** moves you on.
  - Each unit also has simplified versions of Nahre Sol's exercises (circling scales, left-hand swells, turnaround chains, ii-V-I warm-ups, measured trills, Chopin seeds) with a button to her full version in your PDFs, plus a theory page from *Elements of Music* and an ear/creative task.
- **Books**: your PDFs inside the app. Annotate with pen, highlighter, text and finger-number stamps. Notes are saved per page and the PDFs are never changed.
- **Make PDFs playable**: PianoCraft reads typeset sheet music (PDFs made by MuseScore, Sibelius, Dorico, Finale or LilyPond) by itself, a whole book in a second or two: notes, rhythms, ties, beams, triplets, key and time changes, 8va lines, clef changes, and guitar scores (moved down an octave, with bass and tune split between the hands). Bars that don't add up exactly are flagged, so compare those with the page. Scanned books (photos of pages) go through the free Audiveris program instead. Use **Make this page playable** for one page, **📚 Whole book** for a whole book, or Library → **Make all PDFs playable** for a whole folder (several folders separated by `;`). PDFs that aren't music are skipped, and your files are only ever read, never changed.
- **Practice**: sheet music plus falling notes, tempo, metronome, count-in, repeat ×N, drag to loop bars, simplify either hand, change key or **Easiest key**, and **Peek** for memory work.
- **Keyboard**: 61, 76 or 88 keys. Zoom and the scroll strip move around it, and it follows the music. Names are on every key, including C♯/D♭ on the black keys.
- **Scales & technique**: 16 scales and modes (major, three minors, seven modes, pentachords, pentatonics, blues, whole tone, chromatic) in 1-4 octaves.
  - Exercise types: five-finger, scale, arpeggio (eight chord types), triad inversions, cadence, progressions (ii-V-I, I-IV-V-I, turnarounds), measured trills, LH swells, Chopin seeds and rhythm reading.
  - Patterns: straight, broken thirds, groups of 3 and 4, circling, turnaround and rising fragments.
  - Hands: either hand alone, together one or two octaves apart, a tenth apart, or contrary motion.
  - Seven rhythms, accents, crescendo-diminuendo, and key chains (fifths, fourths, whole tones, minor thirds, chromatic).
  - Fingering is automatic and matches the standard tables.
- **Note reading**: Nahre Sol's landmark method in four levels. Answer on screen or on your piano.
- **Difficulty slider** (Practice, like a rhythm game): 1 = just the tune on the main beats, 2 = the whole tune, 3 = + held bass notes, 4 = + bass line, 5 = tune with one harmony note, 6 = full right hand, 7 = everything. Works on every piece, including your imports.
- **Repertoire**: 55 graded pieces built in (Grade 1–8, Mutopia Project, public domain / CC BY-SA). Includes all the Anna Magdalena minuets and polonaises, Schumann *Album for the Young*, Tchaikovsky *Children's Album*, Burgmüller Op. 100, Czerny, Handel, Clementi, Für Elise, Bach's C major Prelude and Inventions, Satie, Joplin, Chopin preludes and waltz, *Träumerei*, *Moonlight* (I), Debussy's Arabesque No. 1 and Clair de lune. Credits are in `repertoire/LICENSES.md`.
- **Library / Find music**: import MIDI and MusicXML, and search free sources.

## Making scanned books playable (Audiveris)
Only needed for scans. Typeset PDFs are read by PianoCraft itself.
1. Install Audiveris, free from https://github.com/Audiveris/audiveris/releases. Use the Windows **.msi** installer under *Assets*, not the source code.
2. In PianoCraft, press **Make this page playable** (about a minute) or **📚 Whole book** (runs in the background).
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
Progress, annotations (`progress.json`, `annotations.json`) and your converted and imported music (`library.json`) live in `%APPDATA%\PianoCraft\`.

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
- `ui/`: the egui front end (app, falling notes and keyboard, staff).

License: MIT OR Apache-2.0.

The ♯/♭/♮ symbols use a 4 KB subset of the DejaVu Sans font (free licence, see `assets/DEJAVU-LICENSE.txt`).

