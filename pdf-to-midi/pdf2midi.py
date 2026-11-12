#!/usr/bin/env python3
"""
pdf2midi.py - convert vector sheet-music PDFs (MuseScore / SMuFL-font exports) to MIDI.

This is NOT image-based OMR.  MuseScore PDFs embed noteheads, clefs, rests, accidentals,
flags and dots as font glyphs (SMuFL code points) and draw staff lines, stems, beams, ties
and barlines as vector paths, so every symbol has an exact position that we read directly.

Usage:  python3 -I pdf2midi.py score.pdf [-o out.mid] [--single-track] [--bpm N] [--report]
"""
import sys, argparse, collections
from fractions import Fraction as F
import pdfplumber
import mido

PPQ = 480
LETTER_SEMI = [0, 2, 4, 5, 7, 9, 11]
SHARP_ORDER = [3, 0, 4, 1, 5, 2, 6]            # F C G D A E B (letter idx, C=0)
FLAT_ORDER = [6, 2, 5, 1, 4, 0, 3]             # B E A D G C F

HEADS = {0xE0A2: 'whole', 0xE0A3: 'half', 0xE0A4: 'black',
         0xF4BC: 'whole', 0xF4BD: 'half', 0xF4BE: 'black'}      # Dorico's Bravura alternates
RESTS = {0xF4E3: F(2), 0xE4E2: F(8), 0xE4E3: F(4), 0xE4E4: F(2), 0xE4E5: F(1), 0xE4E6: F(1, 2),
         0xE4E7: F(1, 4), 0xE4E8: F(1, 8), 0xE4E9: F(1, 16), 0xE4EA: F(1, 32)}
ACCS = {0xE260: -1, 0xE261: 0, 0xE262: 1, 0xE263: 2, 0xE264: -2}
FLAGS = {0xE240: 1, 0xE241: 1, 0xE242: 2, 0xE243: 2, 0xE244: 3, 0xE245: 3, 0xE246: 4, 0xE247: 4,
         0xE248: 5, 0xE249: 5}
# clef glyph -> diatonic number (C0=0, C4=28) of the pitch on the clef's own line
CLEFS = {0xE050: 32, 0xE07A: 32, 0xE052: 25, 0xE053: 39, 0xE051: 18, 0xE054: 46,
         0xE062: 24, 0xE07C: 24, 0xE064: 17, 0xE065: 31, 0xE063: 10,
         0xE05C: 28, 0xE07B: 28}
DYN = {0xE520: 45, 0xE527: 33, 0xE528: 24, 0xE52C: 62, 0xE52D: 80, 0xE522: 100, 0xE52F: 70}
TIMESIG_DIGITS = {0xE080 + i: i for i in range(10)}
TUPLET_DIGITS = {0xE880 + i: i for i in range(10)}
TUPLET_ALT = {2: [3], 3: [2, 4], 4: [3, 2], 5: [4, 6, 8], 6: [4, 6, 8], 7: [4, 8, 6], 8: [6, 4, 12], 9: [8, 6], 10: [8], 11: [8], 12: [8]}
TUPLET_NORMAL = {2: 3, 3: 2, 4: 3, 5: 4, 6: 4, 7: 4, 8: 6, 9: 8, 10: 8, 11: 8, 12: 8}
DOT = 0xE1E7
OTTAVA = set(range(0xE510, 0xE51C))
OTTAVA_15 = set(range(0xE515, 0xE51A))
MET_NOTES = {0xECA3: F(2), 0xECA5: F(1), 0xECA7: F(1, 2), 0xECA1: F(4), 0xECA9: F(1, 4)}
MET_DOT = 0xECB7
GRACE_Q = F(1, 8)


# ---- Sibelius (Opus / Norfolk / Helsinki-style) fonts: translate their code points to SMuFL equivalents
OPUS_STD = {0x153: 0xE0A4, 0x2D9: 0xE0A3, 0x77: 0xE0A2,                  # heads: black, half, whole
            0x23: 0xE262, 0x62: 0xE260, 0x6E: 0xE261, 0x78: 0xE263,      # sharp, flat, natural, double sharp
            0x152: 0xE4E5, 0x2030: 0xE4E6, 0x2248: 0xE4E7,               # rests: quarter, 8th, 16th
            0xD3: 0xF4E3,                                                # Opus half/whole rest (same glyph on the middle line)
            0x4A: 0xE240, 0x6A: 0xE241,                                  # 8th flags
            0x52: 0xE242, 0x72: 0xE243,                                  # 16th flags (verify)
            0x26: 0xE050, 0x3F: 0xE062}                                  # G clef, F clef
OPUS_STD.update({0x30 + i: 0xE080 + i for i in range(10)})               # time signature digits
OPUS_SPECIAL = {0x2122: 0xE1E7,                                          # augmentation dot
                0x201C: 0xE510, 0x201D: 0xE510}                          # "8" of an 8va line


def translate(fontname, cp):
    """Return the SMuFL-equivalent code point for a Sibelius music-font character (or cp unchanged)."""
    f = fontname.split('+')[-1]
    if f.startswith(('Opus', 'Norfolk')) and not f.endswith('TextStd') and 'SpecialII' not in f:
        if 'Special' in f:
            return OPUS_SPECIAL.get(cp, 0)
        return OPUS_STD.get(cp, 0)
    return cp


class G:
    def __init__(s, ch, H, cp=None):
        s.cp = ord(ch['text'][0]) if cp is None else cp
        s.x = ch['matrix'][4]
        s.y = H - ch['matrix'][5]
        s.w = ch['x1'] - ch['x0']
        s.size = ch['size']

    def __repr__(s):
        return 'G(%04X x=%.1f y=%.1f)' % (s.cp, s.x, s.y)


class Staff:
    def __init__(s, ys, x0, x1):
        s.ys = ys
        s.top, s.bottom = ys[0], ys[-1]
        s.sp = (ys[-1] - ys[0]) / 4.0
        s.x0, s.x1 = x0, x1
        s.system = None
        s.index = None

    def step(s, y):
        return (s.bottom - y) / (s.sp / 2.0)

    def dist(s, y):
        return s.top - y if y < s.top else (y - s.bottom if y > s.bottom else 0.0)


class Beam:
    def __init__(s, pts):
        xs = [p[0] for p in pts]
        s.x0, s.x1 = min(xs), max(xs)
        L = [p[1] for p in pts if abs(p[0] - s.x0) < 0.05]
        R = [p[1] for p in pts if abs(p[0] - s.x1) < 0.05]
        s.yl = sum(L) / len(L)
        s.yr = sum(R) / len(R)

    def yc(s, x):
        if s.x1 - s.x0 < 0.01:
            return s.yl
        return s.yl + (s.yr - s.yl) * (x - s.x0) / (s.x1 - s.x0)


class Head:
    pass


class Event:
    pass


def cluster(vals, tol):
    out = []
    for v in sorted(vals):
        if out and v - out[-1][-1] <= tol:
            out[-1].append(v)
        else:
            out.append([v])
    return out


# ------------------------------------------------------------------------------------
def read_page(page):
    H = page.height
    glyphs = []
    for ch in page.chars:
        t = ch['text']
        if not t:
            continue
        fn = ch['fontname'].split('+')[-1]
        if fn.startswith(('Opus', 'Norfolk')) and not fn.endswith('TextStd'):
            cp = translate(fn, ord(t[0]))
            if cp and ch['matrix'][3] > 0:
                glyphs.append(G(ch, H, cp))
        elif 0xE000 <= ord(t[0]) <= 0xF8FF and ch['matrix'][3] > 0:
            glyphs.append(G(ch, H))
    texts = []
    for ch in page.chars:
        t = ch['text']
        fn = ch['fontname'].split('+')[-1]
        if t and not (0xE000 <= ord(t[0]) <= 0xF8FF) and not (fn.startswith(('Opus', 'Norfolk')) and not fn.endswith('TextStd')):
            texts.append((ch['x0'], H - ch['matrix'][5], t, ch['fontname'], ch['x1'] - ch['x0']))
    hlines, vlines = [], []
    for l in page.lines:
        w, h = l['x1'] - l['x0'], l['bottom'] - l['top']
        if abs(h) < 0.05:
            hlines.append((min(l['x0'], l['x1']), max(l['x0'], l['x1']), l['top'], l['linewidth']))
        elif abs(w) < 0.05:
            vlines.append((l['x0'], l['top'], l['bottom'], l['linewidth']))
    beams_raw, arcs = [], []
    for c in page.curves:
        ops = [o[0] for o in c['path']]
        if 'c' in ops:
            p0 = c['path'][0][1]
            p1 = next((o[3] for o in c['path'][1:] if o[0] == 'c'), None)
            if p1:
                arcs.append((p0, p1))
        elif c['fill'] and len(c['pts']) >= 4:
            beams_raw.append(c['pts'])
    return glyphs, texts, hlines, vlines, beams_raw, arcs, [(r['x0'], r['x1'], r['top'], r['bottom']) for r in page.rects if r['fill']]


def find_staves(hlines):
    byy = collections.defaultdict(list)
    for x0, x1, y, lw in hlines:
        if x1 - x0 > 20 and lw < 2.5:
            byy[round(y, 1)].append((x0, x1))
    ys = sorted(byy)
    staves, i = [], 0
    while i <= len(ys) - 5:
        grp = ys[i:i + 5]
        gaps = [grp[k + 1] - grp[k] for k in range(4)]
        if max(gaps) - min(gaps) < 0.6 and 2.0 < gaps[0] < 8:
            xs0 = min(x for y in grp for x, _ in byy[y])
            xs1 = max(x for y in grp for _, x in byy[y])
            if xs1 - xs0 > 100:
                staves.append(Staff(grp, xs0, xs1))
                i += 5
                continue
        i += 1
    return staves


def group_systems(staves, vlines):
    systems, cur = [], []
    for s in staves:
        if cur:
            prev = cur[-1]
            joined = any(abs(x - s.x0) < 1.5 and lw > 0.15 * s.sp and t <= prev.bottom + 1 and b >= s.top - 1
                         for x, t, b, lw in vlines)
            if not joined:
                systems.append(cur)
                cur = []
        cur.append(s)
    if cur:
        systems.append(cur)
    for si, sy in enumerate(systems):
        for k, s in enumerate(sy):
            s.system, s.index = si, k
    return systems


def midi_of(dia, alter):
    return 12 * (dia // 7 + 1) + LETTER_SEMI[dia % 7] + alter


def read_key(staff, glyphs, first_x):
    """Return (n_accidentals signed, [glyphs]) for the key signature run at the system start."""
    accs = sorted([g for g in glyphs if g.cp in ACCS and ACCS[g.cp] in (1, -1) and staff.dist(g.y) < staff.sp * 3
                   and staff.x0 < g.x < first_x - staff.sp * 0.5], key=lambda g: g.x)
    run = []
    for g in accs:
        if run and g.x - run[-1].x > staff.sp * 1.7:
            break
        if not run and g.x - staff.x0 > staff.sp * 16:
            break
        run.append(g)
    if not run:
        return 0, []
    kind = ACCS[run[0].cp]
    run = [g for g in run if ACCS[g.cp] == kind][:7]
    return kind * len(run), run


def staff_keys(s, mine, sysglyphs, bounds):
    """Key signature in force for every bar of this staff (handles mid-system key changes).
    Returns (list of signed accidental counts, set of glyph ids that belong to key signatures)."""
    sp = s.sp
    heads_all = [g for g in sysglyphs if g.cp in HEADS]
    evs = [g for g in mine if g.cp in HEADS or g.cp in RESTS]
    first_x = min([g.x for g in evs] + [bounds[1] if len(bounds) > 1 else s.x1])
    n0, run0 = read_key(s, mine, first_x)
    keys, ids = [n0], set(id(g) for g in run0)

    def attached(g):
        return any(abs(h.y - g.y) < 0.4 * sp and 0.2 * sp < h.x - g.x < 2.2 * sp for h in heads_all)
    for mi in range(1, len(bounds) - 1):
        lo, hi = bounds[mi], bounds[mi + 1]
        fx = min([g.x for g in evs if lo <= g.x < hi] + [hi])
        cand = sorted([g for g in sysglyphs if g.cp in ACCS and lo - 1 <= g.x < fx - 0.5 * sp and g.x - lo < 14 * sp
                       and s.top - 1.5 * sp <= g.y <= s.bottom + 1.5 * sp and not attached(g)], key=lambda g: g.x)
        run = []
        for g in cand:
            if run and g.x - run[-1].x > 1.7 * sp:
                break
            if not run and g.x - lo > 3.8 * sp:
                break
            run.append(g)
        if run:
            for g in run:
                ids.add(id(g))
            sh = [g for g in run if ACCS[g.cp] in (1, -1)]
            keys.append((ACCS[sh[0].cp] * len(sh)) if sh else 0)
        else:
            keys.append(keys[-1])
    return keys, ids


def key_alter(n):
    d = {}
    if n > 0:
        for l in SHARP_ORDER[:n]:
            d[l] = 1
    elif n < 0:
        for l in FLAT_ORDER[:-n]:
            d[l] = -1
    return d


# ------------------------------------------------------------------------------------
class Converter:
    def __init__(self, path):
        try:
            self.pdf = pdfplumber.open(path)
        except Exception:
            # some exporters write PDFs pdfminer cannot parse; qpdf rewrites them into a clean file
            import subprocess, tempfile, os
            fixed = os.path.join(tempfile.mkdtemp(prefix='p2m_'), 'fixed.pdf')
            subprocess.run(['qpdf', path, fixed], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            self.pdf = pdfplumber.open(fixed)
        self.notes = []            # dict(start,end,midi,staff,vel_tick)
        self.tempo_events = []     # (tick F, bpm)
        self.dyn_events = []       # (tick F, vel)
        self.cur = F(0)
        self.meter = (4, 4)
        self.mno = 0
        self.warnings = []
        self.open_ties = {}
        self.log = []
        self.sig_events = []       # (tick, (num,den))
        self.stats = collections.Counter()
        self.overlay = collections.defaultdict(list)
        self.soft = []
        self.bad_boxes = []
        self.cur_box = None

    def run(self):
        for pno, page in enumerate(self.pdf.pages):
            self.do_page(pno, page)
        return self

    def warn(self, msg):
        self.warnings.append(msg)

    # --------------------------------------------------------------------- page
    def do_page(self, pno, page):
        glyphs, texts, hlines, vlines, braw, arcs, rects = read_page(page)
        staves = find_staves(hlines)
        if not staves:
            self.warn('page %d: no staff lines found (skipped)' % (pno + 1))
            return
        systems = group_systems(staves, vlines)
        beams = [Beam(b) for b in braw]
        sp0 = staves[0].sp
        for (x0, x1, t, b) in rects:          # flat beams are drawn as thin rects
            if 0.35 * sp0 <= b - t <= 0.65 * sp0 and x1 - x0 > 2:
                beams.append(Beam([(x0, t), (x1, t), (x1, b), (x0, b)]))
        ctx = dict(pno=pno, glyphs=glyphs, texts=texts, hlines=hlines, vlines=vlines, beams=beams,
                   arcs=arcs, allstaves=staves)
        for sy in systems:
            self.do_system(sy, ctx)

    # ------------------------------------------------------------------- system
    def do_system(self, sysstaves, ctx):
        allstaves, glyphs, vlines = ctx['allstaves'], ctx['glyphs'], ctx['vlines']
        sp = sysstaves[0].sp
        ytop, ybot = sysstaves[0].top - 14 * sp, sysstaves[-1].bottom + 14 * sp
        sg = []
        ledgers = [(a_, b_, y) for a_, b_, y, lw in ctx['hlines'] if lw > 0.145 * sp and 0.8 * sp < b_ - a_ < 3.6 * sp]

        def head_owner(g):
            ok = []
            for st in allstaves:
                d = st.dist(g.y)
                if d == 0:
                    return st
                need = int(d / st.sp + 0.3)
                good = True
                for k in range(1, need + 1):
                    yl = st.bottom + k * st.sp if g.y > st.bottom else st.top - k * st.sp
                    if not any(abs(y - yl) < 0.3 * st.sp and a_ - 1 <= g.x + g.w / 2 <= b_ + 1 for a_, b_, y in ledgers):
                        good = False
                        break
                if good:
                    ok.append((d, st))
            if ok:
                return min(ok, key=lambda t: t[0])[1]
            return min(allstaves, key=lambda s: s.dist(g.y))
        for g in glyphs:
            if ytop <= g.y <= ybot:
                o = head_owner(g) if g.cp in HEADS else min(allstaves, key=lambda s: s.dist(g.y))
                if o in sysstaves:
                    sg.append((g, o))
        x0 = min(s.x0 for s in sysstaves)
        x1 = max(s.x1 for s in sysstaves)
        st0 = sysstaves[0]
        bx = [x for x, t, b, lw in vlines if lw > 0.145 * sp and t <= st0.bottom - sp and b >= st0.top + sp and x0 + 1 < x < x1 + 1]
        bar_x = [c[-1] for c in cluster(bx, 2.5)]
        bounds = [x0]
        for b in bar_x:
            if b > bounds[-1] + sp * 3 and b < x1 - 1:
                bounds.append(b)
        bounds.append(x1)
        nmeas = len(bounds) - 1

        def measure_of(x):
            return sum(1 for b in bounds[1:-1] if x >= b - 0.5)

        # time signatures (first staff)
        tsig = {}
        tg = [g for g, o in sg if o is st0 and g.cp in TIMESIG_DIGITS]
        for col in cluster([g.x for g in tg], sp * 3.5):
            cg = [g for g in tg if col[0] - 0.1 <= g.x <= col[-1] + 0.1]
            mid = st0.top + 2 * sp
            up = sorted([g for g in cg if g.y < mid], key=lambda g: g.x)
            dn = sorted([g for g in cg if g.y >= mid], key=lambda g: g.x)
            if up and dn:
                tsig[measure_of(col[0])] = (int(''.join(str(TIMESIG_DIGITS[g.cp]) for g in up)),
                                            int(''.join(str(TIMESIG_DIGITS[g.cp]) for g in dn)))
        for g, o in sg:                         # common / cut time
            if o is st0 and g.cp in (0xE08A, 0xE08B):
                tsig[measure_of(g.x)] = (4, 4) if g.cp == 0xE08A else (2, 2)

        allkey, skeys = set(), {}
        for s in sysstaves:
            mine_ = [g for g, o in sg if o is s]
            skeys[s] = staff_keys(s, mine_, [g for g, o in sg], bounds)
            allkey |= skeys[s][1]
        events = []
        for s in sysstaves:
            mine = [g for g, o in sg if o is s]
            events += self.build_events(s, mine, ctx, bounds, measure_of, [g for g, o in sg], allkey, skeys[s][0])
        self.apply_tuplets(sysstaves, events, sg, ctx, bounds)
        self.apply_tremolo(sysstaves, events, ctx)
        dyns = [(g.x, DYN[g.cp]) for g, o in sg if g.cp in DYN]
        tempos = self.read_tempos(sg, ctx)
        tempo_x = tempos
        for mi in range(nmeas):
            if mi in tsig:
                self.meter = tsig[mi]
                self.sig_events.append((self.cur, self.meter))
            mlen = F(self.meter[0] * 4, self.meter[1])
            mev = [e for e in events if e.mi == mi]
            if not mev and mi == nmeas - 1 and bounds[mi + 1] - bounds[mi] < 1.2 * sp:
                continue          # a stray barline pair at a system edge, not a real (empty) bar
            self.cur_box = (ctx['pno'], bounds[mi], bounds[mi + 1], sysstaves[0].top - 7 * sp, sysstaves[-1].bottom + 7 * sp)
            snap = self._snapshot(ctx['pno'])
            nbad = len(self.bad_boxes)
            L = self.place_measure(mev, mlen, mi, bounds, sp, dyns, tempo_x, sysstaves[0].system, ctx['pno'])
            if len(self.bad_boxes) > nbad:
                groups = self.implicit_triplets(mev)
                if groups:
                    self._restore(snap, ctx['pno'])
                    for g in groups:
                        for e in g:
                            e.dur_before, e.dur = e.dur, e.dur * F(2, 3)
                            e.tuplet = True
                    nbad = len(self.bad_boxes)
                    L2 = self.place_measure(mev, mlen, mi, bounds, sp, dyns, tempo_x, sysstaves[0].system, ctx['pno'])
                    if len(self.bad_boxes) == nbad:
                        L = L2
                        self.stats['implicit_triplets'] += len(groups)
                    else:
                        self._restore(snap, ctx['pno'])
                        for g in groups:
                            for e in g:
                                e.dur, e.tuplet = e.dur_before, False
                        L = self.place_measure(mev, mlen, mi, bounds, sp, dyns, tempo_x, sysstaves[0].system, ctx['pno'])
            self.cur += L
            self.mno += 1

    def _snapshot(self, pno):
        return (len(self.notes), len(self.warnings), len(self.bad_boxes), len(self.dyn_events), len(self.tempo_events),
                len(self.soft), len(self.log), len(self.overlay[pno]), dict(self.open_ties), collections.Counter(self.stats), self.mno)

    def _restore(self, snap, pno):
        n, w, b, d, t, so, lg, ov, ties, stats, mno = snap
        del self.notes[n:], self.warnings[w:], self.bad_boxes[b:], self.dyn_events[d:], self.tempo_events[t:], self.soft[so:], self.log[lg:]
        del self.overlay[pno][ov:]
        self.open_ties = ties
        self.stats = stats
        self.mno = mno

    @staticmethod
    def implicit_triplets(mev):
        """Unmarked triplets: engravers often write the '3' only on the first group. Candidates are beamed groups of
        exactly three equal-length, un-tupleted notes (or two such groups sharing a beam)."""
        by = collections.defaultdict(list)
        for e in mev:
            if getattr(e, 'bid', None) is not None and e.kind == 'chord' and not e.grace and not getattr(e, 'tuplet', False):
                by[(e.staff.index, e.bid)].append(e)
        out = []
        for g in by.values():
            g.sort(key=lambda e: e.mx)
            if len(g) in (3, 6) and len({e.dur for e in g}) == 1 and g[0].dur in (F(1, 2), F(1, 4), F(1, 8)):
                out.append(g)
        return out

    # ------------------------------------------------------------------- events
    def build_events(self, s, mine, ctx, bounds, measure_of, sysglyphs, allkey, keys):
        sp = s.sp
        beams, arcs, hlines, vlines = ctx['beams'], ctx['arcs'], ctx['hlines'], ctx['vlines']
        hs = sorted([g for g in mine if g.cp in HEADS], key=lambda g: g.x)
        first_x = min([g.x for g in hs] + [bounds[1] if len(bounds) > 1 else s.x1])
        keyids = set()
        accs = [g for g in sysglyphs if g.cp in ACCS and id(g) not in keyids and id(g) not in allkey]
        flags = [g for g in sysglyphs if g.cp in FLAGS]
        rests = [g for g in mine if g.cp in RESTS]
        dots = [g for g in sysglyphs if g.cp == DOT]
        clefs = sorted([g for g in mine if g.cp in CLEFS], key=lambda g: g.x)
        stems = [(x, t, b) for x, t, b, lw in vlines
                 if lw < 0.145 * sp and (b - t) > 1.6 * sp and s.top - 9 * sp <= (t + b) / 2 <= s.bottom + 9 * sp
                 and s.x0 - 1 <= x <= s.x1 + 1]
        grace_cut = sp * 4 * 0.85

        def clef_for(x):
            c = None
            for g in clefs:
                if g.x <= x + 0.5:
                    c = g
            if c is None and clefs:
                c = clefs[0]
            if c is None:
                return 32, 2
            return CLEFS[c.cp], round(s.step(c.y))

        heads = []
        for g in hs:
            h = Head()
            h.x, h.y, h.w, h.kind = g.x, g.y, g.w, HEADS[g.cp]
            h.grace = g.size < grace_cut
            h.step = round(s.step(g.y))
            cd, cstep = clef_for(g.x)
            h.dia = cd + (h.step - cstep)
            cand = [a for a in accs if abs(a.y - g.y) < sp * 0.4 and 0.2 * sp < g.x - a.x < 5.5 * sp]
            h.acc = ACCS[max(cand, key=lambda a: a.x).cp] if cand else None
            h.stem = None
            h.tie_in = h.tie_out = False
            h.staff = s
            heads.append(h)
        # accidentals -> pitch (per-measure memory)
        keyalts = [key_alter(k) for k in keys]
        state, lastm = {}, -1
        for h in sorted(heads, key=lambda h: h.x):
            mi = measure_of(h.x)
            if mi != lastm:
                state, lastm = {}, mi
            if h.acc is not None:
                state[h.dia] = h.acc
                alter = h.acc
            else:
                alter = state.get(h.dia, keyalts[min(mi, len(keyalts) - 1)].get(h.dia % 7, 0))
            h.midi0 = midi_of(h.dia, alter)
            h.midi = h.midi0
        # stems -> chords
        stem_heads = collections.defaultdict(list)
        for h in heads:
            if h.kind == 'whole':
                continue
            best = None
            for si, (x, t, b) in enumerate(stems):
                if not (t - 0.45 * sp <= h.y <= b + 0.45 * sp):
                    continue
                d = min(abs(x - (h.x + h.w)), abs(x - h.x))
                if d < 0.4 * sp and (best is None or d < best[0]):
                    best = (d, si)
            if best:
                stem_heads[best[1]].append(h)
                h.stem = best[1]
        groups = [(si, hl) for si, hl in stem_heads.items()]
        loose = sorted([h for h in heads if h.stem is None], key=lambda h: h.x)
        cur = []
        for h in loose:
            if cur and h.x - cur[-1][-1].x <= 1.3 * h.w and h.kind == cur[-1][0].kind:
                cur[-1].append(h)
            else:
                cur.append([h])
        groups += [(None, hl) for hl in cur]
        events = []
        for si, hl in groups:
            e = Event()
            e.kind, e.staff, e.heads = 'chord', s, hl
            e.grace = all(h.grace for h in hl)
            e.whole_measure = False
            e.rest_whole = False
            e.stem_x = None
            e.trem = None
            e.right = max(h.x + h.w for h in hl)
            if si is None:
                e.dir = 0
                xs = collections.Counter(round(h.x, 1) for h in hl)
                e.mx = max(xs.items(), key=lambda kv: (kv[1], kv[0]))[0]
                e.beams = 0
                e.tipy = None
                if hl[0].kind != 'whole':
                    self.stats['head_without_stem'] += 1
                base = F(4) if hl[0].kind == 'whole' else (F(2) if hl[0].kind == 'half' else F(1))
            else:
                x, t, b = stems[si]
                ymax, ymin = max(h.y for h in hl), min(h.y for h in hl)
                up = abs(ymax - b) <= abs(ymin - t)
                e.dir = 1 if up else -1
                tip = t if up else b
                e.tipy = tip
                e.stem_x = x
                e.mx = (x - hl[0].w) if up else x
                nfl = 0
                for g in flags:
                    if abs(g.x - x) < 0.3 * sp and abs(g.y - tip) < 1.2 * sp:
                        nfl = max(nfl, FLAGS[g.cp])
                e.beams = max(nfl, self.count_beams(beams, x, tip, e.dir, sp))
                e.bid = self.beam_id(beams, x, tip, e.dir, sp)
                base = F(2) if hl[0].kind == 'half' else (F(1, 2 ** e.beams) if e.beams else F(1))
            e.base = base
            events.append(e)
        for g in rests:
            e = Event()
            e.kind, e.staff, e.heads, e.dir, e.grace = 'rest', s, [], 0, False
            e.mx, e.right, e.base, e.tipy = g.x, g.x + g.w, RESTS[g.cp], None
            e.whole_measure = False
            e.rest_whole = g.cp in (0xE4E3, 0xE4E2, 0xF4E3)
            e.stem_x = None
            e.trem = None
            e.beams = 0
            e.rg = g
            events.append(e)
        # dots: each dot belongs to the nearest compatible note/rest on its left
        for e in events:
            e.dotx = []
        for d in dots:
            best = None
            for e in events:
                pen = None
                if e.kind == 'chord':
                    for h in e.heads:
                        if h.step % 2:
                            if abs(d.y - h.y) < 0.3 * sp:
                                pen = 0 if pen is None else min(pen, 0)
                        else:
                            if abs(d.y - (h.y - sp / 2)) < 0.3 * sp:
                                pen = 0 if pen is None else min(pen, 0)
                            elif abs(d.y - (h.y + sp / 2)) < 0.3 * sp:
                                pen = 1.2 * sp if pen is None else min(pen, 1.2 * sp)
                else:
                    if e.rg.y - 2.4 * sp <= d.y <= e.rg.y + 0.8 * sp:
                        pen = 0
                gap = d.x - e.right
                if pen is not None and 0.1 * sp < gap < 2.4 * sp:
                    cost = gap + pen
                    if best is None or cost < best[0]:
                        best = (cost, e)
            if best:
                best[1].dotx.append(d.x)
        for e in events:
            nd = len(cluster(e.dotx, 0.3 * sp)) if e.dotx else 0
            e.dots = nd
            dur, add = e.base, e.base
            for _ in range(nd):
                add /= 2
                dur += add
            e.dur = dur
        for e in events:
            e.mi = measure_of(e.mx)
        self.apply_ottava(s, heads, mine, ctx)
        self.detect_ties(s, heads, arcs)
        return events

    # ------------------------------------------------------------------ helpers
    @staticmethod
    def beam_id(beams, x, tip, d, sp):
        """index of the beam polygon nearest the stem tip (identifies the beamed group), or None"""
        best = None
        for i, b in enumerate(beams):
            if b.x0 - 0.11 * sp <= x <= b.x1 + 0.11 * sp:
                off = (b.yc(min(max(x, b.x0), b.x1)) - tip) * (1 if d == 1 else -1)
                if -0.22 * sp <= off <= 0.8 * sp and (best is None or off < best[0]):
                    best = (off, i)
        return best[1] if best else None

    @staticmethod
    def count_beams(beams, x, tip, d, sp):
        offs = []
        for b in beams:
            if b.x0 - 0.11 * sp <= x <= b.x1 + 0.11 * sp:
                yc = b.yc(min(max(x, b.x0), b.x1))
                off = (yc - tip) * (1 if d == 1 else -1)
                if -0.22 * sp <= off <= 3.1 * sp:
                    offs.append(off)
        offs.sort()
        n = 0
        for off in offs:
            exp = 0.25 * sp + 0.75 * sp * n
            if abs(off - exp) <= 0.42 * sp:
                n += 1
            elif off > exp + 0.42 * sp:
                break
        return n

    def apply_ottava(self, s, heads, mine, ctx):
        sp = s.sp
        last_ott_x = -1e9
        for g in sorted(mine, key=lambda g: g.x):
            if g.cp not in OTTAVA:
                continue
            if g.x - last_ott_x < 2.5 * sp:       # the '8' of an 8va is sometimes drawn twice (bold overlay)
                continue
            last_ott_x = g.x
            lines = [(x0, x1, y) for x0, x1, y, lw in ctx['hlines']
                     if lw < 0.25 * sp and abs(y - g.y) < 1.6 * sp and -0.5 * sp < x0 - g.x < 4 * sp and x1 - x0 > 3 * sp]
            end = max([l[1] for l in lines], default=g.x + g.w + 2)
            above = g.y < s.top + 2 * sp
            shift = (24 if g.cp in OTTAVA_15 else 12) * (1 if above else -1)
            n = 0
            for h in heads:
                if g.x - 1 <= h.x <= end + 1:
                    h.midi += shift
                    n += 1
            self.stats['ottava_notes'] += n

    def detect_ties(self, s, heads, arcs):
        sp = s.sp
        for p0, p1 in arcs:
            L, R = (p0, p1) if p0[0] <= p1[0] else (p1, p0)
            if R[0] - L[0] < 1.2 * sp:
                continue
            def best(tip, side):
                c = []
                for h in heads:
                    if abs(tip[1] - h.y) > 1.7 * sp:
                        continue
                    tx = h.x + (0.75 if side == 'L' else 0.25) * h.w
                    dx = tip[0] - tx
                    if (side == 'L' and -0.8 * sp <= dx <= 1.8 * sp) or (side == 'R' and -1.8 * sp <= dx <= 0.8 * sp):
                        c.append((abs(dx) + abs(tip[1] - h.y) * 0.5, h))
                return min(c, key=lambda t: t[0])[1] if c else None
            A, B = best(L, 'L'), best(R, 'R')
            if A and B and A is not B and A.midi0 == B.midi0 and B.x > A.x + sp:
                between = [h for h in heads if h.midi0 == A.midi0 and A.x + 1 < h.x < B.x - 1]
                if not between:
                    A.tie_out = True
                    B.tie_in = True
            elif A and not B and R[0] >= s.x1 - 6 * sp:
                A.tie_out = True
            elif B and not A and L[0] <= s.x0 + 18 * sp:
                B.tie_in = True

    def apply_tremolo(self, sysstaves, events, ctx):
        """Two-note tremolo: floating strokes between two stems that do not touch either stem."""
        sp = sysstaves[0].sp
        stemmed = [e for e in events if e.kind == 'chord' and e.stem_x is not None and not e.grace]
        pairs = collections.defaultdict(list)
        for b in ctx['beams']:
            if b.x1 - b.x0 < 1.5 * sp or b.x1 - b.x0 > 14 * sp:
                continue
            yc = (b.yl + b.yr) / 2
            if not (sysstaves[0].top - 8 * sp <= yc <= sysstaves[-1].bottom + 8 * sp):
                continue
            touch = [e for e in stemmed if abs(e.stem_x - b.x0) < 0.18 * sp or abs(e.stem_x - b.x1) < 0.18 * sp]
            if touch:
                continue
            L = [e for e in stemmed if b.x0 - 1.06 * sp <= e.stem_x <= b.x0 - 0.22 * sp and e.staff.dist(yc) < 6 * sp]
            R = [e for e in stemmed if b.x1 + 0.22 * sp <= e.stem_x <= b.x1 + 1.06 * sp and e.staff.dist(yc) < 6 * sp]
            if not (L and R):
                sl = [e for e in events if e.kind == 'chord' and e.stem_x is None and not e.grace and e.staff.dist(yc) < 6 * sp
                      and 0.3 * sp <= b.x0 - e.right <= 6.5 * sp]
                sr = [e for e in events if e.kind == 'chord' and e.stem_x is None and not e.grace and e.staff.dist(yc) < 6 * sp
                      and 0.3 * sp <= e.mx - b.x1 <= 6.5 * sp]
                if sl and sr:
                    e1 = min(sl, key=lambda e: b.x0 - e.right)
                    e2 = min(sr, key=lambda e: e.mx - b.x1)
                    if e1 is not e2:
                        pairs[(id(e1), id(e2))].append((e1, e2, b))
                continue
            if L and R:
                e1 = min(L, key=lambda e: abs(e.stem_x - (b.x0 - 0.62 * sp)))
                e2 = min(R, key=lambda e: abs(e.stem_x - (b.x1 + 0.62 * sp)))
                if e1 is not e2:
                    pairs[(id(e1), id(e2))].append((e1, e2, b))
        for key, lst in pairs.items():
            e1, e2 = lst[0][0], lst[0][1]
            k = len(lst)
            e1.trem = (e2, k)
            e2.trem = (e1, -k)
            e1.dur /= 2          # in a two-note tremolo the pair together fills the written time
            e2.dur /= 2
            self.stats['tremolo_pairs'] += 1

    def apply_tuplets(self, sysstaves, events, sg, ctx, bounds):
        """Find tuplet numbers (SMuFL digit glyphs or italic text digits) and scale the members."""
        sp = sysstaves[0].sp
        beams, hl = ctx['beams'], ctx['hlines']
        ytop, ybot = sysstaves[0].top - 6 * sp, sysstaves[-1].bottom + 6 * sp
        digits = []                     # (n, x, y, w)
        for g, o in sg:
            if g.cp in TUPLET_DIGITS:
                digits.append((TUPLET_DIGITS[g.cp], g.x, g.y, g.w))
        x0, x1 = bounds[0], bounds[-1]
        for tx, ty, t, font, tw in ctx['texts']:
            if t.isdigit() and len(t) == 1 and ('Italic' in font or 'OpusText' in font) and ytop <= ty <= ybot and x0 + 8 * sp < tx < x1 \
                    and not any(abs(tx - b) < 6 * sp and ty < sysstaves[0].top for b in bounds[:1]):
                digits.append((int(t), tx, ty, tw))
        cand = [e for e in events if not e.grace]
        for n, gx, gy, gw in digits:
            if n not in TUPLET_NORMAL:
                continue
            cx = gx + gw / 2.0
            cy = gy - 0.5 * sp
            segs = [(a_, b_) for a_, b_, y, lw in hl if lw < 0.25 * sp and abs(y - cy) < 1.0 * sp and b_ - a_ > 0.6 * sp
                    and not any(s.top - 0.5 <= y <= s.bottom + 0.5 for s in sysstaves)]
            left = [q for q in segs if gx - 5 * sp <= q[1] <= gx + 0.3 * sp]
            right = [q for q in segs if gx + gw - 0.3 * sp <= q[0] <= gx + gw + 5 * sp]
            members = None
            if left and right:                                   # bracketed tuplet
                xa, xb = min(q[0] for q in left), max(q[1] for q in right)
                members = [e for e in cand if xa - 0.6 * sp <= e.mx <= xb + 0.6 * sp
                           and (e.kind == 'rest' or e.beams == 0 or True)]
                above = gy < sysstaves[0].top + 2 * sp or True
                members = [e for e in members if abs((e.tipy if e.tipy is not None else gy) - gy) < 9 * sp]
            else:                                                # beamed tuplet: n consecutive columns nearest the digit
                bc = [e for e in cand if e.kind == 'chord' and e.beams >= 1 and abs(e.mx - cx) < (n + 2) * 4 * sp]
                cols = []
                for e in sorted(bc, key=lambda e: e.mx):
                    if cols and e.mx - cols[-1][-1].mx <= 1.0 * sp:
                        cols[-1].append(e)
                    else:
                        cols.append([e])
                best = None
                for i in range(0, len(cols) - n + 1):
                    run = cols[i:i + n]
                    ctr = (run[0][0].mx + run[-1][0].mx + run[-1][0].heads[0].w) / 2.0
                    sc = abs(ctr - cx)
                    if best is None or sc < best[0]:
                        best = (sc, run)
                if best and best[0] < 2.2 * sp:
                    ref = min(e.base for c in best[1] for e in c)
                    members = [e for c in best[1] for e in c if e.base == ref]
            if not members or len(members) < 2:
                self.warn('tuplet "%d" at x=%.0f: could not identify members' % (n, gx))
                continue
            scale = F(TUPLET_NORMAL[n], n)
            self.tup_id = getattr(self, 'tup_id', 0) + 1
            for e in members:
                e.tup = (self.tup_id, n, e.dur)
                e.dur *= scale
                e.tuplet = True
            self.stats['tuplets'] += 1

    def read_tempos(self, sg, ctx):
        out = []
        for g, o in sg:
            if g.cp in MET_NOTES:
                unit = MET_NOTES[g.cp]
                x = g.x + g.w
                for g2, _ in sg:
                    if g2.cp == MET_DOT and abs(g2.y - g.y) < 8 and 0 < g2.x - x < 8:
                        unit *= F(3, 2)
                        x = g2.x + g2.w
                digs = sorted([(tx, t) for tx, ty, t, _f, _w in ctx['texts'] if abs(ty - g.y) < 5 and 0 < tx - x < 40], key=lambda a: a[0])
                num = ''.join(t for _, t in digs if t.isdigit())
                if num:
                    out.append((g.x, float(num) * float(unit)))
        return out

    # ------------------------------------------------------------ time placement
    def place_measure(self, mev, mlen, mi, bounds, sp, dyns, tempos, sysno, pno):
        tag = 'p%d sys%d bar%d (#%d)' % (pno + 1, sysno + 1, mi + 1, self.mno + 1)
        graces = [e for e in mev if e.grace]
        mev = [e for e in mev if not e.grace]
        cands = [e for e in mev if e.kind == 'rest' and e.rest_whole]
        non = [o for o in mev if o not in cands]
        for e in cands:
            others = [o for o in non if o.staff is e.staff]
            if not others:
                e.whole_measure = True
            else:
                left = bounds[mi] if mi > 0 else min([o.mx for o in non] + [bounds[mi + 1]]) - sp
                center = (left + bounds[mi + 1]) / 2.0
                rc = (e.mx + e.right) / 2.0
                e.whole_measure = abs(rc - center) < 0.8 * sp
            if e.whole_measure:
                e.dur = mlen
        full = [e for e in cands if e.whole_measure]
        reg = sorted([e for e in mev if e not in full], key=lambda e: e.mx)
        def displaced(col, e):
            # second-interval displacement: a note shifted one head width sideways belongs to the same column
            if e.kind != 'chord':
                return False
            for m_ in col:
                if m_.kind != 'chord' or m_.staff is not e.staff:
                    continue
                dx = e.mx - m_.mx
                if 1.6 * sp <= dx <= 2.6 * sp and m_.heads[0].kind == 'whole':
                    if any(abs(a.y - b.y) <= 0.6 * sp for a in m_.heads for b in e.heads):
                        return True
            return False
        def rest_before(col, e):
            # a (short) rest sitting clearly left of a note in its own staff is an earlier event, not the same column
            if e.kind != 'chord':
                return False
            return any(r.kind == 'rest' and r.staff is e.staff and r.base < 1 and e.mx - r.mx > 0.5 * sp for r in col)
        def sweep():
            cols = []
            for e in reg:
                if cols and (e.mx - cols[-1][-1].mx <= 1.0 * sp or displaced(cols[-1], e)) and not rest_before(cols[-1], e):
                    cols[-1].append(e)
                else:
                    cols.append([e])
            chains = []
            t = F(0)
            col_ticks = []
            for ci, col in enumerate(cols):
                if ci > 0:
                    ends = [c['end'] for c in chains if c['end'] > t]
                    t = min(ends) if ends else t
                col_ticks.append(t)
                for e in sorted(col, key=lambda e: (e.staff.index, -e.dir)):
                    e.t = t
                    ch = None
                    free = [c for c in chains if c['end'] == t and not c.get('used')]
                    for pred in (lambda c: c['staff'] is e.staff and c['dir'] == e.dir,
                                 lambda c: c['staff'] is e.staff,
                                 lambda c: c['dir'] == e.dir,
                                 lambda c: True):
                        m = [c for c in free if pred(c)]
                        if m:
                            ch = m[0]
                            break
                    if ch is None:
                        ch = dict(end=t, staff=e.staff, dir=e.dir)
                        chains.append(ch)
                    ch['end'] = t + e.dur
                    ch['used'] = True
                for c in chains:
                    c['used'] = False
            return cols, col_ticks, chains

        cols, col_ticks, chains = sweep()
        if chains and max(c['end'] for c in chains) != mlen:
            # try other tuplet ratios (e.g. 7 in the time of 4 / 6 / 8) until the bar fits
            groups = collections.OrderedDict()
            for e in reg:
                if getattr(e, 'tup', None):
                    groups.setdefault(e.tup[0], []).append(e)
            if groups:
                import itertools
                gl = list(groups.items())
                opts = [TUPLET_ALT.get(g[1][0].tup[1], [g[1][0].tup[1]]) for g in gl]
                orig = [(e, e.dur) for _, es in gl for e in es]
                found = False
                for combo in itertools.islice(itertools.product(*[[F(x) for x in o] for o in opts]), 2000):
                    for (gid, es), normal in zip(gl, combo):
                        for e in es:
                            e.dur = e.tup[2] * normal / e.tup[1]
                    cols, col_ticks, chains = sweep()
                    if max(c['end'] for c in chains) == mlen:
                        found = True
                        self.stats['tuplet_ratio_fitted'] += 1
                        break
                if not found:
                    for e, d in orig:
                        e.dur = d
                    cols, col_ticks, chains = sweep()
        ends = set(c['end'] for c in chains)
        real = mlen
        if chains:
            mx_end = max(ends)
            if mx_end != mlen:
                if self.mno == 0 and mx_end < mlen:
                    real = mx_end
                    self.warn('%s: short bar (%s of %s quarters) - treated as pickup' % (tag, mx_end, mlen))
                else:
                    self.warn('%s: BAR LENGTH %s quarters, expected %s' % (tag, mx_end, mlen))
                    self.bad_boxes.append((self.mno + 1,) + self.cur_box)
            elif len(ends) > 1:
                self.stats['soft_voice_mismatch'] += 1
                self.soft.append('%s: voices end at %s' % (tag, sorted(str(x) for x in ends)))
        for e in full:
            e.t = F(0)
        base = self.cur
        # emit notes
        for e in sorted(reg, key=lambda e: (e.t, e.staff.index)):
            if e.kind != 'chord':
                continue
            if e.trem is not None:
                partner, k = e.trem
                if k > 0:
                    unit = F(1, 2 ** (min(k, 3) + 1))
                    total = e.dur + partner.dur
                    st, n = base + e.t, 0
                    while st < base + e.t + total - F(1, 1000):
                        who = e if n % 2 == 0 else partner
                        ln = min(unit, base + e.t + total - st)
                        for h in who.heads:
                            self.notes.append(dict(start=st, end=st + ln, midi=h.midi, staff=who.staff.index))
                        st += unit
                        n += 1
                for h in e.heads:
                    self.overlay[pno].append((h.x, h.y, '%s~' % (e.t if e.t else '0'), h.midi, e.staff.index))
                continue
            for h in e.heads:
                self.emit(base + e.t, e.dur, h, e.staff)
                self.overlay[pno].append((h.x, h.y, '%s' % (e.t if e.t else '0'), h.midi, e.staff.index))
        # grace notes: tiny notes squeezed in just before the next main event on same staff
        for gc in sorted(graces, key=lambda e: e.mx):
            nxt = [e for e in reg if e.staff is gc.staff and e.kind == 'chord' and e.mx > gc.mx]
            if not nxt:
                continue
            tgt = min(nxt, key=lambda e: e.mx)
            k = sum(1 for g2 in graces if g2.staff is gc.staff and gc.mx <= g2.mx < tgt.mx)
            st = base + tgt.t - GRACE_Q * k
            for h in gc.heads:
                self.notes.append(dict(start=st, end=st + GRACE_Q, midi=h.midi, staff=gc.staff.index))
        # dynamics & tempo mapped to column onsets
        def tick_at(x):
            for e_t, e_x in zip(col_ticks, [c[0].mx for c in cols]):
                if e_x >= x - 1.2 * sp:
                    return base + e_t
            return None
        for dx, v in dyns:
            if bounds[mi] - 0.5 <= dx < bounds[mi + 1] - 0.5:
                tk = tick_at(dx)
                self.dyn_events.append((tk if tk is not None else base, v))
        for tx, bpm in tempos:
            if (mi == 0 and tx < bounds[1]) or (bounds[mi] - 0.5 <= tx < bounds[mi + 1] - 0.5):
                tk = tick_at(tx)
                self.tempo_events.append((tk if tk is not None else base, bpm))
        self.log.append((tag, [(e.t, e.kind, e.staff.index, e.dur, [h.midi for h in e.heads]) for e in sorted(reg + full, key=lambda e: (e.t, e.staff.index))]))
        return real

    def emit(self, start, dur, h, staff):
        key = (staff.index, h.midi0)
        if h.tie_in and key in self.open_ties and self.open_ties[key]['end'] == start:
            n = self.open_ties[key]
            n['end'] = start + dur
        else:
            n = dict(start=start, end=start + dur, midi=h.midi, staff=staff.index)
            self.notes.append(n)
        if h.tie_out:
            self.open_ties[key] = n
        else:
            self.open_ties.pop(key, None)

    # ------------------------------------------------------------------ output
    def write_midi(self, path, single=False, bpm_override=None):
        dyn = sorted(self.dyn_events)
        def vel_at(t):
            v = 64
            for tt, vv in dyn:
                if tt <= t:
                    v = vv
                else:
                    break
            return v
        tempos = sorted(set(self.tempo_events))
        if bpm_override:
            tempos = [(F(0), bpm_override)]
        elif not tempos:
            tempos = [(F(0), 120.0)]
            self.warn('no tempo marking found; defaulted to 120 BPM (use --bpm)')
        elif tempos[0][0] > 0:
            tempos.insert(0, (F(0), tempos[0][1]))
        mid = mido.MidiFile(type=1, ticks_per_beat=PPQ)
        tt = mido.MidiTrack()
        mid.tracks.append(tt)
        tt.append(mido.MetaMessage('track_name', name='Tempo', time=0))
        evs = []
        for tk, bpm in tempos:
            evs.append((int(tk * PPQ), mido.MetaMessage('set_tempo', tempo=mido.bpm2tempo(bpm))))
        for tk, (n, d) in sorted(self.sig_events):
            evs.append((int(tk * PPQ), mido.MetaMessage('time_signature', numerator=n, denominator=d)))
        evs.sort(key=lambda a: a[0])
        last = 0
        for tick, m in evs:
            m.time = tick - last
            last = tick
            tt.append(m)
        staves = sorted(set(n['staff'] for n in self.notes))
        groups = [staves] if single else [[s] for s in staves]
        names = ['Right hand', 'Left hand'] if not single else ['Piano']
        for gi, grp in enumerate(groups):
            tr = mido.MidiTrack()
            mid.tracks.append(tr)
            nm = names[gi] if gi < len(names) else 'Staff %d' % (gi + 1)
            tr.append(mido.MetaMessage('track_name', name=nm, time=0))
            tr.append(mido.Message('program_change', program=0, channel=0, time=0))
            evs = []
            for n in self.notes:
                if n['staff'] not in grp:
                    continue
                if not 0 <= n['midi'] <= 127:
                    continue
                st, en = max(0, int(n['start'] * PPQ)), max(0, int(n['end'] * PPQ))   # a leading grace note can start before beat 0
                if en <= st:
                    en = st + 1
                evs.append((st, 1, mido.Message('note_on', note=n['midi'], velocity=vel_at(n['start']), channel=0)))
                evs.append((en, 0, mido.Message('note_off', note=n['midi'], velocity=0, channel=0)))
            evs.sort(key=lambda a: (a[0], a[1]))
            last = 0
            for tick, _, m in evs:
                m.time = tick - last
                last = tick
                tr.append(m)
        mid.save(path)
        return mid


def write_overlay(pdf, conv, outdir):
    import os, subprocess
    from PIL import Image, ImageDraw
    os.makedirs(outdir, exist_ok=True)
    names = 'C C# D D# E F F# G G# A A# B'.split()
    dpi = 150
    subprocess.run(['pdftoppm', '-r', str(dpi), '-png', pdf, os.path.join(outdir, 'pg')], check=True)
    k = dpi / 72.0
    for pno, items in conv.overlay.items():
        cand = [f for f in os.listdir(outdir) if f.startswith('pg-') and f.endswith('.png') and int(f[3:-4]) == pno + 1]
        im = Image.open(os.path.join(outdir, cand[0])).convert('RGB')
        d = ImageDraw.Draw(im)
        for x, y, t, midi, st in items:
            col = (220, 0, 0) if st == 0 else (0, 90, 220)
            d.text((x * k - 2, y * k - 30 - (st * 0)), '%s%d' % (names[midi % 12], midi // 12 - 1), fill=col)
            d.text((x * k - 2, y * k + 14), t, fill=(0, 140, 0))
        im.save(os.path.join(outdir, 'ov-%02d.png' % (pno + 1)))
    for num, pno, x0, x1, y0, y1 in conv.bad_boxes:
        f = os.path.join(outdir, 'ov-%02d.png' % (pno + 1))
        im = Image.open(f)
        c = im.crop((int((x0 - 6) * k), int(y0 * k), int((x1 + 6) * k), int(y1 * k)))
        c = c.resize((int(c.width * 1.6), int(c.height * 1.6)), Image.LANCZOS)
        c.save(os.path.join(outdir, 'bar-%03d.png' % num))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('pdf')
    ap.add_argument('-o', '--out')
    ap.add_argument('--single-track', action='store_true')
    ap.add_argument('--bpm', type=float, help='override tempo (quarter-note BPM)')
    ap.add_argument('--report', action='store_true', help='print every bar with detected notes')
    ap.add_argument('--overlay', metavar='DIR', help='write annotated page PNGs (needs pdftoppm + Pillow) for checking')
    a = ap.parse_args()
    out = a.out or (a.pdf.rsplit('.', 1)[0] + '.mid')
    c = Converter(a.pdf).run()
    c.write_midi(out, single=a.single_track, bpm_override=a.bpm)
    if a.report:
        names = 'C C# D D# E F F# G G# A A# B'.split()
        for tag, evs in c.log:
            print(tag)
            for t, kind, st, dur, ms in evs:
                print('   t=%-6s st%d %-5s dur=%-6s %s' % (t, st, kind, dur, ' '.join('%s%d' % (names[m % 12], m // 12 - 1) for m in ms)))
    if a.overlay:
        write_overlay(a.pdf, c, a.overlay)
    print('bars: %d  notes: %d  tempo events: %s' % (c.mno, len(c.notes), sorted(set(c.tempo_events))[:4]))
    print('stats:', dict(c.stats))
    for w in c.warnings:
        print('WARN', w)
    print('wrote', out)


if __name__ == '__main__':
    main()
