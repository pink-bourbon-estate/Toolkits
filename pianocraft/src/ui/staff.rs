//! A simple grand-staff renderer: one line ("system") of bars at a time, with a moving cursor.

use crate::model::{Hand, Note, Piece};
use crate::ui::falling::hand_color;
use egui::{epaint::PathShape, Align2, Color32, FontId, Pos2, Rect, Shape, Stroke, Ui};
use std::collections::HashMap;

const INK: Color32 = Color32::from_rgb(30, 30, 35);
const PAPER: Color32 = Color32::from_rgb(250, 248, 240);

/// Sharps/flats in key signature order.
const SHARP_ORDER: [usize; 7] = [3, 0, 4, 1, 5, 2, 6]; // F C G D A E B  (letter idx C=0)
const FLAT_ORDER: [usize; 7] = [6, 2, 5, 1, 4, 0, 3]; // B E A D G C F

/// Spell a pitch as (letter index 0..6 for C..B, octave, alteration) given the key.
pub fn spell(pitch: u8, key_fifths: i32) -> (usize, i32, i32) {
    let pc = (pitch % 12) as i32;
    let oct = pitch as i32 / 12 - 1;
    // naturals
    const NAT: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
    let key_alter = key_alters(key_fifths);
    // prefer a spelling that matches the key signature, then naturals, then the key's accidental direction
    let dir = if key_fifths >= 0 { 1 } else { -1 };
    for pass in 0..4 {
        for (li, nat) in NAT.iter().enumerate() {
            let alt = match pass {
                0 => key_alter[li],
                1 => 0,
                2 => dir,
                _ => -dir,
            };
            let v = nat + alt;
            if v.rem_euclid(12) == pc {
                let o = oct + if v < 0 { 1 } else if v >= 12 { -1 } else { 0 };
                return (li, o, alt);
            }
        }
    }
    (0, oct, 0)
}

fn key_alters(f: i32) -> [i32; 7] {
    let mut a = [0; 7];
    if f > 0 {
        for &li in SHARP_ORDER.iter().take(f.min(7) as usize) {
            a[li] = 1;
        }
    } else if f < 0 {
        for &li in FLAT_ORDER.iter().take((-f).min(7) as usize) {
            a[li] = -1;
        }
    }
    a
}

/// Diatonic step number (C0 = 0).
fn diatonic(letter: usize, octave: i32) -> i32 {
    octave * 7 + letter as i32
}

struct StaffGeom {
    sp: f32, // distance between staff lines
    treble_bottom: f32,
    bass_bottom: f32,
}

impl StaffGeom {
    fn y(&self, hand: Hand, d: i32) -> f32 {
        match hand {
            Hand::Right => self.treble_bottom - (d - diatonic(2, 4)) as f32 * self.sp / 2.0, // E4 bottom line
            Hand::Left => self.bass_bottom - (d - diatonic(4, 2)) as f32 * self.sp / 2.0,     // G2 bottom line
        }
    }
    fn bottom_d(hand: Hand) -> i32 {
        match hand {
            Hand::Right => diatonic(2, 4),
            Hand::Left => diatonic(4, 2),
        }
    }
}

fn draw_sharp(sh: &mut Vec<Shape>, c: Pos2, sp: f32, col: Color32) {
    let s = Stroke::new(1.2, col);
    let h = sp * 1.3;
    let w = sp * 0.45;
    sh.push(Shape::line_segment([Pos2::new(c.x - w * 0.4, c.y - h / 2.0), Pos2::new(c.x - w * 0.4, c.y + h / 2.0 + 2.0)], s));
    sh.push(Shape::line_segment([Pos2::new(c.x + w * 0.4, c.y - h / 2.0 - 2.0), Pos2::new(c.x + w * 0.4, c.y + h / 2.0)], s));
    let t = Stroke::new(2.2, col);
    sh.push(Shape::line_segment([Pos2::new(c.x - w, c.y - sp * 0.15), Pos2::new(c.x + w, c.y - sp * 0.45)], t));
    sh.push(Shape::line_segment([Pos2::new(c.x - w, c.y + sp * 0.45), Pos2::new(c.x + w, c.y + sp * 0.15)], t));
}

fn draw_flat(sh: &mut Vec<Shape>, c: Pos2, sp: f32, col: Color32) {
    let s = Stroke::new(1.3, col);
    let x = c.x - sp * 0.25;
    sh.push(Shape::line_segment([Pos2::new(x, c.y - sp * 1.4), Pos2::new(x, c.y + sp * 0.5)], s));
    let pts: Vec<Pos2> = (0..=10)
        .map(|i| {
            let t = i as f32 / 10.0 * std::f32::consts::PI;
            Pos2::new(x + t.sin() * sp * 0.55, c.y + sp * 0.5 - (1.0 - t.cos()) * sp * 0.38)
        })
        .collect();
    sh.push(Shape::Path(PathShape::line(pts, Stroke::new(1.6, col))));
}

fn draw_natural(sh: &mut Vec<Shape>, c: Pos2, sp: f32, col: Color32) {
    let s = Stroke::new(1.2, col);
    let w = sp * 0.3;
    sh.push(Shape::line_segment([Pos2::new(c.x - w, c.y - sp * 1.1), Pos2::new(c.x - w, c.y + sp * 0.35)], s));
    sh.push(Shape::line_segment([Pos2::new(c.x + w, c.y - sp * 0.35), Pos2::new(c.x + w, c.y + sp * 1.1)], s));
    let t = Stroke::new(2.0, col);
    sh.push(Shape::line_segment([Pos2::new(c.x - w, c.y - sp * 0.2), Pos2::new(c.x + w, c.y - sp * 0.4)], t));
    sh.push(Shape::line_segment([Pos2::new(c.x - w, c.y + sp * 0.4), Pos2::new(c.x + w, c.y + sp * 0.2)], t));
}

fn notehead(sh: &mut Vec<Shape>, c: Pos2, sp: f32, filled: bool, col: Color32) {
    let rx = sp * 0.62;
    let ry = sp * 0.44;
    let tilt: f32 = -0.35;
    let pts: Vec<Pos2> = (0..20)
        .map(|i| {
            let a = i as f32 / 20.0 * std::f32::consts::TAU;
            let (x, y) = (a.cos() * rx, a.sin() * ry);
            Pos2::new(c.x + x * tilt.cos() - y * tilt.sin(), c.y + x * tilt.sin() + y * tilt.cos())
        })
        .collect();
    if filled {
        sh.push(Shape::convex_polygon(pts, col, Stroke::NONE));
    } else {
        sh.push(Shape::closed_line(pts, Stroke::new(1.8, col)));
    }
}

pub struct StaffOpts {
    pub pos: f64,
    pub show_names: bool,
    pub show_fingers: bool,
    pub prefer_flats: bool,
    pub loop_range: Option<(f64, f64)>,
}

/// Draw the system (line of bars) containing the playhead.
pub fn staff(ui: &mut Ui, rect: Rect, piece: &Piece, o: &StaffOpts) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, PAPER);
    let sp = (rect.height() / 15.0).clamp(6.0, 13.0);
    let mid = rect.center().y;
    let g = StaffGeom { sp, treble_bottom: mid - sp * 1.6, bass_bottom: mid + sp * 1.6 + sp * 4.0 };
    let mut sh: Vec<Shape> = Vec::new();

    // header: clef labels, key signature, time signature
    let left = rect.left() + 10.0;
    let kf = piece.key_at(o.pos.max(0.0));
    let ka = key_alters(kf);
    let nacc = kf.unsigned_abs().min(7) as f32;
    let header_w = 58.0 + nacc * sp * 0.9 + sp * 2.2;
    let x0 = left + header_w;
    let right = rect.right() - 10.0;

    // decide bars per line from density
    let bar = piece.bar_len();
    let min_ppb = 78.0f32;
    let avail = right - x0;
    let bars_per_line = ((avail / (bar as f32 * min_ppb)).floor() as usize).clamp(1, 8);
    let cur_bar = piece.bar_at(o.pos.max(0.0)).min(piece.bar_count());
    let line_first = ((cur_bar - 1) / bars_per_line) * bars_per_line + 1;
    let line_last = (line_first + bars_per_line - 1).min(piece.bar_count());
    let t0 = piece.bar_start(line_first) - if line_first == 1 { piece.pickup } else { 0.0 };
    let t1 = piece.bar_start(line_last + 1);
    let ppb = avail / (t1 - t0) as f32;
    let x_of = |t: f64| x0 + ((t - t0) as f32) * ppb + sp * 1.2;

    let mut shapes_pre: Vec<Shape> = Vec::new();
    // staff lines
    for hand in [Hand::Right, Hand::Left] {
        let bot = if hand == Hand::Right { g.treble_bottom } else { g.bass_bottom };
        for i in 0..5 {
            let y = bot - i as f32 * sp;
            sh.push(Shape::line_segment([Pos2::new(left, y), Pos2::new(right, y)], Stroke::new(1.0, INK)));
        }
        // clef + hand label
        let col = hand_color(hand).gamma_multiply(0.95);
        if hand == Hand::Right {
            // treble clef glyph (from egui's icon font), its curl sits on the G line
            painter.text(Pos2::new(left + 22.0, bot - 1.0 * sp), Align2::CENTER_CENTER, "\u{1D11E}", FontId::proportional(sp * 6.4), INK);
        } else {
            // bass clef drawn by hand: a hook starting on the F line and two dots
            let f_y = bot - 3.0 * sp;
            let cx = left + 14.0;
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let t = i as f32 / 24.0;
                    let a = std::f32::consts::PI * (1.05 - 1.35 * t);
                    let r = sp * (1.05 - 0.15 * t);
                    Pos2::new(cx + sp * 0.4 + a.cos() * r + t * sp * 0.6, f_y - a.sin() * r * 0.95 + t * t * sp * 2.6)
                })
                .collect();
            shapes_pre.push(Shape::line(pts, Stroke::new(sp * 0.3, INK)));
            shapes_pre.push(Shape::circle_filled(Pos2::new(cx - sp * 0.55, f_y), sp * 0.3, INK));
            shapes_pre.push(Shape::circle_filled(Pos2::new(cx + sp * 2.15, f_y - sp * 0.5), sp * 0.22, INK));
            shapes_pre.push(Shape::circle_filled(Pos2::new(cx + sp * 2.15, f_y + sp * 0.5), sp * 0.22, INK));
        }
        painter.text(Pos2::new(left + 2.0, bot + sp * 0.2), Align2::LEFT_TOP, if hand == Hand::Right { "RH" } else { "LH" }, FontId::proportional(sp * 0.9), col);
        // key signature
        for (i, &li) in (if kf >= 0 { SHARP_ORDER } else { FLAT_ORDER }).iter().take(nacc as usize).enumerate() {
            // standard positions: treble sharps F5 C5 G5 D5 A4 E5 B4 ; flats B4 E5 A4 D5 G4 C5 F4
            let tre_oct = if kf >= 0 { [5, 5, 5, 5, 4, 5, 4][i] } else { [4, 5, 4, 5, 4, 5, 4][i] };
            let d = if hand == Hand::Right { diatonic(li, tre_oct) } else { diatonic(li, tre_oct) - 14 };
            let c = Pos2::new(left + 58.0 + i as f32 * sp * 0.9, g.y(hand, d));
            if ka[li] > 0 {
                draw_sharp(&mut sh, c, sp, INK);
            } else {
                draw_flat(&mut sh, c, sp, INK);
            }
        }
        // time signature on first line only
        if line_first == 1 {
            let tx = left + 58.0 + nacc * sp * 0.9 + sp * 0.9;
            painter.text(Pos2::new(tx, bot - 3.0 * sp), Align2::CENTER_CENTER, piece.ts_num.to_string(), FontId::proportional(sp * 2.1), INK);
            painter.text(Pos2::new(tx, bot - 1.0 * sp), Align2::CENTER_CENTER, piece.ts_den.to_string(), FontId::proportional(sp * 2.1), INK);
        }
    }
    // brace-ish line and bar lines
    let top_y = g.treble_bottom - 4.0 * sp;
    sh.push(Shape::line_segment([Pos2::new(left, top_y), Pos2::new(left, g.bass_bottom)], Stroke::new(2.0, INK)));
    for b in line_first..=line_last + 1 {
        let x = if b == line_last + 1 { right } else { x_of(piece.bar_start(b)) - sp * 1.2 };
        if b == line_first && line_first != 1 {
            continue;
        }
        let w = if b == piece.bar_count() + 1 { 3.0 } else { 1.0 };
        sh.push(Shape::line_segment([Pos2::new(x, top_y), Pos2::new(x, g.bass_bottom)], Stroke::new(w, INK)));
        if b <= line_last {
            painter.text(Pos2::new(x + 2.0, top_y - 4.0), Align2::LEFT_BOTTOM, b.to_string(), FontId::proportional(10.0), Color32::GRAY);
        }
    }
    // loop shading
    if let Some((a, b)) = o.loop_range {
        let xa = x_of(a.max(t0)) - sp * 1.2;
        let xb = x_of(b.min(t1)) - sp * 1.2;
        if b > t0 && a < t1 {
            painter.rect_filled(Rect::from_min_max(Pos2::new(xa, top_y - 2.0), Pos2::new(xb.min(right), g.bass_bottom + 2.0)), 0.0, Color32::from_rgba_unmultiplied(200, 80, 200, 28));
        }
    }

    // beam groups: per hand, consecutive short notes (top of chord) within one beat
    let beat = if matches!((piece.ts_num, piece.ts_den), (6 | 9 | 12, 8)) { 1.5 } else { 1.0 };
    let mut beamed: HashMap<(Hand, i64), (bool, f32)> = HashMap::new(); // (hand, start*1000) -> (stem up, stem end y)
    for hand in [Hand::Right, Hand::Left] {
        let mut onsets: Vec<(f64, f64, i32)> = Vec::new(); // start, dur, diatonic of outermost note
        for n in piece.notes.iter().filter(|n| n.hand == hand && n.start >= t0 - 1e-6 && n.start < t1 - 1e-6) {
            let (li, oct, _) = spell(n.pitch, piece.key_at(n.start));
            let d = diatonic(li, oct);
            match onsets.last_mut() {
                Some(o) if (o.0 - n.start).abs() < 1e-6 => o.2 = o.2.max(d),
                _ => onsets.push((n.start, n.dur, d)),
            }
        }
        let mut i = 0;
        while i < onsets.len() {
            let b = ((onsets[i].0 - piece.pickup) / beat + 1e-6).floor();
            let mut j = i;
            while j < onsets.len() && onsets[j].1 < 0.99 && ((onsets[j].0 - piece.pickup) / beat + 1e-6).floor() == b {
                j += 1;
            }
            if j - i >= 2 {
                let grp = &onsets[i..j];
                let mid_d = StaffGeom::bottom_d(hand) + 4;
                let avg = grp.iter().map(|o| o.2 as f32).sum::<f32>() / grp.len() as f32;
                let up = avg < mid_d as f32;
                let ys: Vec<f32> = grp.iter().map(|o| g.y(hand, o.2)).collect();
                let end_y = if up { ys.iter().cloned().fold(f32::MAX, f32::min) - sp * 3.2 } else { ys.iter().cloned().fold(f32::MIN, f32::max) + sp * 3.2 };
                for o in grp {
                    beamed.insert((hand, (o.0 * 1000.0).round() as i64), (up, end_y));
                }
                // draw beams
                let xa = x_of(grp[0].0) + if up { sp * 0.58 } else { -sp * 0.58 };
                let xb = x_of(grp[grp.len() - 1].0) + if up { sp * 0.58 } else { -sp * 0.58 };
                sh.push(Shape::line_segment([Pos2::new(xa, end_y), Pos2::new(xb, end_y)], Stroke::new(sp * 0.42, INK)));
                // second beam for 16ths (between consecutive 16ths)
                let off = if up { sp * 0.75 } else { -sp * 0.75 };
                for w in grp.windows(2) {
                    if w[0].1 < 0.374 && w[1].1 < 0.374 {
                        let x1 = x_of(w[0].0) + if up { sp * 0.58 } else { -sp * 0.58 };
                        let x2 = x_of(w[1].0) + if up { sp * 0.58 } else { -sp * 0.58 };
                        sh.push(Shape::line_segment([Pos2::new(x1, end_y + off), Pos2::new(x2, end_y + off)], Stroke::new(sp * 0.42, INK)));
                    }
                }
                // lone 16th next to an 8th: short stub
                for (k, o) in grp.iter().enumerate() {
                    if o.1 < 0.374 {
                        let paired = (k > 0 && grp[k - 1].1 < 0.374) || (k + 1 < grp.len() && grp[k + 1].1 < 0.374);
                        if !paired {
                            let x = x_of(o.0) + if up { sp * 0.58 } else { -sp * 0.58 };
                            let dx = if k + 1 < grp.len() { sp } else { -sp };
                            sh.push(Shape::line_segment([Pos2::new(x, end_y + off), Pos2::new(x + dx, end_y + off)], Stroke::new(sp * 0.42, INK)));
                        }
                    }
                }
            }
            i = j.max(i + 1);
        }
    }

    // notes
    let mut shown_acc: HashMap<(Hand, usize, i32, usize), i32> = HashMap::new(); // (hand, bar, octave, letter) -> alter
    let notes: Vec<&Note> = piece.notes.iter().filter(|n| n.start >= t0 - 1e-6 && n.start < t1 - 1e-6).collect();
    for n in &notes {
        let (li, oct, alt) = spell(n.pitch, piece.key_at(n.start));
        let d = diatonic(li, oct);
        let x = x_of(n.start);
        let y = g.y(n.hand, d);
        let sounding = n.start <= o.pos && o.pos < n.start + n.dur;
        let col = if sounding { hand_color(n.hand).gamma_multiply(1.0) } else { INK };
        // ledger lines
        let bot_d = StaffGeom::bottom_d(n.hand);
        let top_d = bot_d + 8;
        let mut ld = bot_d - 2;
        while ld >= d {
            let ly = g.y(n.hand, ld);
            sh.push(Shape::line_segment([Pos2::new(x - sp, ly), Pos2::new(x + sp, ly)], Stroke::new(1.0, INK)));
            ld -= 2;
        }
        let mut ld = top_d + 2;
        while ld <= d {
            let ly = g.y(n.hand, ld);
            sh.push(Shape::line_segment([Pos2::new(x - sp, ly), Pos2::new(x + sp, ly)], Stroke::new(1.0, INK)));
            ld += 2;
        }
        // accidentals
        let bar_i = piece.bar_at(n.start);
        let kkey = (n.hand, bar_i, oct, li);
        let current = *shown_acc.get(&kkey).unwrap_or(&ka[li]);
        if alt != current {
            let ac = Pos2::new(x - sp * 1.5, y);
            match alt {
                1 => draw_sharp(&mut sh, ac, sp * 0.85, col),
                -1 => draw_flat(&mut sh, ac, sp * 0.85, col),
                _ => draw_natural(&mut sh, ac, sp * 0.85, col),
            }
            shown_acc.insert(kkey, alt);
        }
        let filled = n.dur < 1.99;
        if sounding {
            sh.push(Shape::circle_filled(Pos2::new(x, y), sp * 0.95, hand_color(n.hand).gamma_multiply(0.35)));
        }
        notehead(&mut sh, Pos2::new(x, y), sp, filled, col);
        // dot
        let q16 = (n.dur * 4.0).round() as i32;
        if matches!(q16, 3 | 6 | 12) {
            let dy = if (d - bot_d) % 2 == 0 { -sp * 0.5 } else { 0.0 };
            sh.push(Shape::circle_filled(Pos2::new(x + sp * 1.0, y + dy), 1.8, col));
        }
        // stem + flags
        if n.dur < 3.99 {
            let bk = beamed.get(&(n.hand, (n.start * 1000.0).round() as i64)).copied();
            let up = bk.map(|b| b.0).unwrap_or(d < bot_d + 4);
            let (sx, sy0, mut sy1) = if up { (x + sp * 0.58, y - sp * 0.15, y - sp * 3.4) } else { (x - sp * 0.58, y + sp * 0.15, y + sp * 3.4) };
            if let Some((_, ey)) = bk {
                sy1 = ey;
            }
            sh.push(Shape::line_segment([Pos2::new(sx, sy0), Pos2::new(sx, sy1)], Stroke::new(1.3, col)));
            let flags = if bk.is_some() { 0 } else if n.dur < 0.374 { 2 } else if n.dur < 0.74 { 1 } else { 0 };
            for f in 0..flags {
                let fy = sy1 + if up { f as f32 * sp * 0.7 } else { -(f as f32) * sp * 0.7 };
                let ey = fy + if up { sp * 1.3 } else { -sp * 1.3 };
                sh.push(Shape::line_segment([Pos2::new(sx, fy), Pos2::new(sx + sp * 0.9, ey)], Stroke::new(1.8, col)));
            }
        }
        // labels
        if o.show_names {
            let name = crate::model::pitch_letter(n.pitch, o.prefer_flats || piece.key_at(n.start) < 0);
            let ly = if n.hand == Hand::Right { g.treble_bottom + sp * 1.6 } else { g.bass_bottom + sp * 1.7 };
            let ly = ly.max(y + sp * 1.3).min(rect.bottom() - sp * 1.2);
            painter.text(Pos2::new(x, ly), Align2::CENTER_TOP, name, FontId::proportional(sp * 0.95), hand_color(n.hand).gamma_multiply(0.85));
        }
        if o.show_fingers {
            if let Some(f) = n.finger {
                let fy = if n.hand == Hand::Right { (g.treble_bottom - 4.0 * sp - sp * 0.9).min(y - sp * 1.4) } else { (g.bass_bottom + sp * 1.2).max(y + sp * 1.4) };
                let fy = fy.clamp(rect.top() + sp * 0.7, rect.bottom() - sp * 0.7);
                painter.text(Pos2::new(x, fy), Align2::CENTER_CENTER, f.to_string(), FontId::proportional(sp * 1.0), Color32::from_rgb(120, 60, 160));
            }
        }
    }
    painter.extend(shapes_pre);
    painter.extend(sh);
    // key changes inside this line
    for (b, f) in &piece.key_changes {
        if *b > t0 + 1e-6 && *b < t1 - 1e-6 {
            let x = x_of(*b) - sp * 1.2;
            painter.line_segment([Pos2::new(x - 3.0, top_y), Pos2::new(x - 3.0, g.bass_bottom)], Stroke::new(1.0, INK));
            let tonic = crate::generate::pc_name(((f * 7).rem_euclid(12)) as u8, *f);
            let acc = if *f > 0 { format!("{f}♯") } else if *f < 0 { format!("{}♭", -f) } else { "no ♯/♭".into() };
            painter.text(Pos2::new(x + 2.0, top_y - 16.0), Align2::LEFT_BOTTOM, format!("→ key of {tonic} ({acc})"), FontId::proportional(11.0), Color32::from_rgb(160, 60, 60));
        }
    }
    // cursor
    if o.pos >= t0 && o.pos <= t1 {
        let cx = x_of(o.pos);
        painter.line_segment([Pos2::new(cx, top_y - 6.0), Pos2::new(cx, g.bass_bottom + 6.0)], Stroke::new(2.0, Color32::from_rgba_unmultiplied(220, 60, 60, 200)));
    }
    painter.text(Pos2::new(right, rect.bottom() - 4.0), Align2::RIGHT_BOTTOM, format!("bars {line_first}–{line_last} of {}", piece.bar_count()), FontId::proportional(10.0), Color32::GRAY);
}
