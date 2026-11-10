//! Technique generator: any scale/mode, 1-4 octaves, many patterns, rhythms, accents,
//! hand arrangements and key cycles – with automatic fingering.
//!
//! Exercise families follow the Russian School scale tables (scales, tonic triads with
//! inversions, short arpeggios, triplet sequences, contrary motion) and simplified versions of
//! Nahre Sol's exercises (circling scales, turnaround chains, left-hand swells, measured trills,
//! ii-V-I warm-ups, Chopin "seed" figures).

use crate::model::{Hand, Note, Origin, Piece, FLAT_NAMES, SHARP_NAMES};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScaleType {
    Major,
    NaturalMinor,
    HarmonicMinor,
    MelodicMinor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Locrian,
    MajorPentachord,
    MinorPentachord,
    MajorPentatonic,
    MinorPentatonic,
    Blues,
    WholeTone,
    Chromatic,
}

impl ScaleType {
    pub const ALL: [ScaleType; 16] = [
        ScaleType::Major, ScaleType::NaturalMinor, ScaleType::HarmonicMinor, ScaleType::MelodicMinor,
        ScaleType::Dorian, ScaleType::Phrygian, ScaleType::Lydian, ScaleType::Mixolydian, ScaleType::Locrian,
        ScaleType::MajorPentachord, ScaleType::MinorPentachord, ScaleType::MajorPentatonic, ScaleType::MinorPentatonic,
        ScaleType::Blues, ScaleType::WholeTone, ScaleType::Chromatic,
    ];
    pub fn label(self) -> &'static str {
        match self {
            ScaleType::Major => "Major (Ionian)",
            ScaleType::NaturalMinor => "Natural minor (Aeolian)",
            ScaleType::HarmonicMinor => "Harmonic minor",
            ScaleType::MelodicMinor => "Melodic minor (up raised, down natural)",
            ScaleType::Dorian => "Dorian",
            ScaleType::Phrygian => "Phrygian",
            ScaleType::Lydian => "Lydian",
            ScaleType::Mixolydian => "Mixolydian",
            ScaleType::Locrian => "Locrian",
            ScaleType::MajorPentachord => "5-note major (pentachord)",
            ScaleType::MinorPentachord => "5-note minor (pentachord)",
            ScaleType::MajorPentatonic => "Major pentatonic",
            ScaleType::MinorPentatonic => "Minor pentatonic",
            ScaleType::Blues => "Blues",
            ScaleType::WholeTone => "Whole tone",
            ScaleType::Chromatic => "Chromatic",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            ScaleType::Major => "major",
            ScaleType::NaturalMinor => "natural minor",
            ScaleType::HarmonicMinor => "harmonic minor",
            ScaleType::MelodicMinor => "melodic minor",
            ScaleType::Dorian => "Dorian",
            ScaleType::Phrygian => "Phrygian",
            ScaleType::Lydian => "Lydian",
            ScaleType::Mixolydian => "Mixolydian",
            ScaleType::Locrian => "Locrian",
            ScaleType::MajorPentachord => "major five-note",
            ScaleType::MinorPentachord => "minor five-note",
            ScaleType::MajorPentatonic => "major pentatonic",
            ScaleType::MinorPentatonic => "minor pentatonic",
            ScaleType::Blues => "blues",
            ScaleType::WholeTone => "whole-tone",
            ScaleType::Chromatic => "chromatic",
        }
    }
    /// Ascending intervals within one octave (without the octave).
    pub fn up(self) -> Vec<i32> {
        match self {
            ScaleType::Major => vec![0, 2, 4, 5, 7, 9, 11],
            ScaleType::NaturalMinor => vec![0, 2, 3, 5, 7, 8, 10],
            ScaleType::HarmonicMinor => vec![0, 2, 3, 5, 7, 8, 11],
            ScaleType::MelodicMinor => vec![0, 2, 3, 5, 7, 9, 11],
            ScaleType::Dorian => vec![0, 2, 3, 5, 7, 9, 10],
            ScaleType::Phrygian => vec![0, 1, 3, 5, 7, 8, 10],
            ScaleType::Lydian => vec![0, 2, 4, 6, 7, 9, 11],
            ScaleType::Mixolydian => vec![0, 2, 4, 5, 7, 9, 10],
            ScaleType::Locrian => vec![0, 1, 3, 5, 6, 8, 10],
            ScaleType::MajorPentachord => vec![0, 2, 4, 5, 7],
            ScaleType::MinorPentachord => vec![0, 2, 3, 5, 7],
            ScaleType::MajorPentatonic => vec![0, 2, 4, 7, 9],
            ScaleType::MinorPentatonic => vec![0, 3, 5, 7, 10],
            ScaleType::Blues => vec![0, 3, 5, 6, 7, 10],
            ScaleType::WholeTone => vec![0, 2, 4, 6, 8, 10],
            ScaleType::Chromatic => (0..12).collect(),
        }
    }
    fn down(self) -> Vec<i32> {
        if self == ScaleType::MelodicMinor { ScaleType::NaturalMinor.up() } else { self.up() }
    }
    /// Pentachords don't repeat by octave – they're a five-finger position.
    pub fn is_position(self) -> bool {
        matches!(self, ScaleType::MajorPentachord | ScaleType::MinorPentachord)
    }
    /// Semitones from the tonic up to the major key that gives the key signature.
    fn parent_major(self) -> i32 {
        match self {
            ScaleType::Major | ScaleType::MajorPentachord | ScaleType::MajorPentatonic | ScaleType::WholeTone | ScaleType::Chromatic => 0,
            ScaleType::NaturalMinor | ScaleType::HarmonicMinor | ScaleType::MelodicMinor | ScaleType::MinorPentachord
            | ScaleType::MinorPentatonic | ScaleType::Blues => 3,
            ScaleType::Dorian => 10,
            ScaleType::Phrygian => 8,
            ScaleType::Lydian => 7,
            ScaleType::Mixolydian => 5,
            ScaleType::Locrian => 1,
        }
    }
    pub fn is_minorish(self) -> bool {
        self.parent_major() == 3
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exercise {
    FiveFinger,
    Scale,
    Arpeggio,
    TriadInversions,
    Cadence,
    Progression,
    Trill,
    LhSwell,
    EtudeSeed,
    Rhythm,
}

impl Exercise {
    pub const ALL: [Exercise; 10] = [
        Exercise::FiveFinger, Exercise::Scale, Exercise::Arpeggio, Exercise::TriadInversions, Exercise::Cadence,
        Exercise::Progression, Exercise::Trill, Exercise::LhSwell, Exercise::EtudeSeed, Exercise::Rhythm,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Exercise::FiveFinger => "Five-finger position",
            Exercise::Scale => "Scale",
            Exercise::Arpeggio => "Arpeggio",
            Exercise::TriadInversions => "Tonic triad + inversions",
            Exercise::Cadence => "Cadence (I–IV–I–V–I)",
            Exercise::Progression => "Chord progression / ii–V–I",
            Exercise::Trill => "Measured trill",
            Exercise::LhSwell => "Left-hand swell",
            Exercise::EtudeSeed => "Etude seed (Chopin idea, simplified)",
            Exercise::Rhythm => "Rhythm reading",
        }
    }
    pub fn tip(self) -> &'static str {
        match self {
            Exercise::FiveFinger => "Russian School: start non-legato, dropping arm weight into each key; then legato. Nahre Sol: say the note names, feel the 'geography' of the position.",
            Exercise::Scale => "Russian School: hands separately first, one octave then two; together in contrary motion before parallel. Thumb passes early and smoothly. Even sound.",
            Exercise::Arpeggio => "Let the wrist travel sideways; the thumb prepares early. Russian School: short three-note groups before long arpeggios.",
            Exercise::TriadInversions => "Russian School table: lower the hand softly into each chord, 'immersing the fingers to the bottom of the key'. Try pp → ff chord by chord.",
            Exercise::Cadence => "Listen to how each chord leads home. Top note sings, others softer (Nahre Sol, Elements ch.6: tonic and dominant).",
            Exercise::Progression => "Nahre Sol's warm-up idea, simplified: keep common notes still and move the others the shortest way (voice-leading). Then go round the keys.",
            Exercise::Trill => "Nahre Sol's trill exercise, simplified: measured trill – eighths, triplets, then sixteenths – loose, from the fingertips, while the other hand changes chords.",
            Exercise::LhSwell => "Nahre Sol's 'Left Hand Swells', simplified: the left hand runs the scale and swells up and back while the right hand holds the harmony.",
            Exercise::EtudeSeed => "Nahre Sol's 'Modified Chopin' idea taken further down: the core hand-shape of a Chopin etude, slow and small.",
            Exercise::Rhythm => "Nahre Sol, Elements ch.3: everything starts with rhythm. Clap or tap it while counting out loud (Russian School), then play it on one key.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pattern {
    Straight,
    Thirds,
    Groups3,
    Groups4,
    Circling,
    Turnaround,
    Fragments5,
}

impl Pattern {
    pub const ALL: [Pattern; 7] = [Pattern::Straight, Pattern::Thirds, Pattern::Groups3, Pattern::Groups4, Pattern::Circling, Pattern::Turnaround, Pattern::Fragments5];
    pub fn label(self) -> &'static str {
        match self {
            Pattern::Straight => "Straight up and down",
            Pattern::Thirds => "Broken thirds (1-3, 2-4…)",
            Pattern::Groups3 => "Groups of 3 (Russian triplet sequence)",
            Pattern::Groups4 => "Groups of 4 (1234, 2345…)",
            Pattern::Circling => "Circling (Nahre Sol style: 1231, 2342…)",
            Pattern::Turnaround => "Turnaround (123432, 234543…)",
            Pattern::Fragments5 => "Rising five-note fragments",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Motion {
    RightOnly,
    LeftOnly,
    Parallel,
    ParallelTwoOct,
    Tenths,
    Contrary,
}

impl Motion {
    pub const ALL: [Motion; 6] = [Motion::RightOnly, Motion::LeftOnly, Motion::Parallel, Motion::ParallelTwoOct, Motion::Tenths, Motion::Contrary];
    pub fn label(self) -> &'static str {
        match self {
            Motion::RightOnly => "Right hand",
            Motion::LeftOnly => "Left hand",
            Motion::Parallel => "Together, an octave apart",
            Motion::ParallelTwoOct => "Together, two octaves apart",
            Motion::Tenths => "Together, a third/tenth apart",
            Motion::Contrary => "Contrary motion (from the same note)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rhythm {
    Quarters,
    Eighths,
    Triplets,
    Sixteenths,
    Dotted,
    ReverseDotted,
    Swing,
}

impl Rhythm {
    pub const ALL: [Rhythm; 7] = [Rhythm::Quarters, Rhythm::Eighths, Rhythm::Triplets, Rhythm::Sixteenths, Rhythm::Dotted, Rhythm::ReverseDotted, Rhythm::Swing];
    pub fn label(self) -> &'static str {
        match self {
            Rhythm::Quarters => "Even quarters",
            Rhythm::Eighths => "Even eighths",
            Rhythm::Triplets => "Triplets",
            Rhythm::Sixteenths => "Sixteenths",
            Rhythm::Dotted => "Dotted (long–short)",
            Rhythm::ReverseDotted => "Reverse dotted (short–long)",
            Rhythm::Swing => "Swing",
        }
    }
    fn dur(self, i: usize) -> f64 {
        match self {
            Rhythm::Quarters => 1.0,
            Rhythm::Eighths => 0.5,
            Rhythm::Triplets => 1.0 / 3.0,
            Rhythm::Sixteenths => 0.25,
            Rhythm::Dotted => if i % 2 == 0 { 0.75 } else { 0.25 },
            Rhythm::ReverseDotted => if i % 2 == 0 { 0.25 } else { 0.75 },
            Rhythm::Swing => if i % 2 == 0 { 2.0 / 3.0 } else { 1.0 / 3.0 },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Accent {
    None,
    Every2,
    Every3,
    Every4,
    Swell,
}

impl Accent {
    pub const ALL: [Accent; 5] = [Accent::None, Accent::Every2, Accent::Every3, Accent::Every4, Accent::Swell];
    pub fn label(self) -> &'static str {
        match self {
            Accent::None => "Even",
            Accent::Every2 => "Accent every 2",
            Accent::Every3 => "Accent every 3",
            Accent::Every4 => "Accent every 4",
            Accent::Swell => "Crescendo up, diminuendo down",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Cycle {
    Single,
    RussianOrder,
    Fifths,
    Fourths,
    WholeTones,
    MinorThirds,
    Chromatic,
}

impl Cycle {
    pub const ALL: [Cycle; 7] = [Cycle::Single, Cycle::RussianOrder, Cycle::Fifths, Cycle::Fourths, Cycle::WholeTones, Cycle::MinorThirds, Cycle::Chromatic];
    pub fn label(self) -> &'static str {
        match self {
            Cycle::Single => "Just this key",
            Cycle::RussianOrder => "Next 4 keys up in fifths (Russian order)",
            Cycle::Fifths => "All 12 keys, up in fifths",
            Cycle::Fourths => "All 12 keys, up in fourths",
            Cycle::WholeTones => "Whole tones (6 keys – Nahre Sol chains)",
            Cycle::MinorThirds => "Minor thirds (4 keys – Nahre Sol chains)",
            Cycle::Chromatic => "All 12 keys, up in semitones",
        }
    }
    fn tonics(self, start: u8) -> Vec<u8> {
        let (step, n) = match self {
            Cycle::Single => (0, 1),
            Cycle::RussianOrder => (7, 5),
            Cycle::Fifths => (7, 12),
            Cycle::Fourths => (5, 12),
            Cycle::WholeTones => (2, 6),
            Cycle::MinorThirds => (3, 4),
            Cycle::Chromatic => (1, 12),
        };
        (0..n).map(|i| ((start as i32 + step * i).rem_euclid(12)) as u8).collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChordType {
    Major,
    Minor,
    Diminished,
    Augmented,
    Dominant7,
    Major7,
    Minor7,
    Diminished7,
}

impl ChordType {
    pub const ALL: [ChordType; 8] = [ChordType::Major, ChordType::Minor, ChordType::Diminished, ChordType::Augmented, ChordType::Dominant7, ChordType::Major7, ChordType::Minor7, ChordType::Diminished7];
    pub fn label(self) -> &'static str {
        match self {
            ChordType::Major => "Major triad",
            ChordType::Minor => "Minor triad",
            ChordType::Diminished => "Diminished triad",
            ChordType::Augmented => "Augmented triad",
            ChordType::Dominant7 => "Dominant 7th",
            ChordType::Major7 => "Major 7th",
            ChordType::Minor7 => "Minor 7th",
            ChordType::Diminished7 => "Diminished 7th",
        }
    }
    pub fn tones(self) -> Vec<i32> {
        match self {
            ChordType::Major => vec![0, 4, 7],
            ChordType::Minor => vec![0, 3, 7],
            ChordType::Diminished => vec![0, 3, 6],
            ChordType::Augmented => vec![0, 4, 8],
            ChordType::Dominant7 => vec![0, 4, 7, 10],
            ChordType::Major7 => vec![0, 4, 7, 11],
            ChordType::Minor7 => vec![0, 3, 7, 10],
            ChordType::Diminished7 => vec![0, 3, 6, 9],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Progression {
    TwoFiveOne,
    OneFourFiveOne,
    Turnaround,
    Pop,
}

impl Progression {
    pub const ALL: [Progression; 4] = [Progression::TwoFiveOne, Progression::OneFourFiveOne, Progression::Turnaround, Progression::Pop];
    pub fn label(self) -> &'static str {
        match self {
            Progression::TwoFiveOne => "ii – V – I",
            Progression::OneFourFiveOne => "I – IV – V – I",
            Progression::Turnaround => "I – vi – IV – V (turnaround)",
            Progression::Pop => "I – V – vi – IV",
        }
    }
    /// (root offset from tonic, chord quality) using diatonic triads of a major key
    fn chords(self) -> Vec<(i32, bool)> {
        // bool = minor
        match self {
            Progression::TwoFiveOne => vec![(2, true), (7, false), (0, false), (0, false)],
            Progression::OneFourFiveOne => vec![(0, false), (5, false), (7, false), (0, false)],
            Progression::Turnaround => vec![(0, false), (9, true), (5, false), (7, false)],
            Progression::Pop => vec![(0, false), (7, false), (9, true), (5, false)],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TechSpec {
    pub ex: Exercise,
    pub scale: ScaleType,
    pub tonic: u8,
    pub octaves: u8,
    pub pattern: Pattern,
    pub motion: Motion,
    pub rhythm: Rhythm,
    pub accent: Accent,
    pub cycle: Cycle,
    pub chord: ChordType,
    pub progression: Progression,
    pub sevenths: bool,
    pub trill_fingers: (u8, u8),
    pub trill_half_step: bool,
    pub seed: u32,
}

impl Default for TechSpec {
    fn default() -> Self {
        Self {
            ex: Exercise::Scale,
            scale: ScaleType::Major,
            tonic: 0,
            octaves: 1,
            pattern: Pattern::Straight,
            motion: Motion::RightOnly,
            rhythm: Rhythm::Quarters,
            accent: Accent::None,
            cycle: Cycle::Single,
            chord: ChordType::Major,
            progression: Progression::OneFourFiveOne,
            sevenths: false,
            trill_fingers: (2, 3),
            trill_half_step: false,
            seed: 1,
        }
    }
}

impl TechSpec {
    pub fn id(&self) -> String {
        let s = format!("{self:?}");
        let mut h: u64 = 1469598103934665603;
        for b in s.bytes() {
            h = (h ^ b as u64).wrapping_mul(1099511628211);
        }
        format!("tech-{h:016x}")
    }
    pub fn title(&self) -> String {
        let key = pc_name(self.tonic, sig_for(self.tonic, self.scale));
        let cyc = if self.cycle == Cycle::Single { String::new() } else { format!(" · {}", short_cycle(self.cycle)) };
        match self.ex {
            Exercise::Scale => format!("{key} {} scale{cyc}", self.scale.short()),
            Exercise::FiveFinger => format!("{key} {} five-finger{cyc}", if self.scale.is_minorish() { "minor" } else { "major" }),
            Exercise::Arpeggio => format!("{key} {} arpeggio{cyc}", self.chord.label().to_lowercase()),
            Exercise::TriadInversions => format!("{key} {} triad + inversions{cyc}", if self.scale.is_minorish() { "minor" } else { "major" }),
            Exercise::Cadence => format!("{key} cadence I–IV–I–V–I{cyc}"),
            Exercise::Progression => format!("{} in {key}{cyc}", self.progression.label()),
            Exercise::Trill => format!("Measured trill {}-{} in {key}{cyc}", self.trill_fingers.0, self.trill_fingers.1),
            Exercise::LhSwell => format!("Left-hand swell – {key} {}{cyc}", self.scale.short()),
            Exercise::EtudeSeed => format!("{} – {key}", seed_name(self.seed)),
            Exercise::Rhythm => format!("Rhythm reading – {} (no. {})", self.rhythm.label().to_lowercase(), self.seed),
        }
    }
    pub fn subtitle(&self) -> String {
        let mut parts = vec![self.motion.label().to_string()];
        if matches!(self.ex, Exercise::Scale | Exercise::Arpeggio | Exercise::LhSwell) {
            parts.push(format!("{} oct", self.octaves));
        }
        if matches!(self.ex, Exercise::Scale | Exercise::Arpeggio | Exercise::FiveFinger) && self.pattern != Pattern::Straight {
            parts.push(self.pattern.label().split(" (").next().unwrap_or("").to_lowercase());
        }
        if self.rhythm != Rhythm::Quarters {
            parts.push(self.rhythm.label().to_lowercase());
        }
        if self.accent != Accent::None {
            parts.push(self.accent.label().to_lowercase());
        }
        parts.join(" · ")
    }
}

fn short_cycle(c: Cycle) -> &'static str {
    match c {
        Cycle::Single => "",
        Cycle::RussianOrder => "5 keys in fifths",
        Cycle::Fifths => "12 keys in fifths",
        Cycle::Fourths => "12 keys in fourths",
        Cycle::WholeTones => "whole-tone chain",
        Cycle::MinorThirds => "minor-third chain",
        Cycle::Chromatic => "12 keys chromatic",
    }
}

pub fn seed_name(seed: u32) -> &'static str {
    match seed % 3 {
        1 => "Wide arpeggio seed (Chopin Op. 10 No. 1 idea)",
        2 => "Chromatic outer-finger seed (Chopin Op. 10 No. 2 idea)",
        _ => "Black-key seed (Chopin Op. 10 No. 5 idea)",
    }
}

/// Key signature (fifths) for a major key with tonic `pc`, choosing the common spelling.
pub fn major_sig(pc: u8) -> i32 {
    [0, -5, 2, -3, 4, -1, -6, 1, -4, 3, -2, 5][(pc % 12) as usize]
}

pub fn sig_for(tonic: u8, scale: ScaleType) -> i32 {
    let mut f = major_sig(((tonic as i32 + scale.parent_major()) % 12) as u8);
    // F# major / D# minor read better as sharps when the tonic itself is a sharp key
    if f == -6 && matches!(tonic, 6 | 3) && scale.is_minorish() {
        f = 6;
    }
    f
}

pub fn pc_name(pc: u8, fifths: i32) -> &'static str {
    if fifths < 0 { FLAT_NAMES[(pc % 12) as usize] } else { SHARP_NAMES[(pc % 12) as usize] }
}

// ------------------------------------------------------------------ fingering

fn black(p: u8) -> bool {
    crate::model::is_black(p)
}

/// Automatic fingering for a single line using dynamic programming over finger choices.
pub fn finger_line(pitches: &[u8], hand: Hand) -> Vec<u8> {
    let n = pitches.len();
    if n == 0 {
        return vec![];
    }
    let rh = hand == Hand::Right;
    let note_cost = |i: usize, f: u8| -> f32 {
        let p = pitches[i];
        let mut c = 0.0;
        if f == 1 && black(p) {
            c += 6.0;
        }
        if f == 5 && black(p) {
            c += 1.5;
        }
        // finger 5 belongs at the outer edge of a run
        let next_up = i + 1 < n && pitches[i + 1] > p;
        let next_down = i + 1 < n && pitches[i + 1] < p;
        if f == 5 && ((rh && next_up) || (!rh && next_down)) {
            c += 4.0;
        }
        if i == 0 {
            let want = if rh { 1 } else { 5 };
            if f != want {
                c += if black(p) { 0.0 } else { 0.6 };
            }
        }
        if i == n - 1 && n > 2 {
            let want_top = if rh { 5 } else { 1 };
            let at_top = if rh { pitches[i] >= *pitches.iter().max().unwrap() } else { pitches[i] >= *pitches.iter().max().unwrap() };
            if at_top && f == want_top && !black(p) {
                c -= 0.5;
            }
        }
        c
    };
    let trans = |p1: u8, f1: u8, p2: u8, f2: u8| -> f32 {
        let d = p2 as i32 - p1 as i32;
        if d == 0 {
            return if f1 == f2 { 0.0 } else { 0.4 };
        }
        let up = d > 0;
        // in "finger order" space: RH fingers increase to the right; LH decrease
        let natural = if rh { (up && f2 > f1) || (!up && f2 < f1) } else { (up && f2 < f1) || (!up && f2 > f1) };
        let s = d.unsigned_abs() as f32;
        if natural {
            let g = (f2 as i32 - f1 as i32).unsigned_abs() as f32;
            let thumb = f1 == 1 || f2 == 1;
            let max_span = if thumb { 2.0 + 2.6 * g } else { 2.0 * g + 0.5 };
            let min_span = (g - 1.0).max(0.0);
            (s - max_span).max(0.0) * 1.2 + (min_span - s).max(0.0) * 0.7
        } else {
            // thumb passes under (towards the outside) or a finger crosses over the thumb
            let under = if rh { up && f2 == 1 && f1 > 1 } else { !up && f2 == 1 && f1 > 1 };
            let over = if rh { !up && f1 == 1 && f2 > 1 } else { up && f1 == 1 && f2 > 1 };
            let other = if under { f1 } else if over { f2 } else { return 99.0 };
            let base = match other {
                3 => 1.0,
                4 => 1.25,
                2 => 2.6,
                _ => 99.0,
            };
            // crossing onto a black key from a white thumb is comfortable
            base + (s - 5.0).max(0.0) * 0.8
        }
    };
    let mut cost = vec![[f32::INFINITY; 6]; n];
    let mut back = vec![[0u8; 6]; n];
    for f in 1..=5u8 {
        cost[0][f as usize] = note_cost(0, f);
    }
    for i in 1..n {
        for f2 in 1..=5u8 {
            let nc = note_cost(i, f2);
            for f1 in 1..=5u8 {
                let c = cost[i - 1][f1 as usize] + trans(pitches[i - 1], f1, pitches[i], f2) + nc;
                if c < cost[i][f2 as usize] {
                    cost[i][f2 as usize] = c;
                    back[i][f2 as usize] = f1;
                }
            }
        }
    }
    let mut f = (1..=5u8).min_by(|a, b| cost[n - 1][*a as usize].partial_cmp(&cost[n - 1][*b as usize]).unwrap()).unwrap();
    let mut out = vec![0u8; n];
    for i in (0..n).rev() {
        out[i] = f;
        if i > 0 {
            f = back[i][f as usize];
        }
    }
    out
}

/// Standard scale fingering for 7-note scales: the repeating groups 123-1234 (RH) /
/// 54321-321 (LH), rotated so thumbs fall on white keys and 4 on a black key where possible.
/// `idx` are scale-degree indices from the tonic (any sign), `pitches` the sounding notes.
pub fn periodic_fingers(idx: &[i32], pitches: &[i32], hand: Hand) -> Vec<u8> {
    let cycle: [u8; 7] = if hand == Hand::Right { [1, 2, 3, 1, 2, 3, 4] } else { [1, 4, 3, 2, 1, 3, 2] };
    // white-key tonics use the plain 5-4-3-2-1-3-2-1 shape in the left hand
    let tonic_black = idx.iter().zip(pitches).find(|(i, _)| i.rem_euclid(7) == 0).map(|(_, p)| black(*p as u8)).unwrap_or(false);
    let mut best = (f32::INFINITY, 0usize);
    for rot in 0..7usize {
        let mut seen = std::collections::HashSet::new();
        let mut score = rot as f32 * 0.01;
        if hand == Hand::Left && !tonic_black && rot != 0 {
            score += 0.3;
        }
        for (i, p) in idx.iter().zip(pitches) {
            let d = i.rem_euclid(7);
            if !seen.insert(d) {
                continue;
            }
            let f = cycle[((i + rot as i32).rem_euclid(7)) as usize];
            if f == 1 && black(*p as u8) {
                score += 10.0;
            }
            if (hand == Hand::Right || tonic_black) && f == 4 && black(*p as u8) {
                score -= 0.5;
            }
        }
        if score < best.0 {
            best = (score, rot);
        }
    }
    let rot = best.1 as i32;
    let f_of = |i: i32| cycle[((i + rot).rem_euclid(7)) as usize];
    let lo = *idx.iter().min().unwrap_or(&0);
    let hi = *idx.iter().max().unwrap_or(&0);
    idx.iter()
        .map(|&i| {
            let f = f_of(i);
            if hand == Hand::Right && i == hi && f == 1 && hi > lo {
                (f_of(i - 1) + 1).min(5)
            } else if hand == Hand::Left && i == lo && hi > lo {
                let next = f_of(i + 1);
                if next <= 4 { next + 1 } else { 5 }
            } else {
                f
            }
        })
        .collect()
}

/// Chromatic scale: thumb on white keys, 3 on black keys, 2 where two white keys meet.
fn chromatic_fingers(pitches: &[i32], hand: Hand) -> Vec<u8> {
    pitches
        .iter()
        .map(|&p| {
            let pc = p.rem_euclid(12);
            if black(p as u8) {
                3
            } else if (hand == Hand::Right && (pc == 5 || pc == 0)) || (hand == Hand::Left && (pc == 4 || pc == 11)) {
                2
            } else {
                1
            }
        })
        .collect()
}

// ------------------------------------------------------------------ building blocks

/// Pitch of scale index `i` (can be negative) from `base`.
fn scale_pitch(base: i32, up: &[i32], down: &[i32], i: i32, descending: bool) -> i32 {
    let n = up.len() as i32;
    let table = if descending { down } else { up };
    base + i.div_euclid(n) * 12 + table[i.rem_euclid(n) as usize]
}

/// Index sequence for a pattern over 0..=top. Returns (index, descending?) pairs.
fn pattern_indices(p: Pattern, top: i32) -> Vec<(i32, bool)> {
    let mut up: Vec<i32> = Vec::new();
    let mut down: Vec<i32> = Vec::new();
    match p {
        Pattern::Straight => {
            up = (0..=top).collect();
            down = (0..top).rev().collect();
        }
        Pattern::Thirds => {
            let mut s = 0;
            while s + 2 <= top {
                up.extend([s, s + 2]);
                s += 1;
            }
            up.push(top);
            let mut s = top;
            while s - 2 >= 0 {
                down.extend([s, s - 2]);
                s -= 1;
            }
            down.retain(|_| true);
            if down.first() == Some(&top) {
                down.remove(0);
            }
            down.push(0);
        }
        Pattern::Groups3 | Pattern::Groups4 => {
            let g = if p == Pattern::Groups3 { 3 } else { 4 };
            for s in 0..=(top - g + 1).max(0) {
                up.extend((0..g).map(|k| s + k));
            }
            for s in (g - 1..=top).rev() {
                down.extend((0..g).map(|k| s - k));
            }
            down.push(0);
        }
        Pattern::Circling => {
            for s in 0..=(top - 2).max(0) {
                up.extend([s, s + 1, s + 2, s + 1]);
            }
            up.push(top);
            for s in (2..=top).rev() {
                down.extend([s, s - 1, s - 2, s - 1]);
            }
            down.push(0);
            if down.first() == Some(&top) {
                down.remove(0);
            }
        }
        Pattern::Turnaround => {
            for s in 0..=(top - 3).max(0) {
                up.extend([s, s + 1, s + 2, s + 3, s + 2, s + 1]);
            }
            up.push(top);
            for s in (3..=top).rev() {
                down.extend([s, s - 1, s - 2, s - 3, s - 2, s - 1]);
            }
            down.push(0);
            if down.first() == Some(&top) {
                down.remove(0);
            }
        }
        Pattern::Fragments5 => {
            for s in 0..=(top - 4).max(0) {
                up.extend([s, s + 1, s + 2, s + 3, s + 4, s + 3, s + 2, s + 1]);
            }
            up.push(top);
            down = (0..top).rev().collect();
        }
    }
    let mut out: Vec<(i32, bool)> = up.into_iter().map(|i| (i, false)).collect();
    out.extend(down.into_iter().map(|i| (i, true)));
    out
}

struct Builder {
    notes: Vec<Note>,
    t: f64,
    key_changes: Vec<(f64, i32)>,
}

impl Builder {
    /// Add a line for one hand starting at `t` with rhythm/accent; returns end time.
    fn line(&mut self, pitches: &[i32], hand: Hand, start: f64, rhythm: Rhythm, accent: Accent, fingers: Option<Vec<u8>>) -> f64 {
        let ps: Vec<u8> = pitches.iter().map(|p| (*p).clamp(21, 108) as u8).collect();
        let fing = fingers.unwrap_or_else(|| finger_line(&ps, hand));
        let mut t = start;
        let n = ps.len();
        let peak = ps.iter().enumerate().max_by_key(|(_, p)| **p).map(|(i, _)| i).unwrap_or(0);
        for (i, p) in ps.iter().enumerate() {
            let last = i == n - 1;
            let d = rhythm.dur(i);
            let vel = match accent {
                Accent::None => 0.7,
                Accent::Every2 => if i % 2 == 0 { 0.95 } else { 0.55 },
                Accent::Every3 => if i % 3 == 0 { 0.95 } else { 0.55 },
                Accent::Every4 => if i % 4 == 0 { 0.95 } else { 0.55 },
                Accent::Swell => {
                    let x = if i <= peak { i as f32 / peak.max(1) as f32 } else { 1.0 - (i - peak) as f32 / (n - 1 - peak).max(1) as f32 };
                    0.35 + 0.6 * x
                }
            };
            let dur = if last { d.max(1.0) } else { d };
            self.notes.push(Note { vel, ..Note::new(t, dur, *p, hand, fing.get(i).copied()) });
            t += d;
        }
        t + if n > 0 { (rhythm.dur(n - 1)).max(1.0) - rhythm.dur(n - 1) } else { 0.0 }
    }

    fn chord(&mut self, pitches: &[i32], hand: Hand, start: f64, dur: f64, fingers: &[u8], vel: f32) {
        for (j, p) in pitches.iter().enumerate() {
            self.notes.push(Note { vel, ..Note::new(start, dur, (*p).clamp(21, 108) as u8, hand, fingers.get(j).copied()) });
        }
    }
}

fn bar_ceil(t: f64, bar: f64) -> f64 {
    ((t - 1e-6) / bar).ceil() * bar
}

/// Lowest comfortable octave placement.
fn rh_base(pc: u8, octaves: u8) -> i32 {
    let mut b = 60 + pc as i32;
    if pc >= 7 {
        b -= 12;
    }
    if octaves >= 3 {
        b -= 12;
    }
    if b + 12 * octaves as i32 > 103 {
        b -= 12;
    }
    b
}

fn lh_base(pc: u8, octaves: u8) -> i32 {
    let mut b = 48 + pc as i32;
    if pc >= 7 {
        b -= 12;
    }
    if octaves >= 3 {
        b -= 12;
    }
    b.max(28)
}

/// Close-position voicing nearest to `prev` (sum of voice movement), within a comfortable range.
fn voice_lead(pcs: &[i32], prev: &[i32], lo: i32, hi: i32) -> Vec<i32> {
    let mut best: (i32, Vec<i32>) = (i32::MAX, vec![]);
    for root_oct in (lo / 12 - 1)..=(hi / 12 + 1) {
        for inv in 0..pcs.len() {
            let mut v: Vec<i32> = Vec::new();
            let mut last = i32::MIN;
            for k in 0..pcs.len() {
                let pc = pcs[(inv + k) % pcs.len()];
                let mut p = root_oct * 12 + pc;
                while p <= last {
                    p += 12;
                }
                v.push(p);
                last = p;
            }
            if v[0] < lo || *v.last().unwrap() > hi {
                continue;
            }
            let cost: i32 = if prev.is_empty() {
                (v[0] - 64).abs()
            } else {
                v.iter().zip(prev).map(|(a, b)| (a - b).abs()).sum()
            };
            if cost < best.0 {
                best = (cost, v);
            }
        }
    }
    best.1
}

// ------------------------------------------------------------------ main entry

pub fn generate(spec: &TechSpec) -> Piece {
    let mut b = Builder { notes: vec![], t: 0.0, key_changes: vec![] };
    let bar = 4.0;
    let ts = (4, 4);
    let tonics = spec.cycle.tonics(spec.tonic % 12);
    let first_sig = sig_for(tonics[0], spec.scale);
    for (k, &pc) in tonics.iter().enumerate() {
        let start = b.t;
        if k > 0 {
            b.key_changes.push((start, sig_for(pc, spec.scale)));
        }
        let end = segment(&mut b, spec, pc, start);
        b.t = bar_ceil(end, bar);
    }
    let mut p = Piece {
        id: spec.id(),
        title: spec.title(),
        composer: spec.subtitle(),
        notes_text: spec.ex.tip().into(),
        bpm: match spec.rhythm {
            Rhythm::Sixteenths => 60.0,
            Rhythm::Triplets => 66.0,
            Rhythm::Quarters => 80.0,
            _ => 72.0,
        },
        ts_num: ts.0,
        ts_den: ts.1,
        key_fifths: first_sig,
        minor: spec.scale.is_minorish(),
        pickup: 0.0,
        notes: std::mem::take(&mut b.notes),
        origin: Origin::Generated,
        key_changes: b.key_changes,
        src: None,
    };
    p.sort_notes();
    p
}

fn segment(b: &mut Builder, s: &TechSpec, pc: u8, t0: f64) -> f64 {
    let up = s.scale.up();
    let down = s.scale.down();
    let n = up.len() as i32;
    let minor = s.scale.is_minorish();
    match s.ex {
        Exercise::Scale | Exercise::FiveFinger | Exercise::Arpeggio => {
            let (u, d, top, oct) = match s.ex {
                Exercise::FiveFinger => {
                    let five: Vec<i32> = up.iter().take(5).cloned().collect();
                    let five = if five.len() < 5 { vec![0, 2, if minor { 3 } else { 4 }, 5, 7] } else { five };
                    // a five-finger position doesn't wrap by octaves; use a 12-periodic table that stays in place
                    (five.clone(), five, 4, 1u8)
                }
                Exercise::Arpeggio => {
                    let tones = s.chord.tones();
                    let len = tones.len() as i32;
                    (tones.clone(), tones, len * s.octaves as i32, s.octaves)
                }
                _ => {
                    if s.scale.is_position() {
                        (up.clone(), down.clone(), up.len() as i32 - 1, 1)
                    } else {
                        (up.clone(), down.clone(), n * s.octaves as i32, s.octaves)
                    }
                }
            };
            let idx = pattern_indices(s.pattern, top);
            let five_finger = s.ex == Exercise::FiveFinger || (s.ex == Exercise::Scale && s.scale.is_position());
            let pitch = |base: i32, i: i32, desc: bool| -> i32 {
                if five_finger {
                    // positions: index clamps within 0..=4 and mirrors below 0 (contrary)
                    if i >= 0 {
                        base + u[(i as usize).min(u.len() - 1)]
                    } else {
                        let full = if minor { ScaleType::NaturalMinor.up() } else { ScaleType::Major.up() };
                        scale_pitch(base, &full, &full, i.max(-4), desc)
                    }
                } else {
                    scale_pitch(base, &u, &d, i, desc)
                }
            };
            let rb = rh_base(pc, oct);
            let lb = lh_base(pc, oct);
            let periodic = s.ex == Exercise::Scale && s.pattern == Pattern::Straight && u.len() == 7 && !five_finger;
            let chromatic = s.ex == Exercise::Scale && s.scale == ScaleType::Chromatic;
            let fixed_five = |hand: Hand, idxs: &[(i32, bool)]| -> Option<Vec<u8>> {
                if periodic || chromatic {
                    // filled in later from the actual pitches
                    let _ = (hand, idxs);
                    return None;
                }
                if !five_finger {
                    return None;
                }
                Some(idxs.iter().map(|(i, _)| {
                    let k = i.unsigned_abs().min(4) as u8;
                    if hand == Hand::Right { if *i >= 0 { k + 1 } else { k + 1 } } else { if *i >= 0 { 5 - k } else { k + 1 } }
                }).collect())
            };
            let scale_fing = |line: &[i32], idxs: Vec<i32>, hand: Hand| -> Option<Vec<u8>> {
                if periodic {
                    Some(periodic_fingers(&idxs, line, hand))
                } else if chromatic {
                    Some(chromatic_fingers(line, hand))
                } else {
                    None
                }
            };
            let plain: Vec<i32> = idx.iter().map(|(i, _)| *i).collect();
            let rh_line: Vec<i32>;
            let lh_line: Vec<i32>;
            match s.motion {
                Motion::RightOnly => {
                    rh_line = idx.iter().map(|(i, de)| pitch(rb, *i, *de)).collect();
                    let f = scale_fing(&rh_line, plain.clone(), Hand::Right).or(fixed_five(Hand::Right, &idx));
                    return b.line(&rh_line, Hand::Right, t0, s.rhythm, s.accent, f);
                }
                Motion::LeftOnly => {
                    lh_line = idx.iter().map(|(i, de)| pitch(lb, *i, *de)).collect();
                    let f = scale_fing(&lh_line, plain.clone(), Hand::Left).or(fixed_five(Hand::Left, &idx));
                    return b.line(&lh_line, Hand::Left, t0, s.rhythm, s.accent, f);
                }
                Motion::Parallel | Motion::ParallelTwoOct => {
                    let off = if s.motion == Motion::Parallel { 12 } else { 24 };
                    let rb2 = if s.motion == Motion::ParallelTwoOct && oct <= 2 { rb + 12 } else { rb };
                    rh_line = idx.iter().map(|(i, de)| pitch(rb2, *i, *de)).collect();
                    lh_line = idx.iter().map(|(i, de)| pitch(rb2 - off, *i, *de)).collect();
                }
                Motion::Tenths => {
                    let shift = if s.ex == Exercise::Arpeggio { 1 } else { 2 };
                    rh_line = idx.iter().map(|(i, de)| pitch(rb, *i + shift, *de)).collect();
                    lh_line = idx.iter().map(|(i, de)| pitch(rb - 12, *i, *de)).collect();
                }
                Motion::Contrary => {
                    // both hands start on the same note (the tonic nearest middle C)
                    let start = if pc <= 6 { 60 + pc as i32 } else { 48 + pc as i32 };
                    rh_line = idx.iter().map(|(i, de)| pitch(start, *i, *de)).collect();
                    lh_line = idx.iter().map(|(i, de)| pitch(start, -*i, !*de)).collect();
                    let lf = if five_finger { Some(idx.iter().map(|(i, _)| i.unsigned_abs().min(4) as u8 + 1).collect()) } else { scale_fing(&lh_line, plain.iter().map(|i| -i).collect(), Hand::Left) };
                    let rf = scale_fing(&rh_line, plain.clone(), Hand::Right).or(fixed_five(Hand::Right, &idx));
                    let e1 = b.line(&rh_line, Hand::Right, t0, s.rhythm, s.accent, rf);
                    let e2 = b.line(&lh_line, Hand::Left, t0, s.rhythm, s.accent, lf);
                    return e1.max(e2);
                }
            }
            let rshift = if s.motion == Motion::Tenths { 2 } else { 0 };
            let rf = scale_fing(&rh_line, plain.iter().map(|i| i + rshift).collect(), Hand::Right).or(fixed_five(Hand::Right, &idx));
            let lf = scale_fing(&lh_line, plain.clone(), Hand::Left).or(fixed_five(Hand::Left, &idx));
            let e1 = b.line(&rh_line, Hand::Right, t0, s.rhythm, s.accent, rf);
            let e2 = b.line(&lh_line, Hand::Left, t0, s.rhythm, s.accent, lf);
            e1.max(e2)
        }
        Exercise::TriadInversions => {
            let third = if minor { 3 } else { 4 };
            let shapes: [[i32; 3]; 4] = [[0, third, 7], [third, 7, 12], [7, 12, 12 + third], [12, 12 + third, 19]];
            let rf: [[u8; 3]; 4] = [[1, 3, 5], [1, 2, 5], [1, 3, 5], [1, 3, 5]];
            let lf: [[u8; 3]; 4] = [[5, 3, 1], [5, 3, 1], [5, 2, 1], [5, 3, 1]];
            let order = [0usize, 1, 2, 3, 2, 1, 0];
            let broken = s.pattern != Pattern::Straight;
            let mut t = t0;
            for (step, &k) in order.iter().enumerate() {
                let vel = match s.accent {
                    Accent::Swell => 0.35 + 0.6 * (1.0 - (step as f32 - 3.0).abs() / 3.0),
                    _ => 0.75,
                };
                let dur = if step == order.len() - 1 { 4.0 } else { 2.0 };
                let rb = rh_base(pc, 1);
                let lb = rb - 12;
                let hands: Vec<Hand> = match s.motion {
                    Motion::RightOnly => vec![Hand::Right],
                    Motion::LeftOnly => vec![Hand::Left],
                    _ => vec![Hand::Right, Hand::Left],
                };
                for h in hands {
                    let base = if h == Hand::Right { rb } else { lb };
                    let ps: Vec<i32> = shapes[k].iter().map(|iv| base + iv).collect();
                    let f = if h == Hand::Right { rf[k] } else { lf[k] };
                    if broken {
                        for (j, p) in ps.iter().enumerate() {
                            b.notes.push(Note { vel, ..Note::new(t + j as f64 * 0.5, if j == 2 { dur - 1.0 } else { 0.5 }, *p as u8, h, Some(f[j])) });
                        }
                    } else {
                        b.chord(&ps, h, t, dur, &f, vel);
                    }
                }
                t += dur;
            }
            t
        }
        Exercise::Cadence | Exercise::Progression => {
            let chords: Vec<(i32, Vec<i32>)> = if s.ex == Exercise::Cadence {
                let third = if minor { 3 } else { 4 };
                let iv = if minor { vec![5, 8, 12] } else { vec![5, 9, 12] };
                vec![(0, vec![0, third, 7]), (5, iv.clone()), (0, vec![0, third, 7]), (7, vec![7, 11, 14]), (0, vec![0, third, 7])]
                    .into_iter()
                    .map(|(r, c)| (r, c.into_iter().map(|x| x % 12).collect()))
                    .collect()
            } else {
                s.progression
                    .chords()
                    .into_iter()
                    .map(|(r, m)| {
                        let mut c = vec![r, r + if m { 3 } else { 4 }, r + 7];
                        if s.sevenths {
                            // dominant 7 on V, minor 7 on ii/vi, major 7 on I/IV
                            let sev = if r == 7 { 10 } else if m { 10 } else { 11 };
                            c.push(r + sev);
                            c.remove(0); // rootless: left hand has the root
                        }
                        (r, c.into_iter().map(|x| x.rem_euclid(12)).collect())
                    })
                    .collect()
            };
            let mut prev: Vec<i32> = vec![];
            let mut t = t0;
            let n_ch = chords.len();
            for (i, (root, pcs)) in chords.iter().enumerate() {
                let abs: Vec<i32> = pcs.iter().map(|x| (x + pc as i32) % 12).collect();
                let v = voice_lead(&abs, &prev, 57, 79);
                let dur = if s.ex == Exercise::Cadence { if i == n_ch - 1 { 4.0 } else { 2.0 } } else { 4.0 };
                if s.motion != Motion::LeftOnly {
                    let fingers: Vec<u8> = match v.len() { 3 => vec![1, 3, 5], _ => vec![1, 2, 3, 5] };
                    let vel = 0.7;
                    // arpeggiated version for rhythm practice
                    if s.rhythm == Rhythm::Quarters || s.ex == Exercise::Cadence {
                        b.chord(&v, Hand::Right, t, dur, &fingers, vel);
                    } else {
                        let mut tt = t;
                        let mut i2 = 0;
                        while tt < t + dur - 1e-6 {
                            let d = s.rhythm.dur(i2);
                            let k = [0usize, 1, 2, 1][i2 % 4].min(v.len() - 1);
                            b.notes.push(Note { vel, ..Note::new(tt, d, v[k] as u8, Hand::Right, Some(fingers[k])) });
                            tt += d;
                            i2 += 1;
                        }
                    }
                }
                if s.motion != Motion::RightOnly {
                    let mut r = 36 + ((root + pc as i32) % 12);
                    if r < 40 {
                        r += 12;
                    }
                    b.chord(&[r], Hand::Left, t, dur, &[5], 0.65);
                    if s.motion == Motion::Parallel || s.motion == Motion::ParallelTwoOct {
                        b.chord(&[r + 7], Hand::Left, t + dur / 2.0, dur / 2.0, &[1], 0.5);
                    }
                }
                prev = v;
                t += dur;
            }
            t
        }
        Exercise::Trill => {
            let (fa, fb) = s.trill_fingers;
            let step = if s.trill_half_step { 1 } else { 2 };
            let rh = s.motion != Motion::LeftOnly;
            let lo = if rh { rh_base(pc, 1) + 7 } else { lh_base(pc, 1) + 7 };
            let hi = lo + step;
            let hand = if rh { Hand::Right } else { Hand::Left };
            let (f_lo, f_hi) = if rh { (fa.min(fb), fa.max(fb)) } else { (fa.max(fb), fa.min(fb)) };
            let mut t = t0;
            // measured: 1 bar eighths, 1 bar triplets, 1 bar sixteenths, then a held note
            for (dur, count) in [(0.5, 8), (1.0 / 3.0, 12), (0.25, 16)] {
                for i in 0..count {
                    let (p, f) = if i % 2 == 0 { (hi, f_hi) } else { (lo, f_lo) };
                    let vel = if i % (if dur < 0.3 { 4 } else if dur < 0.4 { 3 } else { 2 }) == 0 { 0.85 } else { 0.55 };
                    b.notes.push(Note { vel, ..Note::new(t, dur, p as u8, hand, Some(f)) });
                    t += dur;
                }
            }
            b.notes.push(Note::new(t, 4.0, lo as u8, hand, Some(f_lo)));
            // other hand changes chords (Nahre Sol: trill over moving harmony)
            if matches!(s.motion, Motion::Parallel | Motion::ParallelTwoOct | Motion::Tenths | Motion::Contrary) {
                let third = if minor { 3 } else { 4 };
                let base = lh_base(pc, 1);
                let prog: [[i32; 3]; 4] = [[0, third, 7], [0, 5, 9], [-1, 2, 7], [0, third, 7]];
                for (i, c) in prog.iter().enumerate() {
                    let ps: Vec<i32> = c.iter().map(|x| base + x).collect();
                    b.chord(&ps, Hand::Left, t0 + i as f64 * 4.0, 4.0, &[5, 3, 1], 0.5);
                }
            }
            t + 4.0
        }
        Exercise::LhSwell => {
            let lb = lh_base(pc, s.octaves);
            let top = n * s.octaves as i32;
            let idx = pattern_indices(Pattern::Straight, top);
            let line: Vec<i32> = idx.iter().map(|(i, de)| scale_pitch(lb, &up, &down, *i, *de)).collect();
            let rhythm = if s.rhythm == Rhythm::Quarters { Rhythm::Eighths } else { s.rhythm };
            let fing = if up.len() == 7 { Some(periodic_fingers(&idx.iter().map(|(i, _)| *i).collect::<Vec<_>>(), &line, Hand::Left)) } else { None };
            let end = b.line(&line, Hand::Left, t0, rhythm, Accent::Swell, fing);
            if s.motion != Motion::LeftOnly {
                // RH holds the tonic triad, then the dominant, in half/whole notes
                let rb = rh_base(pc, 1) + 12;
                let third = up.get(2).copied().unwrap_or(4);
                let fifth = up.iter().copied().find(|x| *x == 7 || *x == 6 || *x == 8).unwrap_or(7);
                let len = end - t0;
                let half = (len / 2.0 / 2.0).floor() * 2.0;
                b.chord(&[rb, rb + third, rb + fifth], Hand::Right, t0, half.max(2.0), &[1, 3, 5], 0.55);
                b.chord(&[rb - 1, rb + 2, rb + 7], Hand::Right, t0 + half.max(2.0), (len - half).max(2.0), &[1, 2, 5], 0.55);
            }
            end
        }
        Exercise::EtudeSeed => match s.seed % 3 {
            1 => {
                // wide broken chord 1-2-4-5 over two octaves, chord changes per bar
                let third = if minor { 3 } else { 4 };
                let mut t = t0;
                let rb = rh_base(pc, 2);
                for (k, root) in [0, 5, 7, 0].iter().enumerate() {
                    let shape = if *root == 7 { [-1, 2, 7, 11] } else if *root == 5 { [0, 5, 9, 12] } else { [0, third, 7, 12] };
                    let mut line = vec![];
                    for o in 0..2 {
                        for x in shape {
                            line.push(rb + x + o * 12);
                        }
                    }
                    let top = line.clone();
                    line.extend(top.iter().rev().skip(1).cloned());
                    let fingers: Vec<u8> = (0..line.len()).map(|i| {
                        let pos = if i < 8 { i % 4 } else { 3 - ((i - 8 + 1) % 4) };
                        [1, 2, 4, 5][pos]
                    }).collect();
                    let e = b.line(&line, Hand::Right, t, Rhythm::Sixteenths, Accent::Every4, Some(fingers));
                    let bass = lh_base(pc, 1) + [0, 5, 7, 0][k] - if [0, 5, 7, 0][k] > 5 { 12 } else { 0 };
                    if s.motion != Motion::RightOnly {
                        b.chord(&[bass, bass + 12], Hand::Left, t, e - t, &[5, 1], 0.6);
                    }
                    t = bar_ceil(e, 4.0);
                    let _ = k;
                }
                t
            }
            2 => {
                // chromatic line with 3-4-5 over held lower notes (1-2)
                let rb = rh_base(pc, 1) + 7;
                let mut line: Vec<i32> = (0..=12).map(|i| rb + i).collect();
                line.extend((0..12).rev().map(|i| rb + i));
                let fingers: Vec<u8> = line.iter().map(|p| if black(*p as u8) { 4 } else { 3 }).collect();
                let e = b.line(&line, Hand::Right, t0, Rhythm::Sixteenths, Accent::Every4, Some(fingers));
                let rbl = rh_base(pc, 1);
                let mut tt = t0;
                while tt < e - 1e-6 {
                    b.chord(&[rbl, rbl + if minor { 3 } else { 4 }], Hand::Right, tt, 1.0, &[1, 2], 0.5);
                    tt += 1.0;
                }
                if s.motion != Motion::RightOnly {
                    let lb = lh_base(pc, 1);
                    b.chord(&[lb, lb + 7], Hand::Left, t0, e - t0, &[5, 1], 0.5);
                }
                e
            }
            _ => {
                // black-key pentatonic in triplets (on G-flat if the tonic is C/G/…, otherwise the chosen tonic)
                let base = 54 + 12; // Gb4
                let pent = [0, 2, 4, 7, 9];
                let idx = pattern_indices(Pattern::Groups3, 10);
                let line: Vec<i32> = idx.iter().map(|(i, _)| base + (i / 5) * 12 + pent[(*i % 5) as usize]).collect();
                let e = b.line(&line, Hand::Right, t0, Rhythm::Triplets, Accent::Every3, None);
                if s.motion != Motion::RightOnly {
                    let lb = 42; // Gb2
                    let mut tt = t0;
                    let mut k = 0;
                    while tt < e - 1e-6 {
                        let c: [i32; 3] = if k % 2 == 0 { [lb, lb + 7, lb + 16] } else { [lb + 7, lb + 11, lb + 14] };
                        b.chord(&c, Hand::Left, tt, 2.0, &[5, 2, 1], 0.5);
                        tt += 2.0;
                        k += 1;
                    }
                }
                e
            }
        },
        Exercise::Rhythm => {
            // deterministic pseudo-random 4-bar rhythm on the tonic (RH) built from allowed cells
            let cells: Vec<Vec<f64>> = match s.rhythm {
                Rhythm::Quarters => vec![vec![1.0], vec![1.0], vec![2.0], vec![1.0, 1.0]],
                Rhythm::Eighths => vec![vec![1.0], vec![0.5, 0.5], vec![2.0], vec![0.5, 0.5, 1.0]],
                Rhythm::Dotted | Rhythm::ReverseDotted => vec![vec![1.5, 0.5], vec![1.0], vec![0.5, 0.5], vec![0.75, 0.25], vec![2.0]],
                Rhythm::Sixteenths => vec![vec![0.25, 0.25, 0.25, 0.25], vec![0.5, 0.25, 0.25], vec![0.25, 0.25, 0.5], vec![1.0], vec![0.5, 0.5]],
                Rhythm::Triplets => vec![vec![1.0 / 3.0; 3], vec![1.0], vec![0.5, 0.5], vec![2.0]],
                Rhythm::Swing => vec![vec![2.0 / 3.0, 1.0 / 3.0], vec![1.0], vec![2.0]],
            };
            let mut rng = s.seed.wrapping_mul(2654435761).wrapping_add(pc as u32 * 97 + 1);
            let mut next = || {
                rng ^= rng << 13;
                rng ^= rng >> 17;
                rng ^= rng << 5;
                rng
            };
            let p = rh_base(pc, 1);
            let mut t = t0;
            for bar_i in 0..4 {
                let bar_end = t0 + (bar_i + 1) as f64 * 4.0;
                while t < bar_end - 1e-6 {
                    let cell = &cells[(next() as usize) % cells.len()];
                    let len: f64 = cell.iter().sum();
                    let cell: Vec<f64> = if t + len > bar_end + 1e-6 { vec![bar_end - t] } else { cell.clone() };
                    for (j, d) in cell.iter().enumerate() {
                        let on_beat = ((t - t0) % 1.0).abs() < 1e-6;
                        let vel = if on_beat && j == 0 { 0.85 } else { 0.6 };
                        b.notes.push(Note { vel, ..Note::new(t, *d, p as u8, Hand::Right, Some(3)) });
                        t += d;
                    }
                }
            }
            t
        }
    }
}
