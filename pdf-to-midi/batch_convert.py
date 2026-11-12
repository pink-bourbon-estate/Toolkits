#!/usr/bin/env python3
"""
Batch-convert a folder of MuseScore PDFs to MIDI.

  python3 -I batch_convert.py INPUT_DIR OUTPUT_DIR [--single-track]

Writes one .mid per PDF plus OUTPUT_DIR/summary.csv listing, for every file, the bar count,
note count and number of bars whose length did not add up to the time signature (these are
the ones to check by eye).  Files that fail outright are logged and skipped.
"""
import sys, os, csv, argparse, traceback
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pdf2midi


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('indir')
    ap.add_argument('outdir')
    ap.add_argument('--single-track', action='store_true')
    a = ap.parse_args()
    os.makedirs(a.outdir, exist_ok=True)
    pdfs = sorted(os.path.join(r, f) for r, _, fs in os.walk(a.indir) for f in fs if f.lower().endswith('.pdf'))
    rows = []
    for i, p in enumerate(pdfs, 1):
        name = os.path.splitext(os.path.basename(p))[0]
        out = os.path.join(a.outdir, name + '.mid')
        try:
            c = pdf2midi.Converter(p).run()
            c.write_midi(out, single=a.single_track)
            bad = [w for w in c.warnings if 'BAR LENGTH' in w]
            status = 'OK' if not bad and c.notes else ('CHECK' if c.notes else 'NO NOTES')
            rows.append([name, status, c.mno, len(c.notes), len(bad), '; '.join(w.split(':')[0] for w in bad[:10])])
        except Exception as e:
            traceback.print_exc()
            rows.append([name, 'FAILED', 0, 0, 0, repr(e)])
        print('[%d/%d] %-40s %s' % (i, len(pdfs), name[:40], rows[-1][1]), flush=True)
    with open(os.path.join(a.outdir, 'summary.csv'), 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['file', 'status', 'bars', 'notes', 'bad_bars', 'bad_bar_locations'])
        w.writerows(rows)
    print('done: %d OK, %d CHECK, %d failed' % (sum(r[1] == 'OK' for r in rows), sum(r[1] == 'CHECK' for r in rows),
                                                  sum(r[1] in ('FAILED', 'NO NOTES') for r in rows)))


if __name__ == '__main__':
    main()
