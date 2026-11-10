//! Built-in public-domain pieces. Folk tunes have simple left-hand
//! arrangements written for this app; the Bach/Petzold pieces are the originals.

use crate::model::{Hand, Origin, Piece};
use crate::notation::parse_line;

struct Def {
    id: &'static str,
    title: &'static str,
    composer: &'static str,
    notes_text: &'static str,
    bpm: f64,
    ts: (u32, u32),
    key: i32,
    minor: bool,
    rh: &'static str,
    lh: &'static str,
}

fn build(d: &Def) -> Piece {
    let (mut notes, _) = parse_line(Hand::Right, d.rh, 0.0).unwrap_or_else(|e| panic!("{}: RH {e}", d.id));
    let (lh, _) = parse_line(Hand::Left, d.lh, 0.0).unwrap_or_else(|e| panic!("{}: LH {e}", d.id));
    notes.extend(lh);
    let mut p = Piece {
        id: d.id.into(),
        title: d.title.into(),
        composer: d.composer.into(),
        notes_text: d.notes_text.into(),
        bpm: d.bpm,
        ts_num: d.ts.0,
        ts_den: d.ts.1,
        key_fifths: d.key,
        minor: d.minor,
        pickup: 0.0,
        notes,
        origin: Origin::BuiltIn,
        key_changes: vec![],
        src: None,
    };
    p.sort_notes();
    p
}

const AU_CLAIR_A: &str = "C4:q/1 C4:q C4:q D4:q/2 | E4:h/3 D4:h/2 | C4:q/1 E4:q/3 D4:q/2 D4:q | C4:w/1 |";
const AU_CLAIR_LA: &str = "C3:w/5 | C3:h G2:h | C3:h G2:h | C3:w |";

const ODE_A: &str = "E4:q/3 E4:q F4:q/4 G4:q/5 | G4:q F4:q E4:q D4:q/2 | C4:q/1 C4:q D4:q E4:q | ";
const ODE_END1: &str = "E4:q. D4:e D4:h | ";
const ODE_END2: &str = "D4:q. C4:e C4:h | ";
const ODE_MID: &str = "D4:q/2 D4:q E4:q C4:q/1 | D4:q E4:e F4:e E4:q C4:q | D4:q E4:e F4:e E4:q D4:q | C4:q/1 D4:q/2 G3:h/1 | ";
const ODE_LH_A: &str = "C3:h/5 G3:h/1 | G2:h/5 D3:h/2 | C3:h G3:h | ";

const KORO_A: &str = "E5:q/5 B4:e/2 C5:e/3 D5:q/4 C5:e/3 B4:e/2 | A4:q/1 A4:e C5:e/3 E5:q/5 D5:e/4 C5:e/3 | B4:q./2 C5:e/3 D5:q/4 E5:q/5 | C5:q/3 A4:q/1 A4:h | ";
const KORO_B: &str = "r:e D5:q/2 F5:e/3 A5:q/5 G5:e/4 F5:e/3 | E5:q./2 C5:e/1 E5:q/2 D5:e/1 C5:e | B4:q/2 B4:e C5:e/3 D5:q/4 E5:q/5 | C5:q/3 A4:q/1 A4:h | ";
const KORO_LH_A: &str = "A2:q/5 E3:q/1 A2:q E3:q | A2:q E3:q A2:q E3:q | E2:q/5 B2:q/1 E2:q B2:q | A2:q/5 E3:q/1 A2:h | ";
const KORO_LH_B: &str = "D3:q/5 A3:q/1 D3:q A3:q | C3:q/5 G3:q/1 C3:q G3:q | E2:q/5 B2:q/1 E2:q B2:q | A2:q E3:q A2:h | ";

const FJ: &str = "C4:q/1 D4:q/2 E4:q/3 C4:q/1 | C4:q D4:q E4:q C4:q | E4:q/3 F4:q/4 G4:h/5 | E4:q F4:q G4:h | G4:e/5 A4:e G4:e F4:e/4 E4:q/3 C4:q/1 | G4:e A4:e G4:e F4:e E4:q C4:q | C4:q/1 G3:q/1 C4:h/1 | C4:q G3:q C4:h | ";
const FJ_LH: &str = "r:w | r:w | C3:q/5 D3:q/4 E3:q/3 C3:q/5 | C3:q D3:q E3:q C3:q | E3:q/3 F3:q/2 G3:h/1 | E3:q F3:q G3:h | G3:e/1 A3:e G3:e F3:e/2 E3:q/3 C3:q/5 | G3:e A3:e G3:e F3:e E3:q C3:q | C3:q/5 G2:q C3:h | C3:q G2:q C3:h | ";

const MIN_G_RH: &str = "D5:q/5 G4:e A4:e B4:e C5:e | D5:q G4:q/1 G4:q/2 | E5:q/4 C5:e/1 D5:e/2 E5:e/3 F#5:e/4 | G5:q/5 G4:q/1 G4:q/2 | \
C5:q/4 D5:e C5:e B4:e A4:e | B4:q/3 C5:e B4:e A4:e G4:e/1 | F#4:q/2 G4:e/1 A4:e B4:e G4:e | A4:h./2 | \
D5:q/5 G4:e A4:e B4:e C5:e | D5:q G4:q/1 G4:q/2 | E5:q/4 C5:e/1 D5:e/2 E5:e/3 F#5:e/4 | G5:q/5 G4:q/1 G4:q/2 | \
C5:q/4 D5:e C5:e B4:e A4:e | B4:q/3 C5:e B4:e A4:e G4:e/1 | A4:q/2 B4:e/3 A4:e G4:e/1 F#4:e/2 | G4:h./3 |";
const MIN_G_LH: &str = "G3+B3+D4:h/5 A3:q | B3:h. | C4:h. | B3:h. | A3:h. | G3:h. | D4:q/1 B3:q G3:q | D4:q D3:e/5 C4:e/1 B3:e A3:e | \
B3:h/2 A3:q | G3:q/4 B3:q G3:q | C4:h. | B3:q/2 C4:e B3:e A3:e G3:e/4 | A3:h/2 F#3:q | G3:h B3:q/1 | C4:q/2 D4:q/1 D3:q/5 | G3:h/1 G2:q |";

const MUS_A_RH: &str = "A5:q/5 G5:s/4 F#5:s/3 E5:s/2 D5:s/1 | A5:q/5 G5:s/4 F#5:s/3 E5:s/2 D5:s/1 | F#4:s/2 G4:s/3 A4:e/4 G4:e/3 F#4:e/2 | E4:e/1 A4:e/4 F#4:e/2 D4:e/1 | \
A5:q/5 G5:s F#5:s E5:s D5:s | A5:q G5:s F#5:s E5:s D5:s | F#4:s/2 G4:s A4:e G4:e F#4:e | E4:e/1 A4:e/4 D4:q/1 | ";
const MUS_A_LH: &str = "D2:e/5 D3:e/1 D2:e D3:e | D2:e D3:e D2:e D3:e | F#3:s/3 G3:s/2 A3:e/1 G3:e/2 F#3:e/3 | E3:e/4 A3:e/1 F#3:e/3 D3:e/5 | \
D2:e D3:e D2:e D3:e | D2:e D3:e D2:e D3:e | F#3:s/3 G3:s A3:e G3:e F#3:e | E3:e/4 A3:e/1 D3:q/5 | ";
const MUS_B_RH: &str = "C#5:s/2 D5:s/3 E5:e/4 C#5:s/2 D5:s/3 E5:e/4 | A5:e/5 E5:e/2 E5:q/2 | A5:e/5 E5:e/2 A5:e/5 E5:e/2 | D5:s/5 C#5:s/4 B4:s/3 A4:s/2 B4:e/3 E4:e/1 | \
E5:e/4 D#5:e/3 E4:e/1 D5:e~/3 | D5:e C#5:e/2 A5:e/5 G#5:e/4 | E5:e/4 D#5:e/3 E4:e/1 D5:e~/3 | D5:e C#5:e/2 A5:e/5 G#5:e/4 | \
E5:s/4 D#5:s/3 C#5:s/2 D#5:s/3 E5:s/4 D#5:s/3 C#5:s/2 D#5:s/3 | E5:e/5 G#4:e/1 A4:e/2 D5:e/5 | C#5:s/3 D5:s/4 E5:e/5 A4:e/2 D4:e/1 | C#4:s/2 D4:s/3 E4:e/4 A3:q/1 | ";
const MUS_B_LH: &str = "A2:e/5 A3:e/1 A2:e A3:e | A2:e A3:e A2:e A3:e | A2:e A3:e A2:e A3:e | A2:e A3:e E2:e E3:e | \
E2:e/5 E3:e/1 E2:e E3:e | E2:e E3:e E2:e E3:e | E2:e E3:e E2:e E3:e | E2:e E3:e E2:e E3:e | \
E2:e E3:e E2:e E3:e | E2:e/5 D3:e/2 C#3:e/3 D3:e/2 | E3:q/1 A2:e/5 D3:e/1 | C#3:s/3 D3:s/2 E3:e/1 A2:q/5 | ";

pub fn all() -> Vec<Piece> {
    let au_rh = format!("{AU_CLAIR_A}{AU_CLAIR_A}D4:q/2 D4:q D4:q D4:q | A3:h/1 A3:h | D4:q/4 C4:q/3 B3:q/2 A3:q/1 | G3:w/1 | {AU_CLAIR_A}");
    let au_lh = format!("{AU_CLAIR_LA}{AU_CLAIR_LA}G2:w | D3:w | D3:w | G2:w | {AU_CLAIR_LA}");
    let ode_rh = format!("{ODE_A}{ODE_END1}{ODE_A}{ODE_END2}{ODE_MID}{ODE_A}{ODE_END2}");
    let ode_lh = format!(
        "{ODE_LH_A}G2:h D3:h | {ODE_LH_A}G2:h C3:h | G2:h D3:h | C3:h G2:h | G2:h D3:h | C3:h G2:h | {ODE_LH_A}G2:h C3:h |"
    );
    let koro_rh = format!("{KORO_A}{KORO_B}");
    let koro_lh = format!("{KORO_LH_A}{KORO_LH_B}");
    let fj_rh = format!("{FJ}C4:w | C4:w |");
    let mus_rh = format!("{MUS_A_RH}{MUS_B_RH}{MUS_A_RH}");
    let mus_lh = format!("{MUS_A_LH}{MUS_B_LH}{MUS_A_LH}");

    let defs = [
        Def {
            id: "au-clair",
            title: "Au clair de la lune",
            composer: "French folk song (arr. PianoCraft)",
            notes_text: "Singing legato tune in five-finger C position. Sing it first, then play.",
            bpm: 84.0, ts: (4, 4), key: 0, minor: false, rh: Box::leak(au_rh.into_boxed_str()), lh: Box::leak(au_lh.into_boxed_str()),
        },
        Def {
            id: "ode-to-joy",
            title: "Ode to Joy",
            composer: "L. van Beethoven (arr. PianoCraft)",
            notes_text: "Theme from Symphony No. 9. Keep the RH thumb on middle C. Transpose to G and F once secure.",
            bpm: 96.0, ts: (4, 4), key: 0, minor: false, rh: Box::leak(ode_rh.into_boxed_str()), lh: Box::leak(ode_lh.into_boxed_str()),
        },
        Def {
            id: "frere-jacques",
            title: "Frère Jacques (canon)",
            composer: "French round (arr. PianoCraft)",
            notes_text: "Two-voice canon: the left hand copies the right two bars later. First step into polyphony.",
            bpm: 92.0, ts: (4, 4), key: 0, minor: false, rh: Box::leak(fj_rh.into_boxed_str()), lh: FJ_LH,
        },
        Def {
            id: "korobeiniki",
            title: "Korobeiniki (The Peddlers)",
            composer: "Russian folk song (arr. PianoCraft)",
            notes_text: "A minor. Crisp, bouncy LH quarters; RH shapes each phrase. Yes, it's the Tetris tune.",
            bpm: 120.0, ts: (4, 4), key: 0, minor: true, rh: Box::leak(koro_rh.into_boxed_str()), lh: Box::leak(koro_lh.into_boxed_str()),
        },
        Def {
            id: "minuet-g",
            title: "Minuet in G, BWV Anh. 114",
            composer: "Christian Petzold (Anna Magdalena Notebook)",
            notes_text: "First half (bars 1-16), original notes. Dance feel: lean on beat 1, light on 2 and 3.",
            bpm: 108.0, ts: (3, 4), key: 1, minor: false, rh: MIN_G_RH, lh: MIN_G_LH,
        },
        Def {
            id: "musette-d",
            title: "Musette in D, BWV Anh. 126",
            composer: "Anon./J.S. Bach (Anna Magdalena Notebook)",
            notes_text: "Original notes, repeats written out once (A B A). The LH octave 'drone' imitates bagpipes.",
            bpm: 92.0, ts: (2, 4), key: 2, minor: false, rh: Box::leak(mus_rh.into_boxed_str()), lh: Box::leak(mus_lh.into_boxed_str()),
        },
    ];
    defs.iter().map(build).collect()
}
