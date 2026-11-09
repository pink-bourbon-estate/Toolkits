//! "Guitar-hero" falling notes and the on-screen keyboard, with zoom and scrolling.

use crate::model::{is_black, Hand, Piece, FLAT_NAMES, SHARP_NAMES};
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::collections::HashSet;

pub const RH_COLOR: Color32 = Color32::from_rgb(64, 170, 255);
pub const LH_COLOR: Color32 = Color32::from_rgb(255, 150, 60);

pub fn hand_color(h: Hand) -> Color32 {
    match h {
        Hand::Right => RH_COLOR,
        Hand::Left => LH_COLOR,
    }
}

/// Instrument sizes: (keys, lowest MIDI note, highest MIDI note)
pub const KEYBOARDS: [(u8, u8, u8, &str); 3] = [
    (61, 36, 96, "61 keys (C2–C7)"),
    (76, 28, 103, "76 keys (E1–G7)"),
    (88, 21, 108, "88 keys (A0–C8)"),
];

/// Visible part of the keyboard. All drawing (falling notes, keys) uses the same view.
pub struct KeyView {
    pub lo: u8,
    pub hi: u8,
    /// Left edge of the view, in white-key units from `lo`.
    pub left: f32,
    /// How many white keys fit across the view.
    pub visible: f32,
    pub follow: bool,
}

fn white_count_below(p: u8) -> f32 {
    // number of white keys strictly below p (from MIDI 0)
    let oct = (p / 12) as f32;
    let pc = p % 12;
    let w_in = [0, 1, 1, 2, 2, 3, 4, 4, 5, 5, 6, 6][pc as usize] as f32;
    oct * 7.0 + w_in
}

impl KeyView {
    pub fn new(keys: u8) -> KeyView {
        let (_, lo, hi, _) = KEYBOARDS.iter().find(|k| k.0 == keys).copied().unwrap_or(KEYBOARDS[1]);
        let mut v = KeyView { lo, hi, left: 0.0, visible: 29.0, follow: true };
        v.fit(48, 76);
        v
    }
    pub fn set_keys(&mut self, keys: u8) {
        let (_, lo, hi, _) = KEYBOARDS.iter().find(|k| k.0 == keys).copied().unwrap_or(KEYBOARDS[1]);
        self.lo = lo;
        self.hi = hi;
        self.clamp();
    }
    pub fn keys(&self) -> u8 {
        (self.hi - self.lo + 1) as u8
    }
    fn total_whites(&self) -> f32 {
        white_count_below(self.hi + 1) - white_count_below(self.lo)
    }
    /// White-key position (fractional for black keys = centre between neighbours).
    fn pos(&self, p: u8) -> f32 {
        let base = white_count_below(self.lo);
        if is_black(p) {
            white_count_below(p) - base
        } else {
            white_count_below(p) - base + 0.5
        }
    }
    fn clamp(&mut self) {
        let tw = self.total_whites();
        self.visible = self.visible.clamp(10.0, tw);
        self.left = self.left.clamp(0.0, (tw - self.visible).max(0.0));
    }
    /// Show pitches lo..=hi with a little room either side.
    pub fn fit(&mut self, lo: u8, hi: u8) {
        let lo = lo.max(self.lo);
        let hi = hi.min(self.hi).max(lo);
        let a = self.pos(lo) - 1.5;
        let b = self.pos(hi) + 1.5;
        self.visible = (b - a).max(17.0);
        self.left = (a + b) / 2.0 - self.visible / 2.0;
        self.clamp();
    }
    /// Scroll just enough so the pitches are in view (used while playing).
    pub fn ensure_visible(&mut self, lo: u8, hi: u8) {
        let a = self.pos(lo.max(self.lo)) - 1.0;
        let b = self.pos(hi.min(self.hi)) + 1.0;
        if b - a > self.visible {
            self.fit(lo, hi);
            return;
        }
        if a < self.left {
            self.left = a;
        } else if b > self.left + self.visible {
            self.left = b - self.visible;
        }
        self.clamp();
    }
    fn ww(&self, r: Rect) -> f32 {
        r.width() / self.visible
    }
    /// x position and width of a key inside `r`.
    pub fn key_x(&self, r: Rect, p: u8) -> (f32, f32) {
        let ww = self.ww(r);
        if is_black(p) {
            let bw = ww * 0.62;
            (r.left() + (self.pos(p) - self.left) * ww - bw / 2.0, bw)
        } else {
            (r.left() + (self.pos(p) - 0.5 - self.left) * ww, ww)
        }
    }
    fn on_screen(&self, p: u8) -> bool {
        let x = self.pos(p);
        p >= self.lo && p <= self.hi && x > self.left - 1.0 && x < self.left + self.visible + 1.0
    }

    /// The zoom/scroll strip: a miniature of the whole keyboard with the visible part outlined.
    pub fn scrollbar(&mut self, ui: &mut Ui, rect: Rect, piece_range: Option<(u8, u8)>) {
        let resp = ui.allocate_rect(rect, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 3.0, Color32::from_rgb(30, 32, 40));
        let tw = self.total_whites();
        let sx = rect.width() / tw;
        for p in self.lo..=self.hi {
            let x = rect.left() + self.pos(p) * sx;
            if is_black(p) {
                painter.line_segment([Pos2::new(x, rect.top() + 2.0), Pos2::new(x, rect.center().y)], Stroke::new(sx * 0.6, Color32::from_rgb(10, 10, 12)));
            } else if p % 12 == 0 {
                painter.text(Pos2::new(x - sx * 0.5 + 1.0, rect.bottom() - 1.0), Align2::LEFT_BOTTOM, format!("C{}", p / 12 - 1), FontId::proportional(9.0), Color32::from_gray(150));
            }
        }
        if let Some((a, b)) = piece_range {
            let xa = rect.left() + (self.pos(a) - 0.5) * sx;
            let xb = rect.left() + (self.pos(b) + 0.5) * sx;
            painter.rect_filled(Rect::from_min_max(Pos2::new(xa, rect.bottom() - 3.0), Pos2::new(xb, rect.bottom())), 0.0, Color32::from_rgb(90, 160, 230));
        }
        let vr = Rect::from_min_max(Pos2::new(rect.left() + self.left * sx, rect.top()), Pos2::new(rect.left() + (self.left + self.visible) * sx, rect.bottom()));
        painter.rect_filled(vr, 3.0, Color32::from_rgba_unmultiplied(240, 220, 120, 40));
        painter.rect_stroke(vr, 3.0, Stroke::new(1.5, Color32::from_rgb(240, 220, 120)), StrokeKind::Inside);
        if resp.dragged() || resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                self.left = (p.x - rect.left()) / sx - self.visible / 2.0;
                self.follow = false;
                self.clamp();
            }
        }
        // mouse wheel over the strip zooms
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.1 {
                let c = self.left + self.visible / 2.0;
                self.visible *= if scroll > 0.0 { 0.92 } else { 1.08 };
                self.left = c - self.visible / 2.0;
                self.clamp();
            }
        }
        resp.on_hover_text("Drag to scroll the keyboard · mouse wheel to zoom");
    }

    pub fn zoom_by(&mut self, f: f32) {
        let c = self.left + self.visible / 2.0;
        self.visible *= f;
        self.left = c - self.visible / 2.0;
        self.clamp();
    }
}

pub struct FallingOpts {
    pub pos: f64,
    pub lookahead: f64,
    pub show_names: bool,
    pub show_fingers: bool,
    pub prefer_flats: bool,
    pub loop_range: Option<(f64, f64)>,
    pub hidden: bool,
}

pub fn note_label(p: u8, prefer_flats: bool) -> &'static str {
    const SH: [&str; 12] = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];
    const FL: [&str; 12] = ["C", "D♭", "D", "E♭", "E", "F", "G♭", "G", "A♭", "A", "B♭", "B"];
    if prefer_flats { FL[(p % 12) as usize] } else { SH[(p % 12) as usize] }
}

pub fn falling_notes(ui: &mut Ui, rect: Rect, piece: &Piece, view: &KeyView, o: &FallingOpts) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::from_rgb(18, 20, 26));
    for p in view.lo..=view.hi {
        if !view.on_screen(p) {
            continue;
        }
        let (x, w) = view.key_x(rect, p);
        if is_black(p) {
            painter.rect_filled(Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, rect.height())), 0.0, Color32::from_rgb(24, 27, 34));
        }
        if p % 12 == 0 || p % 12 == 5 {
            painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], Stroke::new(1.0, if p % 12 == 0 { Color32::from_rgb(48, 52, 64) } else { Color32::from_rgb(34, 38, 48) }));
        }
    }
    let ppb = rect.height() as f64 / o.lookahead;
    let y_of = |beat: f64| rect.bottom() - ((beat - o.pos) * ppb) as f32;
    let bar = piece.bar_len();
    let first_bar = piece.bar_at(o.pos.max(0.0));
    for b in first_bar..first_bar + (o.lookahead / bar).ceil() as usize + 2 {
        let y = y_of(piece.bar_start(b));
        if y >= rect.top() && y <= rect.bottom() {
            painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], Stroke::new(1.0, Color32::from_rgb(55, 60, 75)));
            painter.text(Pos2::new(rect.left() + 4.0, y - 2.0), Align2::LEFT_BOTTOM, format!("{b}"), FontId::proportional(11.0), Color32::from_rgb(110, 115, 130));
        }
    }
    if let Some((a, b)) = o.loop_range {
        for t in [a, b] {
            let y = y_of(t);
            if y >= rect.top() && y <= rect.bottom() {
                painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], Stroke::new(2.0, Color32::from_rgb(200, 80, 200)));
            }
        }
    }
    if o.hidden {
        painter.text(rect.center(), Align2::CENTER_CENTER, "Notes hidden – play from memory\n(press Peek if you need a reminder)", FontId::proportional(18.0), Color32::from_gray(120));
    } else {
        for n in &piece.notes {
            if n.start + n.dur < o.pos - 0.01 || n.start > o.pos + o.lookahead || !view.on_screen(n.pitch) {
                continue;
            }
            let (x, w) = view.key_x(rect, n.pitch);
            let y1 = y_of(n.start);
            let y0 = y_of(n.start + n.dur) + 1.5;
            let r = Rect::from_min_max(Pos2::new(x + 1.5, y0.max(rect.top())), Pos2::new(x + w - 1.5, y1.min(rect.bottom())));
            if r.height() <= 0.0 {
                continue;
            }
            let mut c = hand_color(n.hand);
            if is_black(n.pitch) {
                c = c.gamma_multiply(0.75);
            }
            let sounding = n.start <= o.pos && o.pos < n.start + n.dur;
            if sounding {
                c = c.lerp_to_gamma(Color32::WHITE, 0.25);
            }
            painter.rect_filled(r, 4.0, c);
            painter.rect_stroke(r, 4.0, Stroke::new(1.0, Color32::from_black_alpha(120)), StrokeKind::Inside);
            let mut label = String::new();
            if o.show_names {
                label.push_str(note_label(n.pitch, o.prefer_flats));
            }
            if o.show_fingers {
                if let Some(f) = n.finger {
                    if !label.is_empty() {
                        label.push(if r.height() > 30.0 { '\n' } else { ' ' });
                    }
                    label.push_str(&f.to_string());
                }
            }
            if !label.is_empty() && r.height() > 14.0 {
                painter.text(Pos2::new(r.center().x, r.bottom() - 3.0), Align2::CENTER_BOTTOM, label, FontId::proportional((w * 0.45).clamp(9.0, 14.0)), Color32::from_rgb(10, 12, 18));
            }
        }
    }
    painter.line_segment([Pos2::new(rect.left(), rect.bottom() - 1.0), Pos2::new(rect.right(), rect.bottom() - 1.0)], Stroke::new(2.0, Color32::from_rgb(240, 220, 120)));
}

pub struct KeyboardOpts<'a> {
    pub lit: &'a [(u8, Hand, Option<u8>)],
    pub pressed: &'a HashSet<u8>,
    pub prefer_flats: bool,
    pub names: KeyNames,
    /// Highlight these keys softly (e.g. the scale being practised, or reading-trainer answers).
    pub marked: &'a [(u8, Color32)],
}

#[derive(Clone, Copy, PartialEq)]
pub enum KeyNames {
    All,
    WhiteOnly,
    COnly,
}

/// Draws the keyboard. Returns a key that was clicked, if any.
pub fn keyboard(ui: &mut Ui, rect: Rect, view: &KeyView, o: &KeyboardOpts) -> Option<u8> {
    let resp = ui.allocate_rect(rect, Sense::click());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::from_rgb(12, 12, 14));
    let lit = |p: u8| o.lit.iter().find(|(lp, _, _)| *lp == p).map(|(_, h, f)| (hand_color(*h), *f));
    let black_h = rect.height() * 0.63;
    let ww = rect.width() / view.visible;
    let click_pos = if resp.clicked() { resp.interact_pointer_pos() } else { None };
    let mut clicked = None;
    let mark = |p: u8| o.marked.iter().find(|(m, _)| *m == p).map(|(_, c)| *c);

    // white keys
    for p in view.lo..=view.hi {
        if is_black(p) || !view.on_screen(p) {
            continue;
        }
        let (x, w) = view.key_x(rect, p);
        let r = Rect::from_min_size(Pos2::new(x + 0.5, rect.top()), Vec2::new(w - 1.0, rect.height() - 1.0));
        let pressed = o.pressed.contains(&p);
        match lit(p) {
            Some((c, _)) => {
                painter.rect_filled(r, egui::CornerRadius { nw: 0, ne: 0, sw: 4, se: 4 }, c);
                painter.rect_filled(Rect::from_min_max(Pos2::new(r.left(), r.bottom() - 6.0), r.max), 4, c.gamma_multiply(0.7));
            }
            None => {
                // ivory with a soft shadow under the black keys and a lip at the front
                painter.rect_filled(r, egui::CornerRadius { nw: 0, ne: 0, sw: 4, se: 4 }, Color32::from_rgb(246, 245, 238));
                painter.rect_filled(Rect::from_min_max(r.min, Pos2::new(r.right(), r.top() + 8.0)), 0, Color32::from_rgb(214, 212, 204));
                painter.rect_filled(Rect::from_min_max(Pos2::new(r.left(), r.bottom() - 7.0), r.max), egui::CornerRadius { nw: 0, ne: 0, sw: 4, se: 4 }, Color32::from_rgb(226, 224, 216));
                if let Some(c) = mark(p) {
                    painter.rect_filled(Rect::from_min_max(Pos2::new(r.left() + 2.0, black_h + rect.top() + 4.0), Pos2::new(r.right() - 2.0, r.bottom() - 9.0)), 3.0, c.gamma_multiply(0.35));
                }
            }
        }
        if pressed {
            painter.rect_stroke(r.shrink(2.0), 3.0, Stroke::new(3.0, Color32::from_rgb(60, 200, 90)), StrokeKind::Inside);
        }
        let show = match o.names {
            KeyNames::All | KeyNames::WhiteOnly => true,
            KeyNames::COnly => p % 12 == 0,
        };
        let size = (ww * 0.42).clamp(8.0, 15.0);
        let txt_col = if lit(p).is_some() { Color32::from_rgb(15, 18, 25) } else if p % 12 == 0 { Color32::from_rgb(60, 60, 70) } else { Color32::from_rgb(120, 120, 125) };
        if let Some((_, Some(f))) = lit(p) {
            painter.text(Pos2::new(r.center().x, rect.top() + black_h + (rect.height() - black_h) * 0.35), Align2::CENTER_CENTER, f.to_string(), FontId::proportional(size * 1.3), Color32::from_rgb(15, 18, 25));
        }
        if show && ww > 9.0 {
            let name = if p % 12 == 0 { format!("C{}", p as i32 / 12 - 1) } else { SHARP_NAMES[(p % 12) as usize].to_string() };
            painter.text(Pos2::new(r.center().x, r.bottom() - 9.0), Align2::CENTER_BOTTOM, name, FontId::proportional(size), txt_col);
        }
        if p == 60 && lit(p).is_none() {
            painter.circle_filled(Pos2::new(r.center().x, r.bottom() - 9.0 - size - 6.0), 2.5, Color32::from_rgb(170, 170, 170));
        }
        if let Some(cp) = click_pos {
            if r.contains(cp) {
                clicked = Some(p);
            }
        }
    }
    // black keys
    for p in view.lo..=view.hi {
        if !is_black(p) || !view.on_screen(p) {
            continue;
        }
        let (x, w) = view.key_x(rect, p);
        let r = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, black_h));
        match lit(p) {
            Some((c, _)) => {
                painter.rect_filled(r, egui::CornerRadius { nw: 0, ne: 0, sw: 3, se: 3 }, c.gamma_multiply(0.9));
            }
            None => {
                painter.rect_filled(r.translate(Vec2::new(1.5, 2.0)), 3.0, Color32::from_black_alpha(90));
                painter.rect_filled(r, egui::CornerRadius { nw: 0, ne: 0, sw: 3, se: 3 }, Color32::from_rgb(28, 28, 32));
                // bevel highlight
                painter.rect_filled(Rect::from_min_max(Pos2::new(r.left() + 2.0, r.top()), Pos2::new(r.right() - 2.0, r.bottom() - 6.0)), 2.0, Color32::from_rgb(44, 44, 50));
                if let Some(c) = mark(p) {
                    painter.rect_filled(r.shrink(3.0), 2.0, c.gamma_multiply(0.45));
                }
            }
        }
        if o.pressed.contains(&p) {
            painter.rect_stroke(r.shrink(1.5), 2.0, Stroke::new(3.0, Color32::from_rgb(60, 200, 90)), StrokeKind::Inside);
        }
        let txt_col = if lit(p).is_some() { Color32::from_rgb(10, 12, 18) } else { Color32::from_rgb(200, 200, 205) };
        let size = (w * 0.42).clamp(7.0, 12.0);
        if let Some((_, Some(f))) = lit(p) {
            painter.text(Pos2::new(r.center().x, r.top() + black_h * 0.3), Align2::CENTER_CENTER, f.to_string(), FontId::proportional(size * 1.4), txt_col);
        }
        if o.names == KeyNames::All && w > 8.0 {
            let sharp = SHARP_NAMES[(p % 12) as usize].replace('#', "♯");
            let flat = FLAT_NAMES[(p % 12) as usize].replace('b', "♭");
            let txt = if o.prefer_flats { format!("{flat}\n{sharp}") } else { format!("{sharp}\n{flat}") };
            painter.text(Pos2::new(r.center().x, r.bottom() - 4.0), Align2::CENTER_BOTTOM, txt, FontId::proportional(size), txt_col);
        }
        if let Some(cp) = click_pos {
            if r.contains(cp) {
                clicked = Some(p);
            }
        }
    }
    clicked
}
