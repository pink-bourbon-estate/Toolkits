# PDF to MIDI

Python scripts that turn vector sheet-music PDFs (piano scores exported from MuseScore, Dorico or Sibelius) into MIDI and MusicXML by reading the notation glyphs, lines and beams directly, not by image recognition. Scanned PDFs are not supported (they are listed so you can handle them another way).

## Use
One file:
```
python -I pdf2midi.py score.pdf -o score.mid [--single-track] [--bpm 120] [--report]
```
A whole folder tree:
```
python -I batch_tree.py "path/to/folder" --jobs 4 --timeout 600
```
For every PDF it writes `NAME.mid` and `NAME.musicxml.xml` next to the PDF, and in the root folder:
- `_conversion_report.csv` – per file: status, font verdict (known music font / unknown fonts / scan), fonts found, bars, notes, bars that do not add up, comment
- `_converted.txt` – converted files (`CONVERTED-CHECK` = some bars do not add up to the time signature, so spot-check them)
- `_unconverted.txt` – files that could not be converted, with the reason

`batch_convert.py` is the older flat-folder version (MIDI plus `summary.csv`).

## Requirements
- Python 3.9+
- `pip install pdfplumber mido pillow`
- optional: `qpdf` on your PATH (repairs PDFs that will not open)

## Limits
Guitar tab, repeats/endings (not unrolled), grace notes in some layouts, and tempo (defaults to 120 BPM when none is printed) are not handled well. Output accuracy is checked by bar length, so always listen through files marked `CONVERTED-CHECK`.

Your PDFs and the generated MIDI/MusicXML are never committed (`.gitignore` excludes them).
