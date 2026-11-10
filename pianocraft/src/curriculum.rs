//! The lesson plan: Russian School of Piano Playing, chapter by chapter (by default starting at
//! 1a, chapter V – any unit can be picked), combined with Nahre Sol's principles and simplified versions of her exercises.
//!
//! Russian School method (from the book's own explanatory notes): sing and pick out by ear,
//! count aloud, hands separately before together, learn by heart phrase by phrase "like a poem",
//! transpose from the very beginning, scales in ascending fifths from C – hands separately,
//! one octave then two, contrary motion before parallel, tonic triads with inversions, short arpeggios.
//!
//! Nahre Sol (Elements of Music + exercise books): the keyboard's geography, read by reference
//! points, everything starts with rhythm, melodies breathe, scales/modes as building blocks,
//! harmony through voice-leading; practise scales musically (circling figures, chains through keys,
//! left-hand swells, trills over moving chords).

use crate::books::{BookId, RS1A_SCALE_TABLE, RS1B_SCALE_TABLE};
use crate::generate::*;
use crate::model::Piece;
use crate::transform::{Arrangement, LhMode, RhMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ------------------------------------------------------------------ practice ladders

#[derive(Clone, Debug)]
pub struct Rung {
    pub label: &'static str,
    pub how: &'static str,
    pub rh: RhMode,
    pub lh: LhMode,
    pub you_rh: bool,
    pub you_lh: bool,
    pub tempo_pct: u32,
    pub listen: bool,
    pub transpose: bool,
    pub metronome: bool,
    /// Memory step: notes are hidden until you ask for a peek.
    pub memorise: bool,
}

impl Rung {
    fn new(label: &'static str, how: &'static str, rh: RhMode, lh: LhMode, you_rh: bool, you_lh: bool, tempo_pct: u32) -> Rung {
        Rung { label, how, rh, lh, you_rh, you_lh, tempo_pct, listen: false, transpose: false, metronome: false, memorise: false }
    }
    pub fn arrangement(&self, transpose: i32) -> Arrangement {
        Arrangement { rh: self.rh, lh: self.lh, transpose }
    }
}

/// The Russian-School learning sequence for a piece.
pub fn ladder(piece: &Piece, is_drill: bool) -> Vec<Rung> {
    let rh = piece.has_hand(crate::model::Hand::Right);
    let lh = piece.has_hand(crate::model::Hand::Left);
    let mut v = Vec::new();
    if is_drill {
        if rh && lh {
            v.push(Rung::new("Right hand alone", "Slowly and evenly. Say the note names (Nahre Sol) or count out loud (Russian School).", RhMode::Full, LhMode::Off, true, false, 60));
            v.push(Rung::new("Left hand alone", "Same again with the left hand. Watch the thumb.", RhMode::Off, LhMode::Full, false, true, 60));
            v.push(Rung::new("Hands together, slowly", "Both hands. Listen for both lines sounding exactly together.", RhMode::Full, LhMode::Full, true, true, 60));
        } else {
            v.push(Rung::new("Slowly", "Slowly and evenly, with a calm wrist.", RhMode::Full, LhMode::Full, rh, lh, 60));
        }
        v.push(Rung::new("A bit faster", "Same sound quality at a quicker tempo.", RhMode::Full, LhMode::Full, rh, lh, 80));
        v.push(Rung::new("Target tempo", "Even, singing, no bumps on the thumb. Then try it with your eyes closed.", RhMode::Full, LhMode::Full, rh, lh, 100));
        return v;
    }
    v.push(Rung { listen: true, ..Rung::new("Listen and sing", "Russian School: hear it first. Listen twice following the notes, then sing or hum the tune. Find the key and time signature.", RhMode::Full, LhMode::Full, false, false, 100) });
    if rh {
        v.push(Rung { listen: true, metronome: true, ..Rung::new("Count and clap", "Clap the right-hand rhythm while counting out loud (\"one-and-two-and\"). Nahre Sol: everything starts with rhythm.", RhMode::Full, LhMode::Off, false, false, 70) });
        v.push(Rung::new("Right hand alone", "Slowly, counting out loud, with the written fingering.", RhMode::Full, LhMode::Off, true, false, 60));
    }
    if lh {
        v.push(Rung::new("Left hand alone", "Slowly. The left hand deserves the same care – it's half the music.", RhMode::Off, LhMode::Full, false, true, 60));
    }
    if rh && lh {
        v.push(Rung::new("Hands together, slowly", "Phrase by phrase. Slow is fine; stopping isn't. Count silently now.", RhMode::Full, LhMode::Full, true, true, 60));
    }
    v.push(Rung { memorise: true, ..Rung::new("Learn it by heart", "Russian School: memorise like a poem, one phrase at a time. The notes are hidden – press 'Peek' if you get lost.", RhMode::Full, LhMode::Full, rh, lh, 70) });
    v.push(Rung::new("Up to tempo, with character", "Full speed. Shape the phrases: where does each one go? Loud/soft, legato/staccato as marked.", RhMode::Full, LhMode::Full, rh, lh, 100));
    v.push(Rung { transpose: true, ..Rung::new("Transpose", "Russian School: play it in a new key, by ear and memory. It shows you really know it.", RhMode::Full, LhMode::Full, rh, lh, 80) });
    v
}

// ------------------------------------------------------------------ progress

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PieceProgress {
    pub rung: usize,
    pub tempo_pct: Option<u32>,
    pub section: Option<(usize, usize)>,
    pub done: bool,
    pub too_hard_count: u32,
    pub nailed_count: u32,
    #[serde(default)]
    pub transpose_to: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub day: String,
    pub piece: String,
    pub rung: String,
    pub result: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Progress {
    pub pieces: HashMap<String, PieceProgress>,
    pub log: Vec<LogEntry>,
    pub minutes_by_day: HashMap<String, f32>,
    /// Imported and converted music. Kept in its own file (library.json) because converted
    /// books can be large; read from progress.json too for older versions.
    #[serde(default, skip_serializing)]
    pub library: Vec<Piece>,
    #[serde(skip)]
    pub library_dirty: std::cell::Cell<bool>,
    /// Bumped whenever the library changes (so screens can cache what they show).
    #[serde(skip)]
    pub lib_gen: std::cell::Cell<u64>,
    #[serde(default)]
    pub scan_folder: String,
    #[serde(default)]
    pub midi_port: Option<String>,
    #[serde(default)]
    pub unit_override: Option<usize>,
    /// Course layout version (2 = starts at 1a chapter I; earlier versions started at chapter V).
    #[serde(default)]
    pub course_v: u32,
    /// "pdf path#page" -> ids of the pieces converted from that page, in order.
    #[serde(default)]
    pub page_conv: HashMap<String, Vec<String>>,
    /// Extra music folders (besides `scan_folder`), and PDFs found in them last time.
    #[serde(default)]
    pub pdf_index: Vec<String>,
    /// Where your books are.
    #[serde(default)]
    pub books: HashMap<BookId, String>,
    /// Book piece number -> library id of its playable (converted) version.
    #[serde(default)]
    pub book_links: HashMap<u32, String>,
    /// Ticked-off simple tasks (theory reading, ear tasks) by id.
    #[serde(default)]
    pub ticks: HashMap<String, String>,
    #[serde(default)]
    pub audiveris: String,
    #[serde(default)]
    pub reading_best: HashMap<String, u32>,
    #[serde(default)]
    pub keyboard_keys: Option<u8>,
    #[serde(default = "yes")]
    pub auto_update: bool,
    #[serde(default)]
    pub last_update_check: u64,
    #[serde(default)]
    pub install_prompt_dismissed: bool,
}

fn yes() -> bool {
    true
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn lib_dir() -> PathBuf {
    data_dir().join("library")
}

fn piece_file(id: &str) -> PathBuf {
    let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).take(120).collect();
    lib_dir().join(format!("{safe}.json"))
}

/// A piece without its notes, for the index: title, source, key, and the exercises inside it.
fn light(p: &Piece) -> Piece {
    let mut l = p.clone();
    l.notes.clear();
    if let Some(src) = l.src.as_mut() {
        if src.sections.is_empty() {
            let page = src.bars.first().map(|b| b.page).unwrap_or(1);
            src.sections.push(crate::model::Section { beat: 0.0, end: p.end_beat(), page, title: p.title.clone() });
        }
        src.bars.clear();
    }
    l
}

pub fn data_dir() -> PathBuf {
    let d = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("PianoCraft");
    let _ = std::fs::create_dir_all(&d);
    d
}

impl Progress {
    pub fn load() -> Progress {
        let p = data_dir().join("progress.json");
        let mut pr: Progress = std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        // The library is a small index (titles, sources, exercises); each piece's notes live in
        // their own file and are read only when you open it.
        match std::fs::read_to_string(lib_dir().join("index.json")).ok().and_then(|s| serde_json::from_str::<Vec<Piece>>(&s).ok()) {
            Some(index) => pr.library = index,
            None => {
                // older versions: everything in one big library.json (or inside progress.json)
                let old = data_dir().join("library.json");
                let full: Vec<Piece> = std::fs::read_to_string(&old).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(|| std::mem::take(&mut pr.library));
                pr.library = vec![];
                let mut ok = true;
                for p in full {
                    ok &= pr.add_piece(p);
                }
                if ok && pr.save_index() {
                    let _ = std::fs::remove_file(old);
                }
            }
        }
        if pr.course_v < 2 {
            // four chapters were added in front of the old first unit
            pr.unit_override = pr.unit_override.map(|u| u + 4);
            pr.course_v = 2;
        }
        pr
    }
    /// Add (or replace) a piece: its notes go to their own file, the index keeps the rest.
    /// Returns false if the file couldn't be written.
    pub fn add_piece(&mut self, p: Piece) -> bool {
        let _ = std::fs::create_dir_all(lib_dir());
        let ok = serde_json::to_string(&p).map(|s| std::fs::write(piece_file(&p.id), s).is_ok()).unwrap_or(false);
        let l = light(&p);
        self.library.retain(|x| x.id != l.id);
        self.library.push(l);
        self.library_changed();
        ok
    }

    /// Read a piece's notes from disk. Where it came from is taken from the index (which is what
    /// relinking updates).
    pub fn load_piece(&self, id: &str) -> Option<Piece> {
        let entry = self.library.iter().find(|p| p.id == id)?;
        let mut p: Piece = std::fs::read_to_string(piece_file(id)).ok().and_then(|s| serde_json::from_str(&s).ok())?;
        p.origin = entry.origin.clone();
        p.title = entry.title.clone();
        if let (Some(src), Some(e)) = (p.src.as_mut(), entry.src.as_ref()) {
            src.pdf = e.pdf.clone();
        }
        Some(p)
    }

    pub fn remove_piece(&mut self, id: &str) {
        self.library.retain(|p| p.id != id);
        let _ = std::fs::remove_file(piece_file(id));
        self.library_changed();
    }

    fn save_index(&self) -> bool {
        let _ = std::fs::create_dir_all(lib_dir());
        let p = lib_dir().join("index.json");
        match serde_json::to_string(&self.library) {
            Ok(s) => {
                let tmp = p.with_extension("json.tmp");
                std::fs::write(&tmp, s).is_ok() && std::fs::rename(tmp, p).is_ok()
            }
            Err(_) => false,
        }
    }

    /// Call after changing `library`; it's written on the next save.
    pub fn library_changed(&self) {
        self.library_dirty.set(true);
        self.lib_gen.set(self.lib_gen.get() + 1);
    }
    pub fn save(&self) {
        if self.library_dirty.replace(false) && !self.save_index() {
            self.library_dirty.set(true);
        }
        let p = data_dir().join("progress.json");
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let tmp = p.with_extension("json.tmp");
            if std::fs::write(&tmp, s).is_ok() {
                let _ = std::fs::rename(tmp, p);
            }
        }
    }
    pub fn streak(&self) -> u32 {
        let mut n = 0;
        let mut d = today_num();
        if !self.minutes_by_day.get(&day_string(d)).is_some_and(|m| *m > 0.5) {
            d -= 1;
        }
        while self.minutes_by_day.get(&day_string(d)).is_some_and(|m| *m > 0.5) {
            n += 1;
            d -= 1;
        }
        n
    }
    pub fn done(&self, id: &str) -> bool {
        self.pieces.get(id).is_some_and(|p| p.done) || self.ticks.contains_key(id)
    }
    /// Where a book is. (Not checked on disk here: this is asked many times a second, and checking
    /// a file on Google Drive is slow. Opening a missing file just shows an error on the page.)
    pub fn book_path(&self, b: BookId) -> Option<PathBuf> {
        self.books.get(&b).filter(|p| !p.trim().is_empty()).map(PathBuf::from)
    }
}

// ------------------------------------------------------------------ units

#[derive(Clone, Debug)]
pub enum Task {
    /// Generated technique, with an optional pointer to the full original in your books.
    Tech { role: &'static str, spec: TechSpec, original: Option<(BookId, usize, &'static str)> },
    /// Something to read in a book (theory), optionally followed by the note-reading trainer.
    Theory { id: &'static str, book: BookId, page: usize, title: &'static str, what: &'static str, reading: Option<u8> },
    /// Ear training / creative task (sing, pick out by ear, improvise, transpose).
    Ear { id: &'static str, title: &'static str, what: &'static str },
    /// A built-in playable piece that fits this unit.
    Builtin { id: &'static str, why: &'static str },
}

pub struct Unit {
    pub title: &'static str,
    pub book: BookId,
    pub page: usize,
    pub pieces: Vec<u32>,
    /// What the Russian School is teaching in this chapter.
    pub russian: &'static str,
    /// The Nahre Sol principle that goes with it.
    pub nahre: &'static str,
    pub tasks: Vec<Task>,
}

#[allow(clippy::too_many_arguments)]
fn t(ex: Exercise, scale: ScaleType, tonic: u8, octaves: u8, pattern: Pattern, motion: Motion, rhythm: Rhythm) -> TechSpec {
    TechSpec { ex, scale, tonic, octaves, pattern, motion, rhythm, ..TechSpec::default() }
}

const C: u8 = 0;
const D: u8 = 2;
const E: u8 = 4;
const F: u8 = 5;
const G: u8 = 7;
const A: u8 = 9;
const BB: u8 = 10;
const B: u8 = 11;

pub fn units() -> Vec<Unit> {
    use Exercise as X;
    use Motion as M;
    use Pattern as P;
    use Rhythm as R;
    use ScaleType as S;
    vec![
        Unit {
            title: "Singing by ear, the keyboard, notes",
            book: BookId::RussianSchool1a, page: 8,
            pieces: (1..=14).collect(),
            russian: "Sing each tune first, then pick it out on the piano with one finger (2, 3 or 4), each hand in turn. Learn the names of the keys, crotchets, minims and semibreves, and where the notes sit on the stave.",
            nahre: "Find your landmarks first (Elements, ch.2): middle C, treble G and bass F. Read every other note as a step or skip from the nearest one.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: t(X::FiveFinger, S::Major, C, 1, P::Straight, M::RightOnly, R::Quarters), original: None },
                Task::Tech { role: "Rhythm", spec: TechSpec { seed: 1, ..t(X::Rhythm, S::Major, C, 1, P::Straight, M::RightOnly, R::Quarters) }, original: None },
                Task::Theory { id: "th-0a", book: BookId::ElementsOfMusic, page: 16, title: "Landmark notes (Elements pp. 16–20)", what: "Read it, then do the note-reading trainer (Landmarks) until you get 20 in a row.", reading: Some(0) },
                Task::Ear { id: "ear-0a", title: "Pick it out", what: "Sing no. 1 'Tra-la-la', then find it on the piano with your 3rd finger. Then start it on a different key." },
            ],
        },
        Unit {
            title: "Different fingers (non legato)",
            book: BookId::RussianSchool1a, page: 15,
            pieces: (15..=24).collect(),
            russian: "Now each finger gets a turn. Play detached, with a loose arm and a round hand, the weight of the arm dropping into each key.",
            nahre: "Relaxed and steady before fast (Elements, ch.3): count out loud and feel the beat in your body before you play.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: t(X::FiveFinger, S::Major, C, 1, P::Straight, M::Parallel, R::Quarters), original: None },
                Task::Tech { role: "Other keys", spec: t(X::FiveFinger, S::Major, G, 1, P::Straight, M::Parallel, R::Quarters), original: None },
                Task::Tech { role: "Rhythm", spec: TechSpec { seed: 2, ..t(X::Rhythm, S::Major, C, 1, P::Straight, M::RightOnly, R::Quarters) }, original: None },
                Task::Ear { id: "ear-0b", title: "Somewhere else", what: "No. 22 'Exercise' says: start it on D, E, F and A too. Do that by ear, without writing anything down." },
            ],
        },
        Unit {
            title: "Sharps, flats and naturals",
            book: BookId::RussianSchool1a, page: 18,
            pieces: (25..=31).collect(),
            russian: "A sharp raises a note a semitone, a flat lowers it, a natural cancels them. Accidentals last for the rest of the bar; the key signature lasts for the whole piece.",
            nahre: "Hear the half step: play E–F and B–C, the white-key semitones, then make the same sound on black keys.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: t(X::FiveFinger, S::Major, D, 1, P::Straight, M::Parallel, R::Quarters), original: None },
                Task::Tech { role: "Minor", spec: t(X::FiveFinger, S::NaturalMinor, A, 1, P::Straight, M::Parallel, R::Quarters), original: None },
                Task::Ear { id: "ear-0c", title: "Major or minor?", what: "Play a five-finger pattern from C, then from C with E♭. Sing both. Which sounds happy, which sad?" },
            ],
        },
        Unit {
            title: "Smooth playing (legato)",
            book: BookId::RussianSchool1a, page: 20,
            pieces: (32..=45).collect(),
            russian: "Join the notes: one finger lets go exactly as the next one plays. Listen for the overlap – no gaps, no smudges – and let the wrist follow the phrase.",
            nahre: "Shape the line: every phrase rises and falls like a breath. Sing it, and play it the way you sang it.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: TechSpec { accent: Accent::Swell, ..t(X::FiveFinger, S::Major, C, 1, P::Straight, M::Parallel, R::Quarters) }, original: None },
                Task::Tech { role: "Contrary", spec: t(X::FiveFinger, S::Major, C, 1, P::Straight, M::Contrary, R::Quarters), original: None },
                Task::Theory { id: "th-0d", book: BookId::ElementsOfMusic, page: 59, title: "What makes a melody? (Elements pp. 59–61)", what: "Then mark the phrases of no. 34 'Latvian Folk Song' with the pen in the book view.", reading: None },
                Task::Ear { id: "ear-0d", title: "Legato by ear", what: "Sing no. 40 'Byelorussian Folk Song', then play it legato in C and in G." },
            ],
        },
        Unit {
            title: "Quavers (eighth notes)",
            book: BookId::RussianSchool1a, page: 24,
            pieces: (46..=58).collect(),
            russian: "Two quavers fit in one beat. Count out loud – \"one-and-two-and\" – and sing the folk songs before you play them.",
            nahre: "Everything starts with rhythm (Elements, ch.3): feel the steady beat first, then divide it in two.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: t(X::FiveFinger, S::Major, C, 1, P::Circling, M::Parallel, R::Eighths), original: Some((BookId::NahreScales, 2, "Circular Scales – this is the five-note seed of it")) },
                Task::Tech { role: "Scale", spec: t(X::Scale, S::Major, C, 1, P::Straight, M::Contrary, R::Quarters), original: Some((BookId::RussianSchool1a, RS1A_SCALE_TABLE, "C major in the scale table")) },
                Task::Tech { role: "Rhythm", spec: TechSpec { seed: 1, ..t(X::Rhythm, S::Major, C, 1, P::Straight, M::RightOnly, R::Eighths) }, original: None },
                Task::Theory { id: "th-1", book: BookId::ElementsOfMusic, page: 23, title: "Reading rhythm (Elements pp. 23–27)", what: "How note values and rests divide the beat. Read it with a pencil in hand, tapping the examples.", reading: None },
                Task::Ear { id: "ear-1", title: "Sing it, then find it", what: "Before opening the music for no. 47 or 57, listen (or have someone play it), sing the first phrase, and pick it out by ear in C. Then in G – the Russian School transposes from the start." },
            ],
        },
        Unit {
            title: "The bass clef",
            book: BookId::RussianSchool1a, page: 30,
            pieces: (59..=67).collect(),
            russian: "The left hand gets its own melodies. Read the bass clef without converting from treble.",
            nahre: "Read by reference points (Elements, ch.2): learn a few landmark notes and read everything else as a step or skip from them.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: t(X::FiveFinger, S::Major, G, 1, P::Straight, M::Parallel, R::Quarters), original: None },
                Task::Tech { role: "Scale", spec: t(X::Scale, S::Major, G, 1, P::Straight, M::Contrary, R::Quarters), original: Some((BookId::RussianSchool1a, RS1A_SCALE_TABLE, "Scale table")) },
                Task::Tech { role: "Chords", spec: t(X::TriadInversions, S::Major, C, 1, P::Straight, M::Parallel, R::Quarters), original: Some((BookId::RussianSchool1a, RS1A_SCALE_TABLE, "Triads with inversions in the table")) },
                Task::Theory { id: "th-2", book: BookId::ElementsOfMusic, page: 16, title: "Treble and bass clef reference points (Elements pp. 16–20)", what: "Nahre Sol's landmark method. After reading, do the note-reading trainer below until you get 20 in a row.", reading: Some(1) },
                Task::Ear { id: "ear-2", title: "Left hand sings", what: "Play no. 61 'Melody' with the left hand only, then sing it while you play. Can you play it starting on G instead?" },
            ],
        },
        Unit {
            title: "Dotted rhythms",
            book: BookId::RussianSchool1a, page: 32,
            pieces: (68..=72).collect(),
            russian: "A dot adds half the note's value. Count the dot – don't guess it.",
            nahre: "Dotted notes (Elements p. 27) and accents (p. 40): the weight of the bar falls on beat one.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: t(X::FiveFinger, S::Major, D, 1, P::Straight, M::Parallel, R::Dotted), original: None },
                Task::Tech { role: "Scale", spec: t(X::Scale, S::Major, D, 1, P::Straight, M::Parallel, R::Quarters), original: Some((BookId::RussianSchool1b, RS1B_SCALE_TABLE, "D major")) },
                Task::Tech { role: "Rhythm", spec: TechSpec { seed: 3, ..t(X::Rhythm, S::Major, D, 1, P::Straight, M::RightOnly, R::Dotted) }, original: None },
                Task::Theory { id: "th-3", book: BookId::ElementsOfMusic, page: 27, title: "Dotted notes, time signatures, accents (Elements pp. 27, 31–32, 40)", what: "Then clap the dotted rhythm exercise, counting out loud.", reading: Some(2) },
                Task::Ear { id: "ear-3", title: "From memory", what: "Clap the rhythm of no. 70 'On the Hill' from memory, then play it starting on D." },
            ],
        },
        Unit {
            title: "Anacrusis (pick-up notes)",
            book: BookId::RussianSchool1a, page: 34,
            pieces: (73..=77).collect(),
            russian: "A piece can start before beat one. Breathe before the pick-up; it leads into the strong beat.",
            nahre: "A good melody breathes and has a shape (Elements ch.4, p. 61). Agogic stress (p. 43): lean into the important note.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: t(X::FiveFinger, S::Major, A, 1, P::Turnaround, M::Parallel, R::Eighths), original: None },
                Task::Tech { role: "Scale", spec: t(X::Scale, S::Major, C, 2, P::Straight, M::RightOnly, R::Eighths), original: Some((BookId::RussianSchool1a, RS1A_SCALE_TABLE, "Two octaves")) },
                Task::Tech { role: "Cadence", spec: t(X::Cadence, S::Major, C, 1, P::Straight, M::Parallel, R::Quarters), original: None },
                Task::Theory { id: "th-4", book: BookId::ElementsOfMusic, page: 59, title: "What makes a melody? (Elements pp. 59–61)", what: "Shape, balance, breathing. Look at no. 76 'Our Land' and mark where each phrase breathes (use the pen in the book view).", reading: None },
                Task::Ear { id: "ear-4", title: "Answer the question", what: "Play the first two bars of no. 76, then make up two bars that 'answer' and end on the home note. Nahre Sol: question and answer phrases." },
            ],
        },
        Unit {
            title: "Semiquavers (sixteenth notes)",
            book: BookId::RussianSchool1a, page: 36,
            pieces: (78..=89).collect(),
            russian: "Four even semiquavers to a beat from loose fingers. Make triplet-sequence exercises from each scale you know.",
            nahre: "Five-note scales are the building blocks (Elements ch.5, p. 71): major and minor.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: t(X::FiveFinger, S::MinorPentachord, A, 1, P::Groups3, M::Parallel, R::Eighths), original: None },
                Task::Tech { role: "Scale", spec: t(X::Scale, S::Major, C, 1, P::Groups3, M::RightOnly, R::Eighths), original: Some((BookId::RussianSchool1a, 42, "Triplet sequence exercises (explanatory notes)")) },
                Task::Tech { role: "Minor scale", spec: t(X::Scale, S::HarmonicMinor, A, 1, P::Straight, M::Parallel, R::Quarters), original: Some((BookId::RussianSchool1a, RS1A_SCALE_TABLE, "A minor, harmonic and melodic")) },
                Task::Tech { role: "Short arpeggios", spec: TechSpec { chord: ChordType::Major, ..t(X::Arpeggio, S::Major, C, 1, P::Groups3, M::RightOnly, R::Eighths) }, original: None },
                Task::Tech { role: "Trill", spec: TechSpec { trill_fingers: (2, 3), ..t(X::Trill, S::Major, C, 1, P::Straight, M::RightOnly, R::Eighths) }, original: Some((BookId::NahreTrills, 1, "The full trills exercise")) },
                Task::Theory { id: "th-5", book: BookId::ElementsOfMusic, page: 70, title: "Scales and five-note scales (Elements pp. 70–71)", what: "Then look at the scale table at the back of Russian School 1a (p. 43).", reading: Some(2) },
                Task::Ear { id: "ear-5", title: "Three keys", what: "Play no. 79 'On the Lawn' in C, then in G and F by ear." },
            ],
        },
        Unit {
            title: "Dynamics and phrasing",
            book: BookId::RussianSchool1b, page: 8,
            pieces: (90..=99).collect(),
            russian: "Crescendo, diminuendo, the pause (fermata). In duets one person plays the tune, the other the accompaniment – swap roles.",
            nahre: "Shape each phrase: where is its peak? (Elements ch.4.) Scales get the same treatment – they swell and recede.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: TechSpec { accent: Accent::Swell, ..t(X::Scale, S::Major, C, 1, P::Circling, M::RightOnly, R::Eighths) }, original: Some((BookId::NahreScales, 2, "Circular Scales (full version)")) },
                Task::Tech { role: "Scale", spec: t(X::Scale, S::Major, D, 2, P::Straight, M::ParallelTwoOct, R::Eighths), original: Some((BookId::RussianSchool1b, 64, "Scales two octaves apart (explanatory notes)")) },
                Task::Tech { role: "Chords", spec: TechSpec { accent: Accent::Swell, ..t(X::TriadInversions, S::Major, D, 1, P::Straight, M::Parallel, R::Quarters) }, original: Some((BookId::RussianSchool1b, 64, "Chords pp → ff")) },
                Task::Tech { role: "Left-hand swell", spec: t(X::LhSwell, S::Major, C, 1, P::Straight, M::Parallel, R::Eighths), original: Some((BookId::NahreScales, 24, "Left Hand Swells (full version)")) },
                Task::Theory { id: "th-6", book: BookId::ElementsOfMusic, page: 61, title: "Melody: shape and breath (Elements pp. 61–69)", what: "Draw a curve over the melody of no. 93 'Lullaby' in the book view to show where it rises and falls.", reading: None },
                Task::Ear { id: "ear-6", title: "Duet roles", what: "For the four-hand pieces (94, 99): learn your part, then sing the other while you play. Once a page is made playable, the app can play the other part." },
            ],
        },
        Unit {
            title: "Grace notes and double notes",
            book: BookId::RussianSchool1b, page: 13,
            pieces: (100..=109).collect(),
            russian: "Grace notes go just before the beat (no. 103). Double notes need a level hand and both notes exactly together (Gnesina studies 107–108).",
            nahre: "Intervals (Elements pp. 28–30): see 3rds and 6ths as shapes instead of reading each note.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: t(X::Scale, S::Major, C, 1, P::Thirds, M::RightOnly, R::Eighths), original: None },
                Task::Tech { role: "Scale", spec: t(X::Scale, S::Major, F, 1, P::Straight, M::Parallel, R::Quarters), original: Some((BookId::RussianSchool1a, 44, "F major")) },
                Task::Tech { role: "Minor scale", spec: t(X::Scale, S::HarmonicMinor, E, 1, P::Straight, M::Parallel, R::Quarters), original: Some((BookId::RussianSchool1b, RS1B_SCALE_TABLE, "E minor")) },
                Task::Tech { role: "Trill", spec: TechSpec { trill_fingers: (1, 3), ..t(X::Trill, S::Major, F, 1, P::Straight, M::RightOnly, R::Eighths) }, original: Some((BookId::NahreTrills, 1, "Trill fingerings 3131, 2121")) },
                Task::Theory { id: "th-7", book: BookId::ElementsOfMusic, page: 28, title: "Intervals (Elements pp. 28–30)", what: "Then the interval stage of the reading trainer.", reading: Some(3) },
                Task::Ear { id: "ear-7", title: "Sing the other voice", what: "Play no. 106 'Russian Song' and sing the left-hand line while your right hand plays." },
            ],
        },
        Unit {
            title: "Studies and chords",
            book: BookId::RussianSchool1b, page: 17,
            pieces: (110..=119).collect(),
            russian: "Chords: lower the hand softly into the keys, 'to the bottom of the key', never throw it.",
            nahre: "Triads (Elements pp. 85–88): major, minor, diminished, augmented – and the I, IV, V chords of a key.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: t(X::TriadInversions, S::Major, G, 1, P::Groups3, M::RightOnly, R::Eighths), original: None },
                Task::Tech { role: "Scale", spec: t(X::Scale, S::Major, BB, 1, P::Straight, M::Parallel, R::Quarters), original: Some((BookId::RussianSchool1b, 67, "B-flat major")) },
                Task::Tech { role: "Chord colours", spec: TechSpec { chord: ChordType::Diminished, ..t(X::Arpeggio, S::Major, C, 1, P::Straight, M::RightOnly, R::Eighths) }, original: None },
                Task::Tech { role: "Cadence", spec: t(X::Cadence, S::Major, G, 1, P::Straight, M::Parallel, R::Quarters), original: None },
                Task::Theory { id: "th-8", book: BookId::ElementsOfMusic, page: 85, title: "Building triads; I, IV and V (Elements pp. 85–89)", what: "Play each chord type on C (try the arpeggio exercise with Major, Minor, Diminished, Augmented).", reading: None },
                Task::Ear { id: "ear-8", title: "Hum the top", what: "Play I–IV–V–I in C and hum the top voice. Then in G." },
            ],
        },
        Unit {
            title: "Dances",
            book: BookId::RussianSchool1b, page: 22,
            pieces: (120..=129).collect(),
            russian: "Each dance has its character: Telemann's Gavotte (122), the Valse (123), Papageno (126). Two-octave scales hands together.",
            nahre: "The same progression travelling through keys – a simplified 'turnaround chain'.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: TechSpec { progression: Progression::OneFourFiveOne, cycle: Cycle::RussianOrder, ..t(X::Progression, S::Major, C, 1, P::Straight, M::Parallel, R::Quarters) }, original: Some((BookId::NahreScales, 10, "Turnaround Chains (full version)")) },
                Task::Tech { role: "Scale", spec: t(X::Scale, S::Major, G, 2, P::Straight, M::Contrary, R::Eighths), original: None },
                Task::Tech { role: "Minor scale", spec: t(X::Scale, S::MelodicMinor, A, 1, P::Straight, M::Parallel, R::Quarters), original: Some((BookId::RussianSchool1b, RS1B_SCALE_TABLE, "Melodic minor")) },
                Task::Tech { role: "Arpeggio", spec: t(X::Arpeggio, S::Major, G, 2, P::Straight, M::RightOnly, R::Eighths), original: None },
                Task::Theory { id: "th-9", book: BookId::ElementsOfMusic, page: 90, title: "Inversions; tonic and dominant (Elements pp. 90–94)", what: "Find a I and a V chord in no. 122 Gavotte and mark them in the book view.", reading: None },
                Task::Builtin { id: "minuet-g", why: "A dance from the Anna Magdalena Notebook – playable now." },
            ],
        },
        Unit {
            title: "Triplets and modes",
            book: BookId::RussianSchool1b, page: 28,
            pieces: (130..=139).collect(),
            russian: "A triplet: three equal notes in one beat (no. 133). Bartók's 'Song' (137) uses a mode – that's what gives it its colour.",
            nahre: "Modes (Elements pp. 72–77): the same white notes, a different home note. Feel each one's colour.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: t(X::Scale, S::Dorian, D, 1, P::Circling, M::RightOnly, R::Triplets), original: Some((BookId::NahreScales, 26, "Dorian Swirls (full version)")) },
                Task::Tech { role: "Mode", spec: t(X::Scale, S::Mixolydian, G, 1, P::Straight, M::Parallel, R::Triplets), original: Some((BookId::ElementsOfMusic, 73, "The 7 major modes")) },
                Task::Tech { role: "Rhythm", spec: TechSpec { seed: 10, ..t(X::Rhythm, S::Major, C, 1, P::Straight, M::RightOnly, R::Triplets) }, original: None },
                Task::Tech { role: "Trill", spec: TechSpec { trill_fingers: (3, 4), ..t(X::Trill, S::Major, G, 1, P::Straight, M::RightOnly, R::Eighths) }, original: Some((BookId::NahreTrills, 1, "Trills with whole tone above")) },
                Task::Theory { id: "th-10", book: BookId::ElementsOfMusic, page: 72, title: "Modes, key signatures, tetrachords (Elements pp. 72–79)", what: "Play the seven modes starting on C (Scales & technique → Scale → choose the mode).", reading: None },
                Task::Ear { id: "ear-10", title: "Improvise in Dorian", what: "Hold a low D in the left hand (pedal) and improvise four bars on the white keys, always coming home to D. Then try Mixolydian on G." },
            ],
        },
        Unit {
            title: "Two voices – Baroque",
            book: BookId::RussianSchool1b, page: 34,
            pieces: (140..=149).collect(),
            russian: "Polyphony: each hand is its own singer. Silent finger change (no. 142). Minuet (L. Mozart), Bourrée (Krieger).",
            nahre: "Voice-leading (Elements pp. 95–97): move each voice the shortest way. Her ii–V–I warm-ups, simplified.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: TechSpec { progression: Progression::TwoFiveOne, ..t(X::Progression, S::Major, C, 1, P::Straight, M::Parallel, R::Quarters) }, original: Some((BookId::Nahre251, 1, "A – The Figure Eight (full version)")) },
                Task::Tech { role: "Scale", spec: t(X::Scale, S::Major, A, 2, P::Straight, M::ParallelTwoOct, R::Eighths), original: Some((BookId::RussianSchool1b, 66, "A major")) },
                Task::Tech { role: "Minor scale", spec: t(X::Scale, S::HarmonicMinor, B, 1, P::Straight, M::Parallel, R::Quarters), original: Some((BookId::RussianSchool1b, 66, "B minor")) },
                Task::Tech { role: "Lydian swell", spec: t(X::LhSwell, S::Lydian, F, 1, P::Straight, M::Parallel, R::Eighths), original: Some((BookId::NahreScales, 25, "BONUS: Lydian Swells")) },
                Task::Theory { id: "th-11", book: BookId::ElementsOfMusic, page: 95, title: "Voice-leading (Elements pp. 95–97)", what: "Then sing the left hand of no. 146 Bourrée while playing the right.", reading: None },
                Task::Builtin { id: "minuet-g", why: "Two-voice Baroque writing – compare it with the L. Mozart Minuet (144)." },
            ],
        },
        Unit {
            title: "Character pieces",
            book: BookId::RussianSchool1b, page: 44,
            pieces: (150..=163).collect(),
            russian: "Contrast and character: polka, march, mazurka, cradle song. Make each sound like what it is.",
            nahre: "Syncopation and swing (Elements pp. 55–58). Chopin 'seeds': the core shape of an etude, slowly.",
            tasks: vec![
                Task::Tech { role: "Warm-up", spec: TechSpec { seed: 1, ..t(X::EtudeSeed, S::Major, C, 2, P::Straight, M::Parallel, R::Sixteenths) }, original: Some((BookId::NahreChopin, 2, "Modified Chopin no. 1A (easier version)")) },
                Task::Tech { role: "Scales", spec: TechSpec { cycle: Cycle::Fifths, ..t(X::Scale, S::Major, C, 1, P::Straight, M::Parallel, R::Eighths) }, original: Some((BookId::RussianSchool1b, RS1B_SCALE_TABLE, "Scale tables")) },
                Task::Tech { role: "Trill", spec: TechSpec { trill_fingers: (2, 3), ..t(X::Trill, S::Major, C, 1, P::Straight, M::Parallel, R::Eighths) }, original: Some((BookId::NahreTrills, 1, "Trills over moving chords")) },
                Task::Tech { role: "Rhythm", spec: TechSpec { seed: 12, ..t(X::Rhythm, S::Major, C, 1, P::Straight, M::RightOnly, R::Swing) }, original: None },
                Task::Theory { id: "th-12", book: BookId::ElementsOfMusic, page: 55, title: "Syncopation, hemiola, swing, feel (Elements pp. 55–58)", what: "Find a syncopation in no. 152 Polka or 162 March.", reading: None },
                Task::Builtin { id: "musette-d", why: "Bach's Musette – compare with L. Mozart's Musette (157)." },
                Task::Ear { id: "ear-12", title: "In a different character", what: "Nahre Sol's favourite game: play no. 162 March soft and legato like a lullaby. What changes? What stays?" },
            ],
        },
    ]
}

pub fn book_piece_id(n: u32) -> String {
    format!("book-{n}")
}

pub fn current_unit(p: &Progress) -> usize {
    let us = units();
    us.iter()
        .position(|u| !u.pieces.iter().all(|n| p.done(&book_piece_id(*n))))
        .unwrap_or(us.len() - 1)
}

// ------------------------------------------------------------------ adaptive feedback

pub enum Feedback {
    TooHard,
    Nailed,
}

pub fn apply_feedback(pp: &mut PieceProgress, rungs: &[Rung], bars: usize, fb: Feedback) -> String {
    let rung = &rungs[pp.rung.min(rungs.len() - 1)];
    let tempo = pp.tempo_pct.unwrap_or(rung.tempo_pct);
    match fb {
        Feedback::TooHard => {
            pp.too_hard_count += 1;
            if rung.listen {
                return "Listen again – no rush. Follow the notes with your eyes, and sing along.".into();
            }
            if tempo > 40 {
                let t = tempo.saturating_sub(10).max(40);
                pp.tempo_pct = Some(t);
                return format!("Slowed to {t}%. Slow and steady builds it.");
            }
            let (a, b) = pp.section.unwrap_or((1, bars.max(1)));
            if b > a + 1 {
                let half = (b - a + 1) / 2;
                let nb = (a + half.max(2) - 1).min(b);
                pp.section = Some((a, nb));
                return format!("Smaller chunk: just bars {a}–{nb} for now (Russian School: learn it phrase by phrase).");
            }
            if pp.rung > 0 {
                pp.rung -= 1;
                pp.tempo_pct = None;
                pp.section = None;
                return format!("Stepping back to '{}' to rebuild it.", rungs[pp.rung].label);
            }
            "This one may be too big a step right now. Go back to the previous piece in the book, or try an easier version (Arrange panel).".into()
        }
        Feedback::Nailed => {
            pp.nailed_count += 1;
            if let Some((a, b)) = pp.section {
                if b < bars {
                    let len = b - a + 1;
                    let na = b + 1;
                    let nb = (b + len).min(bars);
                    pp.section = Some((na, nb));
                    return format!("Next chunk: bars {na}–{nb}.");
                }
                pp.section = None;
                return "All chunks done – now join them up: whole piece at this step.".into();
            }
            if tempo < rung.tempo_pct {
                let t = (tempo + 10).min(rung.tempo_pct);
                pp.tempo_pct = Some(t);
                return format!("Good. Tempo up to {t}%.");
            }
            if pp.rung + 1 < rungs.len() {
                pp.rung += 1;
                pp.tempo_pct = None;
                return format!("Step up: '{}'.", rungs[pp.rung].label);
            }
            pp.done = true;
            "Mastered! Keep it in your fingers – play it once a week.".into()
        }
    }
}

// ---- dates without extra crates ----
pub fn today_num() -> i64 {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    secs / 86_400
}

pub fn day_string(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dd = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, dd)
}

pub fn today() -> String {
    day_string(today_num())
}
