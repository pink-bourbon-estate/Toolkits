"""Minimal MusicXML writer for pdf2midi Converter results (one part per staff, voices split by overlap)."""
from fractions import Fraction as F
from xml.sax.saxutils import escape
DIV = 480
STEP = {0: ('C', 0), 1: ('C', 1), 2: ('D', 0), 3: ('D', 1), 4: ('E', 0), 5: ('F', 0), 6: ('F', 1), 7: ('G', 0), 8: ('G', 1), 9: ('A', 0), 10: ('A', 1), 11: ('B', 0)}
TYPES = [(F(4), 'whole'), (F(2), 'half'), (F(1), 'quarter'), (F(1, 2), 'eighth'), (F(1, 4), '16th'), (F(1, 8), '32nd')]


def tname(q):
    for v, n in TYPES:
        if q == v:
            return n, 0
        if q == v * F(3, 2):
            return n, 1
    return None, 0


def pitch(m):
    s, a = STEP[m % 12]
    return '<pitch><step>%s</step>%s<octave>%d</octave></pitch>' % (s, '<alter>1</alter>' if a else '', m // 12 - 1)


def el(q, chord, midi, tie_s, tie_e, rest=False):
    d = int(q * DIV)
    t, dots = tname(q)
    x = '<note>' + ('<chord/>' if chord else '') + ('<rest/>' if rest else pitch(midi)) + '<duration>%d</duration>' % d
    if tie_e: x += '<tie type="stop"/>'
    if tie_s: x += '<tie type="start"/>'
    if t: x += '<type>%s</type>' % t + '<dot/>' * dots
    if tie_e or tie_s:
        x += '<notations>' + ('<tied type="stop"/>' if tie_e else '') + ('<tied type="start"/>' if tie_s else '') + '</notations>'
    return x + '</note>'


def write(conv, path):
    sigs = sorted(set((F(t), s) for t, s in conv.sig_events)) or [(F(0), (4, 4))]
    end = max((F(n['end']) for n in conv.notes), default=F(0))
    bars, t, k, cur = [], F(0), 0, sigs[0][1]
    while t < end or not bars:
        while k < len(sigs) and sigs[k][0] <= t:
            cur = sigs[k][1]; k += 1
        ln = F(cur[0] * 4, cur[1])
        bars.append((t, ln, cur))
        t += ln
    staves = sorted(set(n['staff'] for n in conv.notes))
    out = ['<?xml version="1.0" encoding="UTF-8"?><score-partwise version="3.1"><part-list>']
    for i, s in enumerate(staves):
        out.append('<score-part id="P%d"><part-name>%s</part-name></score-part>' % (i + 1, 'Right hand' if i == 0 else 'Left hand' if i == 1 else 'Staff %d' % (i + 1)))
    out.append('</part-list>')
    for i, s in enumerate(staves):
        notes = sorted((n for n in conv.notes if n['staff'] == s and 0 <= n['midi'] <= 127 and F(n['end']) > F(n['start']) and F(n['start']) >= 0),
                       key=lambda n: (F(n['start']), n['midi']))
        # group into chords (same start+end), then split into voices by overlap
        chords = {}
        for n in notes:
            chords.setdefault((F(n['start']), F(n['end'])), []).append(n['midi'])
        voices = []
        for (a, b), ms in sorted(chords.items()):
            for v in voices:
                if v[-1][1] <= a:
                    v.append((a, b, ms)); break
            else:
                voices.append([(a, b, ms)])
        out.append('<part id="P%d">' % (i + 1))
        for bi, (bs, bl, sig) in enumerate(bars):
            be = bs + bl
            x = '<measure number="%d">' % (bi + 1)
            if bi == 0 or sig != bars[bi - 1][2]:
                x += '<attributes>%s<time><beats>%d</beats><beat-type>%d</beat-type></time>%s</attributes>' % (
                    '<divisions>%d</divisions>' % DIV if bi == 0 else '', sig[0], sig[1],
                    '<clef><sign>%s</sign><line>%d</line></clef>' % (('G', 2) if i == 0 else ('F', 4)) if bi == 0 else '')
            for vi, v in enumerate(voices):
                if vi:
                    x += '<backup><duration>%d</duration></backup>' % int(bl * DIV)
                pos = bs
                for a, b, ms in v:
                    if b <= bs or a >= be:
                        continue
                    ca, cb = max(a, bs), min(b, be)
                    if ca > pos:
                        x += el(ca - pos, False, 0, 0, 0, rest=True)
                    for j, m in enumerate(ms):
                        x += el(cb - ca, j > 0, m, b > be, a < bs)
                    pos = cb
                if pos < be:
                    x += el(be - pos, False, 0, 0, 0, rest=True)
            out.append(x.replace('<note>', '<note>') + '</measure>')
        out.append('</part>')
    out.append('</score-partwise>')
    open(path, 'w', encoding='utf-8').write(''.join(out))
