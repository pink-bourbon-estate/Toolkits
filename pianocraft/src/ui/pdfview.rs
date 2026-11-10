//! Book page viewer with annotations (pen, highlighter, text, finger numbers), using
//! hayro – the same pure-Rust PDF renderer PdfCraft uses. Rendering runs on a background thread.

use crate::curriculum::data_dir;
use egui::{Color32, ColorImage, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Ui, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

struct Req {
    path: PathBuf,
    page: usize,
    scale: f32,
}

enum Resp {
    Page { path: PathBuf, page: usize, scale: f32, img: ColorImage, count: usize },
    Error { path: PathBuf, msg: String },
}

pub struct Renderer {
    tx: Sender<Req>,
    rx: Receiver<Resp>,
}

impl Renderer {
    pub fn start() -> Renderer {
        let (tx, rrx) = channel::<Req>();
        let (rtx, rx) = channel::<Resp>();
        std::thread::spawn(move || {
            use hayro::hayro_interpret::InterpreterSettings;
            use hayro::hayro_syntax::Pdf;
            use hayro::vello_cpu::color::palette::css::WHITE;
            use hayro::{render, PixmapSettings, RenderCache, RenderSettings};
            let mut loaded: Option<(PathBuf, Pdf)> = None;
            while let Ok(mut req) = rrx.recv() {
                // skip stale requests, keep the newest
                while let Ok(r) = rrx.try_recv() {
                    req = r;
                }
                if loaded.as_ref().map(|(p, _)| p != &req.path).unwrap_or(true) {
                    match std::fs::read(&req.path).map_err(|e| e.to_string()).and_then(|d| Pdf::new(d).map_err(|e| format!("{e:?}"))) {
                        Ok(pdf) => loaded = Some((req.path.clone(), pdf)),
                        Err(msg) => {
                            let _ = rtx.send(Resp::Error { path: req.path.clone(), msg });
                            continue;
                        }
                    }
                }
                let (_, pdf) = loaded.as_ref().unwrap();
                let pages = pdf.pages();
                let count = pages.len();
                let idx = req.page.clamp(1, count.max(1)) - 1;
                let Some(page) = pages.get(idx) else { continue };
                let cache = RenderCache::new();
                let pm = render(page, &cache, &InterpreterSettings::default(), &RenderSettings::default(), &PixmapSettings { x_scale: req.scale, y_scale: req.scale, bg_color: WHITE });
                let (w, h) = (pm.width() as usize, pm.height() as usize);
                let mut rgba = Vec::with_capacity(w * h * 4);
                for px in pm.data() {
                    rgba.extend_from_slice(&[px.r, px.g, px.b, 255]);
                }
                let img = ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                let _ = rtx.send(Resp::Page { path: req.path, page: idx + 1, scale: req.scale, img, count });
            }
        });
        Renderer { tx, rx }
    }
}

// ------------------------------------------------------------------ annotations

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AStroke {
    pub pts: Vec<[f32; 2]>,
    pub color: [u8; 4],
    pub width: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AText {
    pub pos: [f32; 2],
    pub text: String,
    pub color: [u8; 4],
    pub size: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PageNotes {
    pub strokes: Vec<AStroke>,
    pub texts: Vec<AText>,
    /// order of additions for undo: 0 = stroke, 1 = text
    pub order: Vec<u8>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Annotations {
    pub pages: HashMap<String, PageNotes>,
}

impl Annotations {
    fn file() -> PathBuf {
        data_dir().join("annotations.json")
    }
    pub fn load() -> Annotations {
        std::fs::read_to_string(Self::file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    pub fn save(&self) {
        if let Ok(s) = serde_json::to_string(self) {
            let _ = std::fs::write(Self::file(), s);
        }
    }
    fn key(path: &Path, page: usize) -> String {
        format!("{}#{}", path.file_name().and_then(|n| n.to_str()).unwrap_or("?"), page)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Look,
    Pen,
    Highlighter,
    Text,
    Finger(u8),
    Eraser,
}

pub struct BookView {
    renderer: Renderer,
    pub path: Option<PathBuf>,
    pub page: usize,
    pub page_count: usize,
    pub zoom: f32,
    tex: Option<(PathBuf, usize, f32, TextureHandle)>,
    requested: Option<(PathBuf, usize, f32)>,
    pub error: Option<String>,
    pub tool: Tool,
    pub color: Color32,
    pub notes: Annotations,
    drawing: Option<AStroke>,
    text_edit: Option<([f32; 2], String)>,
    /// The bar being played, as fractions of the page: [left, top, right, bottom].
    pub highlight: Option<[f32; 4]>,
    last_scrolled: Option<[f32; 4]>,
}

pub const INK_COLORS: [(Color32, &str); 5] = [
    (Color32::from_rgb(220, 40, 40), "red"),
    (Color32::from_rgb(30, 90, 220), "blue"),
    (Color32::from_rgb(20, 150, 70), "green"),
    (Color32::from_rgb(140, 50, 170), "purple"),
    (Color32::from_rgb(20, 20, 20), "black"),
];

impl BookView {
    pub fn new() -> BookView {
        BookView {
            renderer: Renderer::start(),
            path: None,
            page: 1,
            page_count: 0,
            zoom: 1.0,
            tex: None,
            requested: None,
            error: None,
            tool: Tool::Look,
            color: INK_COLORS[0].0,
            notes: Annotations::load(),
            drawing: None,
            text_edit: None,
            highlight: None,
            last_scrolled: None,
        }
    }

    pub fn open(&mut self, path: PathBuf, page: usize) {
        if self.path.as_ref() != Some(&path) {
            self.page_count = 0;
            self.tex = None;
        }
        self.path = Some(path);
        self.page = page.max(1);
        self.error = None;
    }

    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(r) = self.renderer.rx.try_recv() {
            match r {
                Resp::Page { path, page, scale, img, count } => {
                    self.page_count = count;
                    let tex = ctx.load_texture(format!("pdf-{page}"), img, TextureOptions::LINEAR);
                    self.tex = Some((path, page, scale, tex));
                }
                Resp::Error { msg, .. } => self.error = Some(msg),
            }
        }
    }

    /// Toolbar for annotation tools. Returns true if the page changed.
    pub fn toolbar(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(self.page > 1, egui::Button::new("◀")).clicked() {
                self.page -= 1;
                changed = true;
            }
            let count = self.page_count.max(self.page);
            ui.add(egui::DragValue::new(&mut self.page).range(1..=count.max(1)).prefix("page "));
            ui.label(format!("of {}", if self.page_count > 0 { self.page_count.to_string() } else { "…".into() }));
            if ui.add_enabled(self.page_count == 0 || self.page < self.page_count, egui::Button::new("▶")).clicked() {
                self.page += 1;
                changed = true;
            }
            ui.separator();
            if ui.small_button("−").on_hover_text("Zoom out").clicked() {
                self.zoom = (self.zoom / 1.2).max(0.4);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.small_button("+").on_hover_text("Zoom in").clicked() {
                self.zoom = (self.zoom * 1.2).min(4.0);
            }
            ui.separator();
            ui.selectable_value(&mut self.tool, Tool::Look, "👁 Look");
            ui.selectable_value(&mut self.tool, Tool::Pen, "✏ Pen");
            ui.selectable_value(&mut self.tool, Tool::Highlighter, "Highlighter");
            ui.selectable_value(&mut self.tool, Tool::Text, "T Text");
            for f in 1..=5u8 {
                ui.selectable_value(&mut self.tool, Tool::Finger(f), f.to_string()).on_hover_text(format!("Stamp finger {f}"));
            }
            ui.selectable_value(&mut self.tool, Tool::Eraser, "Eraser");
            for (c, name) in INK_COLORS {
                let sel = self.color == c;
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
                ui.painter().circle_filled(r.center(), 7.0, c);
                if sel {
                    ui.painter().circle_stroke(r.center(), 8.5, Stroke::new(2.0, Color32::WHITE));
                }
                if resp.on_hover_text(name).clicked() {
                    self.color = c;
                }
            }
            if ui.small_button("Undo").clicked() {
                if let Some(path) = &self.path {
                    let k = Annotations::key(path, self.page);
                    if let Some(pn) = self.notes.pages.get_mut(&k) {
                        match pn.order.pop() {
                            Some(0) => {
                                pn.strokes.pop();
                            }
                            Some(_) => {
                                pn.texts.pop();
                            }
                            None => {}
                        }
                        self.notes.save();
                    }
                }
            }
        });
        if changed {
            self.text_edit = None;
        }
        changed
    }

    /// Draw the page filling `ui`'s available area. Returns where the page was clicked with the
    /// Look tool (fractions of the page), so a click on a bar can jump the music there.
    pub fn show(&mut self, ui: &mut Ui) -> Option<[f32; 2]> {
        self.poll(ui.ctx());
        let Some(path) = self.path.clone() else {
            ui.label("No book open.");
            return None;
        };
        if let Some(e) = &self.error {
            ui.colored_label(Color32::LIGHT_RED, format!("Couldn't open this PDF: {e}"));
            return None;
        }
        let avail = ui.available_width() - 14.0;
        let ppp = ui.ctx().pixels_per_point();
        // page widths are usually ~595pt; aim for crisp pixels at the displayed size
        let disp_w = avail * self.zoom;
        let scale = ((disp_w * ppp / 600.0) * 4.0).round() / 4.0;
        let scale = scale.clamp(0.5, 4.0);
        let have = self.tex.as_ref().is_some_and(|(p, pg, s, _)| *p == path && *pg == self.page && (*s - scale).abs() < 0.01);
        let want = (path.clone(), self.page, scale);
        if !have && self.requested.as_ref() != Some(&want) {
            let _ = self.renderer.tx.send(Req { path: path.clone(), page: self.page, scale });
            self.requested = Some(want);
        }
        if !have {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        }
        let Some((_, shown_page, _, tex)) = self.tex.clone() else {
            ui.spinner();
            return None;
        };
        let mut clicked = None;
        let size = tex.size_vec2();
        let w = disp_w;
        let h = w * size.y / size.x;
        let key = Annotations::key(&path, shown_page);
        egui::ScrollArea::both().id_salt("bookscroll").auto_shrink([false, false]).show(ui, |ui| {
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), if self.tool == Tool::Look { Sense::click() } else { Sense::click_and_drag() });
            ui.painter().image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            let to_norm = |p: Pos2| [((p.x - rect.left()) / rect.width()), ((p.y - rect.top()) / rect.height())];
            let from_norm = |n: [f32; 2]| Pos2::new(rect.left() + n[0] * rect.width(), rect.top() + n[1] * rect.height());
            // the bar being played
            if shown_page == self.page {
                if let Some(hl) = self.highlight {
                    let r = Rect::from_min_max(from_norm([hl[0], hl[1]]), from_norm([hl[2], hl[3]]));
                    ui.painter().rect_filled(r, 3.0, Color32::from_rgba_unmultiplied(255, 200, 40, 50));
                    ui.painter().rect_stroke(r, 3.0, Stroke::new(2.0, Color32::from_rgba_unmultiplied(230, 150, 0, 200)), egui::StrokeKind::Inside);
                    // keep it in view when it moves to another line
                    let moved_line = self.last_scrolled.is_none_or(|l| (l[1] - hl[1]).abs() > 0.01);
                    if moved_line {
                        ui.scroll_to_rect(r.expand(30.0), Some(egui::Align::Center));
                        self.last_scrolled = Some(hl);
                    }
                }
            }
            if self.tool == Tool::Look && resp.clicked() {
                clicked = resp.interact_pointer_pos().map(to_norm);
            }
            let pn = self.notes.pages.entry(key.clone()).or_default();
            // interactions
            let mut dirty = false;
            match self.tool {
                Tool::Pen | Tool::Highlighter => {
                    let hl = self.tool == Tool::Highlighter;
                    if resp.drag_started() {
                        let c = if hl { [self.color.r(), self.color.g(), self.color.b(), 70] } else { [self.color.r(), self.color.g(), self.color.b(), 255] };
                        self.drawing = Some(AStroke { pts: vec![], color: c, width: if hl { 0.018 } else { 0.0028 } });
                    }
                    if let (Some(s), Some(p)) = (self.drawing.as_mut(), resp.interact_pointer_pos()) {
                        if resp.dragged() {
                            let n = to_norm(p);
                            if s.pts.last().is_none_or(|l| (l[0] - n[0]).abs() + (l[1] - n[1]).abs() > 0.002) {
                                s.pts.push(n);
                            }
                        }
                    }
                    if resp.drag_stopped() {
                        if let Some(s) = self.drawing.take() {
                            if s.pts.len() > 1 {
                                pn.strokes.push(s);
                                pn.order.push(0);
                                dirty = true;
                            }
                        }
                    }
                }
                Tool::Text => {
                    if resp.clicked() {
                        if let Some(p) = resp.interact_pointer_pos() {
                            self.text_edit = Some((to_norm(p), String::new()));
                        }
                    }
                }
                Tool::Finger(f) => {
                    if resp.clicked() {
                        if let Some(p) = resp.interact_pointer_pos() {
                            pn.texts.push(AText { pos: to_norm(p), text: f.to_string(), color: [self.color.r(), self.color.g(), self.color.b(), 255], size: 0.022 });
                            pn.order.push(1);
                            dirty = true;
                        }
                    }
                }
                Tool::Eraser => {
                    if resp.dragged() || resp.clicked() {
                        if let Some(p) = resp.interact_pointer_pos() {
                            let n = to_norm(p);
                            let before = pn.strokes.len() + pn.texts.len();
                            pn.strokes.retain(|s| !s.pts.iter().any(|q| (q[0] - n[0]).abs() < 0.012 && (q[1] - n[1]).abs() < 0.009));
                            pn.texts.retain(|t| !((t.pos[0] - n[0]).abs() < 0.02 && (t.pos[1] - n[1]).abs() < 0.015));
                            if pn.strokes.len() + pn.texts.len() != before {
                                pn.order = std::iter::repeat_n(0, pn.strokes.len()).chain(std::iter::repeat_n(1, pn.texts.len())).collect();
                                dirty = true;
                            }
                        }
                    }
                }
                Tool::Look => {}
            }
            // paint annotations
            let painter = ui.painter_at(rect);
            let all = pn.strokes.iter().chain(self.drawing.iter());
            for s in all {
                let c = Color32::from_rgba_unmultiplied(s.color[0], s.color[1], s.color[2], s.color[3]);
                let pts: Vec<Pos2> = s.pts.iter().map(|n| from_norm(*n)).collect();
                painter.add(egui::Shape::line(pts, Stroke::new(s.width * rect.width(), c)));
            }
            for t in &pn.texts {
                let c = Color32::from_rgba_unmultiplied(t.color[0], t.color[1], t.color[2], t.color[3]);
                painter.text(from_norm(t.pos), egui::Align2::CENTER_CENTER, &t.text, egui::FontId::proportional(t.size * rect.width()), c);
            }
            if let Some((pos, text)) = self.text_edit.as_mut() {
                let at = from_norm(*pos);
                let mut done = false;
                egui::Area::new(egui::Id::new("anno-text")).fixed_pos(at).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        let r = ui.add(egui::TextEdit::singleline(text).hint_text("Type a note, Enter to place").desired_width(220.0));
                        r.request_focus();
                        if r.lost_focus() {
                            done = true;
                        }
                    });
                });
                if done {
                    let (pos, text) = self.text_edit.take().unwrap();
                    if !text.trim().is_empty() {
                        pn.texts.push(AText { pos, text, color: [self.color.r(), self.color.g(), self.color.b(), 255], size: 0.016 });
                        pn.order.push(1);
                        dirty = true;
                    }
                }
            }
            if dirty {
                self.notes.save();
            }
            if self.drawing.is_some() {
                ui.ctx().request_repaint();
            }
        });
        clicked
    }
}

// ------------------------------------------------------------------ Audiveris (optical music recognition)

pub fn find_audiveris(configured: &str) -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = vec![];
    if !configured.trim().is_empty() {
        c.push(PathBuf::from(configured.trim().trim_matches('"')));
    }
    for base in [r"C:\Program Files\Audiveris", r"C:\Program Files (x86)\Audiveris"] {
        c.push(PathBuf::from(base).join("Audiveris.exe"));
        c.push(PathBuf::from(base).join("bin").join("Audiveris.bat"));
    }
    if let Some(local) = dirs::data_local_dir() {
        c.push(local.join("Programs").join("Audiveris").join("Audiveris.exe"));
    }
    c.push(PathBuf::from("/opt/audiveris/bin/Audiveris"));
    c.into_iter().find(|p| p.exists())
}
