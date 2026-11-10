//! Built-in reader for *digitally typeset* sheet-music PDFs (MuseScore, Sibelius, Finale,
//! Dorico, LilyPond-with-SMuFL…). These PDFs don't contain pictures of notes: every notehead,
//! clef, accidental and rest is a character from a music font, drawn at an exact position, and
//! staff lines, stems, barlines and beams are exact vector lines. So instead of guessing from
//! pixels (optical recognition), we read the symbols directly.
//!
//! Two music-font layouts are understood:
//! * SMuFL (MuseScore 3/4 "MScore"/"Leland"/"Bravura", Dorico, Finale 27+): Unicode code points U+E000…
//! * Sonata layout (Sibelius "Opus", Finale "Maestro", "Petrucci", "Engraver"): classic 8-bit codes,
//!   which show up either as Mac-Roman characters or as U+F0xx.
//!
//! Scanned books (pictures of pages) can't be read this way – see `pdfview.rs` (Audiveris).

use crate::model::{Hand, Note, Origin, Piece};
use hayro::hayro_interpret::font::GlyphRun;
use hayro::hayro_interpret::kurbo::{Affine, BezPath, PathEl, Point, Rect};
use hayro::hayro_interpret::{
    interpret_page, BlendMode, ClipPath, Context, Device, DrawMode, DrawProps, Image, ImageDrawProps, InterpreterCache, InterpreterSettings, SoftMask,
};
use hayro::hayro_syntax::Pdf;
use hayro::hayro_interpret::TransformExt;

/// hayro glyph units → em (calibrated: noteheads are drawn at 4 staff-spaces per em).
const GLYPH_SCALE: f64 = 1.0;

// ------------------------------------------------------------------ symbols

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sym {
    HeadBlack,
    HeadHalf,
    HeadWhole,
    ClefG,
    ClefF,
    ClefC,
    Sharp,
    Flat,
    Natural,
    DoubleSharp,
    DoubleFlat,
    Dot,
    RestWhole,
    RestHalf,
    RestQuarter,
    Rest8,
    Rest16,
    Rest32,
    Flag(u8),
    TimeDigit(u8),
    CommonTime,
    CutTime,
    TupletDigit(u8),
    Eight, // small "8" under a clef (guitar treble clef)
    /// start of an 8va (+12) / 8vb (-12) line
    Ottava(i8),
}

/// SMuFL code point → symbol.
fn smufl(c: u32) -> Option<Sym> {
    Some(match c {
        0xE0A2 | 0xE0A0 | 0xF4BC => Sym::HeadWhole,
        0xE0A3 | 0xF4BD => Sym::HeadHalf,
        // Bravura's optional (Dorico) notehead and clef forms live at U+F4xx
        0xE0A4 | 0xF4BE => Sym::HeadBlack,
        0xF472 => Sym::ClefG,
        0xF474 => Sym::ClefF,
        0xE510 | 0xE511 => Sym::Ottava(12),
        0xE512 | 0xE51C => Sym::Ottava(-12),
        0xE050 | 0xE07A => Sym::ClefG,
        0xE052 => Sym::ClefG, // gClef8vb handled via flag below
        0xE062 | 0xE07C => Sym::ClefF,
        0xE05C | 0xE07B => Sym::ClefC,
        0xE260 => Sym::Flat,
        0xE261 => Sym::Natural,
        0xE262 => Sym::Sharp,
        0xE263 => Sym::DoubleSharp,
        0xE264 => Sym::DoubleFlat,
        0xE1E7 => Sym::Dot,
        0xE4E3 => Sym::RestWhole,
        0xE4E4 => Sym::RestHalf,
        0xE4E5 => Sym::RestQuarter,
        0xE4E6 => Sym::Rest8,
        0xE4E7 => Sym::Rest16,
        0xE4E8 => Sym::Rest32,
        0xE240 | 0xE241 => Sym::Flag(1),
        0xE242 | 0xE243 => Sym::Flag(2),
        0xE244 | 0xE245 => Sym::Flag(3),
        0xE080..=0xE089 => Sym::TimeDigit((c - 0xE080) as u8),
        0xE08A => Sym::CommonTime,
        0xE08B => Sym::CutTime,
        0xE880..=0xE889 => Sym::TupletDigit((c - 0xE880) as u8),
        _ => return None,
    })
}

/// Unicode character → Mac-Roman byte, for the characters the Sonata layout uses.
fn mac_roman(c: char) -> Option<u8> {
    let u = c as u32;
    if u < 0x80 {
        return Some(u as u8);
    }
    if (0xF020..=0xF0FF).contains(&u) {
        return Some((u - 0xF000) as u8);
    }
    Some(match c {
        'œ' => 0xCF,
        'Œ' => 0xCE,
        '˙' => 0xFA,
        '‰' => 0xE4,
        '≈' => 0xC5,
        'Ó' => 0xEE,
        '∑' => 0xB7,
        '™' => 0xAA,
        'Â' => 0xE5,
        '∫' => 0xBA,
        '∂' => 0xB6,
        '¨' => 0xAC,
        'Ú' => 0xF2,
        'Ô' => 0xEF,
        '·' => 0xE1,
        'Î' => 0xEB,
        '¥' => 0xB4,
        '“' => 0xD2,
        _ => return None,
    })
}

/// Sonata-layout byte → symbol.
fn sonata(b: u8) -> Option<Sym> {
    Some(match b {
        0xCF => Sym::HeadBlack,
        0xFA => Sym::HeadHalf,
        b'w' => Sym::HeadWhole,
        b'&' => Sym::ClefG,
        b'?' => Sym::ClefF,
        b'B' => Sym::ClefC,
        b'#' => Sym::Sharp,
        b'b' => Sym::Flat,
        b'n' => Sym::Natural,
        0xDC => Sym::DoubleSharp,
        0xBA => Sym::DoubleFlat,
        b'.' | b'k' | 0xAA => Sym::Dot,
        0xB7 | 0xE3 => Sym::RestWhole,
        0xEE => Sym::RestHalf,
        0xCE => Sym::RestQuarter,
        0xE4 => Sym::Rest8,
        0xC5 => Sym::Rest16,
        0xA8 => Sym::Rest32,
        b'j' | b'J' => Sym::Flag(1),
        b'r' | b'R' => Sym::Flag(2),
        0xEF | 0xC6 => Sym::Flag(3),
        b'0'..=b'9' => Sym::TimeDigit(b - b'0'),
        b'c' => Sym::CommonTime,
        b'C' => Sym::CutTime,
        0xD2 => Sym::Ottava(12), // Sibelius "8va"
        _ => return None,
    })
}

#[derive(Clone, Debug)]
pub struct Glyph {
    pub sym: Option<Sym>,
    pub ch: char,
    pub x: f64,
    pub y: f64,
    /// Font size in page units (music fonts are drawn at 4 staff-spaces per em).
    pub size: f64,
    pub smufl: bool,
    /// Part of a run of ordinary text (a title, lyrics, a direction) rather than music symbols.
    pub text: bool,
}

#[derive(Clone, Debug)]
pub struct Seg {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub w: f64,
}

#[derive(Default)]
pub struct PageMarks {
    pub glyphs: Vec<Glyph>,
    pub segs: Vec<Seg>,
    /// Filled shapes (beams, slurs, …): bounding box and outline points.
    pub fills: Vec<(Rect, Vec<Point>)>,
    pub width: f64,
    pub height: f64,
    /// Number of pictures on the page (a scanned page is one big picture).
    pub images: usize,
}

struct Collector<'m> {
    m: &'m mut PageMarks,
}

/// Flatten a path into polylines; the flag says whether the piece contained curves
/// (slurs, ties, noteheads drawn as outlines…), which must never be taken for straight lines.
fn flatten(path: &BezPath, t: Affine) -> Vec<(Vec<Point>, bool)> {
    let mut out = vec![];
    let mut cur: Vec<Point> = vec![];
    let mut curved = false;
    let mut last = Point::ZERO;
    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => {
                if cur.len() > 1 {
                    out.push((std::mem::take(&mut cur), curved));
                }
                cur = vec![t * *p];
                curved = false;
                last = *p;
            }
            PathEl::LineTo(p) => {
                cur.push(t * *p);
                last = *p;
            }
            PathEl::QuadTo(c, p) => {
                for k in 1..=8 {
                    let u = k as f64 / 8.0;
                    let q = last.lerp(*c, u).lerp(c.lerp(*p, u), u);
                    cur.push(t * q);
                }
                curved = true;
                last = *p;
            }
            PathEl::CurveTo(c1, c2, p) => {
                for k in 1..=8 {
                    let u = k as f64 / 8.0;
                    let a = last.lerp(*c1, u);
                    let b = c1.lerp(*c2, u);
                    let c = c2.lerp(*p, u);
                    let q = a.lerp(b, u).lerp(b.lerp(c, u), u);
                    cur.push(t * q);
                }
                curved = true;
                last = *p;
            }
            PathEl::ClosePath => {
                if let Some(f) = cur.first().copied() {
                    cur.push(f);
                }
            }
        }
    }
    if cur.len() > 1 {
        out.push((cur, curved));
    }
    out
}

impl<'a> Device<'a> for Collector<'_> {
    fn draw_path(&mut self, path: &BezPath, props: DrawProps<'a>, draw_mode: &DrawMode) {
        let t = props.transform;
        let scale = t.determinant().abs().sqrt();
        let stroke = match draw_mode {
            DrawMode::Stroke(s) | DrawMode::FillAndStroke(_, s) => Some(s.line_width as f64 * scale),
            _ => None,
        };
        for (poly, curved) in flatten(path, t) {
            let bb = poly.iter().fold(Rect::new(f64::MAX, f64::MAX, f64::MIN, f64::MIN), |r, p| Rect::new(r.x0.min(p.x), r.y0.min(p.y), r.x1.max(p.x), r.y1.max(p.y)));
            match stroke {
                Some(_) if curved => self.m.fills.push((bb, poly)),
                Some(w) => {
                    for pair in poly.windows(2) {
                        self.m.segs.push(Seg { x0: pair[0].x, y0: pair[0].y, x1: pair[1].x, y1: pair[1].y, w: w.max(0.1) });
                    }
                }
                None => {
                    // thin filled rectangles are lines (staff lines, stems, barlines); others are beams etc.
                    if curved {
                        self.m.fills.push((bb, poly));
                    } else if bb.width() < 3.0 && bb.height() > 2.0 * bb.width() {
                        self.m.segs.push(Seg { x0: bb.center().x, y0: bb.y0, x1: bb.center().x, y1: bb.y1, w: bb.width() });
                    } else if bb.height() < 3.0 && bb.width() > 2.0 * bb.height() && poly.len() <= 6 && (poly[0].y - bb.center().y).abs() <= bb.height() {
                        // axis-aligned thin bar (a beam is slanted or thicker; keep both)
                        self.m.segs.push(Seg { x0: bb.x0, y0: bb.center().y, x1: bb.x1, y1: bb.center().y, w: bb.height() });
                        self.m.fills.push((bb, poly));
                    } else {
                        self.m.fills.push((bb, poly));
                    }
                }
            }
        }
    }
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn draw_glyph_run(&mut self, run: &GlyphRun<'_, 'a>, props: DrawProps<'a>, draw_mode: &DrawMode) {
        if matches!(draw_mode, DrawMode::Invisible) {
            return;
        }
        // A run that contains ordinary letters – ones no music font uses for a symbol – is text.
        let uni = |g: &hayro::hayro_interpret::font::PositionedGlyph<'_>| match g.as_unicode() {
            Some(hayro::hayro_interpret::hayro_cmap::BfString::Char(c)) => c,
            Some(hayro::hayro_interpret::hayro_cmap::BfString::String(s)) => s.chars().next().unwrap_or('\0'),
            None => char::from_u32(g.char_code()).unwrap_or('\0'),
        };
        let run_is_text = run.glyphs().iter().any(|g| {
            let c = uni(g);
            c.is_alphabetic() && !(0xE000..=0xF8FF).contains(&(c as u32)) && (!c.is_ascii() || !"wbnkjJrRcCBpfmszU".contains(c)) && mac_roman(c).and_then(sonata).is_none()
        });
        for g in run.glyphs() {
            let t = props.transform * g.transform();
            let origin = t * Point::ZERO;
            if std::env::var("VS_DEBUG").is_ok() && self.m.glyphs.len() < 3 { eprintln!("props {:?} glyph {:?}", props.transform.as_coeffs(), g.transform().as_coeffs()); }
            let up = t * Point::new(0.0, 1000.0) - origin;
            let size = up.hypot() * GLYPH_SCALE;
            let ch = match g.as_unicode() {
                Some(hayro::hayro_interpret::hayro_cmap::BfString::Char(c)) => c,
                Some(hayro::hayro_interpret::hayro_cmap::BfString::String(s)) => s.chars().next().unwrap_or('\0'),
                None => char::from_u32(g.char_code()).unwrap_or('\0'),
            };
            let u = ch as u32;
            let (sym, sm) = if (0xE000..=0xF8FF).contains(&u) && !(0xF020..=0xF0FF).contains(&u) {
                (smufl(u), true)
            } else {
                (mac_roman(ch).and_then(sonata), false)
            };
            let sym = if run_is_text { None } else { sym };
            self.m.glyphs.push(Glyph { sym, ch, x: origin.x, y: origin.y, size, smufl: sm, text: run_is_text });
        }
    }
    fn draw_image(&mut self, _: Image<'a, '_>, _: ImageDrawProps<'a>) {
        self.m.images += 1;
    }
    fn pop_clip(&mut self) {}
    fn pop_transparency_group(&mut self) {}
}

/// Extract every glyph and line from each page (coordinates: points, origin top-left, y down).
pub fn extract(bytes: Vec<u8>) -> Result<Vec<PageMarks>, String> {
    let pdf = Pdf::new(bytes).map_err(|e| format!("Not a readable PDF: {e:?}"))?;
    let cache = InterpreterCache::new();
    let settings = InterpreterSettings::default();
    let mut pages = vec![];
    for page in pdf.pages().iter() {
        let (w, h) = page.render_dimensions();
        let transform = page.initial_transform(true).to_kurbo();
        let target = Rect::new(0.0, 0.0, w as f64, h as f64);
        let mut ctx = Context::new(transform, target, &cache, page.xref(), settings.clone());
        let mut marks = PageMarks { width: w as f64, height: h as f64, ..Default::default() };
        interpret_page(page, &mut ctx, &mut Collector { m: &mut marks });
        // some programs draw the same symbol twice (e.g. arpeggio/cross-staff): keep one
        let mut seen = std::collections::HashSet::new();
        marks.glyphs.retain(|g| seen.insert((g.ch as u32, (g.x * 4.0).round() as i64, (g.y * 4.0).round() as i64)));
        mark_text(&mut marks.glyphs);
        pages.push(marks);
    }
    Ok(pages)
}

/// Text is often drawn one character at a time, so group neighbouring glyphs of the same size on
/// the same line into words: a group with any ordinary letter in it (one that no music font uses
/// for a symbol) is text – a title, lyrics, a direction – not music.
fn mark_text(gl: &mut [Glyph]) {
    let clearly_text = |g: &Glyph| {
        let c = g.ch;
        !g.smufl && c.is_alphabetic() && !(0xE000..=0xF8FF).contains(&(c as u32)) && (!c.is_ascii() || !"wbnkjJrRcCBpfmszU".contains(c)) && mac_roman(c).and_then(sonata).is_none()
    };
    let n = gl.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|a, b| gl[*a].y.partial_cmp(&gl[*b].y).unwrap_or(std::cmp::Ordering::Equal));
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], i: usize) -> usize {
        let mut r = i;
        while p[r] != r {
            r = p[r];
        }
        let mut i = i;
        while p[i] != r {
            let nx = p[i];
            p[i] = r;
            i = nx;
        }
        r
    }
    for oi in 0..n {
        let i = order[oi];
        if gl[i].smufl || gl[i].size <= 0.0 {
            continue;
        }
        for &j in order.iter().skip(oi + 1) {
            if gl[j].y - gl[i].y > gl[i].size * 0.3 {
                break;
            }
            if gl[j].smufl {
                continue;
            }
            let ratio = gl[i].size.max(gl[j].size) / gl[i].size.min(gl[j].size).max(0.01);
            if ratio < 1.08 && (gl[i].x - gl[j].x).abs() < gl[i].size * 1.3 {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                if a != b {
                    parent[a] = b;
                }
            }
        }
    }
    let mut texty = std::collections::HashSet::new();
    for i in 0..n {
        if clearly_text(&gl[i]) {
            texty.insert(find(&mut parent, i));
        }
    }
    for i in 0..n {
        if texty.contains(&find(&mut parent, i)) {
            gl[i].text = true;
            gl[i].sym = None;
        }
    }
}

// ------------------------------------------------------------------ recognition

#[derive(Clone, Debug)]
struct Staff {
    lines: [f64; 5], // y of the 5 lines, top to bottom
    x0: f64,
    x1: f64,
    sp: f64,
}

impl Staff {
    fn top(&self) -> f64 {
        self.lines[0]
    }
    fn bottom(&self) -> f64 {
        self.lines[4]
    }
    fn mid(&self) -> f64 {
        self.lines[2]
    }
    /// Staff position in half-spaces above the bottom line (0 = bottom line, 8 = top line).
    fn step(&self, y: f64) -> i32 {
        ((self.bottom() - y) / (self.sp / 2.0)).round() as i32
    }
}

fn find_staves(m: &PageMarks) -> Vec<Staff> {
    // long horizontal segments
    let mut hs: Vec<(f64, f64, f64)> = m
        .segs
        .iter()
        .filter(|s| (s.y0 - s.y1).abs() < 0.6 && (s.x1 - s.x0).abs() > m.width * 0.08 && s.w < 1.6) // thick = a beam, not a staff line
        .map(|s| ((s.y0 + s.y1) / 2.0, s.x0.min(s.x1), s.x0.max(s.x1)))
        .collect();
    hs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    // merge segments on the same y (staff lines are often drawn in pieces)
    let mut rows: Vec<(f64, f64, f64)> = vec![];
    for h in hs {
        match rows.last_mut() {
            Some(r) if (r.0 - h.0).abs() < 0.5 => {
                r.1 = r.1.min(h.1);
                r.2 = r.2.max(h.2);
            }
            _ => rows.push(h),
        }
    }
    // runs of equally spaced lines with similar extent; 5 = staff, 6 = guitar tab (skipped)
    let mut staves = vec![];
    let mut i = 0;
    while i + 4 < rows.len() {
        let g = &rows[i..i + 5];
        let d: Vec<f64> = g.windows(2).map(|w| w[1].0 - w[0].0).collect();
        let sp = d.iter().sum::<f64>() / 4.0;
        let even = d.iter().all(|x| (x - sp).abs() < sp * 0.15) && sp > 3.0 && sp < 20.0;
        let same_extent = g.iter().all(|r| (r.1 - g[0].1).abs() < sp * 3.0 && (r.2 - g[0].2).abs() < sp * 3.0);
        if even && same_extent {
            // tab staff has a 6th line at the same spacing
            let six = rows.get(i + 5).is_some_and(|r| (r.0 - g[4].0 - sp).abs() < sp * 0.15 && (r.1 - g[0].1).abs() < sp * 3.0);
            if !six {
                staves.push(Staff { lines: [g[0].0, g[1].0, g[2].0, g[3].0, g[4].0], x0: g[0].1, x1: g[0].2, sp });
                i += 5;
                continue;
            }
            i += 6;
            continue;
        }
        i += 1;
    }
    staves
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum ClefKind {
    G,
    F,
    C,
}

impl ClefKind {
    /// Diatonic step number of the bottom staff line (C0 = 0).
    fn bottom_line(self) -> i32 {
        match self {
            ClefKind::G => 4 * 7 + 2, // E4
            ClefKind::F => 2 * 7 + 4, // G2
            ClefKind::C => 3 * 7 + 3, // F3 (alto)
        }
    }
}

const STEP_PC: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const SHARP_ORDER: [usize; 7] = [3, 0, 4, 1, 5, 2, 6];
const FLAT_ORDER: [usize; 7] = [6, 2, 5, 1, 4, 0, 3];

fn key_alters(f: i32) -> [i32; 7] {
    let mut a = [0; 7];
    if f > 0 {
        for &l in SHARP_ORDER.iter().take(f.min(7) as usize) {
            a[l] = 1;
        }
    } else {
        for &l in FLAT_ORDER.iter().take((-f).min(7) as usize) {
            a[l] = -1;
        }
    }
    a
}

#[derive(Clone, Debug)]
struct Event {
    x: f64,
    /// beats (quarter notes)
    dur: f64,
    pitches: Vec<(i32, i32)>, // (diatonic step, explicit alteration or 99 = none)
    rest: bool,
    stem_down: Option<bool>,
    staff: usize,
    rest_y: Option<f64>,
}

pub struct ReadReport {
    pub pages: usize,
    pub systems: usize,
    pub measures: usize,
    pub clean_measures: usize,
    pub notes: usize,
}

/// Error from `read_pdf*` when the pages are pictures (a scanned book): needs optical recognition.
pub const NOT_TYPESET_SCAN: &str = "This PDF is scanned pictures of pages, so the notes have to be recognised from the images (Audiveris).";
/// Error from `read_pdf*` when there's no music at all (a letter, a receipt, a text-only book…).
pub const NOT_MUSIC: &str = "No sheet music found in this PDF.";

/// Read a typeset PDF into a Piece. Returns the piece and a quality report.
pub fn read_pdf(bytes: Vec<u8>, title: &str) -> Result<(Piece, ReadReport), String> {
    read_pdf_pages(bytes, title, None)
}

/// Like `read_pdf`, but only the given pages (1-based, inclusive).
pub fn read_pdf_pages(bytes: Vec<u8>, title: &str, only: Option<(usize, usize)>) -> Result<(Piece, ReadReport), String> {
    let pages = extract(bytes)?;
    let mut notes: Vec<Note> = vec![];
    let mut t_global = 0.0f64;
    let mut ts = (4u32, 4u32);
    let mut key_fifths = 0i32;
    let mut have_key = false;
    let mut have_ts = false;
    let mut report = ReadReport { pages: pages.len(), systems: 0, measures: 0, clean_measures: 0, notes: 0 };
    let mut pickup = 0.0;
    let mut first_measure = true;
    let mut music_size_seen = 0.0f64;
    let mut guitar = false;
    let mut clefs_state: Vec<ClefKind> = vec![];
    let mut first_key: Option<i32> = None;
    let mut first_ts: Option<(u32, u32)> = None;
    let mut key_changes: Vec<(f64, i32)> = vec![];
    let mut bar_marks: Vec<crate::model::BarMark> = vec![];
    let mut sections: Vec<crate::model::Section> = vec![];

    for (page_no, m) in pages.iter().enumerate() {
        if only.is_some_and(|(a, b)| page_no + 1 < a || page_no + 1 > b) {
            continue;
        }
        let staves = find_staves(m);
        if std::env::var("VS_DEBUG").is_ok() {
            eprintln!("PAGE {} staves {} first measure {}", page_no + 1, staves.len(), report.measures + 1);
        }
        if staves.is_empty() {
            continue;
        }
        let sp = staves.iter().map(|s| s.sp).sum::<f64>() / staves.len() as f64;
        let music_size = 4.0 * sp;
        music_size_seen = music_size;
        // systems: staves whose horizontal extent matches and that are joined by a barline at the left
        let mut systems: Vec<Vec<usize>> = vec![];
        for (i, s) in staves.iter().enumerate() {
            let joined = systems.last().and_then(|sys: &Vec<usize>| sys.last()).is_some_and(|&j| {
                let p = &staves[j];
                (p.x0 - s.x0).abs() < sp && (p.x1 - s.x1).abs() < sp * 2.0 && {
                    // a vertical line connecting the two staves
                    m.segs.iter().any(|g| (g.x0 - g.x1).abs() < 1.0 && (g.x0 - s.x0).abs() < sp * 1.5 && g.y0.min(g.y1) <= p.bottom() + 1.0 && g.y0.max(g.y1) >= s.top() - 1.0)
                        || m.fills.iter().any(|(bb, _)| bb.x1 <= s.x0 + sp && bb.x0 >= s.x0 - sp * 4.0 && bb.y0 <= p.top() + sp && bb.y1 >= s.bottom() - sp)
                }
            });
            if joined {
                systems.last_mut().unwrap().push(i);
            } else {
                systems.push(vec![i]);
            }
        }
        // music glyphs belong to the nearest staff
        let near = |y: f64| -> Option<usize> {
            let mut best = None;
            let mut bd = f64::MAX;
            for (i, s) in staves.iter().enumerate() {
                let d = if y < s.top() { s.top() - y } else if y > s.bottom() { y - s.bottom() } else { 0.0 };
                if d < bd {
                    bd = d;
                    best = Some(i);
                }
            }
            best.filter(|_| bd < sp * 7.0)
        };
        let is_music = |g: &Glyph| (g.size - music_size).abs() < music_size * 0.2 || (g.sym.is_some() && g.smufl && g.size > music_size * 0.55);
        // ---- headings: text in the gap above a system starts a new exercise
        // text = letters, digits and spaces outside the staves (a title can be as big as the music font)
        let in_staff = |y: f64| staves.iter().any(|st| y > st.top() - sp * 1.5 && y < st.bottom() + sp * 1.5);
        let is_text = |g: &Glyph| g.text && g.size < music_size * 2.5 && g.size > 3.0 && !in_staff(g.y);
        let mut text_sizes: Vec<f64> = m.glyphs.iter().filter(|g| is_text(g) && g.ch.is_alphabetic()).map(|g| g.size).collect();
        text_sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let body_size = text_sizes.get(text_sizes.len() / 2).copied().unwrap_or(10.0);
        let gaps: Vec<f64> = systems.windows(2).map(|w| staves[w[1][0]].top() - staves[*w[0].last().unwrap()].bottom()).collect();
        let median_gap = {
            let mut g = gaps.clone();
            g.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            g.get(g.len() / 2).copied().unwrap_or(sp * 10.0)
        };
        let mut prev_bottom = 0.0f64;
        for (sys_no, sys) in systems.iter().enumerate() {
            let sys_top = staves[sys[0]].top();
            let sys_bottom = staves[*sys.last().unwrap()].bottom();
            let gap = sys_top - prev_bottom;
            // (skip the band just under the previous system: pedal marks, 'legato', 'repeat twice'…)
            let region = (prev_bottom + if sys_no == 0 { 0.0 } else { sp * 4.0 }, sys_top - sp * 1.2);
            prev_bottom = sys_bottom;
            {
                // text lines in the gap, top to bottom
                let mut gl: Vec<&Glyph> = m.glyphs.iter().filter(|g| is_text(g) && g.y > region.0 && g.y < region.1).collect();
                let mut seen_pos = std::collections::HashSet::new();
                gl.retain(|g| seen_pos.insert(((g.x * 2.0) as i64, (g.y * 2.0) as i64, g.ch)));
                gl.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal).then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)));
                let mut lines: Vec<(f64, f64, Vec<&Glyph>)> = vec![]; // y, size, glyphs
                for g in gl {
                    match lines.last_mut() {
                        Some(l) if (l.0 - g.y).abs() < g.size * 0.4 => {
                            l.1 = l.1.max(g.size);
                            l.2.push(g)
                        }
                        _ => lines.push((g.y, g.size, vec![g])),
                    }
                }
                let text_of = |gs: &[&Glyph]| -> String {
                    let mut v: Vec<&&Glyph> = gs.iter().collect();
                    v.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
                    // use the PDF's own spaces when it has them; otherwise guess from wide gaps
                    // the PDF's own spaces, plus a space wherever the gap between letters is much wider
                    // than this line's usual letter spacing (or the type size jumps, as in "A  The …")
                    let steps: Vec<f64> = v.windows(2).filter(|w| w[0].ch != ' ' && w[1].ch != ' ').map(|w| (w[1].x - w[0].x) / w[0].size.max(w[1].size)).collect();
                    let mut st = steps.clone();
                    st.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let usual = st.get(st.len() / 2).copied().unwrap_or(0.55);
                    let mut out = String::new();
                    let mut last: Option<&Glyph> = None;
                    for g in v {
                        if let Some(l) = last {
                            let d = (g.x - l.x) / g.size.max(l.size);
                            let jump = g.size.max(l.size) / g.size.min(l.size) > 1.5;
                            let camel = l.ch.is_lowercase() && g.ch.is_uppercase();
                            let inside_word = l.ch.is_uppercase() && g.ch.is_lowercase() && !jump;
                            if (d > usual * 2.2 || jump || camel) && !inside_word && !out.ends_with(' ') && g.ch != ' ' {
                                out.push(' ');
                            }
                        }
                        if !(g.ch == ' ' && out.ends_with(' ')) {
                            out.push(g.ch);
                        }
                        last = Some(g);
                    }
                    out.split_whitespace().collect::<Vec<_>>().join(" ")
                };
                // keep lines that read like headings (words, not a lone page number or lyric syllable),
                // left of the right margin (composers and arrangers sit on the right)
                let heads: Vec<(f64, String)> = lines
                    .iter()
                    .filter(|l| {
                        let letters = l.2.iter().filter(|g| g.ch.is_ascii_alphabetic()).count();
                        let word = text_of(&l.2).split(|c: char| !c.is_alphabetic()).any(|w| w.chars().count() >= 4);
                        ((letters >= 3 && word) || (letters >= 1 && l.2.len() <= 3 && l.1 >= body_size * 1.4)) && l.2.iter().map(|g| g.x).fold(f64::MAX, f64::min) < m.width * 0.6
                    })
                    // lines in the top margin may be a running header: marked so they can be dropped
                    // later if they repeat across the book
                    .map(|l| (l.1, format!("{}{}", if l.0 < m.height * 0.09 { "\u{1}" } else { "" }, text_of(&l.2))))
                    .filter(|h| h.1.trim_start_matches('\u{1}').len() > 0)
                    .collect();
                let big = heads.iter().any(|h| h.0 >= body_size * 1.1);
                if std::env::var("VS_HEADS").is_ok() && !heads.is_empty() {
                    eprintln!("p{} sys{} gap {:.0} med {:.0} body {:.1} big {} heads {:?}", page_no + 1, sys_no, gap, median_gap, body_size, big, heads);
                }
                // a numbered movement can follow a tempo mark on the same line ("♩ = 65  II: Lost Butterfly")
                let heads: Vec<(f64, String)> = heads
                    .into_iter()
                    .map(|(sz, t)| {
                        let mark = if t.starts_with('\u{1}') { "\u{1}" } else { "" };
                        let body = t.trim_start_matches('\u{1}');
                        let words: Vec<&str> = body.split_whitespace().collect();
                        match words.iter().position(|w| w.len() >= 2 && w.trim_end_matches([':', '.']).chars().all(|c| "IVX".contains(c)) && w.ends_with([':', '.'])) {
                            Some(i) if i > 0 => (sz, format!("{mark}{}", words[i..].join(" "))),
                            _ => (sz, t),
                        }
                    })
                    .collect();
                // performance directions are not headings
                const DIRECTIONS: [&str; 22] = ["rit", "rall", "poco", "tempo", "cresc", "dim", "accel", "molto", "piu", "più", "meno", "legat", "dolce", "espress", "rubato", "simile", "sempre", "ped", "pedal", "repeat", "arpeggiate", "fine"];
                let heads: Vec<(f64, String)> = heads
                    .into_iter()
                    .filter(|h| {
                        let t = h.1.to_lowercase();
                        !t.split(|c: char| !c.is_alphabetic()).any(|w| DIRECTIONS.iter().any(|d| w.starts_with(d)))
                    })
                    .collect();
                // "I: Sincerely", "II. Michishirube", "3. Right Hand Rising", "No. 4", "Exercise 2"…
                let numbered = heads.iter().any(|h| {
                    let t = h.1.trim_start_matches('\u{1}').trim();
                    let first = t.split(|c: char| c == ' ' || c == ':' || c == '.').next().unwrap_or("");
                    let roman = !first.is_empty() && first.chars().all(|c| "IVX".contains(c)) && t[first.len()..].starts_with([':', '.', ' ']);
                    let num = !first.is_empty() && first.chars().all(|c| c.is_ascii_digit()) && t[first.len()..].starts_with('.');
                    let lower = t.to_lowercase();
                    roman || num || lower.starts_with("no.") || lower.starts_with("exercise") || lower.starts_with("study") || lower.starts_with("etude")
                });
                let wordy = heads.iter().any(|h| h.1.split_whitespace().filter(|w| w.chars().filter(|c| c.is_alphabetic()).count() >= 2).count() >= 2);
                let new_section = !heads.is_empty()
                    && (sections.is_empty() || big || numbered || (sys_no == 0 && wordy) || (sys_no > 0 && gap > median_gap * 1.35));
                if new_section {
                    // the lines nearest the music (at most three)
                    let from = heads.len().saturating_sub(3);
                    let mut title: Vec<String> = heads[from..].iter().map(|h| h.1.clone()).collect();
                    title.dedup();
                    let title = title.join(" · ");
                    if let Some(last) = sections.last_mut() {
                        last.end = t_global;
                    }
                    sections.push(crate::model::Section { beat: t_global, end: f64::MAX, page: (page_no + 1) as u32, title });
                }
            }
            report.systems += 1;
            // Piano music is a treble staff over a bass staff. With more staves, take the lowest
            // such pair (a vocal line sits on top) – unless the extra staves are below it
            // (e.g. an alternative left hand), where the first pair is the one to play.
            let clef_at = |i: usize| -> Option<ClefKind> {
                let st = &staves[i];
                m.glyphs
                    .iter()
                    .filter(|g| g.x < st.x0 + sp * 5.0 && g.y > st.top() - sp * 3.0 && g.y < st.bottom() + sp * 3.0 && g.size > music_size * 0.6)
                    .find_map(|g| match g.sym {
                        Some(Sym::ClefG) => Some(ClefKind::G),
                        Some(Sym::ClefF) => Some(ClefKind::F),
                        _ => None,
                    })
            };
            let parts: Vec<usize> = if sys.len() > 2 {
                let pairs: Vec<usize> = (0..sys.len() - 1).filter(|&k| clef_at(sys[k]) == Some(ClefKind::G) && clef_at(sys[k + 1]) == Some(ClefKind::F)).collect();
                match pairs.last() {
                    Some(&k) => vec![sys[k], sys[k + 1]],
                    None => sys[sys.len() - 2..].to_vec(),
                }
            } else {
                sys.clone()
            };
            if parts.len() == 1 {
                guitar = true;
            }
            if clefs_state.len() != parts.len() {
                clefs_state = if parts.len() == 2 { vec![ClefKind::G, ClefKind::F] } else { vec![ClefKind::G] };
            }
            let sys_x0 = staves[parts[0]].x0;
            let sys_x1 = staves[parts[0]].x1;
            // barlines: thin vertical lines that run exactly from the top to the bottom line of a staff
            // (or across the whole system), present at the same x in every staff of the system
            let tops: Vec<f64> = parts.iter().map(|&i| staves[i].top()).collect();
            let bots: Vec<f64> = parts.iter().map(|&i| staves[i].bottom()).collect();
            let cand = |st: &Staff| -> Vec<f64> {
                let mut xs: Vec<f64> = vec![];
                for g in &m.segs {
                    if (g.x0 - g.x1).abs() > 0.8 || g.x0 < sys_x0 + sp * 0.5 {
                        continue;
                    }
                    let (ya, yb) = (g.y0.min(g.y1), g.y0.max(g.y1));
                    let covers = ya <= st.top() + sp * 0.35 && yb >= st.bottom() - sp * 0.35;
                    let on_line = |y: f64| tops.iter().chain(bots.iter()).any(|t| (y - t).abs() < sp * 0.35);
                    let ends_on_lines = on_line(ya) && on_line(yb);
                    if covers && ends_on_lines {
                        xs.push(g.x0);
                    }
                }
                xs
            };
            let mut bars: Vec<f64> = cand(&staves[parts[0]]);
            if parts.len() >= 2 {
                let other = cand(&staves[parts[1]]);
                bars.retain(|x| other.iter().any(|o| (o - x).abs() < sp * 0.6));
                // Most programs draw a grand-staff barline as ONE line through both staves.
                // When the system has those, trust only them (stems that happen to run from
                // line to line in both hands would otherwise look like barlines).
                let (t0, b1) = (staves[parts[0]].top(), staves[parts[parts.len() - 1]].bottom());
                let through: Vec<f64> = m
                    .segs
                    .iter()
                    .filter(|g| (g.x0 - g.x1).abs() < 0.8 && g.x0 > sys_x0 + sp * 0.5 && g.y0.min(g.y1) <= t0 + sp * 0.35 && g.y0.max(g.y1) >= b1 - sp * 0.35 && g.y0.min(g.y1) >= t0 - sp * 0.6 && g.y0.max(g.y1) <= b1 + sp * 0.6)
                    .map(|g| g.x0)
                    .collect();
                if !through.is_empty() {
                    bars = through;
                }
            }
            bars.push(sys_x1);
            bars.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            bars.dedup_by(|a, b| (*a - *b).abs() < sp * 0.8);

            // collect glyphs per staff
            let mut per_staff: Vec<Vec<&Glyph>> = vec![vec![]; parts.len()];
            for g in &m.glyphs {
                if g.sym.is_none() || g.x < sys_x0 - sp || g.x > sys_x1 + sp {
                    continue;
                }
                if let Some(si) = near(g.y) {
                    if let Some(pi) = parts.iter().position(|p| *p == si) {
                        per_staff[pi].push(g);
                    }
                }
            }
            for v in per_staff.iter_mut() {
                v.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
            }
            // 8va / 8vb lines: from the "8va" sign along its dashed line to the end hook
            let mut ottavas: Vec<Vec<(f64, f64, i32)>> = vec![vec![]; parts.len()];
            for (pi, gl) in per_staff.iter().enumerate() {
                for g in gl.iter() {
                    let Some(Sym::Ottava(shift)) = g.sym else { continue };
                    if !is_music(g) && g.size < music_size * 0.5 {
                        continue;
                    }
                    let mut end = g.x + sp * 3.0;
                    loop {
                        let next = m
                            .segs
                            .iter()
                            .filter(|d| (d.y0 - d.y1).abs() < 0.3 && (d.y0 - g.y).abs() < sp * 1.6 && d.x0.min(d.x1) > end - sp * 0.2 && d.x0.min(d.x1) < end + sp * 2.5 && (d.x1 - d.x0).abs() < sp * 4.0)
                            .map(|d| d.x0.max(d.x1))
                            .fold(f64::MIN, f64::max);
                        if next > end + 0.1 && next < sys_x1 + sp {
                            end = next;
                        } else {
                            break;
                        }
                    }
                    ottavas[pi].push((g.x - sp * 0.6, end + sp * 0.6, shift as i32));
                }
            }
            // first notehead/rest x in the system: everything before it is clef/key/time
            let first_event_x = per_staff
                .iter()
                .flat_map(|v| v.iter())
                .filter(|g| matches!(g.sym, Some(Sym::HeadBlack | Sym::HeadHalf | Sym::HeadWhole | Sym::RestWhole | Sym::RestHalf | Sym::RestQuarter | Sym::Rest8 | Sym::Rest16)) && is_music(g))
                .map(|g| g.x)
                .fold(f64::MAX, f64::min);

            // clefs, key signature and time signature at the start of the system
            for (pi, gl) in per_staff.iter().enumerate() {
                let st = &staves[parts[pi]];
                let mut sharps = 0;
                let mut flats = 0;
                let mut digits: Vec<(f64, f64, u8)> = vec![];
                for g in gl.iter().filter(|g| g.x < first_event_x - sp * 0.3) {
                    match g.sym {
                        Some(Sym::ClefG) if g.size > music_size * 0.6 => {
                            clefs_state[pi] = ClefKind::G;
                            if g.ch as u32 == 0xE052 {
                                guitar = true;
                            }
                        }
                        Some(Sym::ClefF) if g.size > music_size * 0.6 => clefs_state[pi] = ClefKind::F,
                        Some(Sym::ClefC) if g.size > music_size * 0.6 => clefs_state[pi] = ClefKind::C,
                        Some(Sym::Sharp) if is_music(g) => sharps += 1,
                        Some(Sym::Flat) if is_music(g) => flats += 1,
                        Some(Sym::TimeDigit(d)) if is_music(g) && g.y > st.top() - sp && g.y < st.bottom() + sp => digits.push((g.x, g.y, d)),
                        Some(Sym::CommonTime) if is_music(g) => {
                            ts = (4, 4);
                            have_ts = true;
                        }
                        Some(Sym::CutTime) if is_music(g) => {
                            ts = (2, 2);
                            have_ts = true;
                        }
                        _ => {}
                    }
                }
                // a small "8" under a treble clef = guitar clef (sounds an octave lower)
                if m.glyphs.iter().any(|g| g.ch == '8' && g.x < first_event_x && g.y > st.bottom() && g.y < st.bottom() + sp * 3.0 && g.x < st.x0 + sp * 5.0) {
                    guitar = true;
                }
                if pi == 0 {
                    let k = if sharps > 0 { sharps } else { -flats };
                    key_fifths = k.clamp(-7, 7);
                    have_key = true;
                }
                if !digits.is_empty() {
                    let top: Vec<u8> = digits.iter().filter(|d| d.1 < st.mid()).map(|d| d.2).collect();
                    let bot: Vec<u8> = digits.iter().filter(|d| d.1 > st.mid()).map(|d| d.2).collect();
                    let num = top.iter().fold(0u32, |a, d| a * 10 + *d as u32);
                    let den = bot.iter().fold(0u32, |a, d| a * 10 + *d as u32);
                    if num > 0 && [1, 2, 4, 8, 16].contains(&den) {
                        ts = (num, den);
                        have_ts = true;
                    }
                }
            }
            let _ = have_ts;
            let mut bar_len = ts.0 as f64 * 4.0 / ts.1 as f64;
            let mut alters = key_alters(key_fifths);
            if first_key.is_none() {
                first_key = Some(key_fifths);
                first_ts = Some(ts);
            } else if key_changes.last().map(|k| k.1) != Some(key_fifths) {
                key_changes.push((t_global, key_fifths));
            }

            // per measure
            for mi in 0..bars.len() {
                let mx0 = if mi == 0 { first_event_x - sp * 0.5 } else { bars[mi - 1] };
                let mx1 = bars[mi];
                if mx1 - mx0 < sp * 1.5 || mx1 < first_event_x {
                    continue;
                }
                // a key or time signature change at the start of a bar inside the system
                if mi > 0 {
                    let gl = &per_staff[0];
                    let st = &staves[parts[0]];
                    let first_x = per_staff
                        .iter()
                        .flat_map(|v| v.iter())
                        .filter(|g| g.x >= mx0 && g.x < mx1 && matches!(g.sym, Some(Sym::HeadBlack | Sym::HeadHalf | Sym::HeadWhole | Sym::RestWhole | Sym::RestHalf | Sym::RestQuarter | Sym::Rest8 | Sym::Rest16)) && is_music(g))
                        .map(|g| g.x)
                        .fold(f64::MAX, f64::min);
                    let pre: Vec<&&Glyph> = gl.iter().filter(|g| g.x > mx0 && g.x < first_x - sp * 1.3 && is_music(g) && g.y > st.top() - sp * 2.0 && g.y < st.bottom() + sp * 2.0).collect();
                    let sh = pre.iter().filter(|g| g.sym == Some(Sym::Sharp)).count() as i32;
                    let fl = pre.iter().filter(|g| g.sym == Some(Sym::Flat)).count() as i32;
                    let na = pre.iter().filter(|g| g.sym == Some(Sym::Natural)).count();
                    let newkey = if sh > 0 && fl == 0 { Some(sh) } else if fl > 0 && sh == 0 { Some(-fl) } else if na > 0 && sh == 0 && fl == 0 { Some(0) } else { None };
                    if let Some(k) = newkey {
                        if k.clamp(-7, 7) != key_fifths {
                            key_fifths = k.clamp(-7, 7);
                            alters = key_alters(key_fifths);
                            key_changes.push((t_global, key_fifths));
                        }
                    }
                    let mut top: Vec<(f64, u8)> = vec![];
                    let mut bot: Vec<(f64, u8)> = vec![];
                    for g in &pre {
                        if let Some(Sym::TimeDigit(d)) = g.sym {
                            if g.y < st.mid() { top.push((g.x, d)) } else { bot.push((g.x, d)) }
                        }
                    }
                    top.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                    bot.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                    let num = top.iter().fold(0u32, |a, d| a * 10 + d.1 as u32);
                    let den = bot.iter().fold(0u32, |a, d| a * 10 + d.1 as u32);
                    if num > 0 && [1, 2, 4, 8, 16].contains(&den) {
                        ts = (num, den);
                        bar_len = ts.0 as f64 * 4.0 / ts.1 as f64;
                    }
                }
                let mut events: Vec<Event> = vec![];
                for (pi, gl) in per_staff.iter().enumerate() {
                    let st = &staves[parts[pi]];
                    let mut accs: Vec<(f64, i32, i32)> = vec![]; // x, step, alteration
                    let mut clef = clefs_state[pi];
                    let in_measure: Vec<&&Glyph> = gl.iter().filter(|g| g.x >= mx0 - sp * 0.2 && g.x < mx1).collect();
                    // clef changes inside the measure
                    for g in &in_measure {
                        let s = st.step(g.y);
                        match g.sym {
                            Some(Sym::ClefG) => clef = ClefKind::G,
                            Some(Sym::ClefF) => clef = ClefKind::F,
                            Some(Sym::ClefC) => clef = ClefKind::C,
                            Some(Sym::Sharp) if is_music(g) => accs.push((g.x, s, 1)),
                            Some(Sym::Flat) if is_music(g) => accs.push((g.x, s, -1)),
                            Some(Sym::Natural) if is_music(g) => accs.push((g.x, s, 0)),
                            Some(Sym::DoubleSharp) if is_music(g) => accs.push((g.x, s, 2)),
                            Some(Sym::DoubleFlat) if is_music(g) => accs.push((g.x, s, -2)),
                            _ => {}
                        }
                    }
                    clefs_state[pi] = clef;
                    // noteheads → events (grouped by stem)
                    let heads: Vec<&&Glyph> = in_measure.iter().copied().filter(|g| matches!(g.sym, Some(Sym::HeadBlack | Sym::HeadHalf | Sym::HeadWhole)) && g.size > music_size * 0.8).collect();
                    let mut used = vec![false; heads.len()];
                    for (hi, h) in heads.iter().enumerate() {
                        if used[hi] {
                            continue;
                        }
                        let head_w = sp * 1.18;
                        let base = match h.sym {
                            Some(Sym::HeadWhole) => 4.0,
                            Some(Sym::HeadHalf) => 2.0,
                            _ => 1.0,
                        };
                        // chord members: heads at (almost) the same x, or touching (seconds)
                        let mut chord = vec![hi];
                        for (hj, h2) in heads.iter().enumerate() {
                            if hj != hi && !used[hj] && h2.sym == h.sym && (h2.x - h.x).abs() < head_w * 1.1 {
                                // must share a stem: same stem side – approximate by vertical proximity
                                if (h2.y - h.y).abs() < sp * 8.0 {
                                    chord.push(hj);
                                }
                            }
                        }
                        for &c in &chord {
                            used[c] = true;
                        }
                        let cx_left = chord.iter().map(|&c| heads[c].x).fold(f64::MAX, f64::min);
                        let ys: Vec<f64> = chord.iter().map(|&c| heads[c].y).collect();
                        let (ymin, ymax) = (ys.iter().cloned().fold(f64::MAX, f64::min), ys.iter().cloned().fold(f64::MIN, f64::max));
                        // stem: vertical segment starting at a head edge
                        // A head can carry two stems: one up, one down (two voices sharing a note, as in
                        // classical guitar music where the tune's note also starts the accompaniment).
                        let mut best_up: Option<((f64, f64, bool), f64)> = None; // (x, far y, down), edge distance
                        let mut best_down: Option<((f64, f64, bool), f64)> = None;
                        if base < 4.0 {
                            for g in &m.segs {
                                if (g.x0 - g.x1).abs() > 0.8 || g.w > sp * 0.4 {
                                    continue;
                                }
                                let (ya, yb) = (g.y0.min(g.y1), g.y0.max(g.y1));
                                let len = yb - ya;
                                if len < sp * 1.8 || len > sp * 14.0 {
                                    continue;
                                }
                                let x = g.x0;
                                // stems sit on the left edge (down) or right edge (up) of a notehead
                                let edge_d = (x - cx_left).abs().min((x - (cx_left + head_w)).abs());
                                if edge_d > sp * 0.4 {
                                    continue;
                                }
                                // the stem overlaps the chord vertically and sticks out on one side
                                if ya > ymax + sp * 0.6 || yb < ymin - sp * 0.6 {
                                    continue;
                                }
                                let above = ymin - ya;
                                let below = yb - ymax;
                                let cand = if above > sp * 1.5 && above >= below {
                                    Some((x, ya, false))
                                } else if below > sp * 1.5 {
                                    Some((x, yb, true))
                                } else {
                                    None
                                };
                                if let Some(c) = cand {
                                    let slot = if c.2 { &mut best_down } else { &mut best_up };
                                    if slot.is_none_or(|(_, d)| edge_d < d) {
                                        *slot = Some((c, edge_d));
                                    }
                                }
                            }
                        }
                        // an up stem sits on the right edge, a down stem on the left; both only count
                        // when each is on its proper side
                        let stems: Vec<Option<(f64, f64, bool)>> = match (best_up, best_down) {
                            (Some((u, _)), Some((d, _))) if u.0 > d.0 + head_w * 0.5 => vec![Some(u), Some(d)],
                            (Some((u, du)), Some((d, dd))) => vec![Some(if du <= dd { u } else { d })],
                            (Some((u, _)), None) => vec![Some(u)],
                            (None, Some((d, _))) => vec![Some(d)],
                            (None, None) => vec![None],
                        };
                        for stem in stems {
                        let mut dur = base;
                        if let Some((sx, sy, down)) = stem {
                            if base == 1.0 {
                                // beams crossing the stem near its far end
                                // beams are often drawn in pieces (one per stem gap), so count distinct
                                // bands by where they cross the stem, not the number of shapes
                                let mut bands: Vec<f64> = vec![];
                                for (bb, poly) in &m.fills {
                                    // (slanted beams can have a tall bounding box, so judge by the crossing point)
                                    if bb.x0 - 0.5 <= sx && bb.x1 + 0.5 >= sx && bb.width() > sp * 0.8 && bb.height() < sp * 12.0 {
                                        let near_tip = if down { sy - bb.y0 > -sp * 0.6 && bb.y1 - sy < sp * 12.0 } else { bb.y1 - sy > -sp * 0.6 && sy - bb.y0 < sp * 12.0 };
                                        if near_tip {
                                            let xq = sx.clamp(bb.x0 + 0.01, bb.x1 - 0.01);
                                            let ys: Vec<f64> = poly
                                                .windows(2)
                                                .filter(|w| (w[0].x - xq) * (w[1].x - xq) <= 0.0 && (w[0].x - w[1].x).abs() > 1e-6)
                                                .map(|w| w[0].y + (w[1].y - w[0].y) * (xq - w[0].x) / (w[1].x - w[0].x))
                                                .collect();
                                            let (lo, hi) = ys.iter().fold((f64::MAX, f64::MIN), |a, y| (a.0.min(*y), a.1.max(*y)));
                                            // a beam is a thick band (~half a space); slurs and ties are thin curves
                                            if ys.len() < 2 || hi - lo < sp * 0.28 || hi - lo > sp * 0.9 {
                                                continue;
                                            }
                                            let c = (lo + hi) / 2.0;
                                            let reach = if down { sy - c } else { c - sy };
                                            if reach < -sp * 0.6 || reach > sp * 2.6 {
                                                continue;
                                            }
                                            if !bands.iter().any(|b| (b - c).abs() < sp * 0.35) {
                                                bands.push(c);
                                            }
                                        }
                                    }
                                }
                                let beams = bands.len();
                                // flags
                                let flags = m
                                    .glyphs
                                    .iter()
                                    .filter_map(|g| match g.sym {
                                        Some(Sym::Flag(n)) if (g.x - sx).abs() < sp * 0.6 && (g.y - sy).abs() < sp * 1.5 => Some(n),
                                        _ => None,
                                    })
                                    .max()
                                    .unwrap_or(0);
                                let n = (beams.min(4) as u8).max(flags);
                                dur = 1.0 / f64::powi(2.0, n as i32);
                            }
                        }
                        // dots
                        let dots = in_measure
                            .iter()
                            .filter(|g| g.sym == Some(Sym::Dot) && g.x > cx_left + head_w * 0.8 && g.x < cx_left + head_w + sp * 1.8 && g.y > ymin - sp * 0.9 && g.y < ymax + sp * 0.9)
                            .map(|g| (g.x * 10.0).round() as i64)
                            .collect::<std::collections::BTreeSet<_>>()
                            .len();
                        if dots >= 2 {
                            dur *= 1.75;
                        } else if dots == 1 {
                            dur *= 1.5;
                        }
                        let pitches: Vec<(i32, i32)> = chord
                            .iter()
                            .map(|&c| {
                                let s = st.step(heads[c].y);
                                let acc = accs.iter().filter(|a| a.1 == s && a.0 < heads[c].x && heads[c].x - a.0 < sp * 4.0).map(|a| a.2).next_back().unwrap_or(99);
                                (clef.bottom_line() + s, acc)
                            })
                            .collect();
                        events.push(Event { x: cx_left, dur, pitches, rest: false, stem_down: stem.map(|s| s.2), staff: pi, rest_y: None });
                        }
                    }
                    // rests
                    for g in &in_measure {
                        let d = match g.sym {
                            Some(Sym::RestWhole) => bar_len,
                            Some(Sym::RestHalf) => 2.0,
                            Some(Sym::RestQuarter) => 1.0,
                            Some(Sym::Rest8) => 0.5,
                            Some(Sym::Rest16) => 0.25,
                            Some(Sym::Rest32) => 0.125,
                            _ => continue,
                        };
                        if !is_music(g) {
                            continue;
                        }
                        let dotted = in_measure.iter().any(|o| o.sym == Some(Sym::Dot) && o.x > g.x && o.x < g.x + sp * 2.5 && (o.y - g.y).abs() < sp * 1.5);
                        events.push(Event { x: g.x, dur: if dotted { d * 1.5 } else { d }, pitches: vec![], rest: true, stem_down: None, staff: pi, rest_y: Some(g.y) });
                    }
                    // apply accidentals "for the rest of the measure"
                    let _ = &accs;
                }
                // triplets: tuplet digits near events (a fingering "3" looks the same, so a triplet
                // reading is only used when the bar doesn't add up without it)
                let plain = events.clone();
                for g in m.glyphs.iter().filter(|g| matches!(g.sym, Some(Sym::TupletDigit(3))) || (g.ch == '3' && g.size < music_size * 0.7 && g.size > music_size * 0.2)) {
                    if g.x < mx0 || g.x > mx1 {
                        continue;
                    }
                    let mut idx: Vec<usize> = events.iter().enumerate().filter(|(_, e)| (e.x - g.x).abs() < sp * 3.5 && !e.rest).map(|(i, _)| i).collect();
                    if idx.len() >= 3 {
                        idx.sort_by(|a, b| (events[*a].x - g.x).abs().partial_cmp(&(events[*b].x - g.x).abs()).unwrap_or(std::cmp::Ordering::Equal));
                        for &i in idx.iter().take(3) {
                            if events[i].dur < 2.0 {
                                events[i].dur *= 2.0 / 3.0;
                            }
                        }
                    }
                }
                if events.is_empty() {
                    continue;
                }
                report.measures += 1;
                let mids: Vec<f64> = parts.iter().map(|&i| staves[i].mid()).collect();
                let (mut onset, mut totals, mut staff_clean) = time_measure(&plain, &mids, sp, bar_len, mx1);
                if !staff_clean {
                    let (o2, t2, c2) = time_measure(&events, &mids, sp, bar_len, mx1);
                    if c2 {
                        (onset, totals, staff_clean) = (o2, t2, c2);
                    } else {
                        // triplets whose "3" is only printed in the first bars: quavers that make a
                        // voice exactly half as long again as the bar
                        let mut trip = plain.clone();
                        for e in trip.iter_mut() {
                            if e.dur <= 0.5 + 1e-9 {
                                e.dur *= 2.0 / 3.0;
                            }
                        }
                        let (o3, t3, c3) = time_measure(&trip, &mids, sp, bar_len, mx1);
                        if c3 {
                            (onset, totals, staff_clean) = (o3, t3, c3);
                            events = trip;
                        } else {
                            events = plain;
                        }
                    }
                } else {
                    events = plain;
                }
                let _ = &mut onset;
                let total = totals.iter().cloned().fold(0.0, f64::max);
                let clean = staff_clean;
                if std::env::var("VS_DEBUG").is_ok() && !clean && report.measures <= std::env::var("VS_MAX").ok().and_then(|v| v.parse().ok()).unwrap_or(40) {
                    eprintln!("measure {} ts {:?} totals {:?} bar {:.2}", report.measures, ts, totals, bar_len);
                    for (i, e) in events.iter().enumerate() {
                        eprintln!("   x{:.0} s{} t{:.3} {}{:.3} {:?} down={:?}", e.x, e.staff, onset[i], if e.rest { "R" } else { "" }, e.dur, e.pitches.iter().map(|p| p.0).collect::<Vec<_>>(), e.stem_down);
                    }
                }
                let mut scale = 1.0;
                if clean {
                    report.clean_measures += 1;
                } else if first_measure && total < bar_len {
                    pickup = total;
                } else if total > 0.0 {
                    scale = bar_len / total;
                }
                let m_len = if first_measure && total < bar_len - 1e-3 { total } else { bar_len };
                first_measure = false;
                // emit notes, accidentals carried within the measure per staff/step
                let mut carried: std::collections::HashMap<(usize, i32), i32> = std::collections::HashMap::new();
                let mut order: Vec<usize> = (0..events.len()).collect();
                order.sort_by(|a, b| events[*a].x.partial_cmp(&events[*b].x).unwrap_or(std::cmp::Ordering::Equal));
                for &i in &order {
                    let e = &events[i];
                    if e.rest {
                        continue;
                    }
                    for &(step, acc) in &e.pitches {
                        let alt = if acc != 99 {
                            carried.insert((e.staff, step), acc);
                            acc
                        } else {
                            *carried.get(&(e.staff, step)).unwrap_or(&alters[step.rem_euclid(7) as usize])
                        };
                        let mut pitch = (step.div_euclid(7) + 1) * 12 + STEP_PC[step.rem_euclid(7) as usize] + alt;
                        if guitar {
                            pitch -= 12;
                        }
                        if let Some(o) = ottavas[e.staff].iter().find(|o| e.x >= o.0 && e.x <= o.1) {
                            pitch += o.2;
                        }
                        let hand = if parts.len() >= 2 {
                            if e.staff == 0 { Hand::Right } else { Hand::Left }
                        } else {
                            match e.stem_down {
                                Some(true) => Hand::Left,
                                Some(false) => Hand::Right,
                                None => if pitch < 55 { Hand::Left } else { Hand::Right },
                            }
                        };
                        notes.push(Note::new(t_global + onset[i] * scale, (e.dur * scale).max(0.1), pitch.clamp(21, 108) as u8, hand, None));
                        report.notes += 1;
                    }
                }
                bar_marks.push(crate::model::BarMark {
                    beat: t_global,
                    page: (page_no + 1) as u32,
                    rect: Some([
                        (mx0.max(sys_x0) / m.width) as f32,
                        ((staves[parts[0]].top() - sp * 2.5) / m.height) as f32,
                        (mx1 / m.width) as f32,
                        ((staves[parts[parts.len() - 1]].bottom() + sp * 2.5) / m.height) as f32,
                    ]),
                    ts,
                });
                t_global += m_len;
            }
        }
    }
    let _ = music_size_seen;
    if notes.is_empty() {
        // tell a scanned book (pictures of pages) from a document that just isn't music
        let considered: Vec<&PageMarks> = pages.iter().enumerate().filter(|(i, _)| only.is_none_or(|(a, b)| i + 1 >= a && i + 1 <= b)).map(|(_, m)| m).collect();
        let scans = considered.iter().filter(|m| m.images > 0 && m.glyphs.len() < 400).count();
        return Err(if scans * 2 >= considered.len().max(1) { NOT_TYPESET_SCAN.into() } else { NOT_MUSIC.into() });
    }
    // a single guitar line with no stems pointing down: split by pitch instead
    if guitar && !notes.iter().any(|n| n.hand == Hand::Left) {
        for n in &mut notes {
            if n.pitch < 55 {
                n.hand = Hand::Left;
            }
        }
    }
    // the same note read in two voices: keep one, the longer
    notes.sort_by(|a, b| a.pitch.cmp(&b.pitch).then(a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal)));
    let mut merged: Vec<Note> = Vec::with_capacity(notes.len());
    for n in notes {
        match merged.last_mut() {
            Some(l) if l.pitch == n.pitch && (l.start - n.start).abs() < 1e-6 => {
                if n.dur > l.dur {
                    l.dur = n.dur;
                    if n.hand == Hand::Left {
                        l.hand = Hand::Left;
                    }
                }
            }
            _ => merged.push(n),
        }
    }
    let notes = merged;
    let mut p = Piece {
        id: String::new(),
        title: title.to_string(),
        composer: String::new(),
        notes_text: if guitar {
            "Read from a guitar score: bass notes (stems down) go to the left hand; played an octave lower, as a guitar sounds.".into()
        } else {
            "Read directly from your PDF.".into()
        },
        bpm: 90.0,
        ts_num: first_ts.unwrap_or(ts).0,
        ts_den: first_ts.unwrap_or(ts).1,
        key_fifths: first_key.unwrap_or(key_fifths),
        minor: false,
        pickup: if pickup > 0.0 { pickup } else { 0.0 },
        notes,
        origin: Origin::BuiltIn,
        src: {
            if let Some(last) = sections.last_mut() {
                last.end = t_global;
            }
            // Drop running headers (the same words on many pages) and page numbers from titles,
            // then fold sections left without a heading into the one before.
            // compare lines without their page numbers
            let norm = |t: &str| t.trim_start_matches('\u{1}').split_whitespace().filter(|w| !w.chars().all(|c| c.is_ascii_digit())).collect::<Vec<_>>().join(" ").to_lowercase();
            let mut counts: std::collections::HashMap<String, usize> = Default::default();
            for sct in &sections {
                let mut seen = std::collections::HashSet::new();
                for part in sct.title.split(" · ") {
                    let n = norm(part);
                    if !n.is_empty() && seen.insert(n.clone()) {
                        *counts.entry(n).or_default() += 1;
                    }
                }
            }
            let mut cleaned: Vec<crate::model::Section> = vec![];
            for mut sct in sections {
                let parts: Vec<&str> = sct.title.split(" · ").filter(|p| {
                    let n = norm(p);
                    n.chars().count() >= 1 && !(p.starts_with('\u{1}') && counts.get(&n).copied().unwrap_or(0) >= 3)
                }).map(|p| p.trim_start_matches('\u{1}')).collect();
                let parts: Vec<String> = parts.iter().map(|p| norm_keep_case(p)).filter(|p| !p.is_empty()).collect();
                let title: String = parts.join(" · ").chars().take(70).collect();
                match (title.is_empty(), cleaned.last_mut()) {
                    (true, Some(prev)) => prev.end = sct.end,
                    _ => {
                        sct.title = title;
                        cleaned.push(sct);
                    }
                }
            }
            let sections = cleaned;
            Some(crate::model::SourceMap { pdf: String::new(), bars: bar_marks, sections })
        },
        key_changes: {
            // keep real changes only (drop repeats and a "change" to the key already in force)
            let mut v: Vec<(f64, i32)> = vec![];
            let mut cur = first_key.unwrap_or(key_fifths);
            for (t, k) in key_changes {
                if k != cur {
                    v.retain(|x| (x.0 - t).abs() > 1e-6);
                    v.push((t, k));
                    cur = k;
                }
            }
            v
        },
    };
    p.minor = crate::import::guess_minor(&p.notes, p.key_fifths);
    p.sort_notes();
    Ok((p, report))
}

/// Lay out a list of events as one voice: columns by x, each column lasts as long as its
/// shortest event. Returns the voice's length in beats.
fn lay(events: &[Event], ids: &[usize], onset: &mut [f64], sp: f64, bar_len: f64) -> f64 {
    let mut cols: Vec<(f64, Vec<usize>)> = vec![];
    for &i in ids {
        match cols.last_mut() {
            Some(c) if (events[i].x - c.0).abs() < sp * 0.9 => c.1.push(i),
            _ => cols.push((events[i].x, vec![i])),
        }
    }
    let mut t = 0.0;
    let mut end: f64 = 0.0;
    for c in &cols {
        for &i in &c.1 {
            onset[i] = t;
            end = end.max(t + events[i].dur);
        }
        let step = c.1.iter().map(|&i| events[i].dur).fold(f64::MAX, f64::min);
        t += step;
    }
    t.max(end.min(t + 1e-9)).max(if (end - bar_len).abs() < 1e-3 { bar_len } else { t })
}

fn by_x(events: &[Event], mut v: Vec<usize>) -> Vec<usize> {
    v.sort_by(|a, b| events[*a].x.partial_cmp(&events[*b].x).unwrap_or(std::cmp::Ordering::Equal));
    v
}

/// Timing: every staff (and every voice in it) fills the bar by itself.
/// Returns onsets, per-staff totals and whether the bar adds up.
fn time_measure(events: &[Event], mids: &[f64], sp: f64, bar_len: f64, bar_x1: f64) -> (Vec<f64>, Vec<f64>, bool) {
    let mut onset = vec![0.0f64; events.len()];
    let mut totals: Vec<f64> = vec![];
    let mut staff_ok = vec![true; mids.len()];
    for pi in 0..mids.len() {
        let mine = by_x(events, (0..events.len()).filter(|&i| events[i].staff == pi).collect());
        if mine.is_empty() {
            continue;
        }
        let total_a = lay(events, &mine, &mut onset, sp, bar_len);
        if (total_a - bar_len).abs() < 1e-3 {
            totals.push(total_a);
            continue;
        }
        // two voices: stems up / stems down (rests by height)
        let mid = mids[pi];
        let (up, down): (Vec<usize>, Vec<usize>) = mine.iter().partition(|&&i| match events[i].stem_down {
            Some(d) => !d,
            None => events[i].rest_y.map(|y| y < mid).unwrap_or(true),
        });
        let mut on_b = onset.clone();
        let tu = if up.is_empty() { bar_len } else { lay(events, &up, &mut on_b, sp, bar_len) };
        let td = if down.is_empty() { bar_len } else { lay(events, &down, &mut on_b, sp, bar_len) };
        if (tu - bar_len).abs() < 1e-3 && (td - bar_len).abs() < 1e-3 {
            onset = on_b;
            totals.push(bar_len);
            continue;
        }
        staff_ok[pi] = false;
        totals.push(total_a);
    }
    if staff_ok.iter().all(|&c| c) {
        return (onset, totals, true);
    }
    // One line shared between the hands (LH starts a run, RH finishes it; the idle staff's
    // rests are often hidden): read both staves as a single voice.
    if mids.len() >= 2 {
        let all = by_x(events, (0..events.len()).collect());
        let mut on_c = vec![0.0; events.len()];
        let t = lay(events, &all, &mut on_c, sp, bar_len);
        if (t - bar_len).abs() < 1e-3 {
            return (on_c, vec![bar_len], true);
        }
    }
    // A staff that doesn't add up by itself (hidden rests, cross-staff notes): place its
    // notes by lining them up with the staves that do.
    let mut anchors: Vec<(f64, f64)> = (0..events.len()).filter(|&i| staff_ok[events[i].staff]).map(|i| (events[i].x, onset[i])).collect();
    if anchors.is_empty() {
        return (onset, totals, false);
    }
    anchors.push((bar_x1, bar_len));
    anchors.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut all_matched = true;
    for i in 0..events.len() {
        if staff_ok[events[i].staff] {
            continue;
        }
        let x = events[i].x;
        if let Some(a) = anchors.iter().filter(|a| (a.0 - x).abs() < sp * 0.9).min_by(|a, b| (a.0 - x).abs().partial_cmp(&(b.0 - x).abs()).unwrap_or(std::cmp::Ordering::Equal)) {
            onset[i] = a.1;
        } else {
            all_matched = false;
            let k = anchors.iter().position(|a| a.0 > x).unwrap_or(anchors.len() - 1).max(1);
            let (l, r) = (anchors[k - 1], anchors[k]);
            let f = if r.0 > l.0 { ((x - l.0) / (r.0 - l.0)).clamp(0.0, 1.0) } else { 0.0 };
            onset[i] = l.1 + f * (r.1 - l.1);
        }
    }
    (onset, vec![bar_len], all_matched)
}

/// A heading line without page numbers at either end.
fn norm_keep_case(t: &str) -> String {
    let w: Vec<&str> = t.split_whitespace().collect();
    let is_num = |s: &&str| s.chars().all(|c| c.is_ascii_digit());
    let a = w.iter().position(|x| !is_num(x)).unwrap_or(w.len());
    let b = w.iter().rposition(|x| !is_num(x)).map(|i| i + 1).unwrap_or(a);
    w[a..b.max(a)].join(" ")
}

/// Quick check: does this PDF contain typeset music (vector music-font glyphs)?
pub fn is_typeset(bytes: Vec<u8>) -> bool {
    extract(bytes).map(|pages| pages.iter().take(2).any(|p| p.glyphs.iter().filter(|g| matches!(g.sym, Some(Sym::HeadBlack | Sym::HeadHalf))).count() > 8)).unwrap_or(false)
}

#[allow(dead_code)]
pub fn debug_dump(bytes: Vec<u8>) {
    if let Ok(pages) = extract(bytes) {
        for (i, p) in pages.iter().enumerate().take(1) {
            let staves = find_staves(p);
            println!("page {i}: {} glyphs, {} segs, {} fills, {} staves", p.glyphs.len(), p.segs.len(), p.fills.len(), staves.len());
            for s in staves.iter().take(4) {
                println!("  staff y {:?} x {:.0}-{:.0} sp {:.2}", s.lines.map(|v| v.round()), s.x0, s.x1, s.sp);
            }
            let mut sizes: std::collections::BTreeMap<String, (usize, f64)> = Default::default();
            for g in &p.glyphs {
                let k = format!("{:?}", g.sym);
                let e = sizes.entry(k).or_insert((0, 0.0));
                e.0 += 1;
                e.1 = g.size;
            }
            for (k, v) in sizes {
                println!("  {k}: {} (size {:.1})", v.0, v.1);
            }
        }
    }
}

#[allow(dead_code)]
pub fn debug_bars(bytes: Vec<u8>) {
    if let Ok(pages) = extract(bytes) {
        let m = &pages[0];
        let st = find_staves(m);
        let (t, b) = (st[0].top(), st[1].bottom());
        eprintln!("staff0 {:.1}-{:.1} staff1 {:.1}-{:.1}", st[0].top(), st[0].bottom(), st[1].top(), st[1].bottom());
        let mut v: Vec<&Seg> = m.segs.iter().filter(|g| (g.x0 - g.x1).abs() < 0.8 && g.y0.min(g.y1) < b + 5.0 && g.y0.max(g.y1) > t - 5.0 && (g.y0 - g.y1).abs() > 12.0).collect();
        v.sort_by(|a, c| a.x0.partial_cmp(&c.x0).unwrap());
        for g in v.iter().take(60) {
            eprintln!("x {:.1} y {:.1}-{:.1} w {:.2}", g.x0, g.y0.min(g.y1), g.y0.max(g.y1), g.w);
        }
        let fills: Vec<_> = m.fills.iter().filter(|(bb, _)| bb.width() < 3.0 && bb.y0 < b && bb.y1 > t).take(10).collect();
        for (bb, _) in fills { eprintln!("fill {:?}", bb); }
    }
}

#[allow(dead_code)]
pub fn debug_page_glyphs(bytes: Vec<u8>, page: usize) {
    if let Ok(pages) = extract(bytes) {
        let m = &pages[page];
        for g in &m.glyphs {
            if g.sym.is_none() || matches!(g.sym, Some(Sym::TimeDigit(_))) {
                eprintln!("{:?} U+{:04X} x{:.1} y{:.1} size {:.1}", g.ch, g.ch as u32, g.x, g.y, g.size);
            }
        }
        let st = find_staves(m);
        for s in &st { eprintln!("staff {:?}", s.lines.map(|v| v.round())); }
        for s in m.segs.iter().filter(|s| (s.y0 - s.y1).abs() < 0.6 && (s.x1 - s.x0).abs() > m.width * 0.08 && s.y0 > 200.0 && s.y0 < 320.0) {
            eprintln!("hrow y{:.2} x{:.1}-{:.1} w{:.2}", s.y0, s.x0, s.x1, s.w);
        }
        for g in m.segs.iter().filter(|g| (g.y0 - g.y1).abs() < 0.3 && (g.x1 - g.x0).abs() < 6.0 && (g.x1 - g.x0).abs() > 0.5).take(30) {
            eprintln!("hseg x{:.1}-{:.1} y{:.1} w{:.2}", g.x0, g.x1, g.y0, g.w);
        }
    }
}
