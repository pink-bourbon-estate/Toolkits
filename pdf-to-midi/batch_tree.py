#!/usr/bin/env python3
"""
Convert every PDF under ROOT (recursively); write NAME.mid and NAME.musicxml.xml next to each PDF.
Also writes ROOT/_conversion_report.csv, _converted.txt, _unconverted.txt (with reasons + font verdicts).

  python3 -I batch_tree.py ROOT [--jobs N] [--timeout SEC]
"""
import sys, os, csv, json, argparse, subprocess, traceback
from concurrent.futures import ThreadPoolExecutor
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)


def font_verdict(path):
    """Returns (verdict, fonts) from the first pages; verdict explains whether the score is readable."""
    import pdfplumber
    fonts, pua, music = set(), 0, False
    chars = imgs = pages = 0
    try:
        with pdfplumber.open(path) as pdf:
            pages = len(pdf.pages)
            for pg in pdf.pages[:6]:
                imgs += len(pg.images)
                for ch in pg.chars:
                    chars += 1
                    fn = ch['fontname'].split('+')[-1]
                    fonts.add(fn)
                    if fn.startswith(('Opus', 'Norfolk')) and not fn.endswith('TextStd'):
                        music = True
                    elif ch['text'] and 0xE000 <= ord(ch['text'][0]) <= 0xF8FF:
                        pua += 1
                        music = True
                nlines = len(pg.lines) + len(pg.rects)
    except Exception as e:
        return 'UNREADABLE PDF (%s)' % type(e).__name__, ''
    fl = ','.join(sorted(fonts))[:200]
    if music:
        return 'KNOWN music font (%s)' % ('SMuFL/Bravura/MScore/Leland' if pua else 'Opus/Norfolk'), fl
    if imgs and not chars:
        return 'SCAN: images only, no text/vector music', fl
    if imgs and chars < 50:
        return 'LIKELY SCAN: page images + almost no text', fl
    return 'UNKNOWN fonts: no recognised music-notation font', fl


def one(pdf, timeout):
    base = os.path.splitext(pdf)[0]
    r = dict(file=pdf, status='', fonts='', font_verdict='', bars=0, notes=0, bad_bars=0, comment='')
    r['font_verdict'], r['fonts'] = font_verdict(pdf)
    if r['font_verdict'].startswith(('SCAN', 'LIKELY SCAN', 'UNREADABLE')):
        r['status'] = 'UNCONVERTED'
        r['comment'] = 'cannot convert: ' + r['font_verdict']
        return r
    try:
        p = subprocess.run([sys.executable, '-I', os.path.join(HERE, 'batch_tree.py'), '--child', pdf],
                           capture_output=True, text=True, timeout=timeout)
        line = [l for l in p.stdout.splitlines() if l.startswith('RESULT ')]
        if not line:
            r['status'] = 'UNCONVERTED'
            r['comment'] = 'converter crashed: ' + (p.stderr.strip().splitlines() or ['?'])[-1][:200]
            return r
        d = json.loads(line[0][7:])
        r.update(d)
    except subprocess.TimeoutExpired:
        r['status'] = 'UNCONVERTED'
        r['comment'] = 'timed out after %ds' % timeout
    return r


def child(pdf):
    import pdf2midi, musicxml_out
    base = os.path.splitext(pdf)[0]
    c = pdf2midi.Converter(pdf).run()
    bad = [w for w in c.warnings if 'BAR LENGTH' in w]
    if not c.notes:
        print('RESULT ' + json.dumps(dict(status='UNCONVERTED', bars=c.mno, notes=0,
              comment='font recognised/readable but no notes found (guitar tab, unsupported layout, or no piano staves)')))
        return
    c.write_midi(base + '.mid')
    musicxml_out.write(c, base + '.musicxml.xml')
    st = 'CONVERTED' if not bad else 'CONVERTED-CHECK'
    cm = 'ok' if not bad else '%d bars do not add up to the time signature (e.g. %s)' % (len(bad), '; '.join(w.split(':')[0] for w in bad[:3]))
    if any('no tempo' in w for w in c.warnings):
        cm += '; tempo defaulted to 120 BPM'
    print('RESULT ' + json.dumps(dict(status=st, bars=c.mno, notes=len(c.notes), bad_bars=len(bad), comment=cm)))


def main():
    if len(sys.argv) > 2 and sys.argv[1] == '--child':
        return child(sys.argv[2])
    ap = argparse.ArgumentParser()
    ap.add_argument('root')
    ap.add_argument('--jobs', type=int, default=os.cpu_count() or 2)
    ap.add_argument('--timeout', type=int, default=600)
    a = ap.parse_args()
    pdfs = sorted(os.path.join(r, f) for r, _, fs in os.walk(a.root) for f in fs if f.lower().endswith('.pdf'))
    rows = []
    with ThreadPoolExecutor(a.jobs) as ex:
        for i, r in enumerate(ex.map(lambda p: one(p, a.timeout), pdfs), 1):
            rows.append(r)
            print('[%d/%d] %-12s %s' % (i, len(pdfs), r['status'], os.path.relpath(r['file'], a.root)[:70]), flush=True)
    cols = ['file', 'status', 'font_verdict', 'bars', 'notes', 'bad_bars', 'comment', 'fonts']
    with open(os.path.join(a.root, '_conversion_report.csv'), 'w', newline='', encoding='utf-8') as f:
        w = csv.DictWriter(f, cols)
        w.writeheader()
        for r in rows:
            r['file'] = os.path.relpath(r['file'], a.root)
            w.writerow({k: r.get(k, '') for k in cols})
    with open(os.path.join(a.root, '_converted.txt'), 'w', encoding='utf-8') as f:
        for r in rows:
            if r['status'].startswith('CONVERTED'):
                f.write('%s\t%s\t%s\n' % (r['file'], r['status'], r['comment']))
    with open(os.path.join(a.root, '_unconverted.txt'), 'w', encoding='utf-8') as f:
        for r in rows:
            if not r['status'].startswith('CONVERTED'):
                f.write('%s\t%s\t%s\n' % (r['file'], r['font_verdict'], r['comment']))
    n = lambda s: sum(r['status'] == s for r in rows)
    print('done: %d converted, %d converted-with-check, %d unconverted' % (n('CONVERTED'), n('CONVERTED-CHECK'), n('UNCONVERTED')))


if __name__ == '__main__':
    main()
