//! A tiny text notation so pieces can be typed by hand.
//!
//! Tokens are separated by spaces. `|` marks a bar line (ignored, just for reading).
//!   `C4:q`   C4 quarter note        durations: w h q e s  (whole..16th)
//!   `C4:q.`  dotted quarter         `t` = eighth-note triplet
//!   `C4+E4+G4:h` chord             `r:q` = rest
//!   `D5:e~`  tie into the next note of the same pitch
//!   `F#4` / `Bb3` accidentals; octave numbers follow scientific pitch (C4 = middle C)
//!   `C4:q/1` optional finger number after a slash

use crate::model::{Hand, Note};

pub fn parse_pitch(s: &str) -> Option<u8> {
    let b = s.as_bytes();
    if b.is_empty() {
        return None;
    }
    let base: i32 = match b[0].to_ascii_uppercase() {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        b'B' => 11,
        _ => return None,
    };
    let mut i = 1;
    let mut acc = 0;
    while i < b.len() && (b[i] == b'#' || b[i] == b'b') {
        acc += if b[i] == b'#' { 1 } else { -1 };
        i += 1;
    }
    let oct: i32 = s[i..].parse().ok()?;
    let p = (oct + 1) * 12 + base + acc;
    (0..=127).contains(&p).then_some(p as u8)
}

fn parse_dur(s: &str) -> Option<f64> {
    let mut chars = s.chars();
    let base = match chars.next()? {
        'w' => 4.0,
        'h' => 2.0,
        'q' => 1.0,
        'e' => 0.5,
        's' => 0.25,
        't' => 1.0 / 3.0,
        _ => return None,
    };
    let dots = chars.filter(|c| *c == '.').count();
    Some(match dots {
        0 => base,
        1 => base * 1.5,
        _ => base * 1.75,
    })
}

/// Parse one hand's line starting at `start` beats. Returns notes and end time.
pub fn parse_line(hand: Hand, text: &str, start: f64) -> Result<(Vec<Note>, f64), String> {
    let mut t = start;
    let mut out: Vec<Note> = Vec::new();
    let mut tie_open: Vec<usize> = Vec::new(); // indices of notes waiting for a tie
    let cleaned = text.replace('|', " ");
    for tok in cleaned.split_whitespace() {
        let (body, finger) = match tok.split_once('/') {
            Some((b, f)) => (b, f.parse::<u8>().ok()),
            None => (tok, None),
        };
        let (pitches, dur_s) = body
            .split_once(':')
            .ok_or_else(|| format!("missing ':' in '{tok}'"))?;
        let tie = dur_s.ends_with('~');
        let dur = parse_dur(dur_s.trim_end_matches('~'))
            .ok_or_else(|| format!("bad duration in '{tok}'"))?;
        if pitches != "r" {
            let mut new_open = Vec::new();
            for ps in pitches.split('+') {
                let p = parse_pitch(ps).ok_or_else(|| format!("bad pitch '{ps}' in '{tok}'"))?;
                // continue a tie?
                if let Some(pos) = tie_open.iter().position(|&i| out[i].pitch == p) {
                    let idx = tie_open.remove(pos);
                    out[idx].dur += dur;
                    if tie {
                        new_open.push(idx);
                    }
                    continue;
                }
                out.push(Note { finger, ..Note::new(t, dur, p, hand, None) });
                if tie {
                    new_open.push(out.len() - 1);
                }
            }
            tie_open = new_open;
        } else {
            tie_open.clear();
        }
        t += dur;
    }
    Ok((out, t))
}
