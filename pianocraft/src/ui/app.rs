use crate::audio::{Audio, Cmd};
use crate::books::{self, BookId};
use crate::builtin;
use crate::curriculum::{self, apply_feedback, book_piece_id, current_unit, ladder, units, Feedback, PieceProgress, Progress, Rung, Task};
use crate::generate::*;
use crate::import::import_file;
use crate::midi_in::MidiIn;
use crate::model::{key_name, transpose_key, Hand, Note, Origin, Piece};
use crate::transform::{arrange, easiest_transposition, rate, Arrangement, LhMode, RhMode};
use crate::ui::falling::{falling_notes, keyboard, FallingOpts, KeyNames, KeyView, KeyboardOpts, KEYBOARDS, LH_COLOR, RH_COLOR};
use crate::ui::convert::{self, ConvMsg, Converter, Job};
use crate::ui::pdfview::{find_audiveris, BookView};
use crate::ui::reading::{self, Trainer};
use crate::ui::staff::{staff, StaffOpts};
use egui::{Align, Color32, Layout, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Instant;

mod workspace;
use workspace::Item;

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    /// Everything you practise: the browser on the left, the player on the right.
    Main,
    Settings,
}

#[derive(PartialEq, Clone, Copy)]
enum View {
    Both,
    Falling,
    Sheet,
}

struct Current {
    base: Piece,
    rungs: Vec<Rung>,
    pid: String,
    arr: Arrangement,
    arranged: Piece,
    tempo_pct: u32,
    loop_bars: Option<(usize, usize)>,
    reps: u32,
    sound_rh: bool,
    sound_lh: bool,
    book_piece: Option<u32>,
}

/// Practising a book piece straight from the page (no notes yet).
struct BookOnly {
    number: u32,
    rungs: Vec<Rung>,
    bpm: f64,
    beats: u32,
    reps_done: u32,
    reps_target: u32,
}


pub struct App {
    audio: Audio,
    midi: MidiIn,
    midi_ports: Vec<String>,
    progress: Progress,
    builtins: Vec<Piece>,
    tab: Tab,
    cur: Option<Current>,
    book_only: Option<BookOnly>,
    view: View,
    show_names: bool,
    show_fingers: bool,
    key_names: KeyNames,
    lookahead_bars: f32,
    metronome: bool,
    count_in: bool,
    wait_mode: bool,
    echo: bool,
    volume: f32,
    msg: String,
    plan_msg: String,
    search_text: String,
    tech: TechSpec,
    last_frame: Instant,
    finished_prompt: bool,
    drag_from: Option<usize>,
    unit_sel: usize,
    keyview: KeyView,
    bookview: BookView,
    show_book_beside: bool,
    current_book: Option<BookId>,
    conv: Option<Converter>,
    /// What's open: a course unit, a book page, an exercise, a technique drill…
    item: Option<Item>,
    side_open: bool,
    side_filter: String,
    pdf_scan_rx: Option<Receiver<Vec<String>>>,
    last_sync_pos: f64,
    /// The book piece waiting for its page to be converted.
    conv_target: Option<u32>,
    /// Pages Audiveris couldn't read this session (so they aren't retried automatically).
    page_failed: HashSet<String>,
    tech_mode: bool,
    pdf_cache: std::cell::RefCell<Option<std::rc::Rc<workspace::PdfCache>>>,
    /// A file that wasn't where we expected when you opened it (asks you to relink it).
    missing: Option<PathBuf>,
    missing_edit: String,
    missing_item: Option<Item>,
    relink_folder: String,
    exists_cache: std::cell::RefCell<std::collections::HashMap<PathBuf, bool>>,
    /// The last few pieces opened (notes are read from disk only when a piece is opened).
    piece_cache: std::cell::RefCell<std::collections::HashMap<String, Piece>>,
    /// Folder conversions: also send scanned books to Audiveris (slow).
    conv_scans: bool,
    /// Convert again even if a PDF is already in the Library.
    conv_redo: bool,
    /// Open the result when this single-page conversion finishes.
    conv_open: bool,
    trainer: Trainer,
    prev_pressed: HashSet<u8>,
    peek: bool,
    metro_only: bool,
    update: Option<crate::updater::Release>,
    update_rx: Option<Receiver<(bool, Result<Option<crate::updater::Release>, String>)>>,
    install_rx: Option<Receiver<Result<PathBuf, String>>>,
    busy_update: bool,
}

const AUDIVERIS_URL: &str = "https://github.com/Audiveris/audiveris/releases";

fn green() -> Color32 {
    Color32::from_rgb(90, 200, 110)
}
fn accent() -> Color32 {
    Color32::from_rgb(160, 210, 255)
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("accidentals".into(), Arc::new(egui::FontData::from_static(include_bytes!("../../assets/accidentals.ttf"))));
        for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts.families.entry(fam).or_default().push("accidentals".into());
        }
        cc.egui_ctx.set_fonts(fonts);
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        for (_, f) in style.text_styles.iter_mut() {
            f.size *= 1.1;
        }
        cc.egui_ctx.set_style(style);
        cc.egui_ctx.set_theme(egui::ThemePreference::Dark);

        crate::startlog::log("starting audio");
        let audio = Audio::start();
        crate::startlog::log(&format!("audio ready: {}", audio.device_name));
        let mut progress = Progress::load();
        if progress.scan_folder.is_empty() {
            progress.scan_folder = dirs::document_dir().map(|d| d.display().to_string()).unwrap_or_default();
        }
        // find books automatically the first time (your folder, then the usual places)
        if progress.books.is_empty() {
            for folder in books::default_folders(&progress.scan_folder) {
                for (id, p) in books::locate(&folder) {
                    progress.books.entry(id).or_insert_with(|| p.display().to_string());
                }
            }
            progress.save();
        }
        crate::startlog::log("loading progress / midi");
        let mut midi = MidiIn::default();
        let midi_ports = MidiIn::list_ports();
        if let Some(port) = progress.midi_port.clone() {
            if midi_ports.contains(&port) {
                midi.connect(&port, audio.tx.clone());
            }
        }
        let unit_sel = progress.unit_override.unwrap_or_else(|| current_unit(&progress));
        let keys = progress.keyboard_keys.unwrap_or(76);
        let mut app = Self {
            audio,
            midi,
            midi_ports,
            progress,
            builtins: { let mut v = builtin::all(); v.extend(crate::repertoire::pieces()); v },
            tab: Tab::Main,
            cur: None,
            book_only: None,
            view: View::Both,
            show_names: true,
            show_fingers: true,
            key_names: KeyNames::All,
            lookahead_bars: 2.0,
            metronome: false,
            count_in: true,
            wait_mode: false,
            echo: false,
            volume: 0.7,
            msg: String::new(),
            plan_msg: String::new(),
            search_text: String::new(),
            tech: TechSpec::default(),
            last_frame: Instant::now(),
            finished_prompt: false,
            drag_from: None,
            unit_sel,
            keyview: KeyView::new(keys),
            bookview: BookView::new(),
            show_book_beside: true,
            current_book: None,
            conv: None,
            item: None,
            side_open: true,
            side_filter: String::new(),
            pdf_scan_rx: None,
            last_sync_pos: -1.0,
            conv_target: None,
            page_failed: HashSet::new(),
            tech_mode: false,
            pdf_cache: Default::default(),
            missing: None,
            missing_edit: String::new(),
            missing_item: None,
            relink_folder: String::new(),
            exists_cache: Default::default(),
            piece_cache: Default::default(),
            conv_scans: false,
            conv_redo: false,
            conv_open: false,
            trainer: Trainer::new(0),
            prev_pressed: HashSet::new(),
            peek: false,
            metro_only: false,
            update: None,
            update_rx: None,
            install_rx: None,
            busy_update: false,
        };
        if app.progress.auto_update && curriculum::now_secs().saturating_sub(app.progress.last_update_check) > 6 * 3600 {
            app.check_updates(false);
        }
        crate::startlog::log("window ready");
        app.item = Some(Item::Unit(app.unit_sel));
        if app.progress.pdf_index.is_empty() {
            app.rescan_pdfs();
        }
        app.dev_hooks();
        app.audio.send(Cmd::Volume(app.volume));
        app.audio.send(Cmd::CountIn(app.count_in));
        app
    }

    /// Environment hooks used for automated screenshots/tests.
    fn dev_hooks(&mut self) {
        if let Ok(id) = std::env::var("PIANOCRAFT_OPEN") {
            if let Some(p) = self.find_piece(&id) {
                self.open_piece(p, false, None);
                if let Ok(b) = std::env::var("PIANOCRAFT_SEEK").map(|s| s.parse::<f64>().unwrap_or(0.0)) {
                    self.audio.send(Cmd::Seek(b));
                }
            }
        }
        if let Ok(n) = std::env::var("PIANOCRAFT_BOOKPIECE").map(|s| s.parse::<u32>().unwrap_or(46)) {
            self.open_item(Item::BookPiece(n));
        }
        if let Ok(v) = std::env::var("PIANOCRAFT_PDF") {
            // path|page
            let (p, pg) = v.rsplit_once('|').unwrap_or((&v, "1"));
            self.open_item(Item::Page { pdf: PathBuf::from(p), page: pg.parse().unwrap_or(1) });
        }
        if let Ok(v) = std::env::var("PIANOCRAFT_SECTION") {
            let (p, i) = v.rsplit_once('|').unwrap_or((&v, "0"));
            self.open_item(Item::Section { pdf: PathBuf::from(p), idx: i.parse().unwrap_or(0) });
        }
        if std::env::var("PIANOCRAFT_TECH").is_ok() {
            let spec = TechSpec { ex: Exercise::Scale, scale: ScaleType::Dorian, tonic: 2, octaves: 2, pattern: Pattern::Circling, motion: Motion::Parallel, rhythm: Rhythm::Eighths, cycle: Cycle::WholeTones, ..TechSpec::default() };
            self.open_item(Item::Tech(spec));
        }
        if let Ok(l) = std::env::var("PIANOCRAFT_LEVEL").map(|s| s.parse::<usize>().unwrap_or(7)) {
            if let Some(c) = self.cur.as_mut() {
                let (_, r, lh) = crate::transform::LEVELS[(l.max(1) - 1).min(6)];
                c.arr.rh = r;
                c.arr.lh = lh;
            }
            self.reload();
        }
        if std::env::var("PIANOCRAFT_PLAY").is_ok() {
            self.audio.send(Cmd::Play);
        }
        if let Ok(t) = std::env::var("PIANOCRAFT_TAB") {
            match t.as_str() {
                "settings" => self.tab = Tab::Settings,
                "reading" => self.open_item(Item::Reading(0)),
                "find" => self.open_item(Item::Find),
                _ => {}
            }
        }
        if let Ok(u) = std::env::var("PIANOCRAFT_UNIT").map(|s| s.parse::<usize>().unwrap_or(0)) {
            self.open_item(Item::Unit(u));
        }
    }

    fn check_updates(&mut self, manual: bool) {
        self.progress.last_update_check = curriculum::now_secs();
        self.progress.save();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send((manual, crate::updater::check()));
        });
        self.update_rx = Some(rx);
    }

    fn start_update(&mut self) {
        let Some(rel) = self.update.clone() else { return };
        self.busy_update = true;
        self.msg = format!("Downloading {}…", rel.title);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(crate::updater::install(&rel));
        });
        self.install_rx = Some(rx);
    }

    fn start_install(&mut self, desktop: bool) {
        self.busy_update = true;
        self.msg = "Installing…".into();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(crate::install::install(desktop));
        });
        self.install_rx = Some(rx);
    }

    /// Results of update checks, updates and installs.
    fn poll_updates(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.update_rx {
            if let Ok((manual, r)) = rx.try_recv() {
                self.update_rx = None;
                match r {
                    Ok(Some(rel)) => {
                        self.msg = format!("{} is available – click Update at the top.", rel.title);
                        self.update = Some(rel);
                    }
                    Ok(None) if manual => self.msg = format!("PianoCraft is up to date ({}).", crate::updater::version_label()),
                    Err(e) if manual => self.msg = e,
                    _ => {}
                }
            }
        }
        if let Some(rx) = &self.install_rx {
            if let Ok(r) = rx.try_recv() {
                self.install_rx = None;
                self.busy_update = false;
                match r {
                    Ok(exe) => {
                        self.progress.save();
                        self.bookview.notes.save();
                        if crate::updater::relaunch(&exe).is_ok() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        } else {
                            self.msg = "Done – please start PianoCraft again from the Start menu.".into();
                        }
                    }
                    Err(e) => self.msg = e,
                }
            }
        }
        if self.update_rx.is_some() || self.install_rx.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }

    fn find_piece(&self, id: &str) -> Option<Piece> {
        if let Some(p) = self.builtins.iter().find(|p| p.id == id) {
            return Some(p.clone());
        }
        if let Some(p) = self.piece_cache.borrow().get(id) {
            return Some(p.clone());
        }
        // read from disk only now, when it's opened (and keep the last few)
        let p = self.progress.load_piece(id)?;
        let mut c = self.piece_cache.borrow_mut();
        if c.len() >= 4 {
            c.clear();
        }
        c.insert(id.to_string(), p.clone());
        Some(p)
    }

    fn open_piece(&mut self, piece: Piece, is_drill: bool, book_piece: Option<u32>) {
        let rungs = ladder(&piece, is_drill);
        let pid = book_piece.map(book_piece_id).unwrap_or_else(|| piece.id.clone());
        let pp = self.progress.pieces.entry(pid.clone()).or_default().clone();
        let lo = piece.notes.iter().map(|n| n.pitch).min().unwrap_or(48);
        let hi = piece.notes.iter().map(|n| n.pitch).max().unwrap_or(72);
        self.keyview.fit(lo, hi);
        self.keyview.follow = true;
        self.book_only = None;
        self.metro_only = false;
        self.cur = Some(Current {
            arranged: piece.clone(),
            base: piece,
            rungs,
            pid,
            arr: Arrangement::default(),
            tempo_pct: 100,
            loop_bars: None,
            reps: 3,
            sound_rh: true,
            sound_lh: true,
            book_piece,
        });
        if let Some(n) = book_piece {
            if let Some((b, r)) = books::piece_ref(n) {
                if let Some(path) = self.progress.book_path(b) {
                    self.bookview.open(path, r.1);
                }
            }
        }
        self.apply_rung(&pp);
        self.tab = Tab::Main;
        self.finished_prompt = false;
        self.tech_mode = false;
        self.last_sync_pos = -1.0;
    }

    fn apply_rung(&mut self, pp: &PieceProgress) {
        let Some(c) = self.cur.as_mut() else { return };
        let rung = c.rungs[pp.rung.min(c.rungs.len() - 1)].clone();
        let transpose = if rung.transpose {
            pp.transpose_to.unwrap_or_else(|| {
                let avg = c.base.notes.iter().map(|n| n.pitch as f64).sum::<f64>() / c.base.notes.len().max(1) as f64;
                if avg > 62.0 { -5 } else { 7 }
            })
        } else {
            0
        };
        c.arr = rung.arrangement(transpose);
        c.tempo_pct = pp.tempo_pct.unwrap_or(rung.tempo_pct);
        c.loop_bars = pp.section;
        c.reps = if rung.listen { 2 } else { 3 };
        c.sound_rh = true;
        c.sound_lh = true;
        if rung.metronome {
            self.metronome = true;
        }
        self.peek = false;
        self.plan_msg = String::new();
        self.reload();
    }

    fn reload(&mut self) {
        let Some(c) = self.cur.as_mut() else { return };
        c.arranged = arrange(&c.base, &c.arr);
        let p = &c.arranged;
        let click_every = match (p.ts_num, p.ts_den) {
            (6 | 9 | 12, 8) => 1.5,
            (_, d) => 4.0 / d as f64,
        };
        let end = p.bar_start(p.bar_count() + 1);
        self.audio.send(Cmd::Load { notes: p.notes.clone(), bar_len: p.bar_len(), pickup: p.pickup, click_every, end });
        let start = c.loop_bars.map(|(a, _)| p.bar_start(a)).unwrap_or(0.0);
        self.send_settings();
        self.audio.send(Cmd::Seek(start));
    }

    fn send_settings(&self) {
        let Some(c) = self.cur.as_ref() else { return };
        let p = &c.arranged;
        self.audio.send(Cmd::Tempo(c.base.bpm * c.tempo_pct as f64 / 100.0));
        let lr = c.loop_bars.map(|(a, b)| (p.bar_start(a), p.bar_start(b + 1)));
        let lr = lr.or(Some((0.0, p.bar_start(p.bar_count() + 1))));
        self.audio.send(Cmd::Loop(lr, c.reps));
        self.audio.send(Cmd::Metronome(self.metronome));
        self.audio.send(Cmd::Sound { rh: c.sound_rh, lh: c.sound_lh });
        let rung = &c.rungs[self.progress.pieces.get(&c.pid).map(|p| p.rung).unwrap_or(0).min(c.rungs.len() - 1)];
        let (wr, wl) = if rung.you_rh || rung.you_lh { (rung.you_rh, rung.you_lh) } else { (true, true) };
        self.audio.send(Cmd::Wait { enabled: self.wait_mode && self.midi.connected(), rh: wr, lh: wl });
    }

    fn start_metronome_only(&mut self, bpm: f64, beats: u32) {
        self.audio.send(Cmd::Load { notes: vec![], bar_len: beats as f64, pickup: 0.0, click_every: 1.0, end: 1.0e6 });
        self.audio.send(Cmd::Loop(None, 0));
        self.audio.send(Cmd::Tempo(bpm));
        self.audio.send(Cmd::Metronome(true));
        self.audio.send(Cmd::CountIn(false));
        self.audio.send(Cmd::Seek(0.0));
        self.audio.send(Cmd::Play);
        self.metro_only = true;
    }

    fn stop_metronome_only(&mut self) {
        self.audio.send(Cmd::Pause);
        self.audio.send(Cmd::CountIn(self.count_in));
        self.audio.send(Cmd::Metronome(self.metronome));
        self.metro_only = false;
    }

    fn feedback_for(&mut self, pid: String, rungs: Vec<Rung>, bars: usize, title: String, fb: Feedback) -> PieceProgress {
        let pp = self.progress.pieces.entry(pid).or_default();
        let label_before = rungs[pp.rung.min(rungs.len() - 1)].label;
        let result = match fb {
            Feedback::TooHard => "too hard",
            Feedback::Nailed => "nailed it",
        };
        let msg = apply_feedback(pp, &rungs, bars, fb);
        let pp = pp.clone();
        self.progress.log.push(curriculum::LogEntry { day: curriculum::today(), piece: title, rung: label_before.into(), result: result.into() });
        self.progress.save();
        self.plan_msg = msg;
        self.finished_prompt = false;
        pp
    }

    fn feedback(&mut self, fb: Feedback) {
        let Some(c) = self.cur.as_ref() else { return };
        let (pid, rungs, bars, title) = (c.pid.clone(), c.rungs.clone(), c.base.bar_count(), c.base.title.clone());
        let pp = self.feedback_for(pid, rungs, bars, title, fb);
        let msg = self.plan_msg.clone();
        self.apply_rung(&pp);
        self.plan_msg = msg;
    }

    fn import_paths(&mut self, paths: Vec<PathBuf>) {
        let mut last = None;
        for path in paths {
            if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")) {
                self.open_item(Item::Page { pdf: path.clone(), page: 1 });
                continue;
            }
            match import_file(&path) {
                Ok(p) => {
                    let d = rate(&p);
                    last = Some(p.id.clone());
                    self.msg = format!("Imported '{}' – difficulty {:.1}/10. It's under 'Your files' on the left.", p.title, d.score);
                    self.piece_cache.borrow_mut().remove(&p.id);
                    self.progress.add_piece(p);
                    self.progress.save();
                }
                Err(e) => self.msg = format!("{}: {e}", path.display()),
            }
        }
        if let Some(id) = last {
            self.open_item(Item::Whole { id });
        }
    }

    /// Make one page playable (typeset: instant; scanned: Audiveris in the background).
    fn start_omr(&mut self, book: Option<BookId>, pdf: PathBuf, page: usize) {
        self.current_book = book.or(self.current_book);
        self.conv_open = true;
        self.start_convert(vec![Job { pdf, pages: Some((page, page)) }], true);
    }

    /// Convert PDFs in the background. `scans`: send scanned books to Audiveris if installed.
    fn start_convert(&mut self, jobs: Vec<Job>, scans: bool) {
        if self.conv.as_ref().is_some_and(|c| !c.finished) {
            self.msg = "Already converting – wait for it to finish, or press Stop.".into();
            return;
        }
        if jobs.is_empty() {
            self.msg = "Nothing to convert – those PDFs are already in your Library (tick 'convert again' to redo them).".into();
            return;
        }
        let exe = if scans { find_audiveris(&self.progress.audiveris) } else { None };
        self.conv = Some(convert::start(jobs, exe));
    }

    /// Already converted from this PDF?
    fn converted_from(&self, pdf: &std::path::Path) -> bool {
        let s = pdf.display().to_string();
        self.progress.library.iter().any(|p| matches!(&p.origin, Origin::Imported(o) if *o == s) && p.id.starts_with("pdf-"))
    }

    fn poll_convert(&mut self) {
        let Some(c) = self.conv.as_mut() else { return };
        let mut changed = false;
        let mut last_piece: Option<Piece> = None;
        while let Ok(m) = c.rx.try_recv() {
            match m {
                ConvMsg::Working { index, what } => {
                    c.index = index;
                    c.what = what;
                }
                ConvMsg::Piece(p, job) => {
                    c.made += 1;
                    if let Some((a, b)) = job.pages {
                        if a == b {
                            let key = workspace::page_key(&job.pdf, a);
                            let ids = self.progress.page_conv.entry(key).or_default();
                            if !ids.contains(&p.id) {
                                ids.push(p.id.clone());
                            }
                        }
                    }
                    last_piece = Some((*p).clone());
                    self.piece_cache.borrow_mut().remove(&p.id);
                    self.progress.add_piece(*p);
                    changed = true;
                }
                ConvMsg::Skipped { pdf, why } => {
                    if c.total == 1 {
                        if let Some(Item::BookPiece(n)) = &self.item {
                            if let Some((b, r)) = books::piece_ref(*n) {
                                if self.progress.book_path(b).as_deref() == Some(pdf.as_path()) {
                                    self.page_failed.insert(workspace::page_key(&pdf, r.1));
                                }
                            }
                        }
                    }
                    c.skipped.push((pdf, why))
                }
                ConvMsg::Finished => {
                    c.finished = true;
                    changed = true;
                }
            }
        }
        if !changed {
            return;
        }
        self.progress.library_changed();
        if c.finished || c.made % 10 == 0 {
            self.progress.save();
        }
        if c.finished {
            let (made, total, skipped) = (c.made, c.total, c.skipped.clone());
            if total == 1 && made == 0 {
                self.msg = format!("Couldn't make that playable: {}.", skipped.first().map(|s| s.1.as_str()).unwrap_or("no notes found"));
            } else if total == 1 {
                self.msg = String::new();
            } else {
                self.msg = format!(
                    "Finished: {made} playable piece{} from {total} PDF{}{}. They're in the Library under 'Your imports'.",
                    if made == 1 { "" } else { "s" },
                    if total == 1 { "" } else { "s" },
                    if skipped.is_empty() { String::new() } else { format!("; {} skipped (see the list in Library)", skipped.len()) }
                );
            }
            if self.conv_open {
                self.conv_open = false;
                // a book piece waiting for its page: open the playable version now
                match (self.conv_target.take(), self.item.clone()) {
                    (Some(n), Some(Item::BookPiece(m))) if n == m && made > 0 => self.open_item(Item::BookPiece(n)),
                    (Some(_), _) => {}
                    (None, Some(it @ Item::Section { .. })) if made > 0 => self.open_item(it),
                    (None, _) => {
                        if let Some(p) = last_piece {
                            let note = p.notes_text.clone();
                            self.open_item(Item::Whole { id: p.id.clone() });
                            self.msg = note;
                        }
                    }
                }
            }
        }
    }
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).fill(ui.visuals().faint_bg_color).corner_radius(8.0).inner_margin(10.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui)
    });
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        if std::env::var("PIANOCRAFT_FRAMETIME").is_ok() {
            eprintln!("frame gap {:.1} ms", dt * 1000.0);
        }
        self.last_frame = now;
        let playing = self.audio.shared.playing.load(Ordering::Relaxed);
        if playing {
            *self.progress.minutes_by_day.entry(curriculum::today()).or_insert(0.0) += dt.min(0.5) / 60.0;
            ctx.request_repaint();
        }
        if self.audio.shared.finished.swap(false, Ordering::Relaxed) {
            self.finished_prompt = true;
            self.progress.save();
        }
        self.poll_convert();
        self.poll_pdf_scan();
        self.poll_updates(ctx);
        if self.pdf_scan_rx.is_some() || self.conv.as_ref().is_some_and(|c| !c.finished) {
            ctx.request_repaint_after(std::time::Duration::from_millis(300));
        }
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() {
            self.import_paths(dropped);
        }
        if !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Space)) && self.cur.is_some() && self.tab == Tab::Main {
            self.audio.send(if playing { Cmd::Pause } else { Cmd::Play });
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("🎹 PianoCraft").size(20.0).strong());
                ui.add_space(10.0);
                ui.selectable_value(&mut self.tab, Tab::Main, RichText::new("🎵 Practise").size(15.0));
                ui.selectable_value(&mut self.tab, Tab::Settings, RichText::new("⚙ Settings").size(15.0));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(rel) = self.update.clone() {
                        let label = if self.busy_update { "Updating…".to_string() } else { format!("⬆ Update to {}", rel.title.trim_start_matches("PianoCraft ")) };
                        if ui.add_enabled(!self.busy_update, egui::Button::new(RichText::new(label).strong()).fill(Color32::from_rgb(40, 110, 60))).on_hover_text("Downloads the new version, checks it, and restarts PianoCraft").clicked() {
                            self.start_update();
                        }
                    }
                    let streak = self.progress.streak();
                    let mins = self.progress.minutes_by_day.get(&curriculum::today()).copied().unwrap_or(0.0);
                    ui.label(format!("Today {:.0} min · streak {streak}", mins));
                    let midi = if self.midi.connected() { RichText::new("● keyboard").color(green()) } else { RichText::new("○ no keyboard").color(Color32::GRAY) };
                    ui.label(midi);
                });
            });
            ui.add_space(2.0);
        });

        if self.conv.as_ref().is_some_and(|c| !c.finished) {
            let mut stop = false;
            egui::TopBottomPanel::bottom("conv").show(ctx, |ui| {
                let c = self.conv.as_ref().unwrap();
                ui.horizontal(|ui| {
                    ui.spinner();
                    let of = if c.total > 1 { format!("{} of {} · ", c.index + 1, c.total) } else { String::new() };
                    ui.label(RichText::new(format!("{of}{}", c.what)).color(Color32::from_rgb(250, 220, 120)));
                    if c.total > 1 {
                        ui.add(egui::ProgressBar::new(c.index as f32 / c.total as f32).desired_width(160.0));
                        ui.label(format!("{} made, {} skipped", c.made, c.skipped.len()));
                    }
                    if ui.button("Stop").clicked() {
                        stop = true;
                    }
                });
            });
            if stop {
                if let Some(c) = &self.conv {
                    c.cancel();
                }
            }
        }
        if !self.msg.is_empty() {
            egui::TopBottomPanel::bottom("msg").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.msg);
                    if ui.small_button("×").clicked() {
                        self.msg.clear();
                    }
                });
            });
        }

        self.missing_dialog(ctx);
        match self.tab {
            Tab::Main => self.ui_main(ctx),
            Tab::Settings => {
                egui::CentralPanel::default().show(ctx, |ui| self.ui_settings(ui));
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.audio.send(Cmd::Pause);
        self.progress.save();
        self.bookview.notes.save();
        crate::startlog::log("closed cleanly");
        // Leave immediately: shutting down audio/MIDI drivers can hang on some PCs, which would
        // keep an invisible PianoCraft running and stop the next launch.
        std::process::exit(0);
    }
}

impl App {
    fn ui_practice(&mut self, ui: &mut egui::Ui, has_book: bool) {
        let pos = self.audio.shared.pos();
        let playing = self.audio.shared.playing.load(Ordering::Relaxed);
        let waiting = self.audio.shared.waiting.load(Ordering::Relaxed);
        let counting = self.audio.shared.counting_in.load(Ordering::Relaxed);
        let loops_done = self.audio.shared.loops_done.load(Ordering::Relaxed);
        let mut changed = false;
        let mut settings_changed = false;
        let mut fb: Option<Feedback> = None;
        let mut rung_jump: Option<usize> = None;
        let pp = self.cur.as_ref().and_then(|c| self.progress.pieces.get(&c.pid).cloned()).unwrap_or_default();
        let c = self.cur.as_mut().unwrap();
        let rung_i = pp.rung.min(c.rungs.len() - 1);
        let rung = c.rungs[rung_i].clone();
        let diff = rate(&c.arranged);
        let hidden = rung.memorise && !self.peek;

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let title = match c.book_piece {
                    Some(n) => format!("{n}. {}", books::piece_ref(n).map(|r| r.1 .2).unwrap_or("")),
                    None => c.base.title.clone(),
                };
                ui.label(RichText::new(title).size(21.0).strong());
                let key_now = key_name(c.arranged.key_fifths, c.arranged.minor);
                ui.label(RichText::new(format!(
                    "{}   ·   {}   ·   {}/{}   ·   ♩ = {:.0}   ·   difficulty {:.1}/10",
                    if c.base.composer.is_empty() { "—" } else { &c.base.composer },
                    key_now, c.base.ts_num, c.base.ts_den, c.base.bpm * c.tempo_pct as f64 / 100.0, diff.score
                )).weak());
                if !c.base.notes_text.is_empty() {
                    ui.label(RichText::new(&c.base.notes_text).small().italics());
                }
            });
            if has_book {
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    ui.checkbox(&mut self.show_book_beside, "📖 Book page");
                });
            }
        });
        egui::Frame::new().fill(Color32::from_rgb(34, 44, 58)).corner_radius(8.0).inner_margin(8.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(rung_i > 0, egui::Button::new("◀")).on_hover_text("Previous step").clicked() {
                    rung_jump = Some(rung_i - 1);
                }
                ui.label(RichText::new(format!("Step {} of {}: {}", rung_i + 1, c.rungs.len(), rung.label)).size(16.0).strong().color(accent()));
                if ui.add_enabled(rung_i + 1 < c.rungs.len(), egui::Button::new("▶")).on_hover_text("Skip to next step").clicked() {
                    rung_jump = Some(rung_i + 1);
                }
                if pp.done {
                    ui.label(RichText::new("✔ mastered").color(green()));
                }
                ui.separator();
                let who = match (rung.listen, rung.you_rh, rung.you_lh) {
                    (true, _, _) => "You: listen (and clap/sing)",
                    (_, true, true) => "You: both hands",
                    (_, true, false) => "You: right hand",
                    (_, false, true) => "You: left hand",
                    _ => "You: play along",
                };
                ui.label(who);
                if let Some((a, b)) = pp.section {
                    ui.separator();
                    ui.label(RichText::new(format!("Focus: bars {a}–{b}")).color(Color32::from_rgb(230, 150, 230)));
                }
                if rung.memorise {
                    ui.separator();
                    let r = ui.button(if self.peek { "🙈 Hide notes" } else { "👁 Peek" });
                    if r.clicked() {
                        self.peek = !self.peek;
                    }
                }
            });
            ui.label(rung.how);
            if !self.plan_msg.is_empty() {
                ui.label(RichText::new(&self.plan_msg).color(Color32::from_rgb(250, 220, 120)));
            }
        });

        ui.horizontal_wrapped(|ui| {
            let big = |s: &str| RichText::new(s).size(18.0);
            if ui.button(big("⏮")).on_hover_text("Back to start of loop").clicked() {
                let s = c.loop_bars.map(|(a, _)| c.arranged.bar_start(a)).unwrap_or(0.0);
                self.audio.send(Cmd::Seek(s));
            }
            if ui.button(big(if playing { "⏸ Pause" } else { "▶ Play" })).on_hover_text("Space bar").clicked() {
                self.audio.send(if playing { Cmd::Pause } else { Cmd::Play });
                self.finished_prompt = false;
            }
            ui.separator();
            ui.label("Speed");
            settings_changed |= ui.add(egui::Slider::new(&mut c.tempo_pct, 25..=130).suffix("%")).changed();
            ui.separator();
            settings_changed |= ui.toggle_value(&mut self.metronome, "🕑 Metronome").changed();
            if ui.toggle_value(&mut self.count_in, "Count-in").on_hover_text("One bar of clicks before starting").changed() {
                self.audio.send(Cmd::CountIn(self.count_in));
            }
            ui.separator();
            ui.label("Repeat");
            settings_changed |= ui.add(egui::DragValue::new(&mut c.reps).range(0..=50).suffix("×")).on_hover_text("Times to play the loop before stopping (0 = forever)").changed();
            let reps_txt = if c.reps == 0 { format!("pass {}", loops_done + 1) } else { format!("pass {}/{}", (loops_done + 1).min(c.reps), c.reps) };
            ui.label(RichText::new(reps_txt).weak());
            ui.separator();
            ui.label("Bars");
            let nb = c.base.bar_count();
            let (mut a, mut b) = c.loop_bars.unwrap_or((1, nb));
            let ra = ui.add(egui::DragValue::new(&mut a).range(1..=nb));
            ui.label("to");
            let rb = ui.add(egui::DragValue::new(&mut b).range(1..=nb));
            if ra.changed() || rb.changed() {
                if b < a {
                    b = a;
                }
                c.loop_bars = if a == 1 && b == nb { None } else { Some((a, b)) };
                changed = true;
            }
            if c.loop_bars.is_some() && ui.small_button("whole piece").clicked() {
                c.loop_bars = None;
                changed = true;
            }
            if self.midi.connected() {
                ui.separator();
                settings_changed |= ui.toggle_value(&mut self.wait_mode, "Wait for me").on_hover_text("Pauses at each note until you play it on your keyboard").changed();
            }
            if counting {
                ui.label(RichText::new("count-in…").color(Color32::YELLOW));
            }
            if waiting {
                ui.label(RichText::new("waiting for your notes…").color(Color32::YELLOW));
            }
        });

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Difficulty").strong());
            let cur_level = crate::transform::level_of(&c.arr);
            let mut lvl = cur_level.map(|l| l as u32 + 1).unwrap_or(7);
            let resp = ui.add(egui::Slider::new(&mut lvl, 1..=7).show_value(false)).on_hover_text("Like a rhythm game: 1 = just the tune, 7 = every note. Each step brings more notes back.");
            ui.label(RichText::new(match cur_level { Some(l) => crate::transform::LEVELS[l].0, None => "custom (set below)" }).color(accent()));
            if resp.changed() {
                let (_, r, l) = crate::transform::LEVELS[(lvl as usize - 1).min(6)];
                c.arr.rh = r;
                c.arr.lh = l;
                changed = true;
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Right hand").color(RH_COLOR));
            egui::ComboBox::from_id_salt("rh").selected_text(c.arr.rh.label()).show_ui(ui, |ui| {
                for m in RhMode::ALL {
                    changed |= ui.selectable_value(&mut c.arr.rh, m, m.label()).changed();
                }
            });
            settings_changed |= ui.checkbox(&mut c.sound_rh, "🔊").on_hover_text("App plays the right hand out loud").changed();
            ui.separator();
            ui.label(RichText::new("Left hand").color(LH_COLOR));
            egui::ComboBox::from_id_salt("lh").selected_text(c.arr.lh.label()).show_ui(ui, |ui| {
                for m in LhMode::ALL {
                    changed |= ui.selectable_value(&mut c.arr.lh, m, m.label()).changed();
                }
            });
            settings_changed |= ui.checkbox(&mut c.sound_lh, "🔊").on_hover_text("App plays the left hand out loud").changed();
            ui.separator();
            ui.label("Key");
            if ui.small_button("−").on_hover_text("Down a semitone").clicked() {
                c.arr.transpose -= 1;
                changed = true;
            }
            ui.label(RichText::new(format!("{} ({:+})", key_name(transpose_key(c.base.key_fifths, c.arr.transpose), c.base.minor), c.arr.transpose)).strong());
            if ui.small_button("+").on_hover_text("Up a semitone").clicked() {
                c.arr.transpose += 1;
                changed = true;
            }
            if ui.small_button("Easiest key").on_hover_text("Fewest black keys").clicked() {
                c.arr.transpose = easiest_transposition(&c.base);
                changed = true;
            }
            if c.arr.transpose != 0 && ui.small_button("Original").clicked() {
                c.arr.transpose = 0;
                changed = true;
            }
            ui.separator();
            ui.selectable_value(&mut self.view, View::Both, "Sheet + falling");
            ui.selectable_value(&mut self.view, View::Falling, "Falling");
            ui.selectable_value(&mut self.view, View::Sheet, "Sheet");
            ui.checkbox(&mut self.show_names, "Note names");
            ui.checkbox(&mut self.show_fingers, "Fingers");
        });
        c.arr.transpose = c.arr.transpose.clamp(-12, 12);

        // timeline
        let nb = c.base.bar_count();
        let (tl_rect, tl_resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::click_and_drag());
        {
            let painter = ui.painter_at(tl_rect);
            let bw = tl_rect.width() / nb as f32;
            let cur_bar = c.arranged.bar_at(pos.max(0.0));
            for b in 1..=nb {
                let r = Rect::from_min_size(Pos2::new(tl_rect.left() + (b - 1) as f32 * bw, tl_rect.top()), Vec2::new(bw, tl_rect.height()));
                let in_loop = c.loop_bars.is_some_and(|(x, y)| b >= x && b <= y);
                let fill = if in_loop { Color32::from_rgb(110, 60, 120) } else if b % 2 == 0 { Color32::from_rgb(40, 44, 54) } else { Color32::from_rgb(48, 52, 64) };
                painter.rect_filled(r.shrink(0.5), 2.0, fill);
                if bw > 16.0 || b % 4 == 1 {
                    painter.text(r.center(), egui::Align2::CENTER_CENTER, b.to_string(), egui::FontId::proportional(11.0), if b == cur_bar { Color32::WHITE } else { Color32::GRAY });
                }
            }
            let end = c.arranged.bar_start(nb + 1);
            let px = tl_rect.left() + (((pos - c.arranged.pickup) / (end - c.arranged.pickup)).clamp(0.0, 1.0) as f32) * tl_rect.width();
            painter.line_segment([Pos2::new(px, tl_rect.top()), Pos2::new(px, tl_rect.bottom())], Stroke::new(2.0, Color32::from_rgb(240, 220, 120)));
            let bar_at = |x: f32| (((x - tl_rect.left()) / bw).floor() as usize + 1).clamp(1, nb);
            if tl_resp.drag_started() {
                if let Some(p) = tl_resp.interact_pointer_pos() {
                    self.drag_from = Some(bar_at(p.x));
                }
            }
            if tl_resp.dragged() {
                if let (Some(a), Some(p)) = (self.drag_from, tl_resp.interact_pointer_pos()) {
                    let b = bar_at(p.x);
                    c.loop_bars = Some((a.min(b), a.max(b)));
                }
            }
            if tl_resp.drag_stopped() {
                self.drag_from = None;
                changed = true;
            }
            if tl_resp.clicked() {
                if let Some(p) = tl_resp.interact_pointer_pos() {
                    let b = bar_at(p.x);
                    self.audio.send(Cmd::Seek(c.arranged.bar_start(b)));
                }
            }
            tl_resp.on_hover_text("Click a bar to jump there · drag across bars to loop them");
        }

        let fb_h = 42.0;
        let strip_h = 20.0;
        let avail = ui.available_rect_before_wrap();
        let content = Rect::from_min_max(avail.min, Pos2::new(avail.max.x, avail.max.y - fb_h));
        let kb_h = 120.0f32.min(content.height() * 0.3);
        let lit: Vec<(u8, Hand, Option<u8>)> = c.arranged.notes.iter().filter(|n| n.start <= pos && pos < n.start + n.dur).map(|n| (n.pitch, n.hand, if self.show_fingers { n.finger } else { None })).collect();
        let pressed = self.midi.pressed.lock().unwrap().clone();
        let prefer_flats = c.arranged.key_at(pos) < 0;
        let lookahead = self.lookahead_bars as f64 * c.arranged.bar_len();
        let loop_beats = c.loop_bars.map(|(a, b)| (c.arranged.bar_start(a), c.arranged.bar_start(b + 1)));
        // keep the upcoming notes in view
        if self.keyview.follow && playing {
            let upcoming: Vec<u8> = c.arranged.notes.iter().filter(|n| n.start + n.dur > pos && n.start < pos + lookahead).map(|n| n.pitch).collect();
            if let (Some(lo), Some(hi)) = (upcoming.iter().min(), upcoming.iter().max()) {
                self.keyview.ensure_visible(*lo, *hi);
            }
        }
        let sopts = StaffOpts { pos, show_names: self.show_names, show_fingers: self.show_fingers, prefer_flats, loop_range: loop_beats };
        let hidden_box = |ui: &mut egui::Ui, r: Rect| {
            ui.painter().rect_filled(r, 6.0, Color32::from_rgb(40, 40, 46));
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "Sheet hidden – from memory (Peek if you need it)", egui::FontId::proportional(16.0), Color32::GRAY);
        };
        let fall_top;
        match self.view {
            View::Sheet => {
                let sheet = Rect::from_min_max(content.min, Pos2::new(content.max.x, content.max.y - kb_h - strip_h - 6.0));
                if hidden { hidden_box(ui, sheet) } else { staff(ui, sheet, &c.arranged, &sopts) }
                fall_top = None;
            }
            View::Falling | View::Both => {
                let sheet_h = if self.view == View::Both { (content.height() * 0.4).clamp(140.0, 250.0) } else { 0.0 };
                if sheet_h > 0.0 {
                    let sheet = Rect::from_min_size(content.min, Vec2::new(content.width(), sheet_h));
                    if hidden { hidden_box(ui, sheet) } else { staff(ui, sheet, &c.arranged, &sopts) }
                }
                fall_top = Some(content.top() + sheet_h + 6.0);
            }
        }
        if let Some(ft) = fall_top {
            let fall = Rect::from_min_max(Pos2::new(content.left(), ft), Pos2::new(content.right(), content.bottom() - kb_h - strip_h));
            falling_notes(ui, fall, &c.arranged, &self.keyview, &FallingOpts { pos, lookahead, show_names: self.show_names, show_fingers: self.show_fingers, prefer_flats, loop_range: loop_beats, hidden });
        }
        let kb_rect = Rect::from_min_max(Pos2::new(content.left(), content.bottom() - kb_h - strip_h), Pos2::new(content.right(), content.bottom() - strip_h));
        if let Some(k) = keyboard(ui, kb_rect, &self.keyview, &KeyboardOpts { lit: &lit, pressed: &pressed, prefer_flats, names: self.key_names, marked: &[] }) {
            self.audio.send(Cmd::Preview(k));
        }
        let range = c.base.notes.iter().map(|n| n.pitch).min().zip(c.base.notes.iter().map(|n| n.pitch).max());
        let strip = Rect::from_min_max(Pos2::new(content.left() + 150.0, content.bottom() - strip_h + 2.0), content.max);
        self.keyview.scrollbar(ui, strip, range);
        let zoom_rect = Rect::from_min_max(Pos2::new(content.left(), content.bottom() - strip_h + 1.0), Pos2::new(content.left() + 146.0, content.bottom()));
        ui.scope_builder(egui::UiBuilder::new().max_rect(zoom_rect), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                if ui.small_button("−").on_hover_text("Show more keys").clicked() {
                    self.keyview.zoom_by(1.2);
                }
                if ui.small_button("+").on_hover_text("Bigger keys").clicked() {
                    self.keyview.zoom_by(1.0 / 1.2);
                }
                if ui.small_button("fit").on_hover_text("Fit the piece").clicked() {
                    if let Some((a, b)) = range {
                        self.keyview.fit(a, b);
                        self.keyview.follow = true;
                    }
                }
                ui.toggle_value(&mut self.keyview.follow, "follow").on_hover_text("Scroll the keyboard to follow the music");
            });
        });
        let fb_rect = Rect::from_min_max(Pos2::new(avail.left(), avail.bottom() - fb_h + 6.0), avail.max);
        let c = self.cur.as_mut().unwrap();
        ui.scope_builder(egui::UiBuilder::new().max_rect(fb_rect), |ui| {
            ui.horizontal_centered(|ui| {
                let prompt = if self.finished_prompt { RichText::new("Done! How did that go?").strong().color(Color32::from_rgb(250, 220, 120)) } else { RichText::new("How's it going?") };
                ui.label(prompt);
                if ui.button(RichText::new("Too hard – make it easier").size(15.0)).clicked() {
                    fb = Some(Feedback::TooHard);
                }
                if ui.button(RichText::new("↻ Again").size(15.0)).clicked() {
                    let s = c.loop_bars.map(|(a, _)| c.arranged.bar_start(a)).unwrap_or(0.0);
                    self.audio.send(Cmd::Seek(s));
                    self.audio.send(Cmd::Play);
                    self.finished_prompt = false;
                }
                if ui.button(RichText::new("✅ Nailed it – next step").size(15.0)).clicked() {
                    fb = Some(Feedback::Nailed);
                }
                ui.separator();
                ui.add(egui::DragValue::new(&mut self.lookahead_bars).range(1.0..=6.0).speed(0.05).fixed_decimals(0).prefix("see ").suffix(" bars ahead"));
            });
        });

        if settings_changed {
            self.send_settings();
        }
        if changed {
            self.reload();
        }
        if let Some(r) = rung_jump {
            let pid = self.cur.as_ref().unwrap().pid.clone();
            let pp = self.progress.pieces.entry(pid).or_default();
            pp.rung = r;
            pp.tempo_pct = None;
            pp.section = None;
            let pp = pp.clone();
            self.progress.save();
            self.apply_rung(&pp);
        }
        if let Some(f) = fb {
            self.feedback(f);
        }
    }

    // ------------------------------------------------------------------ Technique

    // ------------------------------------------------------------------ Reading trainer

    fn ui_reading(&mut self, ui: &mut egui::Ui) {
        ui.label("Nahre Sol's reference-point method (Elements of Music, ch. 2): learn a few landmark notes, then read every other note as a step or skip from the nearest landmark. Click the key – or play it on your piano.");
        ui.horizontal(|ui| {
            for (i, (name, desc)) in reading::LEVELS.iter().enumerate() {
                if ui.selectable_label(self.trainer.level == i as u8, *name).on_hover_text(*desc).clicked() {
                    self.trainer = Trainer::new(i as u8);
                }
            }
        });
        let key = format!("level{}", self.trainer.level);
        let best = self.progress.reading_best.get(&key).copied().unwrap_or(0);
        ui.label(RichText::new(reading::LEVELS[self.trainer.level as usize].1).weak());
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("In a row: {}", self.trainer.streak)).size(18.0).strong());
            ui.label(format!("best {best}   ·   {}/{} correct", self.trainer.correct, self.trainer.tries));
            if self.trainer.streak >= 20 {
                ui.label(RichText::new("20 in a row – move up a level!").color(green()));
            }
        });
        let q = self.trainer.current;
        let piece = reading::question_piece(q);
        let avail = ui.available_rect_before_wrap();
        let sheet = Rect::from_min_size(avail.min, Vec2::new(avail.width().min(700.0), 230.0));
        staff(ui, sheet, &piece, &StaffOpts { pos: -1.0, show_names: false, show_fingers: false, prefer_flats: false, loop_range: None });
        let mut hint_rect = sheet;
        hint_rect.min.x = sheet.right() + 16.0;
        hint_rect.max.x = avail.right();
        ui.scope_builder(egui::UiBuilder::new().max_rect(hint_rect), |ui| {
            if let Some((p, ok)) = self.trainer.last {
                if ok {
                    ui.label(RichText::new(format!("✔ {}", crate::model::pitch_name(p, false))).size(20.0).color(green()));
                } else {
                    ui.label(RichText::new(format!("× that was {}", crate::model::pitch_name(p, false))).size(20.0).color(Color32::LIGHT_RED));
                    ui.label(if p < 60 {
                        "It was in the BASS clef (bottom staff). Its lines, from the bottom, are G B D F A and its spaces A C E G – so the same line or space is a different note than in the treble clef. The two dots of the clef sit either side of the F line."
                    } else {
                        "It was in the TREBLE clef (top staff). Its lines, from the bottom, are E G B D F and its spaces F A C E. The clef curls round G, the second line."
                    });
                }
            }
            ui.add_space(8.0);
            let lm = reading::nearest_landmark(q);
            ui.collapsing("Hint", |ui| {
                ui.label(format!("Nearest landmark: {}. Count the lines and spaces from it – each line/space is one white key.", crate::model::pitch_name(lm, false)));
            });
        });
        // keyboard to answer
        let kb = Rect::from_min_max(Pos2::new(avail.left(), sheet.bottom() + 20.0), Pos2::new(avail.right(), (sheet.bottom() + 190.0).min(avail.bottom())));
        let mut view = KeyView::new(self.keyview.keys());
        view.fit(36, 84);
        let pressed = self.midi.pressed.lock().unwrap().clone();
        let marks: Vec<(u8, Color32)> = if self.trainer.level == 0 { vec![(48, accent()), (53, accent()), (60, accent()), (67, accent()), (72, accent())] } else { vec![] };
        let answer = keyboard(ui, kb, &view, &KeyboardOpts { lit: &[], pressed: &pressed, prefer_flats: false, names: if self.trainer.level == 0 { KeyNames::All } else { KeyNames::COnly }, marked: &marks });
        let new_press = pressed.difference(&self.prev_pressed).next().copied();
        self.prev_pressed = pressed;
        if let Some(a) = answer.or(new_press) {
            self.audio.send(Cmd::Preview(a));
            self.trainer.answer(a);
            if self.trainer.streak > best {
                self.progress.reading_best.insert(key, self.trainer.streak);
            }
        }
        ui.allocate_rect(Rect::from_min_max(avail.min, kb.max), Sense::hover());
    }

    // ------------------------------------------------------------------ Find music

    fn ui_find(&mut self, ui: &mut egui::Ui) {
        ui.heading("Find music");
        ui.label("Search free, legal sources. Download a MIDI (.mid) or MusicXML (.mxl / .musicxml) file and drag it onto this window. PianoCraft rates it, splits the hands and builds easier versions. Got a PDF instead? Library → 'Make all PDFs playable' reads typeset sheet music straight from the PDF.");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Song or composer:");
            ui.add(egui::TextEdit::singleline(&mut self.search_text).desired_width(360.0).hint_text("e.g. Bells of Notre Dame, Für Elise"));
        });
        let q = urlencode(self.search_text.trim());
        ui.add_space(6.0);
        let sources: [(&str, &str, String); 5] = [
            ("Mutopia Project", "Free classical scores with MIDI downloads.", format!("https://www.mutopiaproject.org/cgibin/make-table.cgi?searchingfor={q}&Instrument=Piano")),
            ("IMSLP", "Huge public-domain library (mostly PDFs).", format!("https://imslp.org/index.php?search={q}")),
            ("MuseScore", "Community arrangements of almost everything; download as MIDI or MusicXML with a free account.", format!("https://musescore.com/sheetmusic?text={q}")),
            ("BitMidi", "Big MIDI collection – pop and film songs.", format!("https://bitmidi.com/search?q={q}")),
            ("Web search", "MIDI or MusicXML anywhere on the web.", format!("https://www.google.com/search?q={q}+piano+(midi+OR+musicxml)")),
        ];
        for (name, desc, url) in sources {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(name).strong());
                        ui.label(RichText::new(desc).small());
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Search ↗").clicked() {
                            let _ = webbrowser::open(&url);
                        }
                    });
                });
            });
        }
    }

    // ------------------------------------------------------------------ Settings

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        let mut action: Option<&str> = None;
        let mut relink_one: Option<(String, String)> = None;
        let mut remove_piece: Option<String> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Version, updates and install");
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("PianoCraft {}", crate::updater::version_label())).strong());
                    let label = if self.update_rx.is_some() { "Checking…" } else { "Check for updates" };
                    if ui.add_enabled(self.update_rx.is_none() && !self.busy_update, egui::Button::new(label)).clicked() {
                        action = Some("check");
                    }
                    if ui.checkbox(&mut self.progress.auto_update, "Check automatically (at most every 6 hours)").changed() {
                        self.progress.save();
                    }
                });
                ui.label(RichText::new("Updates come from github.com/pink-bourbon-estate/toolkits. Each download is checked against its SHA-256 checksum before it replaces PianoCraft. Nothing about you is sent.").small().weak());
                ui.add_space(4.0);
                if crate::install::is_installed() {
                    ui.label(RichText::new("✔ Installed – in your Start menu and in Settings → Apps.").color(green()));
                    ui.horizontal(|ui| {
                        if !crate::install::has_start_menu_entry() && ui.button("Add to Start menu").clicked() {
                            action = Some("startmenu");
                        }
                        if ui.button("Uninstall…").clicked() {
                            action = Some("uninstall");
                        }
                    });
                } else {
                    ui.label("Running from a downloaded copy. Install it so it has a Start menu entry and can update itself:");
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!self.busy_update, egui::Button::new("Install PianoCraft")).clicked() {
                            action = Some("install");
                        }
                        if ui.add_enabled(!self.busy_update, egui::Button::new("Install + desktop shortcut")).clicked() {
                            action = Some("install_desktop");
                        }
                    });
                    if let Some(d) = crate::install::install_dir() {
                        ui.label(RichText::new(format!("Installs to {} (no admin rights needed). Your progress and notes stay where they are.", d.display())).small().weak());
                    }
                }
            });
            ui.add_space(12.0);
            ui.heading("Your music folders");
            card(ui, |ui| {
                ui.label("Every PDF, MIDI and MusicXML file in these folders appears on the left under 'Books & sheet music'. Separate several folders with ;");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.progress.scan_folder).desired_width(520.0).hint_text(r"C:\Music\Piano; C:\Music\Guitar"));
                    if ui.button("Look again").clicked() {
                        action = Some("rescan");
                    }
                });
                ui.label(RichText::new(format!("{} PDFs found.", self.progress.pdf_index.len())).small().weak());
                ui.add_space(4.0);
                let todo: Vec<PathBuf> = self.progress.pdf_index.iter().map(PathBuf::from).filter(|p| self.conv_redo || !self.converted_from(p)).collect();
                ui.horizontal(|ui| {
                    let busy = self.conv.as_ref().is_some_and(|c| !c.finished);
                    if ui.add_enabled(!busy && !todo.is_empty(), egui::Button::new(format!("🔊 Make all {} PDFs playable", todo.len()))).on_hover_text("Typeset sheet music is read in seconds. Scanned books need Audiveris (tick below). PDFs that aren't music are skipped and never changed.").clicked() {
                        action = Some("convert_all");
                    }
                    ui.checkbox(&mut self.conv_scans, "include scanned books (slow)");
                    ui.checkbox(&mut self.conv_redo, "convert again");
                });
                ui.label(RichText::new("You don't have to do this: any PDF is made playable by itself the first time you open an exercise in it.").small().weak());
                if let Some(c) = &self.conv {
                    if c.finished && !c.skipped.is_empty() {
                        ui.collapsing(format!("Skipped last time ({})", c.skipped.len()), |ui| {
                            for (p, why) in &c.skipped {
                                ui.label(RichText::new(format!("{} – {why}", p.file_name().and_then(|n| n.to_str()).unwrap_or("?"))).small());
                            }
                        });
                    }
                }
            });
            ui.add_space(12.0);
            ui.heading("Your loaded resources");
            ui.label("The books the course uses, and where they are. They're checked when you open them; if one has moved, you'll be asked to relink it.");
            card(ui, |ui| {
                ui.label(RichText::new("Moved your music (say, to another drive)?").strong());
                ui.label(RichText::new("Pick the folder it's in now. PianoCraft finds every book and PDF it knows by file name – sub-folders included – and updates all the links, including the pages you've made playable and your notes.").small());
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.relink_folder).desired_width(460.0).hint_text(r"D:\Music"));
                    #[cfg(windows)]
                    if ui.button("Choose folder…").clicked() {
                        if let Some(f) = rfd::FileDialog::new().pick_folder() {
                            self.relink_folder = f.display().to_string();
                        }
                    }
                    if ui.add_enabled(!self.relink_folder.trim().is_empty(), egui::Button::new("🔗 Relink everything")).clicked() {
                        action = Some("relink_all");
                    }
                });
            });
            ui.horizontal(|ui| {
                if ui.button("Find my books in my music folders").clicked() {
                    let mut n = 0;
                    for f in self.progress.scan_folder.split(';').map(|f| f.trim().trim_matches('"')).filter(|f| !f.is_empty()) {
                        for (id, p) in books::locate(&PathBuf::from(f)) {
                            self.progress.books.insert(id, p.display().to_string());
                            n += 1;
                        }
                    }
                    self.msg = format!("Found {n} of {} books.", BookId::ALL.len());
                    self.progress.save();
                }
            });
            // status is only what's been checked (when a file was opened, or with 'Check them all') –
            // nothing here touches the disk by itself
            let status = |p: Option<PathBuf>| -> RichText {
                match p.and_then(|p| self.exists_cache.borrow().get(&p).copied()) {
                    Some(true) => RichText::new("✔").color(green()),
                    Some(false) => RichText::new("missing").color(Color32::LIGHT_RED),
                    None => RichText::new("·").weak(),
                }
            };
            if ui.small_button("Check them all now").on_hover_text("✔ = found, missing = not where it was, · = not checked yet").clicked() {
                action = Some("check_files");
            }
            egui::Grid::new("books").num_columns(3).spacing(Vec2::new(10.0, 6.0)).show(ui, |ui| {
                for b in BookId::ALL {
                    ui.label(status(self.progress.book_path(b)));
                    ui.label(b.title());
                    let mut path = self.progress.books.get(&b).cloned().unwrap_or_default();
                    ui.horizontal(|ui| {
                        if ui.add(egui::TextEdit::singleline(&mut path).desired_width(360.0).hint_text("path to the PDF")).changed() {
                            self.progress.books.insert(b, path.trim().trim_matches('"').to_string());
                            self.progress.save();
                        }
                        #[cfg(windows)]
                        if ui.small_button("…").clicked() {
                            if let Some(p) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file() {
                                self.progress.books.insert(b, p.display().to_string());
                                self.progress.save();
                            }
                        }
                    });
                    ui.end_row();
                }
            });
            // everything else you've made playable or imported
            let files: Vec<(String, String, String)> = self
                .progress
                .library
                .iter()
                .filter_map(|p| match &p.origin {
                    Origin::Imported(o) => Some((p.id.clone(), p.title.clone(), o.clone())),
                    _ => None,
                })
                .collect();
            egui::CollapsingHeader::new(format!("Pieces and PDFs you've made playable or imported ({})", files.len())).show(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("res").max_height(320.0).show(ui, |ui| {
                    egui::Grid::new("res-grid").num_columns(4).striped(true).spacing(Vec2::new(10.0, 4.0)).show(ui, |ui| {
                        for (id, title, path) in &files {
                            ui.label(status(Some(PathBuf::from(path))));
                            ui.label(title);
                            ui.label(RichText::new(path).small().weak());
                            ui.horizontal(|ui| {
                                #[cfg(windows)]
                                if ui.small_button("…").on_hover_text("Point to where it is now").clicked() {
                                    if let Some(p) = rfd::FileDialog::new().pick_file() {
                                        relink_one = Some((path.clone(), p.display().to_string()));
                                    }
                                }
                                if ui.small_button("🗑").on_hover_text("Remove it from PianoCraft (the file itself is not touched)").clicked() {
                                    remove_piece = Some(id.clone());
                                }
                            });
                            ui.end_row();
                        }
                    });
                });
            });
            ui.add_space(12.0);
            ui.heading("Make pages playable (Audiveris)");
            let found = find_audiveris(&self.progress.audiveris);
            card(ui, |ui| {
                ui.label("Audiveris is a free program that reads printed music and turns it into notes. Install it once; then any book page can be heard, slowed down, looped and shown as falling notes.");
                ui.label("1. Download the Windows installer (.msi) from the Audiveris releases page and install it.");
                ui.label("2. Come back here – PianoCraft finds it automatically (or paste the path to Audiveris.exe below).");
                ui.label("3. On any book page press 'Make this page playable' (about a minute per page) or '📚 Whole book' (runs in the background). Old scans can have mistakes – compare with the page. Typeset PDFs (MuseScore, Sibelius, Dorico…) don't need Audiveris at all.");
                ui.horizontal(|ui| {
                    if ui.button("Open the Audiveris download page ↗").clicked() {
                        let _ = webbrowser::open(AUDIVERIS_URL);
                    }
                    match &found {
                        Some(p) => ui.label(RichText::new(format!("✔ found: {}", p.display())).color(green())),
                        None => ui.label(RichText::new("not installed yet").color(Color32::LIGHT_RED)),
                    };
                });
                ui.horizontal(|ui| {
                    ui.label("Audiveris.exe:");
                    if ui.add(egui::TextEdit::singleline(&mut self.progress.audiveris).desired_width(420.0).hint_text(r"C:\Program Files\Audiveris\Audiveris.exe")).changed() {
                        self.progress.save();
                    }
                });
            });
            ui.add_space(12.0);
            ui.heading("Keyboard");
            ui.horizontal(|ui| {
                ui.label("Size:");
                for (k, _, _, name) in KEYBOARDS {
                    if ui.selectable_label(self.keyview.keys() == k, name).clicked() {
                        self.keyview.set_keys(k);
                        self.progress.keyboard_keys = Some(k);
                        self.progress.save();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Names on keys:");
                ui.selectable_value(&mut self.key_names, KeyNames::All, "All keys (C♯/D♭ on black keys)");
                ui.selectable_value(&mut self.key_names, KeyNames::WhiteOnly, "White keys only");
                ui.selectable_value(&mut self.key_names, KeyNames::COnly, "Only the Cs");
            });
            ui.add_space(12.0);
            ui.heading("Sound");
            ui.label(format!("Output: {}", self.audio.device_name));
            if ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0).text("Volume")).changed() {
                self.audio.send(Cmd::Volume(self.volume));
            }
            ui.add_space(12.0);
            ui.heading("Your piano (optional)");
            card(ui, |ui| {
                ui.label(RichText::new("Keyboards with round 5-pin MIDI sockets").strong());
                ui.label("• Many digital pianos have round 5-pin MIDI sockets instead of USB. You need a 'USB MIDI interface cable' (5-pin DIN to USB), about £10–25. Keyboards with a USB port just need a USB cable.");
                ui.label("• The cable's IN plug goes into the piano's MIDI OUT; USB into the computer. Windows sets it up by itself.");
                ui.label("• A guitar/aux-to-USB lead won't work – it carries sound, not notes.");
            });
            ui.horizontal(|ui| {
                if ui.button("🔄 Look for keyboards").clicked() {
                    self.midi_ports = MidiIn::list_ports();
                    if self.midi_ports.is_empty() {
                        self.msg = "No MIDI devices found. Check the cable, then try again.".into();
                    }
                }
                if self.midi.connected() && ui.button("Disconnect").clicked() {
                    self.midi.disconnect();
                    self.progress.midi_port = None;
                    self.progress.save();
                }
            });
            for p in self.midi_ports.clone() {
                ui.horizontal(|ui| {
                    let is = self.midi.port_name.as_deref() == Some(p.as_str());
                    ui.label(if is { format!("● {p}") } else { format!("○ {p}") });
                    if !is && ui.button("Connect").clicked() {
                        self.midi.connect(&p, self.audio.tx.clone());
                        if self.midi.connected() {
                            self.progress.midi_port = Some(p.clone());
                            self.progress.save();
                        }
                    }
                });
            }
            if let Some(e) = &self.midi.last_error {
                ui.label(RichText::new(e).color(Color32::LIGHT_RED));
            }
            if ui.checkbox(&mut self.echo, "Also play my keyboard through the computer").changed() {
                self.audio.send(Cmd::Echo(self.echo));
            }
            ui.add_space(12.0);
            ui.heading("Your data");
            ui.label(format!("Progress, annotations and imported songs: {}", curriculum::data_dir().display()));
            if ui.button("Open that folder").clicked() {
                open_path(&curriculum::data_dir());
            }
            ui.collapsing("Reset progress", |ui| {
                ui.label("Clears every piece's step and the practice log (imported songs and annotations stay).");
                if ui.button(RichText::new("Reset all progress").color(Color32::LIGHT_RED)).clicked() {
                    self.progress.pieces.clear();
                    self.progress.log.clear();
                    self.progress.ticks.clear();
                    self.progress.save();
                    self.msg = "Progress reset.".into();
                }
            });
        });
        if let Some((old, new)) = relink_one {
            let mut map = std::collections::HashMap::new();
            map.insert(old, new);
            self.relink_map(&map);
        }
        if let Some(id) = remove_piece {
            self.piece_cache.borrow_mut().remove(&id);
            self.progress.remove_piece(&id);
            self.progress.book_links.retain(|_, v| *v != id);
            for v in self.progress.page_conv.values_mut() {
                v.retain(|x| *x != id);
            }
            self.progress.save();
        }
        match action {
            Some("check_files") => {
                let mut all: Vec<PathBuf> = self.progress.books.values().map(PathBuf::from).collect();
                all.extend(self.progress.library.iter().filter_map(|p| match &p.origin {
                    Origin::Imported(o) => Some(PathBuf::from(o)),
                    _ => None,
                }));
                let mut c = self.exists_cache.borrow_mut();
                let mut missing = 0;
                for p in all {
                    let e = p.exists();
                    missing += usize::from(!e);
                    c.insert(p, e);
                }
                self.msg = if missing == 0 { "Everything is where it should be.".into() } else { format!("{missing} file(s) missing – relink them above, or use Relink everything if you moved your music.") };
            }
            Some("rescan") => {
                self.progress.save();
                self.exists_cache.borrow_mut().clear();
                self.rescan_pdfs();
            }
            Some("relink_all") => {
                let root = PathBuf::from(self.relink_folder.trim().trim_matches('"'));
                let (done, lost) = self.relink_all(&root, false);
                self.msg = if lost == 0 { format!("Relinked {done} file(s) to {}.", root.display()) } else { format!("Relinked {done} file(s); {lost} still not found in {}.", root.display()) };
            }
            Some("convert_all") => {
                let jobs = self.progress.pdf_index.iter().map(PathBuf::from).filter(|p| self.conv_redo || !self.converted_from(p)).map(|pdf| Job { pdf, pages: None }).collect();
                let scans = self.conv_scans;
                self.start_convert(jobs, scans);
            }
            Some("check") => self.check_updates(true),
            Some("install") => self.start_install(false),
            Some("install_desktop") => self.start_install(true),
            Some("startmenu") => {
                if let Ok(exe) = std::env::current_exe() {
                    self.msg = match crate::install::ensure_start_menu(&exe) {
                        Ok(()) => "Added to the Start menu.".into(),
                        Err(e) => e,
                    };
                }
            }
            Some("uninstall") => {
                self.progress.save();
                self.msg = crate::install::uninstall().unwrap_or_else(|e| e);
            }
            _ => {}
        }
    }
}

fn scan(dir: &PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    fn walk(d: &std::path::Path, depth: u32, out: &mut Vec<PathBuf>) {
        if depth > 5 || out.len() > 2000 {
            return;
        }
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, depth + 1, out);
            } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                if ["mid", "midi", "musicxml", "xml", "mxl", "pdf"].contains(&ext.to_lowercase().as_str()) {
                    out.push(p);
                }
            }
        }
    }
    walk(dir, 0, &mut out);
    out.sort_by_key(|p| (p.extension().map(|e| e.to_ascii_lowercase() == "pdf").unwrap_or(false), p.file_name().map(|n| n.to_os_string())));
    out
}

fn open_path(p: &std::path::Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(p).spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(p).spawn();
    }
}

fn urlencode(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => o.push(b as char),
            b' ' => o.push('+'),
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}

/// Ready-made technique exercises (Russian School and Nahre Sol).
pub(crate) fn tech_presets() -> [(&'static str, TechSpec); 8] {
    [
        ("Russian: C major, contrary", TechSpec { motion: Motion::Contrary, ..TechSpec::default() }),
        ("Russian: triplet sequence", TechSpec { pattern: Pattern::Groups3, rhythm: Rhythm::Eighths, ..TechSpec::default() }),
        ("Russian: triad inversions", TechSpec { ex: Exercise::TriadInversions, motion: Motion::Parallel, ..TechSpec::default() }),
        ("Nahre: circling scale", TechSpec { pattern: Pattern::Circling, rhythm: Rhythm::Eighths, accent: Accent::Swell, ..TechSpec::default() }),
        ("Nahre: LH swell", TechSpec { ex: Exercise::LhSwell, motion: Motion::Parallel, rhythm: Rhythm::Eighths, ..TechSpec::default() }),
        ("Nahre: ii–V–I chain", TechSpec { ex: Exercise::Progression, progression: Progression::TwoFiveOne, motion: Motion::Parallel, cycle: Cycle::WholeTones, ..TechSpec::default() }),
        ("Nahre: measured trill", TechSpec { ex: Exercise::Trill, motion: Motion::Parallel, ..TechSpec::default() }),
        ("Modes from C", TechSpec { scale: ScaleType::Dorian, tonic: 2, ..TechSpec::default() }),
    ]
}

/// The exercise builder. Returns true when something changed.
fn tech_controls(ui: &mut egui::Ui, s: &mut TechSpec) -> bool {
    let before = *s;
    ui.vertical(|ui| {
        ui.label(RichText::new("Exercise").small().strong());
        ui.horizontal_wrapped(|ui| {
            for e in Exercise::ALL {
                ui.selectable_value(&mut s.ex, e, e.label());
            }
        });
        ui.add_space(6.0);
        let uses_scale = matches!(s.ex, Exercise::Scale | Exercise::FiveFinger | Exercise::LhSwell | Exercise::TriadInversions | Exercise::Cadence);
        if uses_scale {
            ui.label(RichText::new("Scale / mode").small().strong());
            egui::ComboBox::from_id_salt("scale").width(ui.available_width().min(320.0)).selected_text(s.scale.label()).show_ui(ui, |ui| {
                for t in ScaleType::ALL {
                    ui.selectable_value(&mut s.scale, t, t.label());
                }
            });
            ui.add_space(6.0);
        }
        if s.ex == Exercise::Arpeggio {
            ui.label(RichText::new("Chord").small().strong());
            ui.horizontal_wrapped(|ui| {
                for c in ChordType::ALL {
                    ui.selectable_value(&mut s.chord, c, c.label());
                }
            });
            ui.add_space(6.0);
        }
        if s.ex == Exercise::Progression {
            ui.label(RichText::new("Progression").small().strong());
            ui.horizontal_wrapped(|ui| {
                for p in Progression::ALL {
                    ui.selectable_value(&mut s.progression, p, p.label());
                }
                ui.checkbox(&mut s.sevenths, "with 7ths (rootless right hand)");
            });
            ui.add_space(6.0);
        }
        if s.ex != Exercise::EtudeSeed {
            ui.label(RichText::new("Starting key").small().strong());
            ui.horizontal_wrapped(|ui| {
                for pc in 0..12u8 {
                    ui.selectable_value(&mut s.tonic, pc, pc_name(pc, sig_for(pc, s.scale)));
                }
            });
            ui.add_space(6.0);
        }
        if matches!(s.ex, Exercise::Scale | Exercise::Arpeggio | Exercise::LhSwell) {
            ui.label(RichText::new("Octaves").small().strong());
            ui.horizontal(|ui| {
                for o in 1..=4u8 {
                    ui.selectable_value(&mut s.octaves, o, o.to_string());
                }
            });
            ui.add_space(6.0);
        }
        if matches!(s.ex, Exercise::Scale | Exercise::Arpeggio | Exercise::FiveFinger | Exercise::TriadInversions) {
            ui.label(RichText::new("Pattern").small().strong());
            egui::ComboBox::from_id_salt("pat").width(ui.available_width().min(320.0)).selected_text(s.pattern.label()).show_ui(ui, |ui| {
                for p in Pattern::ALL {
                    ui.selectable_value(&mut s.pattern, p, p.label());
                }
            });
            ui.add_space(6.0);
        }
        if s.ex != Exercise::Rhythm {
            ui.label(RichText::new("Hands").small().strong());
            ui.horizontal_wrapped(|ui| {
                for m in Motion::ALL {
                    ui.selectable_value(&mut s.motion, m, m.label());
                }
            });
            ui.add_space(6.0);
        }
        if !matches!(s.ex, Exercise::Trill | Exercise::EtudeSeed | Exercise::TriadInversions | Exercise::Cadence) {
            ui.label(RichText::new("Rhythm").small().strong());
            ui.horizontal_wrapped(|ui| {
                for r in Rhythm::ALL {
                    ui.selectable_value(&mut s.rhythm, r, r.label());
                }
            });
            ui.add_space(6.0);
        }
        if matches!(s.ex, Exercise::Scale | Exercise::Arpeggio | Exercise::FiveFinger | Exercise::TriadInversions) {
            ui.label(RichText::new("Accents / dynamics").small().strong());
            ui.horizontal_wrapped(|ui| {
                for a in Accent::ALL {
                    ui.selectable_value(&mut s.accent, a, a.label());
                }
            });
            ui.add_space(6.0);
        }
        if s.ex == Exercise::Trill {
            ui.label(RichText::new("Trill fingers").small().strong());
            ui.horizontal(|ui| {
                for pair in [(1, 2), (1, 3), (2, 3), (2, 4), (3, 4), (3, 5), (4, 5)] {
                    ui.selectable_value(&mut s.trill_fingers, pair, format!("{}-{}", pair.0, pair.1));
                }
                ui.checkbox(&mut s.trill_half_step, "half step");
            });
            ui.add_space(6.0);
        }
        if s.ex == Exercise::EtudeSeed {
            ui.label(RichText::new("Seed").small().strong());
            ui.horizontal(|ui| {
                for k in 1..=3u32 {
                    ui.selectable_value(&mut s.seed, k, seed_name(k));
                }
            });
            ui.add_space(6.0);
        }
        if s.ex == Exercise::Rhythm {
            ui.label(RichText::new("Rhythm no.").small().strong());
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut s.seed).range(1..=999));
                if ui.small_button("New rhythm").clicked() {
                    s.seed += 1;
                }
            });
            ui.add_space(6.0);
        }
        if !matches!(s.ex, Exercise::EtudeSeed | Exercise::Rhythm) {
            ui.label(RichText::new("Keys").small().strong());
            egui::ComboBox::from_id_salt("cyc").width(ui.available_width().min(320.0)).selected_text(s.cycle.label()).show_ui(ui, |ui| {
                for c in Cycle::ALL {
                    ui.selectable_value(&mut s.cycle, c, c.label());
                }
            });
            ui.add_space(6.0);
        }
    });
    *s != before
}
