//! Core music model. Nothing in here knows about the UI or audio
//! (same layering idea as EffectCraft: engine below, egui on top).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hand {
    Right,
    Left,
}

impl Hand {
    pub fn label(self) -> &'static str {
        match self {
            Hand::Right => "RH",
            Hand::Left => "LH",
        }
    }
}

/// One note. Time is measured in quarter-note beats from the start.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub start: f64,
    pub dur: f64,
    pub pitch: u8,
    pub hand: Hand,
    #[serde(default)]
    pub finger: Option<u8>,
    /// Loudness 0..1 (accents, dynamics).
    #[serde(default = "default_vel")]
    pub vel: f32,
}

fn default_vel() -> f32 {
    0.75
}

impl Note {
    pub fn new(start: f64, dur: f64, pitch: u8, hand: Hand, finger: Option<u8>) -> Note {
        Note { start, dur, pitch, hand, finger, vel: 0.75 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Origin {
    BuiltIn,
    Generated,
    Imported(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Piece {
    pub id: String,
    pub title: String,
    pub composer: String,
    /// Short note shown under the title (source, arrangement credit, tips).
    #[serde(default)]
    pub notes_text: String,
    /// Tempo in quarter-note beats per minute.
    pub bpm: f64,
    /// Time signature numerator / denominator.
    pub ts_num: u32,
    pub ts_den: u32,
    /// Key signature in fifths (-7..=7). 0 = C major / A minor.
    pub key_fifths: i32,
    #[serde(default)]
    pub minor: bool,
    /// Beats (quarters) before bar 1 starts (anacrusis / pickup).
    #[serde(default)]
    pub pickup: f64,
    pub notes: Vec<Note>,
    pub origin: Origin,
    /// Key changes after the start: (beat, fifths). Used by exercises that travel through keys.
    #[serde(default)]
    pub key_changes: Vec<(f64, i32)>,
    /// Where the music is in your PDF (pages, bar positions, exercises), when it came from one.
    #[serde(default)]
    pub src: Option<SourceMap>,
}

/// One bar's place in the PDF.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BarMark {
    pub beat: f64,
    /// 1-based PDF page.
    pub page: u32,
    /// The bar on the page as fractions of the page size: [left, top, right, bottom].
    #[serde(default)]
    pub rect: Option<[f32; 4]>,
    /// Time signature in force.
    #[serde(default)]
    pub ts: (u32, u32),
}

/// An exercise or piece inside a PDF (from its heading).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Section {
    pub beat: f64,
    pub end: f64,
    pub page: u32,
    pub title: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SourceMap {
    pub pdf: String,
    pub bars: Vec<BarMark>,
    #[serde(default)]
    pub sections: Vec<Section>,
}

impl Piece {
    /// The PDF page (and bar position on it) being played at `beat`.
    pub fn page_at(&self, beat: f64) -> Option<(u32, Option<[f32; 4]>)> {
        let src = self.src.as_ref()?;
        let m = src.bars.iter().take_while(|m| m.beat <= beat + 1e-6).last().or(src.bars.first())?;
        Some((m.page, m.rect))
    }

    /// The bar whose box on `page` contains the point (fractions of the page), as its start beat.
    pub fn beat_at_point(&self, page: u32, x: f32, y: f32) -> Option<f64> {
        let src = self.src.as_ref()?;
        src.bars.iter().find(|m| m.page == page && m.rect.is_some_and(|r| x >= r[0] && x <= r[2] && y >= r[1] && y <= r[3])).map(|m| m.beat)
    }

    /// A stretch of this piece (beats a..b) as a piece of its own, starting at beat 0.
    pub fn slice(&self, a: f64, b: f64, id: String, title: String) -> Piece {
        let notes: Vec<Note> = self
            .notes
            .iter()
            .filter(|n| n.start >= a - 1e-6 && n.start < b - 1e-6)
            .map(|n| Note { start: n.start - a, dur: n.dur.min(b - n.start).max(0.05), ..n.clone() })
            .collect();
        let src = self.src.as_ref().map(|s| SourceMap {
            pdf: s.pdf.clone(),
            bars: s.bars.iter().filter(|m| m.beat >= a - 1e-6 && m.beat < b - 1e-6).map(|m| BarMark { beat: m.beat - a, ..m.clone() }).collect(),
            sections: vec![],
        });
        let ts = src.as_ref().and_then(|s| s.bars.first()).map(|m| m.ts).filter(|t| t.0 > 0).unwrap_or((self.ts_num, self.ts_den));
        Piece {
            id,
            title,
            composer: self.composer.clone(),
            notes_text: self.notes_text.clone(),
            bpm: self.bpm,
            ts_num: ts.0,
            ts_den: ts.1,
            key_fifths: self.key_at(a),
            minor: self.minor,
            pickup: 0.0,
            notes,
            origin: self.origin.clone(),
            key_changes: self.key_changes.iter().filter(|k| k.0 > a + 1e-6 && k.0 < b - 1e-6).map(|k| (k.0 - a, k.1)).collect(),
            src,
        }
    }

    pub fn bar_len(&self) -> f64 {
        self.ts_num as f64 * 4.0 / self.ts_den as f64
    }

    pub fn end_beat(&self) -> f64 {
        self.notes
            .iter()
            .map(|n| n.start + n.dur)
            .fold(0.0, f64::max)
    }

    /// Number of bars (pickup counts as bar 0 if present).
    pub fn bar_count(&self) -> usize {
        let body = (self.end_beat() - self.pickup).max(0.0);
        (body / self.bar_len() - 1e-6).ceil().max(1.0) as usize
    }

    /// Beat where bar `b` (1-based) begins.
    pub fn bar_start(&self, b: usize) -> f64 {
        self.pickup + (b.saturating_sub(1)) as f64 * self.bar_len()
    }

    pub fn bar_at(&self, beat: f64) -> usize {
        if beat < self.pickup {
            return 1;
        }
        ((beat - self.pickup) / self.bar_len()).floor() as usize + 1
    }

    /// Key signature in force at `beat`.
    pub fn key_at(&self, beat: f64) -> i32 {
        let mut k = self.key_fifths;
        for (b, f) in &self.key_changes {
            if *b <= beat + 1e-6 {
                k = *f;
            }
        }
        k
    }

    pub fn key_name(&self) -> String {
        key_name(self.key_fifths, self.minor)
    }

    pub fn sort_notes(&mut self) {
        self.notes.sort_by(|a, b| {
            a.start
                .partial_cmp(&b.start)
                .unwrap()
                .then(a.pitch.cmp(&b.pitch))
        });
    }

    pub fn has_hand(&self, h: Hand) -> bool {
        self.notes.iter().any(|n| n.hand == h)
    }
}

pub const SHARP_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
pub const FLAT_NAMES: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];

pub fn is_black(pitch: u8) -> bool {
    matches!(pitch % 12, 1 | 3 | 6 | 8 | 10)
}

pub fn pitch_name(pitch: u8, prefer_flats: bool) -> String {
    let pc = (pitch % 12) as usize;
    let oct = pitch as i32 / 12 - 1;
    let n = if prefer_flats { FLAT_NAMES[pc] } else { SHARP_NAMES[pc] };
    format!("{n}{oct}")
}

pub fn pitch_letter(pitch: u8, prefer_flats: bool) -> &'static str {
    let pc = (pitch % 12) as usize;
    if prefer_flats {
        FLAT_NAMES[pc]
    } else {
        SHARP_NAMES[pc]
    }
}

/// Tonic pitch class of a key given in fifths.
pub fn key_tonic_pc(fifths: i32, minor: bool) -> u8 {
    let major = (fifths * 7).rem_euclid(12) as u8;
    if minor {
        (major + 9) % 12
    } else {
        major
    }
}

pub fn key_name(fifths: i32, minor: bool) -> String {
    let pc = key_tonic_pc(fifths, minor) as usize;
    let flats = fifths < 0;
    let n = if flats { FLAT_NAMES[pc] } else { SHARP_NAMES[pc] };
    format!("{n} {}", if minor { "minor" } else { "major" })
}

/// Shift a key signature by `semis` semitones, choosing the simpler spelling.
pub fn transpose_key(fifths: i32, semis: i32) -> i32 {
    // each semitone up = +7 fifths (mod 12)
    let mut f = (fifths + semis * 7).rem_euclid(12);
    if f > 6 {
        f -= 12;
    }
    // prefer F# (6) over Gb (-6) only when we started on the sharp side
    if f == -6 && fifths > 0 {
        f = 6;
    }
    f
}

pub fn fmt_dur(beats: f64) -> &'static str {
    let b = (beats * 4.0).round() as i32;
    match b {
        16 => "whole",
        12 => "dotted half",
        8 => "half",
        6 => "dotted quarter",
        4 => "quarter",
        3 => "dotted eighth",
        2 => "eighth",
        1 => "16th",
        _ => "",
    }
}
