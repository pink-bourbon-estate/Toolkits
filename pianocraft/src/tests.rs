use crate::builtin;
use crate::curriculum::{apply_feedback, ladder, Feedback, PieceProgress};
use crate::generate::*;
use crate::model::Hand;
use crate::transform::*;

#[test]
fn builtins_have_whole_bars() {
    for p in builtin::all() {
        for h in [Hand::Right, Hand::Left] {
            let end = p.notes.iter().filter(|n| n.hand == h).map(|n| n.start + n.dur).fold(0.0, f64::max);
            let bars = end / p.bar_len();
            assert!((bars - bars.round()).abs() < 1e-6, "{} {:?} ends mid-bar: {bars}", p.id, h);
        }
        let r = p.notes.iter().filter(|n| n.hand == Hand::Right).map(|n| n.start + n.dur).fold(0.0, f64::max);
        let l = p.notes.iter().filter(|n| n.hand == Hand::Left).map(|n| n.start + n.dur).fold(0.0, f64::max);
        assert!((r - l).abs() < 1e-6 || p.id == "frere-jacques", "{} hands differ: {r} vs {l}", p.id);
        println!("{:<14} bars={:>3} diff={:.1}", p.id, p.bar_count(), rate(&p).score);
    }
}

#[test]
fn musette_bar_13_tie() {
    let p = builtin::all().into_iter().find(|p| p.id == "musette-d").unwrap();
    // bar 13 starts at 24 beats; D5 (74) at 25.5 lasting 1.0 (tied)
    let n = p.notes.iter().find(|n| (n.start - 25.5).abs() < 1e-9 && n.hand == Hand::Right).unwrap();
    assert_eq!(n.pitch, 74);
    assert!((n.dur - 1.0).abs() < 1e-9);
    assert_eq!(p.bar_count(), 28);
}

#[test]
fn arrangements_and_ladder() {
    let p = builtin::all().into_iter().find(|p| p.id == "minuet-g").unwrap();
    let full = rate(&p).score;
    let easy = arrange(&p, &Arrangement { rh: RhMode::Skeleton, lh: LhMode::Off, transpose: 0 });
    assert!(rate(&easy).score < full);
    assert!(easy.notes.iter().all(|n| n.hand == Hand::Right));
    let roots = arrange(&p, &Arrangement { rh: RhMode::Off, lh: LhMode::Roots, transpose: 0 });
    assert_eq!(roots.notes.len(), 16, "one bass note per 3/4 bar");
    let t = arrange(&p, &Arrangement { transpose: -7, ..Default::default() });
    assert_eq!(t.key_fifths, 0, "G down a fifth = C");
    let rungs = ladder(&p, false);
    let mut pp = PieceProgress::default();
    for _ in 0..30 {
        apply_feedback(&mut pp, &rungs, 16, Feedback::Nailed);
    }
    assert!(pp.done);
    let mut pp = PieceProgress { rung: 3, ..Default::default() };
    let mut msgs = vec![];
    for _ in 0..8 {
        msgs.push(apply_feedback(&mut pp, &rungs, 16, Feedback::TooHard));
    }
    println!("{msgs:#?}");
    assert!(pp.rung < 3);
}

#[test]
fn technique_generates_everything() {
    let mut count = 0;
    for ex in Exercise::ALL {
        for scale in [ScaleType::Major, ScaleType::HarmonicMinor, ScaleType::Dorian, ScaleType::Blues, ScaleType::Chromatic, ScaleType::MajorPentachord] {
            for motion in Motion::ALL {
                for oct in [1u8, 4] {
                    for pattern in [Pattern::Straight, Pattern::Circling, Pattern::Thirds] {
                        let spec = TechSpec { ex, scale, tonic: 7, octaves: oct, pattern, motion, cycle: Cycle::MinorThirds, ..TechSpec::default() };
                        let p = generate(&spec);
                        assert!(!p.notes.is_empty(), "{spec:?}");
                        assert!(p.notes.iter().all(|n| n.dur > 0.0 && (21..=108).contains(&n.pitch)), "{spec:?}");
                        count += 1;
                    }
                }
            }
        }
    }
    println!("generated {count} exercises");
}

#[test]
fn scale_fingerings_match_the_tables() {
    let f = |tonic: u8, hand: Hand| {
        let spec = TechSpec { tonic, octaves: 1, motion: if hand == Hand::Right { Motion::RightOnly } else { Motion::LeftOnly }, ..TechSpec::default() };
        let p = generate(&spec);
        p.notes.iter().take(8).map(|n| n.finger.unwrap()).collect::<Vec<u8>>()
    };
    assert_eq!(f(0, Hand::Right), vec![1, 2, 3, 1, 2, 3, 4, 5], "C RH");
    assert_eq!(f(0, Hand::Left), vec![5, 4, 3, 2, 1, 3, 2, 1], "C LH");
    assert_eq!(f(7, Hand::Right), vec![1, 2, 3, 1, 2, 3, 4, 5], "G RH");
    assert_eq!(f(2, Hand::Right), vec![1, 2, 3, 1, 2, 3, 4, 5], "D RH");
    assert_eq!(f(5, Hand::Right), vec![1, 2, 3, 4, 1, 2, 3, 4], "F RH");
    assert_eq!(f(10, Hand::Right), vec![4, 1, 2, 3, 1, 2, 3, 4], "Bb RH");
    let c = generate(&TechSpec { motion: Motion::RightOnly, ..TechSpec::default() });
    let ps: Vec<u8> = c.notes.iter().map(|n| n.pitch).collect();
    assert_eq!(&ps[..8], &[60, 62, 64, 65, 67, 69, 71, 72]);
}

#[test]
fn contrary_motion_mirrors() {
    let p = generate(&TechSpec { motion: Motion::Contrary, ..TechSpec::default() });
    let lh: Vec<u8> = p.notes.iter().filter(|n| n.hand == Hand::Left).take(5).map(|n| n.pitch).collect();
    assert_eq!(lh, vec![60, 59, 57, 55, 53]);
}

#[test]
fn key_cycles_change_key() {
    let p = generate(&TechSpec { cycle: Cycle::Fifths, ..TechSpec::default() });
    assert_eq!(p.key_changes.len(), 11);
    assert_eq!(p.key_changes[0].1, 1);
}

#[test]
fn books_and_units_are_consistent() {
    for u in crate::curriculum::units() {
        for n in &u.pieces {
            assert!(crate::books::piece_ref(*n).is_some(), "missing piece {n}");
        }
    }
    assert!(crate::books::BookId::RussianSchool1a.matches("The Russian School of Piano Playing - Part 1.pdf"));
    assert!(crate::books::BookId::RussianSchool1b.matches("Russian School Volume 1b.pdf"));
    assert!(crate::books::BookId::NahreScales.matches("nsg-scales-modes-exercises.pdf"));
    assert!(crate::books::BookId::ElementsOfMusic.matches("nahre-sols-elements-of-music.pdf"));
    assert!(!crate::books::BookId::RussianSchool1a.matches("Russian School Volume 1b (Nikolaev).pdf"));
}

#[test]
fn musicxml_import() {
    let xml = r#"<?xml version="1.0"?><score-partwise><work><work-title>Test</work-title></work><part-list/><part id="P1">
<measure number="1"><attributes><divisions>2</divisions><key><fifths>1</fifths></key><time><beats>3</beats><beat-type>4</beat-type></time><staves>2</staves></attributes>
<note><pitch><step>D</step><octave>5</octave></pitch><duration>2</duration><staff>1</staff></note>
<note><pitch><step>G</step><octave>4</octave></pitch><duration>1</duration><staff>1</staff></note>
<note><pitch><step>A</step><octave>4</octave></pitch><duration>1</duration><staff>1</staff></note>
<note><pitch><step>B</step><octave>4</octave></pitch><duration>2</duration><staff>1</staff><tie type="start"/></note>
<backup><duration>6</duration></backup>
<note><pitch><step>G</step><octave>3</octave></pitch><duration>4</duration><staff>2</staff></note>
<note><chord/><pitch><step>B</step><octave>3</octave></pitch><duration>4</duration><staff>2</staff></note>
<note><pitch><step>F</step><alter>1</alter><octave>3</octave></pitch><duration>2</duration><staff>2</staff></note>
</measure>
<measure number="2"><note><pitch><step>B</step><octave>4</octave></pitch><duration>6</duration><staff>1</staff><tie type="stop"/></note>
<backup><duration>6</duration></backup><note><rest/><duration>6</duration><staff>2</staff></note></measure></part></score-partwise>"#;
    let p = crate::import::import_musicxml(xml, "x").unwrap();
    assert_eq!(p.title, "Test");
    assert_eq!(p.key_fifths, 1);
    assert_eq!((p.ts_num, p.ts_den), (3, 4));
    let rh: Vec<_> = p.notes.iter().filter(|n| n.hand == Hand::Right).collect();
    assert_eq!(rh.len(), 4);
    assert!((rh[3].dur - 4.0).abs() < 1e-9, "tie merged: {}", rh[3].dur);
    let fs = p.notes.iter().find(|n| n.pitch == 54).unwrap();
    assert!((fs.start - 2.0).abs() < 1e-9);
    assert_eq!(p.notes.iter().filter(|n| n.hand == Hand::Left).count(), 3);
}

#[test]
fn midi_import_roundtrip() {
    use midly::{num::*, Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
    let mk = |delta: u32, kind| TrackEvent { delta: u28::new(delta), kind };
    let on = |k: u8| TrackEventKind::Midi { channel: u4::new(0), message: MidiMessage::NoteOn { key: u7::new(k), vel: u7::new(80) } };
    let off = |k: u8| TrackEventKind::Midi { channel: u4::new(0), message: MidiMessage::NoteOff { key: u7::new(k), vel: u7::new(0) } };
    let t1 = vec![mk(0, TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000)))), mk(0, on(72)), mk(480, off(72)), mk(0, on(76)), mk(480, off(76)), mk(0, TrackEventKind::Meta(MetaMessage::EndOfTrack))];
    let t2 = vec![mk(0, on(48)), mk(960, off(48)), mk(0, TrackEventKind::Meta(MetaMessage::EndOfTrack))];
    let smf = Smf { header: Header::new(Format::Parallel, Timing::Metrical(u15::new(480))), tracks: vec![t1, t2] };
    let mut buf = Vec::new();
    smf.write_std(&mut buf).unwrap();
    let p = crate::import::import_midi(&buf, "t").unwrap();
    assert_eq!(p.bpm.round(), 120.0);
    assert_eq!(p.notes.iter().filter(|n| n.hand == Hand::Right).count(), 2);
    assert_eq!(p.notes.iter().filter(|n| n.hand == Hand::Left).count(), 1);
    assert_eq!(easiest_transposition(&p), 0);
}

#[test]
fn spelling() {
    use crate::ui::staff::spell;
    assert_eq!(spell(59, 1), (6, 3, 0)); // B3 in G
    assert_eq!(spell(66, 1), (3, 4, 1)); // F#4 in G
    assert_eq!(spell(70, -1), (6, 4, -1)); // Bb4 in F
    assert_eq!(spell(61, 2), (0, 4, 1)); // C#4 in D
    assert_eq!(spell(63, 2), (2, 4, -1).min((1, 4, 1)).max((1, 4, 1))); // D#4 (sharp side)
    assert_eq!(spell(60, 0), (0, 4, 0));
}

#[test]
fn print_fingerings() {
    for tonic in 0..12u8 {
        for hand in [Motion::RightOnly, Motion::LeftOnly] {
            let p = generate(&TechSpec { tonic, octaves: 2, motion: hand, ..TechSpec::default() });
            let f: String = p.notes.iter().take(15).map(|n| (b'0' + n.finger.unwrap()) as char).collect();
            println!("{:>2} {:?} {}", tonic, hand, f);
        }
    }
}
