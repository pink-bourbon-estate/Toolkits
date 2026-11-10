//! The one practice screen: a browser on the left (course, books → pages → exercises,
//! repertoire, technique, note reading, your files) and whatever is open on the right, with a
//! clickable path at the top and ◀ ▶ to the previous / next exercise.

use super::*;
use crate::model::Section;
use crate::ui::convert;
use std::path::Path;

/// Something that can be open in the main area.
#[derive(Clone, PartialEq, Debug)]
pub enum Item {
    /// A chapter of the course (overview, pieces, warm-ups).
    Unit(usize),
    /// A numbered Russian School piece.
    BookPiece(u32),
    /// A page of a PDF.
    Page { pdf: PathBuf, page: usize },
    /// An exercise inside a typeset PDF (found from its heading).
    Section { pdf: PathBuf, idx: usize },
    /// Any piece by id (repertoire, your files, a whole PDF).
    Whole { id: String },
    Tech(TechSpec),
    Reading(u8),
    Find,
}

pub fn page_key(pdf: &Path, page: usize) -> String {
    format!("{}#{}", pdf.display(), page)
}

fn pieces_on_page(b: BookId, page: usize) -> Vec<books::PieceRef> {
    match b {
        BookId::RussianSchool1a => books::RS1A.iter().filter(|r| r.1 == page).copied().collect(),
        BookId::RussianSchool1b => books::RS1B.iter().filter(|r| r.1 == page).copied().collect(),
        _ => vec![],
    }
}

fn all_book_numbers() -> Vec<u32> {
    books::RS1A.iter().chain(books::RS1B.iter()).map(|r| r.0).collect()
}

fn unit_label(i: usize, title: &str) -> String {
    format!("{}. {title}", i + 1)
}

impl App {
    // ------------------------------------------------------------------ the PDFs you have

    /// Look through the music folders for PDFs (in the background – Google Drive can be slow).
    pub(super) fn rescan_pdfs(&mut self) {
        let folders: Vec<PathBuf> = self.progress.scan_folder.split(';').map(|f| f.trim().trim_matches('"')).filter(|f| !f.is_empty()).map(PathBuf::from).collect();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut v: Vec<String> = vec![];
            for f in folders {
                for p in scan(&f) {
                    if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")) {
                        v.push(p.display().to_string());
                    }
                }
            }
            let _ = tx.send(v);
        });
        self.pdf_scan_rx = Some(rx);
    }

    pub(super) fn poll_pdf_scan(&mut self) {
        let Some(rx) = &self.pdf_scan_rx else { return };
        if let Ok(v) = rx.try_recv() {
            self.pdf_scan_rx = None;
            if !v.is_empty() || self.progress.pdf_index.is_empty() {
                self.progress.pdf_index = v;
            }
            // books found in the folders
            for p in &self.progress.pdf_index {
                let name = Path::new(p).file_name().and_then(|n| n.to_str()).unwrap_or("");
                for b in BookId::ALL {
                    if self.progress.book_path(b).is_none() && b.matches(name) {
                        self.progress.books.insert(b, p.clone());
                    }
                }
            }
            self.progress.save();
        }
    }

    fn all_pdfs(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = self.progress.pdf_index.iter().map(PathBuf::from).collect();
        for b in BookId::ALL {
            if let Some(p) = self.progress.book_path(b) {
                if !v.contains(&p) {
                    v.push(p);
                }
            }
        }
        v
    }

    fn book_of(&self, pdf: &Path) -> Option<BookId> {
        BookId::ALL.iter().copied().find(|b| self.progress.book_path(*b).as_deref() == Some(pdf))
    }

    fn pdf_title(&self, pdf: &Path) -> String {
        match self.book_of(pdf) {
            Some(b) => b.short().to_string(),
            None => convert::nice_title(pdf),
        }
    }

    /// The whole-PDF conversion, if there is one.
    fn whole_piece(&self, pdf: &Path) -> Option<&Piece> {
        let id = convert::piece_id(pdf, None, 0);
        self.progress.library.iter().find(|p| p.id == id)
    }

    fn sections_of(&self, pdf: &Path) -> Vec<Section> {
        match self.whole_piece(pdf) {
            Some(p) => {
                let s = p.src.as_ref().map(|s| s.sections.clone()).unwrap_or_default();
                if s.is_empty() {
                    let page = p.src.as_ref().and_then(|s| s.bars.first()).map(|b| b.page).unwrap_or(1);
                    vec![Section { beat: 0.0, end: p.end_beat(), page, title: p.title.clone() }]
                } else {
                    s
                }
            }
            None => vec![],
        }
    }

    fn section_piece(&self, pdf: &Path, idx: usize) -> Option<Piece> {
        let whole = self.whole_piece(pdf)?;
        let secs = self.sections_of(pdf);
        let sct = secs.get(idx)?;
        if secs.len() == 1 {
            return Some(whole.clone());
        }
        let end = if sct.end.is_finite() && sct.end < f64::MAX / 2.0 { sct.end } else { whole.end_beat() + 1.0 };
        let title = if sct.title.is_empty() { format!("{} p.{}", self.pdf_title(pdf), sct.page) } else { sct.title.clone() };
        Some(whole.slice(sct.beat, end, format!("{}#{idx}", whole.id), title))
    }

    /// The playable version of a Russian School piece, if its page has been converted.
    fn playable_for(&self, n: u32) -> Option<Piece> {
        if let Some(id) = self.progress.book_links.get(&n) {
            if let Some(p) = self.find_piece(id) {
                return Some(p);
            }
        }
        let (b, r) = books::piece_ref(n)?;
        let path = self.progress.book_path(b)?;
        let ids = self.progress.page_conv.get(&page_key(&path, r.1))?;
        let parts: Vec<Piece> = ids.iter().filter_map(|id| self.find_piece(id)).collect();
        if parts.is_empty() {
            return None;
        }
        let on_page: Vec<u32> = pieces_on_page(b, r.1).iter().map(|x| x.0).collect();
        let pos = on_page.iter().position(|x| *x == n).unwrap_or(0);
        if parts.len() > 1 && pos < parts.len() {
            return Some(parts[pos].clone());
        }
        let mut p = parts[0].clone();
        if on_page.len() > 1 {
            p.notes_text = format!(
                "Audiveris read page {} as one piece, so this plays everything on the page (nos. {}). Drag across the bars at the top to loop just no. {n}.",
                r.1,
                on_page.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ")
            );
        }
        Some(p)
    }

    fn converting(&self) -> bool {
        self.conv.as_ref().is_some_and(|c| !c.finished)
    }

    // ------------------------------------------------------------------ opening things

    pub(super) fn open_item(&mut self, item: Item) {
        self.audio.send(Cmd::Pause);
        if self.metro_only {
            self.stop_metronome_only();
        }
        self.tab = Tab::Main;
        self.bookview.highlight = None;
        self.item = Some(item.clone());
        match item {
            Item::Unit(i) => self.unit_sel = i.min(units().len() - 1),
            Item::BookPiece(n) => self.open_book_piece(n),
            Item::Page { pdf, page } => {
                self.current_book = self.book_of(&pdf);
                self.bookview.open(pdf, page);
            }
            Item::Section { pdf, idx } => match self.section_piece(&pdf, idx) {
                Some(p) => {
                    self.open_piece(p, false, None);
                    self.item = Some(Item::Section { pdf, idx });
                }
                None => {
                    // not converted yet: do the whole PDF now, then come back here
                    self.bookview.open(pdf.clone(), 1);
                    self.cur = None;
                    self.book_only = None;
                    if !self.converting() {
                        self.conv_open = true;
                        self.start_convert(vec![Job { pdf, pages: None }], true);
                    }
                }
            },
            Item::Whole { id } => {
                if let Some(p) = self.find_piece(&id) {
                    let link = self.progress.book_links.iter().find(|(_, v)| **v == p.id).map(|(n, _)| *n);
                    self.open_piece(p, false, link);
                    self.item = Some(Item::Whole { id });
                }
            }
            Item::Tech(spec) => {
                self.tech = spec;
                let p = generate(&spec);
                self.open_piece(p, true, None);
                self.tech_mode = true;
                self.item = Some(Item::Tech(spec));
            }
            Item::Reading(l) => self.trainer = Trainer::new(l),
            Item::Find => {}
        }
    }

    /// A Russian School piece: the playable version when its page has been converted; otherwise
    /// the page itself with the Russian-School steps, while the page is converted in the background.
    fn open_book_piece(&mut self, n: u32) {
        let Some((b, r)) = books::piece_ref(n) else { return };
        if let Some(p) = self.playable_for(n) {
            self.open_piece(p, false, Some(n));
            self.item = Some(Item::BookPiece(n));
            return;
        }
        let path = self.progress.book_path(b);
        if let Some(p) = &path {
            self.bookview.open(p.clone(), r.1);
        }
        self.current_book = Some(b);
        let dummy = Piece {
            notes: vec![Note::new(0.0, 1.0, 60, Hand::Right, None), Note::new(0.0, 1.0, 48, Hand::Left, None)],
            ..reading::question_piece(60)
        };
        self.cur = None;
        self.book_only = Some(BookOnly { number: n, rungs: ladder(&dummy, false), bpm: 72.0, beats: 4, reps_done: 0, reps_target: 3 });
        self.plan_msg.clear();
        // make the page playable by itself, if Audiveris is there and it hasn't failed on this page
        if let Some(p) = path {
            let key = page_key(&p, r.1);
            if !self.converting() && !self.page_failed.contains(&key) && find_audiveris(&self.progress.audiveris).is_some() {
                self.conv_target = Some(n);
                self.conv_open = true;
                self.start_convert(vec![Job { pdf: p, pages: Some((r.1, r.1)) }], true);
            }
        }
    }

    // ------------------------------------------------------------------ previous / next, and the path

    fn neighbour(&self, it: &Item, d: i32) -> Option<Item> {
        let step = |i: usize, len: usize| -> Option<usize> {
            let j = i as i32 + d;
            (j >= 0 && (j as usize) < len).then_some(j as usize)
        };
        match it {
            Item::Unit(i) => step(*i, units().len()).map(Item::Unit),
            Item::BookPiece(n) => {
                let all = all_book_numbers();
                let i = all.iter().position(|x| x == n)?;
                step(i, all.len()).map(|j| Item::BookPiece(all[j]))
            }
            Item::Page { pdf, page } => {
                let max = if self.bookview.page_count > 0 { self.bookview.page_count } else { usize::MAX };
                step(page - 1, max).map(|j| Item::Page { pdf: pdf.clone(), page: j + 1 })
            }
            Item::Section { pdf, idx } => step(*idx, self.sections_of(pdf).len()).map(|j| Item::Section { pdf: pdf.clone(), idx: j }),
            Item::Whole { id } => {
                let list: Vec<String> = if id.starts_with("rep-") {
                    let mut v: Vec<&Piece> = self.builtins.iter().filter(|p| p.id.starts_with("rep-")).collect();
                    v.sort_by_key(|p| (crate::repertoire::grade_of(&p.id).unwrap_or(9), p.title.clone()));
                    v.iter().map(|p| p.id.clone()).collect()
                } else {
                    self.progress.library.iter().filter(|p| !p.id.starts_with("pdf-")).map(|p| p.id.clone()).collect()
                };
                let i = list.iter().position(|x| x == id)?;
                step(i, list.len()).map(|j| Item::Whole { id: list[j].clone() })
            }
            Item::Reading(l) => step(*l as usize, reading::LEVELS.len()).map(|j| Item::Reading(j as u8)),
            Item::Tech(_) | Item::Find => None,
        }
    }

    fn item_name(&self, it: &Item) -> String {
        match it {
            Item::Unit(i) => units().get(*i).map(|u| unit_label(*i, u.title)).unwrap_or_default(),
            Item::BookPiece(n) => books::piece_ref(*n).map(|(_, r)| format!("{n}. {}", r.2)).unwrap_or_default(),
            Item::Page { page, .. } => format!("page {page}"),
            Item::Section { pdf, idx } => self.sections_of(pdf).get(*idx).map(|s| if s.title.is_empty() { format!("p.{}", s.page) } else { s.title.clone() }).unwrap_or_default(),
            Item::Whole { id } => self.find_piece(id).map(|p| p.title).unwrap_or_default(),
            Item::Tech(s) => s.title(),
            Item::Reading(l) => reading::LEVELS.get(*l as usize).map(|x| x.0.to_string()).unwrap_or_default(),
            Item::Find => "Find music".into(),
        }
    }

    /// The path to what's open: each part can be clicked to go back to it.
    fn crumbs(&self) -> Vec<(String, Option<Item>)> {
        let Some(it) = &self.item else { return vec![] };
        let us = units();
        match it {
            Item::Unit(_) => vec![("Course".into(), None), (self.item_name(it), None)],
            Item::BookPiece(n) => {
                let Some((b, r)) = books::piece_ref(*n) else { return vec![] };
                let unit = us.iter().position(|u| u.pieces.contains(n));
                let mut v = vec![];
                match self.progress.book_path(b) {
                    Some(p) => {
                        v.push((b.short().to_string(), Some(Item::Page { pdf: p.clone(), page: 1 })));
                        v.push((format!("p.{}", r.1), Some(Item::Page { pdf: p, page: r.1 })));
                    }
                    None => v.push((b.short().to_string(), None)),
                }
                v.push((self.item_name(it), None));
                if let Some(u) = unit {
                    v.push((format!("(course: {})", unit_label(u, us[u].title)), Some(Item::Unit(u))));
                }
                v
            }
            Item::Page { pdf, page } => vec![(self.pdf_title(pdf), Some(Item::Page { pdf: pdf.clone(), page: 1 })), (format!("p.{page}"), None)],
            Item::Section { pdf, idx } => {
                let page = self.sections_of(pdf).get(*idx).map(|s| s.page as usize).unwrap_or(1);
                vec![
                    (self.pdf_title(pdf), Some(Item::Page { pdf: pdf.clone(), page: 1 })),
                    (format!("p.{page}"), Some(Item::Page { pdf: pdf.clone(), page })),
                    (self.item_name(it), None),
                ]
            }
            Item::Whole { id } => {
                let name = self.item_name(it);
                if let Some(g) = crate::repertoire::grade_of(id) {
                    vec![("Repertoire".into(), None), (format!("Grade {g}"), None), (name, None)]
                } else {
                    vec![("Your files".into(), None), (name, None)]
                }
            }
            Item::Tech(_) => vec![("Scales & technique".into(), None), (self.item_name(it), None)],
            Item::Reading(_) => vec![("Note reading".into(), None), (self.item_name(it), None)],
            Item::Find => vec![("Find music".into(), None)],
        }
    }

    fn ui_crumbs(&mut self, ui: &mut egui::Ui) {
        let mut go: Option<Item> = None;
        ui.horizontal(|ui| {
            if ui.selectable_label(self.side_open, "☰").on_hover_text("Show or hide the list on the left").clicked() {
                self.side_open = !self.side_open;
            }
            if let Some(it) = self.item.clone() {
                let prev = self.neighbour(&it, -1);
                let next = self.neighbour(&it, 1);
                let pr = ui.add_enabled(prev.is_some(), egui::Button::new(RichText::new("◀").size(16.0)));
                if let Some(p) = &prev {
                    if pr.on_hover_text(format!("Previous: {}", self.item_name(p))).clicked() {
                        go = prev.clone();
                    }
                }
                let nr = ui.add_enabled(next.is_some(), egui::Button::new(RichText::new("▶").size(16.0)));
                if let Some(n) = &next {
                    if nr.on_hover_text(format!("Next: {}", self.item_name(n))).clicked() {
                        go = next.clone();
                    }
                }
                ui.add_space(6.0);
                for (i, (label, target)) in self.crumbs().into_iter().enumerate() {
                    if i > 0 && !label.starts_with('(') {
                        ui.label(RichText::new("›").weak());
                    }
                    match target {
                        Some(t) => {
                            if ui.link(RichText::new(&label).size(15.0)).clicked() {
                                go = Some(t);
                            }
                        }
                        None => {
                            ui.label(RichText::new(&label).size(15.0).strong());
                        }
                    }
                }
            }
        });
        if let Some(g) = go {
            self.open_item(g);
        }
    }

    // ------------------------------------------------------------------ the screen

    pub(super) fn ui_main(&mut self, ctx: &egui::Context) {
        let mut go: Option<Item> = None;
        if self.side_open {
            egui::SidePanel::left("browser").resizable(true).default_width(310.0).width_range(220.0..=520.0).show(ctx, |ui| {
                go = self.ui_sidebar(ui);
            });
        }
        if let Some(g) = go {
            self.open_item(g);
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui_crumbs(ui);
            ui.separator();
            match self.item.clone() {
                Some(Item::Unit(i)) => self.ui_unit(ui, i),
                Some(Item::Page { .. }) => self.ui_book_browse(ui),
                Some(Item::Reading(_)) => self.ui_reading(ui),
                Some(Item::Find) => self.ui_find(ui),
                Some(_) => self.ui_player(ui),
                None => self.ui_unit(ui, self.unit_sel),
            }
        });
    }

    // ------------------------------------------------------------------ left: the browser

    fn ui_sidebar(&mut self, ui: &mut egui::Ui) -> Option<Item> {
        let mut go: Option<Item> = None;
        let cur = self.item.clone();
        let sel = |it: &Item| cur.as_ref() == Some(it);
        ui.add_space(4.0);
        ui.add(egui::TextEdit::singleline(&mut self.side_filter).hint_text("🔎 Search pieces, books, exercises…").desired_width(f32::INFINITY));
        ui.add_space(4.0);
        let us = units();
        let filter = self.side_filter.trim().to_lowercase();
        let pdfs = self.all_pdfs();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if !filter.is_empty() {
                // one flat list of everything that matches
                let hit = |s: &str| s.to_lowercase().contains(&filter);
                for n in all_book_numbers() {
                    let it = Item::BookPiece(n);
                    let name = self.item_name(&it);
                    if hit(&name) && ui.selectable_label(sel(&it), format!("📘 {name}")).clicked() {
                        go = Some(it);
                    }
                }
                for pdf in &pdfs {
                    let t = self.pdf_title(pdf);
                    if hit(&t) && ui.selectable_label(false, format!("📄 {t}")).clicked() {
                        go = Some(Item::Page { pdf: pdf.clone(), page: 1 });
                    }
                    for (i, s) in self.sections_of(pdf).iter().enumerate() {
                        if hit(&s.title) && ui.selectable_label(false, format!("    § {} (p.{})", s.title, s.page)).clicked() {
                            go = Some(Item::Section { pdf: pdf.clone(), idx: i });
                        }
                    }
                }
                for p in self.builtins.iter().chain(self.progress.library.iter().filter(|p| !p.id.starts_with("pdf-"))) {
                    if (hit(&p.title) || hit(&p.composer)) && ui.selectable_label(false, format!("🎼 {}", p.title)).clicked() {
                        go = Some(Item::Whole { id: p.id.clone() });
                    }
                }
                return;
            }

            // ---- continue where you are
            let u = &us[self.unit_sel.min(us.len() - 1)];
            if let Some(n) = u.pieces.iter().copied().find(|n| !self.progress.done(&book_piece_id(*n))) {
                if ui.button(RichText::new(format!("▶ Continue: {}", self.item_name(&Item::BookPiece(n)))).strong().color(accent())).clicked() {
                    go = Some(Item::BookPiece(n));
                }
                ui.add_space(4.0);
            }

            // ---- the course
            egui::CollapsingHeader::new(RichText::new("📘 Course – Russian School + Nahre Sol").strong()).default_open(true).show(ui, |ui| {
                let suggested = current_unit(&self.progress);
                let mut last_book = None;
                for (i, u) in us.iter().enumerate() {
                    if last_book != Some(u.book) {
                        ui.label(RichText::new(if u.book == BookId::RussianSchool1a { "Book 1a" } else { "Book 1b" }).small().weak());
                        last_book = Some(u.book);
                    }
                    let done = u.pieces.iter().filter(|n| self.progress.done(&book_piece_id(**n))).count();
                    let mark = if done == u.pieces.len() { "✔ " } else if i == suggested { "▶ " } else { "" };
                    let open_here = matches!(&cur, Some(Item::BookPiece(n)) if u.pieces.contains(n)) || matches!(&cur, Some(Item::Unit(x)) if *x == i);
                    egui::CollapsingHeader::new(format!("{mark}{}   {done}/{}", unit_label(i, u.title), u.pieces.len()))
                        .id_salt(("unit", i))
                        .default_open(i == self.unit_sel)
                        .open(if open_here { Some(true) } else { None })
                        .show(ui, |ui| {
                            if ui.selectable_label(sel(&Item::Unit(i)), "ⓘ Overview and today's plan").clicked() {
                                go = Some(Item::Unit(i));
                            }
                            for &n in &u.pieces {
                                let it = Item::BookPiece(n);
                                let d = self.progress.done(&book_piece_id(n));
                                let txt = RichText::new(format!("{}{}", if d { "✔ " } else { "" }, self.item_name(&it)));
                                if ui.selectable_label(sel(&it), if d { txt.color(green()) } else { txt }).clicked() {
                                    go = Some(it);
                                }
                            }
                            for t in &u.tasks {
                                if let Task::Tech { role, spec, .. } = t {
                                    let it = Item::Tech(*spec);
                                    if ui.selectable_label(sel(&it), RichText::new(format!("🎹 {role}: {}", spec.title())).weak()).clicked() {
                                        go = Some(it);
                                    }
                                }
                            }
                        });
                }
            });

            // ---- every PDF: book → pages → exercises
            egui::CollapsingHeader::new(RichText::new(format!("📚 Books & sheet music ({})", pdfs.len())).strong()).default_open(true).show(ui, |ui| {
                if pdfs.is_empty() {
                    ui.label(RichText::new(if self.pdf_scan_rx.is_some() { "Looking through your folders…" } else { "No PDFs found yet – add your music folders in Settings." }).small().weak());
                }
                // group by folder
                let roots: Vec<PathBuf> = self.progress.scan_folder.split(';').map(|f| PathBuf::from(f.trim().trim_matches('"'))).collect();
                let mut groups: std::collections::BTreeMap<String, Vec<PathBuf>> = Default::default();
                for p in &pdfs {
                    let parent = p.parent().unwrap_or(Path::new(""));
                    let label = roots
                        .iter()
                        .filter_map(|r| parent.strip_prefix(r).ok().map(|rel| (r, rel)))
                        .map(|(r, rel)| {
                            let root = r.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                            if rel.as_os_str().is_empty() { root } else { format!("{root} / {}", rel.display()) }
                        })
                        .next()
                        .unwrap_or_else(|| parent.file_name().and_then(|n| n.to_str()).unwrap_or("Other").to_string());
                    groups.entry(label).or_default().push(p.clone());
                }
                let cur_pdf: Option<PathBuf> = match &cur {
                    Some(Item::Page { pdf, .. }) | Some(Item::Section { pdf, .. }) => Some(pdf.clone()),
                    _ => None,
                };
                for (folder, mut list) in groups {
                    list.sort_by_key(|p| self.pdf_title(p).to_lowercase());
                    let has_cur = cur_pdf.as_ref().is_some_and(|c| list.contains(c));
                    egui::CollapsingHeader::new(format!("📁 {folder} ({})", list.len())).id_salt(("folder", &folder)).open(if has_cur { Some(true) } else { None }).show(ui, |ui| {
                        for pdf in list {
                            self.sidebar_pdf(ui, &pdf, &cur, &mut go);
                        }
                    });
                }
            });

            // ---- repertoire by grade
            egui::CollapsingHeader::new(RichText::new("🎼 Repertoire (graded)").strong()).show(ui, |ui| {
                for g in 1..=8u8 {
                    let list: Vec<&Piece> = self.builtins.iter().filter(|p| crate::repertoire::grade_of(&p.id) == Some(g)).collect();
                    if list.is_empty() {
                        continue;
                    }
                    egui::CollapsingHeader::new(format!("Grade {g} ({})", list.len())).id_salt(("grade", g)).show(ui, |ui| {
                        for p in list {
                            let it = Item::Whole { id: p.id.clone() };
                            let d = self.progress.done(&p.id);
                            if ui.selectable_label(sel(&it), format!("{}{}", if d { "✔ " } else { "" }, p.title)).on_hover_text(&p.composer).clicked() {
                                go = Some(it);
                            }
                        }
                    });
                }
            });

            // ---- technique
            egui::CollapsingHeader::new(RichText::new("🎹 Scales & technique").strong()).show(ui, |ui| {
                if ui.selectable_label(false, "✚ Build your own (every scale, mode, pattern…)").clicked() {
                    go = Some(Item::Tech(self.tech));
                }
                for (name, spec) in tech_presets() {
                    let it = Item::Tech(spec);
                    if ui.selectable_label(sel(&it), name).clicked() {
                        go = Some(it);
                    }
                }
            });

            // ---- note reading
            egui::CollapsingHeader::new(RichText::new("👁 Note reading").strong()).show(ui, |ui| {
                for (i, (name, desc)) in reading::LEVELS.iter().enumerate() {
                    let it = Item::Reading(i as u8);
                    if ui.selectable_label(sel(&it), *name).on_hover_text(*desc).clicked() {
                        go = Some(it);
                    }
                }
            });

            // ---- MIDI / MusicXML you added
            let mine: Vec<(String, String)> = self.progress.library.iter().filter(|p| !p.id.starts_with("pdf-") && !p.id.starts_with("omr-")).map(|p| (p.id.clone(), p.title.clone())).collect();
            let mut add_files: Option<Vec<PathBuf>> = None;
            egui::CollapsingHeader::new(RichText::new(format!("📥 Your files ({})", mine.len())).strong()).show(ui, |ui| {
                ui.label(RichText::new("Drop MIDI or MusicXML files onto the window to add them.").small().weak());
                #[cfg(windows)]
                if ui.small_button("Add a file…").clicked() {
                    add_files = rfd::FileDialog::new().add_filter("Music", &["mid", "midi", "musicxml", "xml", "mxl", "pdf"]).pick_files();
                }
                for (id, title) in mine {
                    let it = Item::Whole { id };
                    if ui.selectable_label(sel(&it), title).clicked() {
                        go = Some(it);
                    }
                }
            });
            if let Some(paths) = add_files {
                self.import_paths(paths);
            }
            if ui.selectable_label(sel(&Item::Find), RichText::new("🔎 Find more music online").strong()).clicked() {
                go = Some(Item::Find);
            }
        });
        go
    }

    /// One PDF in the browser: its pages and the exercises on them.
    fn sidebar_pdf(&self, ui: &mut egui::Ui, pdf: &Path, cur: &Option<Item>, go: &mut Option<Item>) {
        let sel = |it: &Item| cur.as_ref() == Some(it);
        let title = self.pdf_title(pdf);
        let book = self.book_of(pdf);
        let secs = self.sections_of(pdf);
        let here = matches!(cur, Some(Item::Page { pdf: p, .. }) | Some(Item::Section { pdf: p, .. }) if p == pdf);
        let mark = if !secs.is_empty() || matches!(book, Some(BookId::RussianSchool1a | BookId::RussianSchool1b)) { "🔊 " } else { "📄 " };
        egui::CollapsingHeader::new(format!("{mark}{title}")).id_salt(("pdf", pdf)).open(if here { Some(true) } else { None }).show(ui, |ui| {
            let open = Item::Page { pdf: pdf.to_path_buf(), page: 1 };
            if ui.selectable_label(false, RichText::new("Open the book at page 1").weak()).clicked() {
                *go = Some(open);
            }
            match book {
                Some(b @ (BookId::RussianSchool1a | BookId::RussianSchool1b)) => {
                    let list = if b == BookId::RussianSchool1a { books::RS1A } else { books::RS1B };
                    let mut last_page = 0;
                    for r in list {
                        if r.1 != last_page {
                            last_page = r.1;
                            let pit = Item::Page { pdf: pdf.to_path_buf(), page: r.1 };
                            if ui.selectable_label(sel(&pit), RichText::new(format!("p.{}", r.1)).small().weak()).clicked() {
                                *go = Some(pit);
                            }
                        }
                        let it = Item::BookPiece(r.0);
                        let d = self.progress.done(&book_piece_id(r.0));
                        if ui.selectable_label(sel(&it), format!("    {}{}. {}", if d { "✔ " } else { "" }, r.0, r.2)).clicked() {
                            *go = Some(it);
                        }
                    }
                }
                _ if !secs.is_empty() => {
                    let mut last_page = 0;
                    for (i, s) in secs.iter().enumerate() {
                        if s.page != last_page {
                            last_page = s.page;
                            let pit = Item::Page { pdf: pdf.to_path_buf(), page: s.page as usize };
                            if ui.selectable_label(sel(&pit), RichText::new(format!("p.{}", s.page)).small().weak()).clicked() {
                                *go = Some(pit);
                            }
                        }
                        let it = Item::Section { pdf: pdf.to_path_buf(), idx: i };
                        let t = if s.title.is_empty() { format!("Exercise {}", i + 1) } else { s.title.clone() };
                        if ui.selectable_label(sel(&it), format!("    {t}")).clicked() {
                            *go = Some(it);
                        }
                    }
                }
                _ => {
                    if ui.selectable_label(false, "🔊 Make it playable").on_hover_text("Typeset sheet music is read in a second or two; scans use Audiveris").clicked() {
                        *go = Some(Item::Section { pdf: pdf.to_path_buf(), idx: 0 });
                    }
                }
            }
        });
    }

    // ------------------------------------------------------------------ a course chapter

    fn ui_unit(&mut self, ui: &mut egui::Ui, i: usize) {
        let us = units();
        let i = i.min(us.len() - 1);
        let u = &us[i];
        let mut go: Option<Item> = None;
        let mut do_install = false;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if !crate::install::is_installed() && !self.progress.install_prompt_dismissed {
                egui::Frame::new().fill(Color32::from_rgb(28, 60, 44)).corner_radius(8.0).inner_margin(8.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Install PianoCraft: it gets its own Start menu entry and can update itself with one click.");
                        if ui.add_enabled(!self.busy_update, egui::Button::new(RichText::new("Install").strong())).clicked() {
                            do_install = true;
                        }
                        if ui.small_button("Not now").clicked() {
                            self.progress.install_prompt_dismissed = true;
                            self.progress.save();
                        }
                    });
                });
                ui.add_space(6.0);
            }
            let book_path = self.progress.book_path(u.book);
            if book_path.is_none() {
                egui::Frame::new().fill(Color32::from_rgb(70, 50, 20)).corner_radius(8.0).inner_margin(8.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(format!("I can't find your copy of {} yet. Add the folder it's in under Settings → Your music folders.", u.book.title()));
                });
                ui.add_space(6.0);
            }
            ui.horizontal(|ui| {
                ui.heading(unit_label(i, u.title));
                ui.label(RichText::new(format!("{} · from p. {}", u.book.short(), u.page)).weak());
                if let Some(p) = &book_path {
                    if ui.button("📖 Open the chapter").clicked() {
                        go = Some(Item::Page { pdf: p.clone(), page: u.page });
                    }
                }
            });
            ui.columns(2, |cols| {
                card(&mut cols[0], |ui| {
                    ui.label(RichText::new("Russian School").strong().color(Color32::from_rgb(230, 120, 110)));
                    ui.label(u.russian);
                });
                card(&mut cols[1], |ui| {
                    ui.label(RichText::new("Nahre Sol").strong().color(Color32::from_rgb(120, 200, 230)));
                    ui.label(u.nahre);
                });
            });
            ui.add_space(8.0);
            let next_piece = u.pieces.iter().copied().find(|n| !self.progress.done(&book_piece_id(*n)));
            ui.horizontal(|ui| {
                ui.label(RichText::new("1 · Pieces").size(17.0).strong());
                ui.label(RichText::new("one new piece at a time, and keep polishing the last one").small().weak());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if next_piece.is_some() && ui.small_button("I know this whole chapter ✔").on_hover_text("Tick every piece in this chapter and move on").clicked() {
                        for n in &u.pieces {
                            self.progress.ticks.insert(book_piece_id(*n), curriculum::today());
                        }
                        self.progress.save();
                        go = Some(Item::Unit((i + 1).min(us.len() - 1)));
                    }
                });
            });
            for &n in &u.pieces {
                let Some((_, r)) = books::piece_ref(n) else { continue };
                let pid = book_piece_id(n);
                let pp = self.progress.pieces.get(&pid).cloned().unwrap_or_default();
                let done = self.progress.done(&pid);
                let is_next = Some(n) == next_piece;
                let frame = if is_next { egui::Frame::new().fill(Color32::from_rgb(34, 44, 58)).corner_radius(6.0).inner_margin(6.0) } else { egui::Frame::new().inner_margin(egui::Margin::symmetric(6, 2)) };
                frame.show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let title = format!("{n}. {}", r.2);
                        let t = if done { RichText::new(format!("✔ {title}")).color(green()) } else if is_next { RichText::new(format!("▶ {title}")).strong().color(accent()) } else { RichText::new(title) };
                        ui.label(t.size(15.0));
                        if !r.3.is_empty() {
                            ui.label(RichText::new(r.3).small().weak());
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if !done && ui.small_button("I know it ✔").clicked() {
                                self.progress.ticks.insert(pid.clone(), curriculum::today());
                                self.progress.save();
                            }
                            if ui.button(if is_next { "Practise ▶" } else { "Open" }).clicked() {
                                go = Some(Item::BookPiece(n));
                            }
                            ui.label(RichText::new(format!("p.{}", r.1)).small().weak());
                            if !done && (pp.nailed_count > 0 || pp.rung > 0) {
                                ui.label(RichText::new(format!("step {}", pp.rung + 1)).small().color(accent()));
                            }
                        });
                    });
                });
            }
            ui.add_space(10.0);
            ui.label(RichText::new("2 · Warm-up, theory and ear").size(17.0).strong());
            for task in &u.tasks {
                ui.horizontal(|ui| {
                    match task {
                        Task::Tech { role, spec, original } => {
                            let piece_id = generate(spec).id;
                            let st = self.progress.pieces.get(&piece_id).cloned().unwrap_or_default();
                            ui.label(RichText::new(format!("🎹 {role}:")).weak());
                            ui.label(RichText::new(spec.title()).strong());
                            ui.label(RichText::new(spec.subtitle()).small().weak());
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("Start ▶").clicked() {
                                    go = Some(Item::Tech(*spec));
                                }
                                if let Some((b, page, what)) = original {
                                    if let Some(p) = self.progress.book_path(*b) {
                                        if ui.small_button(format!("📖 {}", b.short())).on_hover_text(format!("{what} – p. {page}")).clicked() {
                                            go = Some(Item::Page { pdf: p, page: *page });
                                        }
                                    }
                                }
                                if st.done {
                                    ui.label(RichText::new("✔").color(green()));
                                }
                            });
                        }
                        Task::Theory { id, book, page, title, what, reading } => {
                            let done = self.progress.done(id);
                            ui.label(RichText::new("📖 Read:").weak());
                            ui.label(RichText::new(*title).strong().color(if done { green() } else { Color32::WHITE })).on_hover_text(*what);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if !done && ui.small_button("Done ✔").clicked() {
                                    self.progress.ticks.insert(id.to_string(), curriculum::today());
                                    self.progress.save();
                                }
                                if let Some(l) = reading {
                                    if ui.small_button("Reading trainer ▶").clicked() {
                                        go = Some(Item::Reading(*l));
                                    }
                                }
                                if let Some(p) = self.progress.book_path(*book) {
                                    if ui.small_button(format!("Open p.{page}")).clicked() {
                                        go = Some(Item::Page { pdf: p, page: *page });
                                    }
                                }
                            });
                        }
                        Task::Ear { id, title, what } => {
                            let done = self.progress.done(id);
                            ui.label(RichText::new("👂 Ear:").weak());
                            ui.label(RichText::new(*title).strong().color(if done { green() } else { Color32::WHITE }));
                            ui.label(RichText::new(*what).small());
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if !done && ui.small_button("Done ✔").clicked() {
                                    self.progress.ticks.insert(id.to_string(), curriculum::today());
                                    self.progress.save();
                                }
                            });
                        }
                        Task::Builtin { id, why } => {
                            if let Some(p) = self.find_piece(id) {
                                ui.label(RichText::new("🎼 Extra piece:").weak());
                                ui.label(RichText::new(&p.title).strong());
                                ui.label(RichText::new(*why).small());
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.button("Start ▶").clicked() {
                                        go = Some(Item::Whole { id: id.to_string() });
                                    }
                                });
                            }
                        }
                    }
                });
            }
        });
        if do_install {
            self.start_install(false);
        }
        if let Some(g) = go {
            self.open_item(g);
        }
    }

    // ------------------------------------------------------------------ a book page

    fn ui_book_browse(&mut self, ui: &mut egui::Ui) {
        let Some(pdf) = self.bookview.path.clone() else { return };
        let page = self.bookview.page;
        let mut go: Option<Item> = None;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("On this page:").strong());
            let book = self.book_of(&pdf);
            let refs = book.map(|b| pieces_on_page(b, page)).unwrap_or_default();
            let secs = self.sections_of(&pdf);
            if !refs.is_empty() {
                for r in refs {
                    if ui.button(format!("▶ {}. {}", r.0, r.2)).clicked() {
                        go = Some(Item::BookPiece(r.0));
                    }
                }
            } else if !secs.is_empty() {
                // exercises that start on this page, or the one that carries on onto it
                let here: Vec<usize> = (0..secs.len()).filter(|&i| secs[i].page as usize == page).collect();
                let here = if here.is_empty() { (0..secs.len()).filter(|&i| (secs[i].page as usize) < page).last().into_iter().collect() } else { here };
                for i in here {
                    let t = if secs[i].title.is_empty() { format!("Exercise {}", i + 1) } else { secs[i].title.clone() };
                    let t = if secs[i].page as usize != page { format!("{t} (continued)") } else { t };
                    if ui.button(format!("▶ {t}")).clicked() {
                        go = Some(Item::Section { pdf: pdf.clone(), idx: i });
                    }
                }
            } else if book.is_some_and(|b| matches!(b, BookId::RussianSchool1a | BookId::RussianSchool1b)) {
                ui.label(RichText::new("no numbered pieces – text or exercises only").weak());
            } else if self.converting() {
                ui.spinner();
                ui.label("making it playable…");
            } else if ui.button("🔊 Make this book playable").on_hover_text("Typeset sheet music is read in a second or two; scanned books use Audiveris").clicked() {
                go = Some(Item::Section { pdf: pdf.clone(), idx: 0 });
            }
        });
        self.bookview.toolbar(ui);
        ui.separator();
        self.bookview.show(ui);
        if let Some(Item::Page { page: p, .. }) = self.item.as_mut() {
            *p = self.bookview.page;
        }
        if let Some(g) = go {
            self.open_item(g);
        }
    }

    // ------------------------------------------------------------------ the player

    fn ui_player(&mut self, ui: &mut egui::Ui) {
        if self.book_only.is_some() {
            self.ui_book_only(ui);
            return;
        }
        if self.cur.is_none() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                if self.converting() {
                    ui.spinner();
                    ui.label(RichText::new("Making it playable…").size(17.0));
                } else {
                    ui.label("Couldn't open that – see the message at the bottom.");
                }
            });
            return;
        }
        // technique: the settings sit beside the music and change it straight away
        if self.tech_mode {
            let mut spec = self.tech;
            let mut changed = false;
            egui::SidePanel::right("techset").resizable(true).default_width(380.0).show_inside(ui, |ui| {
                ui.label(RichText::new("Exercise settings").strong());
                ui.label(RichText::new("Changes apply straight away. Fingering is worked out for you.").small().weak());
                egui::ScrollArea::vertical().show(ui, |ui| {
                    changed |= tech_controls(ui, &mut spec);
                    ui.add_space(6.0);
                    ui.label(RichText::new(spec.ex.tip()).small().italics());
                    ui.separator();
                    ui.label(RichText::new("Ready-made").small().strong());
                    for (name, p) in tech_presets() {
                        if ui.small_button(name).clicked() {
                            spec = p;
                            changed = true;
                        }
                    }
                });
            });
            if changed {
                self.open_item(Item::Tech(spec));
                return;
            }
        }
        // the book page beside the music, following it
        let src_pdf: Option<PathBuf> = {
            let c = self.cur.as_ref().unwrap();
            c.base.src.as_ref().map(|s| PathBuf::from(&s.pdf)).filter(|p| !p.as_os_str().is_empty() && p.exists()).or_else(|| {
                c.book_piece.and_then(books::piece_ref).and_then(|(b, _)| self.progress.book_path(b))
            })
        };
        let has_book = src_pdf.is_some();
        if let Some(pdf) = &src_pdf {
            if self.show_book_beside {
                let pos = self.audio.shared.pos();
                let c = self.cur.as_ref().unwrap();
                let at = c.arranged.page_at(pos.max(0.0));
                if self.bookview.path.as_ref() != Some(pdf) {
                    let page = at.map(|a| a.0 as usize).or_else(|| c.book_piece.and_then(books::piece_ref).map(|(_, r)| r.1)).unwrap_or(1);
                    self.bookview.open(pdf.clone(), page);
                }
                if let Some((pg, rect)) = at {
                    // follow the music whenever it moves; otherwise leave the page where you put it
                    if (pos - self.last_sync_pos).abs() > 1e-3 {
                        self.bookview.page = pg as usize;
                        self.last_sync_pos = pos;
                    }
                    self.bookview.highlight = if self.bookview.page == pg as usize { rect } else { None };
                }
                let mut clicked = None;
                egui::SidePanel::left("bookside").resizable(true).default_width(ui.available_width() * 0.45).show_inside(ui, |ui| {
                    self.bookview.toolbar(ui);
                    clicked = self.bookview.show(ui);
                });
                if let Some([x, y]) = clicked {
                    let c = self.cur.as_ref().unwrap();
                    if let Some(b) = c.arranged.beat_at_point(self.bookview.page as u32, x, y) {
                        self.audio.send(Cmd::Seek(b));
                        self.last_sync_pos = b;
                    }
                }
            }
        }
        self.ui_practice(ui, has_book);
    }

    /// A book piece whose page isn't playable yet: the page, the Russian-School steps,
    /// metronome and repetitions – and the page being converted in the background.
    fn ui_book_only(&mut self, ui: &mut egui::Ui) {
        let bo = self.book_only.as_ref().unwrap();
        let n = bo.number;
        let pid = book_piece_id(n);
        let pp = self.progress.pieces.get(&pid).cloned().unwrap_or_default();
        let rungs = bo.rungs.clone();
        let ri = pp.rung.min(rungs.len() - 1);
        let rung = rungs[ri].clone();
        let (book, r) = books::piece_ref(n).unwrap();
        let path = self.progress.book_path(book);
        let key = path.as_ref().map(|p| page_key(p, r.1)).unwrap_or_default();
        let mut fb = None;
        let mut jump = None;
        let mut convert_now = false;
        egui::SidePanel::right("method").resizable(true).default_width(420.0).show_inside(ui, |ui| {
            ui.label(RichText::new(format!("{n}. {}", r.2)).size(22.0).strong());
            if !r.3.is_empty() {
                ui.label(RichText::new(r.3).weak());
            }
            ui.add_space(6.0);
            // sound first: is the page playable yet?
            egui::Frame::new().fill(Color32::from_rgb(40, 40, 30)).corner_radius(8.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                if self.converting() && self.conv_target == Some(n) {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("Making this page playable…").strong());
                    });
                    ui.label(RichText::new("Audiveris is reading the page (about a minute). It opens with sound by itself when it's done – practise the steps below meanwhile.").small());
                } else if self.page_failed.contains(&key) {
                    ui.label(RichText::new("Audiveris couldn't read this page.").strong().color(Color32::from_rgb(250, 200, 120)));
                    ui.label(RichText::new("Scans that are faint, skewed or full of fingering numbers can defeat it. Practise from the page with the metronome, or try again.").small());
                    if ui.small_button("Try again").clicked() {
                        self.page_failed.remove(&key);
                        convert_now = true;
                    }
                } else if find_audiveris(&self.progress.audiveris).is_none() {
                    ui.label(RichText::new("No sound for this page yet").strong());
                    ui.label(RichText::new("Your Russian School books are scans, so they need the free Audiveris program to turn the page into notes. Install it once (Settings → Make pages playable) and every page becomes playable by itself.").small());
                } else if ui.button("🔊 Make this page playable").clicked() {
                    convert_now = true;
                }
            });
            ui.add_space(8.0);
            egui::Frame::new().fill(Color32::from_rgb(34, 44, 58)).corner_radius(8.0).inner_margin(10.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    if ui.add_enabled(ri > 0, egui::Button::new("◀")).clicked() {
                        jump = Some(ri - 1);
                    }
                    ui.label(RichText::new(format!("Step {} of {}: {}", ri + 1, rungs.len(), rung.label)).size(16.0).strong().color(accent()));
                    if ui.add_enabled(ri + 1 < rungs.len(), egui::Button::new("▶")).clicked() {
                        jump = Some(ri + 1);
                    }
                });
                ui.label(rung.how);
                if !self.plan_msg.is_empty() {
                    ui.label(RichText::new(&self.plan_msg).color(Color32::from_rgb(250, 220, 120)));
                }
            });
            ui.add_space(8.0);
            ui.label(RichText::new("Metronome").strong());
            let bo = self.book_only.as_mut().unwrap();
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut bo.bpm, 30.0..=180.0).suffix(" bpm").integer());
                ui.add(egui::DragValue::new(&mut bo.beats).range(2..=12).suffix(" beats/bar"));
            });
            let (bpm, beats) = (bo.bpm, bo.beats);
            ui.horizontal(|ui| {
                if !self.metro_only {
                    if ui.button(RichText::new("▶ Start metronome").size(15.0)).clicked() {
                        self.start_metronome_only(bpm, beats);
                    }
                } else if ui.button(RichText::new("⏸ Stop").size(15.0)).clicked() {
                    self.stop_metronome_only();
                }
            });
            ui.add_space(8.0);
            ui.label(RichText::new("Repetitions").strong());
            let bo = self.book_only.as_mut().unwrap();
            ui.horizontal(|ui| {
                for i in 0..bo.reps_target {
                    let filled = i < bo.reps_done;
                    let (rr, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                    ui.painter().circle(rr.center(), 9.0, if filled { green() } else { Color32::TRANSPARENT }, Stroke::new(2.0, green()));
                }
                if ui.button("+1 clean play-through").clicked() {
                    bo.reps_done += 1;
                }
                ui.add(egui::DragValue::new(&mut bo.reps_target).range(1..=10).prefix("target "));
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Too hard – smaller step").size(15.0)).clicked() {
                    fb = Some(Feedback::TooHard);
                }
                if ui.button(RichText::new("✅ Nailed it – next step").size(15.0)).clicked() {
                    fb = Some(Feedback::Nailed);
                }
            });
        });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.bookview.toolbar(ui);
            self.bookview.show(ui);
        });
        if convert_now {
            if let Some(p) = path {
                self.conv_target = Some(n);
                self.start_omr(Some(book), p, r.1);
            }
        }
        if let Some(j) = jump {
            let pp = self.progress.pieces.entry(pid.clone()).or_default();
            pp.rung = j;
            pp.tempo_pct = None;
            pp.section = None;
            self.progress.save();
            self.book_only.as_mut().unwrap().reps_done = 0;
        }
        if let Some(f) = fb {
            self.feedback_for(pid, rungs, 8, format!("{n}. {}", r.2), f);
            self.book_only.as_mut().unwrap().reps_done = 0;
        }
    }
}
