//! Note-reading trainer using Nahre Sol's reference-point method (Elements of Music ch.2):
//! learn a few landmark notes first, then read everything else as a step or skip from them.

use crate::model::{Hand, Note, Origin, Piece};

pub const LEVELS: [(&str, &str); 4] = [
    ("Landmarks", "Middle C, treble G, bass F, high C, low C – the reference points."),
    ("Next to landmarks", "Landmarks plus the notes one step either side."),
    ("Whole grand staff", "Every line and space note, treble and bass."),
    ("Ledger lines & sharps/flats", "Notes above and below the staves, and black keys."),
];

const LANDMARKS: [u8; 5] = [48, 53, 60, 67, 72]; // C3, F3, C4, G4, C5

pub fn pool(level: u8) -> Vec<u8> {
    let whites = |lo: u8, hi: u8| (lo..=hi).filter(|p| !crate::model::is_black(*p)).collect::<Vec<u8>>();
    match level {
        0 => LANDMARKS.to_vec(),
        1 => {
            let mut v = LANDMARKS.to_vec();
            for l in LANDMARKS {
                for d in [-2i32, -1, 1, 2] {
                    let p = (l as i32 + d) as u8;
                    if !crate::model::is_black(p) && !v.contains(&p) {
                        v.push(p);
                    }
                }
            }
            v
        }
        2 => whites(43, 77),                                                      // G2 .. F5
        _ => (36..=84).collect(),                                                 // C2 .. C6 incl. black keys
    }
}

pub fn nearest_landmark(p: u8) -> u8 {
    *LANDMARKS.iter().min_by_key(|l| (**l as i32 - p as i32).abs()).unwrap()
}

/// One-note "piece" so the normal staff renderer can draw the question.
pub fn question_piece(p: u8) -> Piece {
    let hand = if p >= 60 { Hand::Right } else { Hand::Left };
    Piece {
        id: "reading".into(),
        title: String::new(),
        composer: String::new(),
        notes_text: String::new(),
        bpm: 60.0,
        ts_num: 4,
        ts_den: 4,
        key_fifths: 0,
        minor: false,
        pickup: 0.0,
        notes: vec![Note::new(1.5, 1.0, p, hand, None)],
        origin: Origin::Generated,
        key_changes: vec![],
    }
}

pub struct Trainer {
    pub level: u8,
    pub current: u8,
    pub streak: u32,
    pub correct: u32,
    pub tries: u32,
    pub last: Option<(u8, bool)>,
    retry: Vec<u8>,
    rng: u64,
}

impl Trainer {
    pub fn new(level: u8) -> Trainer {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7) | 1;
        let mut t = Trainer { level, current: 60, streak: 0, correct: 0, tries: 0, last: None, retry: vec![], rng: seed };
        t.next();
        t
    }
    fn rand(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }
    pub fn next(&mut self) {
        let prev = self.current;
        // missed notes come back sooner
        if !self.retry.is_empty() && self.rand() % 3 == 0 {
            self.current = self.retry.remove(0);
            return;
        }
        let pool = pool(self.level);
        for _ in 0..10 {
            let p = pool[(self.rand() as usize) % pool.len()];
            if p != prev {
                self.current = p;
                return;
            }
        }
    }
    /// Returns true if correct.
    pub fn answer(&mut self, p: u8) -> bool {
        self.tries += 1;
        let ok = p == self.current;
        self.last = Some((p, ok));
        if ok {
            self.correct += 1;
            self.streak += 1;
            self.next();
        } else {
            self.streak = 0;
            if !self.retry.contains(&self.current) {
                self.retry.push(self.current);
            }
        }
        ok
    }
}
