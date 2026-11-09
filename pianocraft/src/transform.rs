//! Make any piece easier or harder, change its key, and rate its difficulty.

use crate::model::{is_black, transpose_key, Hand, Note, Piece};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RhMode {
    /// Every note as written.
    Full,
    /// Only the top note of each chord (the tune).
    Melody,
    /// Only the tune's notes that fall on main beats, held longer.
    Skeleton,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LhMode {
    Full,
    /// Lowest note only, every note kept (no chords).
    Bass,
    /// One bass note per half-bar (or bar), held.
    Roots,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Arrangement {
    pub rh: RhMode,
    pub lh: LhMode,
    pub transpose: i32,
}

impl Default for Arrangement {
    fn default() -> Self {
        Self { rh: RhMode::Full, lh: LhMode::Full, transpose: 0 }
    }
}

impl RhMode {
    pub const ALL: [RhMode; 4] = [RhMode::Full, RhMode::Melody, RhMode::Skeleton, RhMode::Off];
    pub fn label(self) -> &'static str {
        match self {
            RhMode::Full => "As written",
            RhMode::Melody => "Tune only (no chords)",
            RhMode::Skeleton => "Main beats only",
            RhMode::Off => "Off",
        }
    }
}

impl LhMode {
    pub const ALL: [LhMode; 4] = [LhMode::Full, LhMode::Bass, LhMode::Roots, LhMode::Off];
    pub fn label(self) -> &'static str {
        match self {
            LhMode::Full => "As written",
            LhMode::Bass => "Bass line (no chords)",
            LhMode::Roots => "Held bass notes",
            LhMode::Off => "Off",
        }
    }
}

fn group_by_onset(notes: &[&Note]) -> Vec<Vec<Note>> {
    let mut groups: Vec<Vec<Note>> = Vec::new();
    for n in notes {
        match groups.last_mut() {
            Some(g) if (g[0].start - n.start).abs() < 1e-3 => g.push((*n).clone()),
            _ => groups.push(vec![(*n).clone()]),
        }
    }
    groups
}

fn top_line(notes: &[&Note]) -> Vec<Note> {
    group_by_onset(notes)
        .into_iter()
        .map(|g| g.into_iter().max_by_key(|n| n.pitch).unwrap())
        .collect()
}

fn bottom_line(notes: &[&Note]) -> Vec<Note> {
    group_by_onset(notes)
        .into_iter()
        .map(|g| g.into_iter().min_by_key(|n| n.pitch).unwrap())
        .collect()
}

/// Keep notes starting on a strong beat; each lasts until the next kept note.
fn skeleton(line: Vec<Note>, piece: &Piece, step: f64) -> Vec<Note> {
    let mut kept: Vec<Note> = Vec::new();
    let mut slot = f64::NEG_INFINITY;
    for n in line {
        let rel = n.start - piece.pickup;
        let this_slot = (rel / step + 1e-6).floor();
        let on_beat = ((rel / step) - this_slot).abs() < 1e-3 || n.dur >= step - 1e-3;
        if on_beat && this_slot > slot {
            slot = this_slot;
            kept.push(n);
        }
    }
    let end = piece.end_beat();
    for i in 0..kept.len() {
        let next = kept.get(i + 1).map(|n| n.start).unwrap_or(end);
        kept[i].dur = (next - kept[i].start).max(0.25);
    }
    kept
}

fn roots(line: Vec<Note>, piece: &Piece) -> Vec<Note> {
    // one held note per half bar in 4/4, per bar otherwise
    let bar = piece.bar_len();
    let step = if (bar - 4.0).abs() < 1e-6 { 2.0 } else { bar };
    let mut out: Vec<Note> = Vec::new();
    let mut seg_i = i64::MIN;
    for n in line {
        let s = ((n.start - piece.pickup) / step + 1e-6).floor() as i64;
        if s != seg_i {
            seg_i = s;
            let seg_start = piece.pickup + s as f64 * step;
            let start = n.start.max(seg_start);
            out.push(Note { vel: n.vel, ..Note::new(start, seg_start + step - start, n.pitch, n.hand, None) });
        }
    }
    out
}

/// Main beat length used by "Main beats only".
fn main_beat(piece: &Piece) -> f64 {
    match (piece.ts_num, piece.ts_den) {
        (6, 8) | (9, 8) | (12, 8) => 1.5,
        (2, 2) => 2.0,
        _ => 1.0,
    }
}

/// Produce the version of the piece the student will actually practise.
pub fn arrange(piece: &Piece, a: &Arrangement) -> Piece {
    let rh: Vec<&Note> = piece.notes.iter().filter(|n| n.hand == Hand::Right).collect();
    let lh: Vec<&Note> = piece.notes.iter().filter(|n| n.hand == Hand::Left).collect();
    let mut notes: Vec<Note> = Vec::new();
    match a.rh {
        RhMode::Full => notes.extend(rh.iter().map(|n| (*n).clone())),
        RhMode::Melody => notes.extend(top_line(&rh)),
        RhMode::Skeleton => notes.extend(skeleton(top_line(&rh), piece, main_beat(piece))),
        RhMode::Off => {}
    }
    match a.lh {
        LhMode::Full => notes.extend(lh.iter().map(|n| (*n).clone())),
        LhMode::Bass => notes.extend(bottom_line(&lh)),
        LhMode::Roots => notes.extend(roots(bottom_line(&lh), piece)),
        LhMode::Off => {}
    }
    let mut out = piece.clone();
    out.notes = notes;
    if a.transpose != 0 {
        out = transpose(&out, a.transpose);
    }
    out.sort_notes();
    out
}

pub fn transpose(piece: &Piece, semis: i32) -> Piece {
    let mut p = piece.clone();
    for n in &mut p.notes {
        n.pitch = (n.pitch as i32 + semis).clamp(21, 108) as u8;
    }
    p.key_fifths = transpose_key(piece.key_fifths, semis);
    for kc in &mut p.key_changes {
        kc.1 = transpose_key(kc.1, semis);
    }
    p
}

/// Choose the transposition (-6..=+6) with the fewest black keys, staying close to the original range.
pub fn easiest_transposition(piece: &Piece) -> i32 {
    let mut best = (usize::MAX, 0i32);
    for s in [0, -1, 1, -2, 2, -3, 3, -4, 4, -5, 5, -6, 6] {
        let blacks = piece.notes.iter().filter(|n| is_black((n.pitch as i32 + s).clamp(0, 127) as u8)).count();
        let score = blacks * 4 + s.unsigned_abs() as usize; // small nudge to stay near home
        if score < best.0 {
            best = (score, s);
        }
    }
    best.1
}

#[derive(Clone, Debug, Default)]
pub struct Difficulty {
    /// 1 (very easy) .. 10 (advanced)
    pub score: f32,
    pub notes_per_sec: f32,
    pub max_chord: usize,
    pub black_key_pct: f32,
    pub hand_span: u8,
    pub hands_together: bool,
}

pub fn rate(piece: &Piece) -> Difficulty {
    if piece.notes.is_empty() {
        return Difficulty { score: 1.0, ..Default::default() };
    }
    let secs = (piece.end_beat() * 60.0 / piece.bpm).max(1.0);
    let onsets = {
        let mut v: Vec<i64> = piece.notes.iter().map(|n| (n.start * 1000.0) as i64).collect();
        v.dedup();
        v.sort();
        v.dedup();
        v.len()
    };
    let nps = onsets as f32 / secs as f32;
    let mut max_chord = 1;
    for h in [Hand::Right, Hand::Left] {
        let hn: Vec<&Note> = piece.notes.iter().filter(|n| n.hand == h).collect();
        for g in group_by_onset(&hn) {
            max_chord = max_chord.max(g.len());
        }
    }
    let black = piece.notes.iter().filter(|n| is_black(n.pitch)).count() as f32 / piece.notes.len() as f32;
    // widest hand stretch inside a single bar
    let mut span = 0u8;
    for h in [Hand::Right, Hand::Left] {
        for b in 1..=piece.bar_count() {
            let s = piece.bar_start(b);
            let e = s + piece.bar_len();
            let ps: Vec<u8> = piece.notes.iter().filter(|n| n.hand == h && n.start >= s && n.start < e).map(|n| n.pitch).collect();
            if let (Some(lo), Some(hi)) = (ps.iter().min(), ps.iter().max()) {
                span = span.max(hi - lo);
            }
        }
    }
    let together = piece.has_hand(Hand::Right) && piece.has_hand(Hand::Left);
    let mut score = 1.0 + nps * 1.1 + (max_chord as f32 - 1.0) * 0.7 + black * 4.0 + (span as f32 - 7.0).max(0.0) * 0.18;
    if together {
        score += 1.0;
    }
    Difficulty {
        score: score.clamp(1.0, 10.0),
        notes_per_sec: nps,
        max_chord,
        black_key_pct: black * 100.0,
        hand_span: span,
        hands_together: together,
    }
}
