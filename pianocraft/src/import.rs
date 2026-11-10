//! Import MIDI (.mid) and MusicXML (.musicxml / .xml / .mxl) into a Piece.

use crate::model::{Hand, Note, Origin, Piece};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

pub fn import_file(path: &Path) -> Result<Piece, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let bytes = std::fs::read(path).map_err(|e| format!("Can't read file: {e}"))?;
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Imported").replace(['_', '-'], " ");
    let mut p = match ext.as_str() {
        "mid" | "midi" => import_midi(&bytes, &stem)?,
        "musicxml" | "xml" => import_musicxml(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?, &stem)?,
        "mxl" => import_musicxml(&unzip_mxl(&bytes)?, &stem)?,
        "pdf" => return Err("PDF sheet music can't be read directly. Convert it to MusicXML first with the free program Audiveris (or MuseScore's PDF import), then import the .musicxml file here.".into()),
        _ => return Err(format!("Unsupported file type '.{ext}'. Use .mid, .musicxml, .xml or .mxl")),
    };
    p.origin = Origin::Imported(path.display().to_string());
    p.id = format!("import-{}", sanitize(&path.display().to_string()));
    if p.notes.is_empty() {
        return Err("No notes found in that file.".into());
    }
    p.sort_notes();
    Ok(p)
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect()
}

fn q(beats: f64) -> f64 {
    (beats * 12.0).round() / 12.0
}

pub fn import_midi(bytes: &[u8], title: &str) -> Result<Piece, String> {
    use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
    let smf = Smf::parse(bytes).map_err(|e| format!("Not a valid MIDI file: {e}"))?;
    let tpq = match smf.header.timing {
        Timing::Metrical(t) => t.as_int() as f64,
        Timing::Timecode(..) => return Err("SMPTE-timed MIDI files aren't supported".into()),
    };
    let mut bpm = None;
    let mut ts = None;
    let mut key = None;
    let mut name = None;
    // per track: notes
    let mut tracks: Vec<Vec<Note>> = Vec::new();
    for track in &smf.tracks {
        let mut t: u64 = 0;
        let mut open: HashMap<(u8, u8), (u64, u8)> = HashMap::new();
        let mut notes = Vec::new();
        for ev in track {
            t += ev.delta.as_int() as u64;
            match ev.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(us)) if bpm.is_none() => bpm = Some(60_000_000.0 / us.as_int() as f64),
                TrackEventKind::Meta(MetaMessage::TimeSignature(n, d, ..)) if ts.is_none() => ts = Some((n as u32, 1u32 << d)),
                TrackEventKind::Meta(MetaMessage::KeySignature(k, minor)) if key.is_none() => key = Some((k as i32, minor)),
                TrackEventKind::Meta(MetaMessage::TrackName(n)) if name.is_none() => {
                    let s = String::from_utf8_lossy(n).trim().to_string();
                    if s.len() > 2 && !s.to_lowercase().contains("piano") && !s.to_lowercase().starts_with("track") {
                        name = Some(s);
                    }
                }
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int();
                    if ch == 9 {
                        continue; // drums
                    }
                    let (on, k, vel) = match message {
                        MidiMessage::NoteOn { key, vel } => (vel.as_int() > 0, key.as_int(), vel.as_int()),
                        MidiMessage::NoteOff { key, .. } => (false, key.as_int(), 0),
                        _ => continue,
                    };
                    if on {
                        open.insert((ch, k), (t, vel));
                    } else if let Some((st, v0)) = open.remove(&(ch, k)) {
                        let start = q(st as f64 / tpq);
                        let dur = q((t - st) as f64 / tpq).max(1.0 / 12.0);
                        notes.push(Note { vel: (v0 as f32 / 127.0).clamp(0.2, 1.0), ..Note::new(start, dur, k, Hand::Right, None) });
                    }
                }
                _ => {}
            }
        }
        if !notes.is_empty() {
            tracks.push(notes);
        }
    }
    let mut all = Vec::new();
    if tracks.len() >= 2 {
        // highest-sounding track = right hand; others split by average pitch
        let avgs: Vec<f64> = tracks.iter().map(|t| t.iter().map(|n| n.pitch as f64).sum::<f64>() / t.len() as f64).collect();
        let top = avgs.iter().cloned().fold(f64::MIN, f64::max);
        for (t, avg) in tracks.into_iter().zip(avgs) {
            let hand = if avg >= top - 1e-9 || (avg >= 62.0 && top - avg < 7.0) { Hand::Right } else { Hand::Left };
            all.extend(t.into_iter().map(|mut n| {
                n.hand = hand;
                n
            }));
        }
    } else if let Some(t) = tracks.pop() {
        all.extend(t.into_iter().map(|mut n| {
            n.hand = if n.pitch >= 60 { Hand::Right } else { Hand::Left };
            n
        }));
    }
    // trim leading silence to the start of its bar
    let (ts_num, ts_den) = ts.unwrap_or((4, 4));
    let bar = ts_num as f64 * 4.0 / ts_den as f64;
    let first = all.iter().map(|n| n.start).fold(f64::MAX, f64::min);
    let shift = (first / bar).floor() * bar;
    for n in &mut all {
        n.start -= shift;
    }
    clean_durations(&mut all);
    let (k, mut minor) = key.unwrap_or_else(|| (guess_key(&all), false));
    if !minor {
        minor = ends_on_relative_minor(&all, k);
    }
    Ok(Piece {
        id: String::new(),
        title: name.unwrap_or_else(|| title_case(title)),
        composer: String::new(),
        notes_text: "Imported from MIDI. Hands were split automatically – check the Arrange panel if a hand looks wrong.".into(),
        bpm: bpm.unwrap_or(100.0).clamp(30.0, 240.0),
        ts_num,
        ts_den,
        key_fifths: k,
        minor,
        pickup: 0.0,
        notes: all,
        origin: Origin::BuiltIn,
        key_changes: vec![],
        src: None,
    })
}

/// MIDI exports often shorten notes (articulation). Snap them back to clean values
/// so the sheet view shows quarters as quarters.
pub fn clean_durations(notes: &mut [Note]) {
    for hand in [Hand::Right, Hand::Left] {
        let mut onsets: Vec<f64> = notes.iter().filter(|n| n.hand == hand).map(|n| n.start).collect();
        onsets.sort_by(|a, b| a.partial_cmp(b).unwrap());
        onsets.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        for n in notes.iter_mut().filter(|n| n.hand == hand) {
            let i = onsets.partition_point(|&t| t <= n.start + 1e-6);
            let end = n.start + n.dur;
            if let Some(&next) = onsets.get(i) {
                if (next - end) > -1e-6 && (next - end) < 0.34 {
                    n.dur = next - n.start;
                    continue;
                }
            }
            // otherwise round to the nearest 16th (or triplet 8th when closer)
            let r16 = (n.dur * 4.0).round() / 4.0;
            let r3 = (n.dur * 3.0).round() / 3.0;
            n.dur = if (r3 - n.dur).abs() + 1e-3 < (r16 - n.dur).abs() { r3 } else { r16 }.max(0.25);
        }
    }
}

/// Major key signature but the bass ends on the relative minor's tonic -> minor.
fn ends_on_relative_minor(notes: &[Note], fifths: i32) -> bool {
    let last_start = notes.iter().map(|n| n.start).fold(f64::MIN, f64::max);
    let lowest_last = notes.iter().filter(|n| (n.start - last_start).abs() < 1e-6).map(|n| n.pitch).min();
    let minor_tonic = crate::model::key_tonic_pc(fifths, true);
    lowest_last.is_some_and(|p| p % 12 == minor_tonic)
}

pub fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let lw = w.to_lowercase();
            if ["bwv", "anh", "op", "no", "k"].contains(&lw.as_str()) && w.len() <= 3 {
                let mut c = lw.chars();
                return match lw.as_str() { "bwv" => "BWV".into(), _ => c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() + if lw == "anh" { "." } else { "" } };
            }
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Pick the major key signature whose scale covers the most notes.
pub fn guess_key(notes: &[Note]) -> i32 {
    let mut best = (0usize, 0i32);
    for f in -6i32..=6 {
        let tonic = (f * 7).rem_euclid(12);
        let scale: Vec<i32> = [0, 2, 4, 5, 7, 9, 11].iter().map(|s| (tonic + s) % 12).collect();
        let hits = notes.iter().filter(|n| scale.contains(&((n.pitch % 12) as i32))).count();
        if hits > best.0 || (hits == best.0 && f.abs() < best.1.abs()) {
            best = (hits, f);
        }
    }
    best.1
}

fn unzip_mxl(bytes: &[u8]) -> Result<String, String> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("Bad .mxl file: {e}"))?;
    let mut root: Option<String> = None;
    if let Ok(mut c) = z.by_name("META-INF/container.xml") {
        let mut s = String::new();
        c.read_to_string(&mut s).ok();
        if let Ok(doc) = parse_xml(&s) {
            root = doc.descendants().find(|n| n.has_tag_name("rootfile")).and_then(|n| n.attribute("full-path")).map(String::from);
        }
    }
    let name = match root {
        Some(r) => r,
        None => (0..z.len())
            .filter_map(|i| z.by_index(i).ok().map(|f| f.name().to_string()))
            .find(|n| !n.starts_with("META-INF") && (n.ends_with(".xml") || n.ends_with(".musicxml")))
            .ok_or("No score inside .mxl")?,
    };
    let mut f = z.by_name(&name).map_err(|e| e.to_string())?;
    let mut s = String::new();
    f.read_to_string(&mut s).map_err(|e| e.to_string())?;
    Ok(s)
}

fn child_text<'a>(n: roxmltree::Node<'a, 'a>, tag: &str) -> Option<&'a str> {
    n.children().find(|c| c.has_tag_name(tag)).and_then(|c| c.text())
}

/// Parse XML, allowing the <!DOCTYPE …> line that MusicXML files (Audiveris, MuseScore,
/// Sibelius, Finale…) always start with. The parser refuses it by default.
fn parse_xml(xml: &str) -> Result<roxmltree::Document<'_>, roxmltree::Error> {
    roxmltree::Document::parse_with_options(xml, roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() })
}

pub fn import_musicxml(xml: &str, fallback_title: &str) -> Result<Piece, String> {
    let doc = parse_xml(xml).map_err(|e| format!("Not valid MusicXML: {e}"))?;
    let root = doc.root_element();
    if root.has_tag_name("score-timewise") {
        return Err("Timewise MusicXML isn't supported; re-export as standard (partwise) MusicXML.".into());
    }
    let title = root
        .descendants()
        .find(|n| n.has_tag_name("work-title") || n.has_tag_name("movement-title"))
        .and_then(|n| n.text())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| title_case(fallback_title));
    let composer = root
        .descendants()
        .find(|n| n.has_tag_name("creator") && n.attribute("type") == Some("composer"))
        .and_then(|n| n.text())
        .unwrap_or("")
        .trim()
        .to_string();

    let parts: Vec<_> = root.children().filter(|n| n.has_tag_name("part")).collect();
    let mut notes: Vec<Note> = Vec::new();
    let mut bpm: Option<f64> = None;
    let mut ts = (4u32, 4u32);
    let mut key = (0i32, false);
    let mut have_key = false;
    let mut pickup = 0.0;
    for (pi, part) in parts.iter().enumerate() {
        let mut divisions = 1.0f64;
        let mut cursor = 0.0f64; // in beats
        let mut last_start = 0.0f64;
        let mut open_ties: HashMap<(u8, u8), usize> = HashMap::new();
        for (mi, measure) in part.children().filter(|n| n.has_tag_name("measure")).enumerate() {
            let m_start = cursor;
            let mut m_max = cursor;
            for el in measure.children().filter(|n| n.is_element()) {
                match el.tag_name().name() {
                    "attributes" => {
                        if let Some(d) = child_text(el, "divisions").and_then(|s| s.trim().parse::<f64>().ok()) {
                            divisions = d;
                        }
                        if let Some(k) = el.children().find(|c| c.has_tag_name("key")) {
                            if !have_key {
                                let f = child_text(k, "fifths").and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                                let minor = child_text(k, "mode").map(|m| m.trim() == "minor").unwrap_or(false);
                                key = (f, minor);
                                have_key = true;
                            }
                        }
                        if let Some(t) = el.children().find(|c| c.has_tag_name("time")) {
                            if pi == 0 && mi == 0 {
                                let b = child_text(t, "beats").and_then(|s| s.trim().split('+').next().and_then(|x| x.parse().ok())).unwrap_or(4);
                                let bt = child_text(t, "beat-type").and_then(|s| s.trim().parse().ok()).unwrap_or(4);
                                ts = (b, bt);
                            }
                        }
                    }
                    "direction" | "sound" => {
                        let snd = if el.has_tag_name("sound") { Some(el) } else { el.descendants().find(|n| n.has_tag_name("sound")) };
                        if let Some(t) = snd.and_then(|s| s.attribute("tempo")).and_then(|t| t.parse::<f64>().ok()) {
                            bpm.get_or_insert(t);
                        }
                    }
                    "backup" => {
                        let d = child_text(el, "duration").and_then(|s| s.trim().parse::<f64>().ok()).unwrap_or(0.0);
                        cursor -= d / divisions;
                    }
                    "forward" => {
                        let d = child_text(el, "duration").and_then(|s| s.trim().parse::<f64>().ok()).unwrap_or(0.0);
                        cursor += d / divisions;
                        m_max = m_max.max(cursor);
                    }
                    "note" => {
                        if el.children().any(|c| c.has_tag_name("grace") || c.has_tag_name("cue")) {
                            continue;
                        }
                        let dur = child_text(el, "duration").and_then(|s| s.trim().parse::<f64>().ok()).unwrap_or(0.0) / divisions;
                        let is_chord = el.children().any(|c| c.has_tag_name("chord"));
                        let start = if is_chord { last_start } else { cursor };
                        if !is_chord {
                            last_start = cursor;
                            cursor += dur;
                            m_max = m_max.max(cursor);
                        }
                        let Some(pitch_el) = el.children().find(|c| c.has_tag_name("pitch")) else { continue };
                        let step = child_text(pitch_el, "step").unwrap_or("C").trim();
                        let alter: f64 = child_text(pitch_el, "alter").and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
                        let octave: i32 = child_text(pitch_el, "octave").and_then(|s| s.trim().parse().ok()).unwrap_or(4);
                        let base = match step {
                            "C" => 0, "D" => 2, "E" => 4, "F" => 5, "G" => 7, "A" => 9, _ => 11,
                        };
                        let pitch = ((octave + 1) * 12 + base + alter.round() as i32).clamp(0, 127) as u8;
                        let staff: u8 = child_text(el, "staff").and_then(|s| s.trim().parse().ok()).unwrap_or(1);
                        let hand = if parts.len() >= 2 {
                            if pi == 0 { Hand::Right } else { Hand::Left }
                        } else if staff >= 2 {
                            Hand::Left
                        } else {
                            Hand::Right
                        };
                        let finger = el.descendants().find(|n| n.has_tag_name("fingering")).and_then(|n| n.text()).and_then(|t| t.trim().chars().next()).and_then(|c| c.to_digit(10)).map(|d| d as u8);
                        let ties: Vec<&str> = el.children().filter(|c| c.has_tag_name("tie")).filter_map(|c| c.attribute("type")).collect();
                        let tie_stop = ties.contains(&"stop");
                        let tie_start = ties.contains(&"start");
                        let k = (pi as u8 * 2 + staff, pitch);
                        if tie_stop {
                            if let Some(&idx) = open_ties.get(&k) {
                                let n: &mut Note = &mut notes[idx];
                                n.dur = (start + dur) - n.start;
                                if !tie_start {
                                    open_ties.remove(&k);
                                }
                                continue;
                            }
                        }
                        notes.push(Note { finger, ..Note::new(q(start), q(dur).max(1.0 / 12.0), pitch, hand, None) });
                        if tie_start {
                            open_ties.insert(k, notes.len() - 1);
                        }
                    }
                    _ => {}
                }
            }
            // measure with implicit="yes" at the start = pickup bar
            let bar_len = ts.0 as f64 * 4.0 / ts.1 as f64;
            if pi == 0 && mi == 0 && measure.attribute("implicit") == Some("yes") {
                pickup = m_max - m_start;
            }
            let filled = m_max - m_start;
            cursor = m_start + if filled > 0.0 { filled } else { bar_len };
        }
    }
    Ok(Piece {
        id: String::new(),
        title,
        composer,
        notes_text: "Imported from MusicXML.".into(),
        bpm: bpm.unwrap_or(96.0).clamp(30.0, 240.0),
        ts_num: ts.0,
        ts_den: ts.1,
        key_fifths: key.0,
        minor: key.1,
        pickup: if pickup > 0.0 && pickup < ts.0 as f64 * 4.0 / ts.1 as f64 { pickup } else { 0.0 },
        notes,
        origin: Origin::BuiltIn,
        key_changes: vec![],
        src: None,
    })
}

/// Unpack a compressed MusicXML (.mxl) file – used for Audiveris output.
pub fn unzip_for_omr(bytes: &[u8]) -> Result<String, String> {
    unzip_mxl(bytes)
}

pub fn guess_minor(notes: &[Note], fifths: i32) -> bool {
    ends_on_relative_minor(notes, fifths)
}
