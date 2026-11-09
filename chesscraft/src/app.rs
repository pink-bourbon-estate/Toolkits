//! ChessCraft: egui app. Board + tree browser + Learn/Drill/Explore modes + spaced repetition.

use crate::model::{self, Progress, Repertoire};
use eframe::egui;
use egui::{Color32, CornerRadius, Pos2, Rect, RichText, Sense, Vec2};
use shakmaty::{Color as ShColor, File, Move, Position, Rank, Square};

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Menu,
    Explore,
    Learn,
    Drill,
}

#[derive(PartialEq, Clone, Copy)]
enum RepId {
    Modern,
    London,
}

pub struct ChessCraftApp {
    modern: Repertoire,
    london: Repertoire,
    progress: Progress,
    mode: Mode,
    rep: RepId,
    /// Current node in the tree the user is standing on (None = start position).
    cur: Option<usize>,
    /// Drill queue: indices of "our move" nodes due for review, with ply path cached.
    drill_queue: Vec<usize>,
    drill_result: Option<(bool, String)>,
    selected_sq: Option<Square>,
    status: String,
    update_msg: Option<Result<Option<crate::updater::Release>, String>>,
}

impl ChessCraftApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (modern, london) = model::load();
        let progress = model::load_progress();
        Self {
            modern,
            london,
            progress,
            mode: Mode::Menu,
            rep: RepId::Modern,
            cur: None,
            drill_queue: Vec::new(),
            drill_result: None,
            selected_sq: None,
            status: String::new(),
            update_msg: None,
        }
    }

    fn rep(&self) -> &Repertoire {
        match self.rep {
            RepId::Modern => &self.modern,
            RepId::London => &self.london,
        }
    }

    fn path_key(&self, idx: usize) -> String {
        let r = self.rep();
        let path = r.path_to(idx);
        format!("{}|{}", r.id, path.iter().map(|i| r.nodes[*i].san.clone()).collect::<Vec<_>>().join(" "))
    }

    fn goto_start(&mut self) {
        self.cur = None;
        self.selected_sq = None;
    }

    fn play_child(&mut self, idx: usize) {
        self.cur = Some(idx);
        self.selected_sq = None;
    }

    fn go_up(&mut self) {
        let r = self.rep();
        self.cur = self.cur.and_then(|i| r.nodes[i].parent);
        self.selected_sq = None;
    }

    /// Rebuild the drill queue: every "our move" node whose spaced-repetition card is due now.
    fn build_drill_queue(&mut self) {
        let now = model::now_unix();
        let r = self.rep();
        let mut due = Vec::new();
        for (i, n) in r.nodes.iter().enumerate() {
            if !n.user {
                continue;
            }
            let key = format!("{}|{}", r.id, r.path_to(i).iter().map(|j| r.nodes[*j].san.clone()).collect::<Vec<_>>().join(" "));
            let card = self.progress.cards.get(&key).cloned().unwrap_or_default();
            if card.due_unix <= now {
                due.push(i);
            }
        }
        use rand::seq::SliceRandom;
        due.shuffle(&mut rand::thread_rng());
        self.drill_queue = due;
    }

    fn grade_current(&mut self, idx: usize, ok: bool) {
        let key = self.path_key(idx);
        let now = model::now_unix();
        let card = self.progress.cards.entry(key).or_default();
        card.grade(ok, now);
        model::save_progress(&self.progress);
    }
}

impl eframe::App for ChessCraftApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("ChessCraft");
                ui.label(RichText::new(crate::updater::version_label()).weak().small());
                ui.separator();
                if ui.selectable_label(self.mode == Mode::Menu, "Menu").clicked() {
                    self.mode = Mode::Menu;
                }
                if ui.selectable_label(self.mode == Mode::Explore, "Explore").clicked() {
                    self.mode = Mode::Explore;
                }
                if ui.selectable_label(self.mode == Mode::Learn, "Learn").clicked() {
                    self.mode = Mode::Learn;
                }
                if ui.selectable_label(self.mode == Mode::Drill, "Drill").clicked() {
                    self.mode = Mode::Drill;
                    self.build_drill_queue();
                    self.drill_result = None;
                }
                ui.separator();
                if ui.selectable_label(self.rep == RepId::Modern, "Modern (Black)").clicked() {
                    self.rep = RepId::Modern;
                    self.goto_start();
                }
                if ui.selectable_label(self.rep == RepId::London, "London (White)").clicked() {
                    self.rep = RepId::London;
                    self.goto_start();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Check for updates").clicked() {
                        self.update_msg = Some(crate::updater::check());
                    }
                });
            });
            if let Some(res) = &self.update_msg {
                ui.horizontal(|ui| match res {
                    Ok(Some(rel)) => {
                        ui.label(format!("Update available: {} (build {})", rel.title, rel.build));
                        if !rel.notes.trim().is_empty() {
                            ui.label(RichText::new(rel.notes.lines().next().unwrap_or("")).weak().small());
                        }
                        if ui.link("Release notes").clicked() {
                            let _ = webbrowser::open(&rel.page);
                        }
                        if ui.button("Install & restart").clicked() {
                            match crate::updater::install(rel) {
                                Ok(exe) => {
                                    let _ = crate::updater::relaunch(&exe);
                                    std::process::exit(0);
                                }
                                Err(e) => self.status = e,
                            }
                        }
                    }
                    Ok(None) => {
                        ui.label("You're up to date.");
                    }
                    Err(e) => {
                        ui.label(RichText::new(e).color(Color32::from_rgb(200, 90, 90)));
                    }
                });
            }
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&self.status).weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !crate::install::is_installed() {
                        if ui.button("Install ChessCraft").clicked() {
                            match crate::install::install(true) {
                                Ok(p) => self.status = format!("Installed to {}", p.display()),
                                Err(e) => self.status = e,
                            }
                        }
                    } else {
                        ui.label(RichText::new(if crate::install::has_start_menu_entry() { "Installed" } else { "Installed (no menu shortcut)" }).weak());
                        if ui.button("Uninstall").clicked() {
                            match crate::install::uninstall() {
                                Ok(m) | Err(m) => self.status = m,
                            }
                        }
                    }
                });
            });
        });

        match self.mode {
            Mode::Menu => self.ui_menu(ctx),
            Mode::Explore => self.ui_explore(ctx),
            Mode::Learn => self.ui_learn(ctx),
            Mode::Drill => self.ui_drill(ctx),
        }
    }
}

// ---------------- shared board drawing ----------------

const LIGHT: Color32 = Color32::from_rgb(235, 225, 205);
const DARK: Color32 = Color32::from_rgb(120, 95, 70);
const HILITE: Color32 = Color32::from_rgba_premultiplied(90, 170, 90, 140);

fn square_rect(board_rect: Rect, sq: Square, flipped: bool) -> Rect {
    let size = board_rect.width().min(board_rect.height()) / 8.0;
    let (mut f, mut r) = (sq.file() as i32, sq.rank() as i32);
    if !flipped {
        r = 7 - r;
    } else {
        f = 7 - f;
    }
    let x = board_rect.left() + f as f32 * size;
    let y = board_rect.top() + r as f32 * size;
    Rect::from_min_size(Pos2::new(x, y), Vec2::splat(size))
}

fn piece_glyph(role: shakmaty::Role, color: ShColor) -> &'static str {
    use shakmaty::Role::*;
    match (color, role) {
        (ShColor::White, Pawn) => "♙",
        (ShColor::White, Knight) => "♘",
        (ShColor::White, Bishop) => "♗",
        (ShColor::White, Rook) => "♖",
        (ShColor::White, Queen) => "♕",
        (ShColor::White, King) => "♔",
        (ShColor::Black, Pawn) => "♟",
        (ShColor::Black, Knight) => "♞",
        (ShColor::Black, Bishop) => "♝",
        (ShColor::Black, Rook) => "♜",
        (ShColor::Black, Queen) => "♛",
        (ShColor::Black, King) => "♚",
    }
}

/// Draws the board for `pos`, flipped if `flipped`. Returns the clicked square, if any.
fn draw_board(ui: &mut egui::Ui, pos: &shakmaty::Chess, flipped: bool, size: f32, hilite: &[Square]) -> Option<Square> {
    let (resp, painter) = ui.allocate_painter(Vec2::splat(size), Sense::click());
    let rect = resp.rect;
    let sq_size = size / 8.0;
    let mut clicked = None;
    for r in 0..8u32 {
        for f in 0..8u32 {
            let sq = Square::from_coords(File::new(f), Rank::new(r));
            let sr = square_rect(rect, sq, flipped);
            let light = (f + r) % 2 == 1;
            painter.rect_filled(sr, CornerRadius::ZERO, if light { LIGHT } else { DARK });
            if hilite.contains(&sq) {
                painter.rect_filled(sr, CornerRadius::ZERO, HILITE);
            }
            if let Some(piece) = pos.board().piece_at(sq) {
                painter.text(sr.center(), egui::Align2::CENTER_CENTER, piece_glyph(piece.role, piece.color), egui::FontId::proportional(sq_size * 0.72), Color32::BLACK);
            }
            if resp.clicked() {
                if let Some(p) = resp.interact_pointer_pos() {
                    if sr.contains(p) {
                        clicked = Some(sq);
                    }
                }
            }
        }
    }
    clicked
}

fn san_for_move(pos: &shakmaty::Chess, mv: &Move) -> String {
    shakmaty::san::San::from_move(pos, mv).to_string()
}

impl ChessCraftApp {
    /// Legal moves from `pos` whose destination or departure touches `sq`, used to let the user
    /// click a piece then a destination square rather than typing SAN.
    fn legal_moves_from(pos: &shakmaty::Chess, sq: Square) -> Vec<Move> {
        pos.legal_moves().iter().filter(|m| m.from() == Some(sq)).cloned().collect()
    }

    fn ui_menu(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.heading("Welcome to ChessCraft");
                ui.add_space(8.0);
                ui.label("Two repertoires, each generated and move-checked by Stockfish 19: the Modern Defence as Black, and the London System as White.");
                ui.add_space(16.0);
                let mut action: Option<(RepId, Mode)> = None;
                for (id, title, tagline, system, total) in [
                    (RepId::Modern, self.modern.title, self.modern.tagline, self.modern.user_move_count().0, self.modern.user_move_count().1),
                    (RepId::London, self.london.title, self.london.tagline, self.london.user_move_count().0, self.london.user_move_count().1),
                ] {
                    ui.group(|ui| {
                        ui.set_width(520.0);
                        ui.heading(title);
                        ui.label(tagline);
                        ui.label(format!("{system} of {total} of your own positions ({:.0}%) use the plain system move; the rest are honest engine-forced deviations.", 100.0 * system as f32 / total.max(1) as f32));
                        ui.horizontal(|ui| {
                            if ui.button("Explore the tree").clicked() {
                                action = Some((id, Mode::Explore));
                            }
                            if ui.button("Learn move by move").clicked() {
                                action = Some((id, Mode::Learn));
                            }
                            if ui.button("Drill due cards").clicked() {
                                action = Some((id, Mode::Drill));
                            }
                        });
                    });
                    ui.add_space(10.0);
                }
                if let Some((id, mode)) = action {
                    self.rep = id;
                    self.goto_start();
                    self.mode = mode;
                    if mode == Mode::Drill {
                        self.build_drill_queue();
                        self.drill_result = None;
                    }
                }
            });
        });
    }

    /// Free exploration: click through the saved tree, see every branch and its note.
    fn ui_explore(&mut self, ctx: &egui::Context) {
        let flipped = self.rep().side == ShColor::Black;
        egui::SidePanel::right("tree").resizable(true).default_width(340.0).show(ctx, |ui| {
            ui.heading("Line");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                if ui.button("⟵ Start position").clicked() {
                    self.goto_start();
                }
                if let Some(cur) = self.cur {
                    for i in self.rep().path_to(cur) {
                        let n = &self.rep().nodes[i];
                        let label = format!("{}{}. {}", n.ply, if n.color == ShColor::White { "." } else { "..." }, n.san);
                        if ui.selectable_label(i == cur, label).clicked() {
                            self.cur = Some(i);
                        }
                    }
                }
                ui.separator();
                if let Some(cur) = self.cur {
                    let n = &self.rep().nodes[cur];
                    ui.label(RichText::new(&n.note).italics());
                    if n.user && !n.system {
                        if let Some(plan) = &n.plan_san {
                            ui.label(RichText::new(format!("Engine-forced deviation from the plan's {plan} (costs ~{} cp if you play the plan instead).", n.deviation_loss)).color(Color32::from_rgb(190, 140, 60)).small());
                        }
                    }
                    if !n.user {
                        let eval = n.cp.map(|c| format!("{:+.2}", c as f32 / 100.0)).unwrap_or_else(|| "—".into());
                        ui.label(RichText::new(format!("Opponent's #{} choice here, engine eval {eval}.", n.rank)).weak().small());
                    }
                }
                ui.separator();
                ui.label("Continuations:");
                let children: Vec<usize> = self.rep().children(self.cur).to_vec();
                for c in children {
                    let n = &self.rep().nodes[c];
                    let tag = if n.user {
                        if n.system { "system" } else { "engine deviation" }
                    } else {
                        "opponent try"
                    };
                    if ui.button(format!("{}  ({tag})", n.san)).clicked() {
                        self.play_child(c);
                    }
                }
                if self.rep().children(self.cur).is_empty() {
                    ui.label(RichText::new("End of this branch.").weak());
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let size = ui.available_width().min(ui.available_height()).min(620.0);
                let pos = self.rep().position_after(self.cur);
                ui.add_space(8.0);
                if ui.button("Go up one move").clicked() {
                    self.go_up();
                }
                draw_board(ui, &pos, flipped, size, &[]);
            });
        });
    }

    /// Guided mode: shows the position, highlights it's your move or plays the opponent's reply
    /// automatically, explaining the plan as you go.
    fn ui_learn(&mut self, ctx: &egui::Context) {
        let side = self.rep().side;
        let flipped = side == ShColor::Black;
        let pos = self.rep().position_after(self.cur);
        let our_move = pos.turn() == side;
        egui::SidePanel::right("learn").resizable(true).default_width(360.0).show(ctx, |ui| {
            ui.heading("Learn");
            ui.separator();
            if ui.button("⟵ Restart from the top").clicked() {
                self.goto_start();
            }
            ui.add_space(8.0);
            if our_move {
                ui.label(RichText::new("Your move — click a piece, then its destination.").strong());
            } else {
                ui.label(RichText::new("Opponent to move — pick a reply to see how the plan answers it.").strong());
            }
            ui.separator();
            let children: Vec<usize> = self.rep().children(self.cur).to_vec();
            if children.is_empty() {
                ui.label("End of the prepared line here — you're on your own from this point, play naturally from the ideas you've learned.");
            }
            for c in children {
                let n = &self.rep().nodes[c].clone();
                let btn = ui.button(&n.san);
                if btn.clicked() {
                    self.play_child(c);
                }
                ui.label(RichText::new(&n.note).weak().small());
                ui.add_space(4.0);
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let size = ui.available_width().min(ui.available_height()).min(620.0);
                ui.add_space(8.0);
                draw_board(ui, &pos, flipped, size, &[]);
            });
        });
    }

    /// Spaced-repetition drill: shows a position where it's our move, asks the user to find the
    /// plan's move by clicking it on the board, grades the card, and advances.
    fn ui_drill(&mut self, ctx: &egui::Context) {
        let side = self.rep().side;
        let flipped = side == ShColor::Black;
        egui::SidePanel::right("drill").resizable(true).default_width(340.0).show(ctx, |ui| {
            ui.heading("Drill");
            ui.separator();
            ui.label(format!("{} cards due now.", self.drill_queue.len()));
            if ui.button("Rebuild queue").clicked() {
                self.build_drill_queue();
                self.drill_result = None;
            }
            ui.separator();
            if let Some((ok, msg)) = &self.drill_result {
                let color = if *ok { Color32::from_rgb(90, 160, 90) } else { Color32::from_rgb(190, 90, 90) };
                ui.label(RichText::new(msg).color(color).strong());
                if ui.button("Next card").clicked() {
                    self.drill_result = None;
                    if !self.drill_queue.is_empty() {
                        self.drill_queue.remove(0);
                    }
                }
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let Some(&idx) = self.drill_queue.first() else {
                    ui.add_space(40.0);
                    ui.heading("Nothing due right now — nice work.");
                    return;
                };
                let r = self.rep();
                let parent = r.nodes[idx].parent;
                let pos = r.position_after(parent);
                let expected_san = r.nodes[idx].san.clone();
                let size = ui.available_width().min(ui.available_height()).min(620.0);
                ui.add_space(8.0);
                ui.label("Find the plan's move from here.");

                let hilite: Vec<Square> = self.selected_sq.map(|s| vec![s]).unwrap_or_default();
                if let Some(clicked) = draw_board(ui, &pos, flipped, size, &hilite) {
                    if let Some(from) = self.selected_sq {
                        let moves = ChessCraftApp::legal_moves_from(&pos, from);
                        if let Some(mv) = moves.iter().find(|m| m.to() == clicked) {
                            let san = san_for_move(&pos, mv);
                            let ok = san == expected_san;
                            self.grade_current(idx, ok);
                            self.drill_result = Some((ok, if ok { format!("Correct — {san}.") } else { format!("The plan plays {expected_san}, not {san}.") }));
                        }
                        self.selected_sq = None;
                    } else if pos.board().piece_at(clicked).is_some_and(|p| p.color == side) {
                        self.selected_sq = Some(clicked);
                    }
                }
            });
        });
    }
}
