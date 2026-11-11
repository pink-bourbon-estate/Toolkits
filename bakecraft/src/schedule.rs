//! schedule.rs - turns recipe steps into a clock-time timeline with alarms.
//!
//! TWO WAYS TO PLAN
//! * "start" mode  - "I'm starting at 08:00, when is everything due?"
//! * "finish" mode - "I want bread out of the oven by 10:00 Saturday - when do I
//!   have to start?" We plan forwards, then slide the whole timeline so the
//!   last step ends at your target.
//!
//! TEMPERATURE ADJUSTMENT (rule of thumb)
//! Yeast and sourdough bacteria roughly DOUBLE their speed for every ~9 C
//! warmer (and halve for every ~9 C cooler) within normal kitchen
//! temperatures. For steps marked `temp_sensitive`:
//!
//! `adjusted minutes = default minutes x 2 ^ ((reference temp - your room temp) / 9)`
//!
//! e.g. a 4 h bulk written for 24 C becomes ~4 h 50 at 21 C. It is an
//! estimate - the "done when" cue on every step is the real test. Fridge
//! steps are NOT adjusted (a fridge is the same temperature all year).
//!
//! SEVERAL BAKES AT ONCE
//! Each recipe you add is a "batch". All batches go into one merged timeline
//! sorted by time, like a bakery production sheet, so clashes show up.

use crate::formulas::{Recipe, Step};
use chrono::{Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Temperature that doubles / halves the speed.
const DOUBLING_DEGREES_C: f64 = 9.0;
/// Safety clamp so silly temperatures cannot produce silly times.
const MIN_FACTOR: f64 = 0.4;
const MAX_FACTOR: f64 = 2.5;

/// How much longer (>1) or shorter (<1) fermentation takes at `room_c` vs `ref_c`.
pub fn temperature_factor(room_c: f64, ref_c: f64) -> f64 {
    2f64.powf((ref_c - room_c) / DOUBLING_DEGREES_C).clamp(MIN_FACTOR, MAX_FACTOR)
}

/// Duration of one step after any manual override and temperature adjustment.
pub fn adjusted_minutes(step: &Step, room_c: f64, ref_c: f64, override_min: Option<i64>) -> i64 {
    let base = override_min.unwrap_or(step.minutes);
    if step.temp_sensitive {
        (base as f64 * temperature_factor(room_c, ref_c)).round() as i64
    } else {
        base
    }
}

/// Whether the plan counts from the start or from the finish.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    /// `anchor` is when you begin.
    Start,
    /// `anchor` is when the last step must end ("ready by").
    Finish,
}

/// One bake in the plan, with its own choices. This is what gets saved to disk.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Batch {
    pub recipe_key: String,
    /// e.g. "Classic sourdough x2"
    pub label: String,
    pub pieces: u32,
    pub piece_weight: f64,
    pub hydration: f64,
    pub yeast_type: String,
    /// The start time OR the finish time (see `mode`).
    pub anchor: NaiveDateTime,
    pub mode: Mode,
    pub room_c: f64,
    /// Step index -> minutes, for manual changes.
    #[serde(default)]
    pub overrides: BTreeMap<usize, i64>,
}

/// One line on the timeline (a step, or a fold inside a step).
#[derive(Clone, Debug)]
pub struct Event {
    /// Position of the batch in the plan (used to nudge a whole bake).
    pub batch_index: usize,
    pub batch: String,
    pub name: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    /// True for a fold inside a step.
    pub is_fold: bool,
    pub alarm: bool,
    pub how: String,
    pub done_when: String,
}

impl Event {
    pub fn minutes(&self) -> i64 {
        (self.end - self.start).num_minutes()
    }
    /// A key that stays the same while the app runs, to remember "done" and "alarm rung".
    pub fn key(&self) -> String {
        format!("{}|{}|{}", self.batch_index, self.name.trim(), self.start.format("%Y%m%d%H%M"))
    }
}

/// Lay the recipe's steps out on the clock for one batch.
pub fn build_timeline(batch: &Batch, batch_index: usize, recipe: &Recipe) -> Vec<Event> {
    let mut events: Vec<Event> = Vec::new();

    // --- Plan forwards from the anchor ------------------------------------------
    let t0 = batch.anchor;
    let mut prev_end = t0;
    for (i, step) in recipe.steps.iter().enumerate() {
        let mins = adjusted_minutes(step, batch.room_c, recipe.ref_temp_c, batch.overrides.get(&i).copied());
        if mins <= 0 {
            continue; // a step set to 0 minutes is skipped
        }
        let start = if i == 0 { t0 } else { prev_end + Duration::minutes(step.start_offset) };
        let end = start + Duration::minutes(mins);
        events.push(Event {
            batch_index,
            batch: batch.label.clone(),
            name: step.name.to_string(),
            start,
            end,
            is_fold: false,
            alarm: step.alarm,
            how: step.how.to_string(),
            done_when: step.done_when.to_string(),
        });

        // Folds sit at fixed intervals from the start of the step, only if they fit.
        for (n, f) in step.folds_at.iter().enumerate() {
            let fold_time = start + Duration::minutes(*f);
            if fold_time < end {
                events.push(Event {
                    batch_index,
                    batch: batch.label.clone(),
                    name: format!("   Fold {} of {}", n + 1, step.folds_at.len()),
                    start: fold_time,
                    end: fold_time + Duration::minutes(5),
                    is_fold: true,
                    alarm: true,
                    how: "Wet hands: lift, stretch and fold the dough over itself on 4 sides.".into(),
                    done_when: String::new(),
                });
            }
        }
        // Overlapping steps (negative offset) must not drag the chain backwards.
        prev_end = if step.start_offset < 0 { prev_end.max(end) } else { end };
    }

    // --- "Finish" mode: slide everything so the last step ends at the anchor ---------
    if batch.mode == Mode::Finish {
        if let Some(last_end) = events.iter().map(|e| e.end).max() {
            let shift = batch.anchor - last_end;
            for e in &mut events {
                e.start += shift;
                e.end += shift;
            }
        }
    }

    // Steps before folds at the same moment.
    events.sort_by_key(|e| (e.start, e.is_fold));
    events
}

/// All batches in one production sheet, sorted by time. `lookup` finds a recipe by key.
pub fn merge_timelines<'a>(batches: &[Batch], lookup: impl Fn(&str) -> Option<&'a Recipe>) -> Vec<Event> {
    let mut all = Vec::new();
    for (i, b) in batches.iter().enumerate() {
        if let Some(r) = lookup(&b.recipe_key) {
            all.extend(build_timeline(b, i, r));
        }
    }
    all.sort_by(|a, b| (a.start, &a.batch).cmp(&(b.start, &b.batch)));
    all
}

/// Hands-on moments from DIFFERENT batches that start within `window_minutes`
/// of each other. Returns pairs of indexes into `events`. Long passive steps
/// (fridge retards) are ignored. `events` must be sorted by start time.
pub fn find_clashes(events: &[Event], window_minutes: i64) -> Vec<(usize, usize)> {
    let hands_on: Vec<usize> = (0..events.len()).filter(|&i| events[i].is_fold || events[i].minutes() <= 30).collect();
    let mut clashes = Vec::new();
    for (pos, &a) in hands_on.iter().enumerate() {
        for &b in &hands_on[pos + 1..] {
            if events[b].start - events[a].start > Duration::minutes(window_minutes) {
                break; // sorted by time, so nothing later can be close enough
            }
            if events[a].batch_index != events[b].batch_index {
                clashes.push((a, b));
            }
        }
    }
    clashes
}

/// Printable production sheet.
pub fn timeline_as_text(events: &[Event]) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut day = String::new();
    for e in events {
        let d = e.start.format("%A %d %b").to_string();
        if d != day {
            lines.push(String::new());
            lines.push(d.to_uppercase());
            lines.push("-".repeat(d.len()));
            day = d;
        }
        let dur = if e.is_fold { String::new() } else { format!("  ({} min)", e.minutes()) };
        lines.push(format!("  {} - {}  [{}]  {}{}", e.start.format("%H:%M"), e.end.format("%H:%M"), e.batch, e.name, dur));
        if !e.how.is_empty() && !e.is_fold {
            lines.push(format!("                 how:  {}", e.how));
        }
        if !e.done_when.is_empty() {
            lines.push(format!("                 done when: {}", e.done_when));
        }
    }
    lines.join("\n").trim().to_string()
}

/// The same timeline as CSV (opens in any spreadsheet).
pub fn timeline_as_csv(events: &[Event]) -> String {
    // Wrap a value in quotes and double any quotes inside it, so commas cannot break the columns.
    let q = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    let mut out = String::from("day,start,end,minutes,bake,step,how,done_when\n");
    for e in events {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            e.start.format("%Y-%m-%d"),
            e.start.format("%H:%M"),
            e.end.format("%H:%M"),
            e.minutes(),
            q(&e.batch),
            q(e.name.trim()),
            q(&e.how),
            q(&e.done_when)
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{build_recipes, find};
    use chrono::NaiveDate;

    fn when(d: u32, h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2030, 1, d).unwrap().and_hms_opt(h, m, 0).unwrap()
    }
    fn batch(key: &str, anchor: NaiveDateTime, mode: Mode, room: f64) -> Batch {
        Batch {
            recipe_key: key.into(),
            label: key.into(),
            pieces: 1,
            piece_weight: 500.0,
            hydration: 70.0,
            yeast_type: "instant".into(),
            anchor,
            mode,
            room_c: room,
            overrides: BTreeMap::new(),
        }
    }

    #[test]
    fn temperature_direction() {
        assert!(temperature_factor(18.0, 24.0) > 1.0); // colder = slower
        assert!(temperature_factor(28.0, 24.0) < 1.0); // warmer = faster
        assert!((temperature_factor(24.0, 24.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ready_by_ends_on_time() {
        let target = when(5, 10, 0);
        for r in build_recipes() {
            let ev = build_timeline(&batch(r.key, target, Mode::Finish, 21.0), 0, &r);
            assert_eq!(ev.iter().map(|e| e.end).max().unwrap(), target, "{}", r.key);
        }
    }

    #[test]
    fn start_mode_starts_on_time() {
        let rs = build_recipes();
        let start = when(5, 7, 0);
        let ev = build_timeline(&batch("baguette", start, Mode::Start, 24.0), 0, find(&rs, "baguette").unwrap());
        assert_eq!(ev.iter().map(|e| e.start).min().unwrap(), start);
    }

    #[test]
    fn preheat_overlaps_retard() {
        let rs = build_recipes();
        let ev = build_timeline(&batch("sourdough_classic", when(1, 8, 0), Mode::Start, 24.0), 0, find(&rs, "sourdough_classic").unwrap());
        let retard = ev.iter().find(|e| e.name.starts_with("Cold retard")).unwrap();
        let preheat = ev.iter().find(|e| e.name.starts_with("Preheat")).unwrap();
        assert_eq!(preheat.end, retard.end);
        assert_eq!(preheat.start, retard.end - Duration::minutes(45));
    }

    #[test]
    fn two_bakes_at_once_can_clash() {
        let rs = build_recipes();
        let at = when(5, 8, 0);
        let batches = vec![batch("focaccia", at, Mode::Start, 24.0), batch("focaccia", at, Mode::Start, 24.0)];
        let ev = merge_timelines(&batches, |k| find(&rs, k));
        assert!(!find_clashes(&ev, 10).is_empty());
    }
}
