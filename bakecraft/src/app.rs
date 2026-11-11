//! app.rs - the window: everything you see and click.
//!
//! HOW AN egui APP WORKS (for beginners)
//! -------------------------------------
//! egui is an "immediate mode" toolkit. Instead of building a window once and
//! then changing it, we describe the WHOLE window again many times a second:
//!
//! ```text
//! fn update(...) {
//!     draw the top bar
//!     draw the tabs
//!     if the "Add" button was clicked { change the plan }
//! }
//! ```
//!
//! Buttons return "was I clicked this frame?", sliders edit a variable in
//! place, and there are no callbacks to wire up. All the state lives in the
//! `App` struct below.
//!
//! LAYOUT
//! ------
//! * Plan      - pick a bake, see every gram, add it to the plan
//! * Timeline  - all bakes merged on one clock with alarms
//! * Water     - what temperature should the water be?
//! * Sources   - the school formulas behind each average
//!
//! Simple layout shows the essentials; Advanced adds every dial.
//! Press Ctrl+K (Cmd+K on a Mac) for the command palette.

use crate::calculator::{calculate, sheet_as_text, Bakesheet, Inputs, Line, YEAST_TYPES};
use crate::formulas::{build_recipes, Recipe};
use crate::schedule::{self, Batch, Event, Mode};
use crate::settings::{export_dir, Plan, Settings, ThemeChoice};
use crate::sources::{self, Key};
use crate::water::{self, MIXING};
use chrono::{Duration, Local, NaiveDateTime};
use egui::{Align, Color32, CornerRadius, Layout, Margin, RichText, Stroke};
use std::collections::{BTreeMap, HashMap};

/// The version shown in the window. CI sets BAKECRAFT_BUILD to the run number.
pub fn version() -> String {
    format!("0.1.{}", option_env!("BAKECRAFT_BUILD").unwrap_or("0"))
}

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

/// One complete set of colours. Soft cream on charcoal (dark) or charcoal on
/// cream (light) with a coral accent - lower glare than pure white on black.
/// Text/background pairs keep at least WCAG AA contrast (4.5:1).
#[derive(Clone, Copy)]
struct Palette {
    bg: Color32,
    surface: Color32,
    surface2: Color32,
    border: Color32,
    text: Color32,
    muted: Color32,
    accent: Color32,
    select: Color32,
    good: Color32,
    now: Color32,
    clash: Color32,
}

fn palette(dark: bool) -> Palette {
    let c = |r, g, b| Color32::from_rgb(r, g, b);
    if dark {
        Palette {
            bg: c(0x1f, 0x1f, 0x1f),
            surface: c(0x26, 0x26, 0x26),
            surface2: c(0x30, 0x30, 0x30),
            border: c(0x4a, 0x4a, 0x4a),
            text: c(0xe3, 0xdf, 0xd1),
            muted: c(0xa1, 0x9d, 0x92),
            accent: c(0xf7, 0x6f, 0x53),
            select: c(0x5a, 0x33, 0x28),
            good: c(0x9c, 0xc9, 0xa4),
            now: c(0x2f, 0x4a, 0x3f),
            clash: c(0x4d, 0x3a, 0x1e),
        }
    } else {
        Palette {
            bg: c(0xf3, 0xef, 0xe4),
            surface: c(0xfb, 0xf9, 0xf3),
            surface2: c(0xe7, 0xe2, 0xd4),
            border: c(0xb9, 0xb3, 0xa2),
            text: c(0x2b, 0x2a, 0x27),
            muted: c(0x5f, 0x5b, 0x51),
            accent: c(0xb8, 0x45, 0x2d),
            select: c(0xf0, 0xc6, 0xb9),
            good: c(0x2f, 0x6b, 0x3a),
            now: c(0xcf, 0xe6, 0xd6),
            clash: c(0xf5, 0xe0, 0xa8),
        }
    }
}

// ---------------------------------------------------------------------------
// Small pieces of state
// ---------------------------------------------------------------------------

/// The four screens.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Plan,
    Timeline,
    Water,
    Sources,
}

/// Everything the command palette (Ctrl+K) can do.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Command {
    Go(Tab),
    ToggleTheme,
    ToggleAdvanced,
    AddToPlan,
    ExportText,
    ExportCsv,
    OpenExportFolder,
    TestAlarm,
    Settings,
    Help,
}

/// Command name + what it does, in the order the palette lists them.
fn commands() -> Vec<(&'static str, Command)> {
    vec![
        ("Go to: Plan a bake", Command::Go(Tab::Plan)),
        ("Go to: Timeline & alarms", Command::Go(Tab::Timeline)),
        ("Go to: Water temperature", Command::Go(Tab::Water)),
        ("Go to: Sources", Command::Go(Tab::Sources)),
        ("Add this bake to the plan", Command::AddToPlan),
        ("Export production sheet (.txt)", Command::ExportText),
        ("Export timeline (.csv)", Command::ExportCsv),
        ("Open the export folder", Command::OpenExportFolder),
        ("Switch light / dark theme", Command::ToggleTheme),
        ("Switch Simple / Advanced layout", Command::ToggleAdvanced),
        ("Test the alarm", Command::TestAlarm),
        ("Settings", Command::Settings),
        ("Help and keyboard shortcuts", Command::Help),
    ]
}

/// The alarm pop-up: which event is ringing (None = a test alarm).
struct Ringing {
    key: Option<String>,
    title: String,
    how: String,
    done_when: String,
}

/// A rounded "card" with a title - the building block of every screen.
/// (A free function, not a method, so the closure inside can borrow the app freely.)
fn card(p: &Palette, font_size: f32, ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !title.is_empty() {
                ui.label(RichText::new(title).strong().size(font_size * 1.1).color(p.accent));
                ui.add_space(2.0);
            }
            body(ui);
        });
    ui.add_space(8.0);
}

/// A table of bakesheet lines: name | grams | % | note.
fn lines_table(p: &Palette, ui: &mut egui::Ui, id: &str, lines: &[Line], show_pct: bool) {
    egui::Grid::new(id).num_columns(4).spacing([18.0, 6.0]).show(ui, |ui| {
        for l in lines {
            let name = RichText::new(&l.name);
            ui.label(if l.is_total() { name.strong() } else { name });
            ui.label(RichText::new(format!("{} g", l.grams)).monospace().strong());
            match (show_pct, l.pct) {
                (true, Some(pct)) => ui.label(RichText::new(format!("{pct:.1} %")).monospace().color(p.muted)),
                _ => ui.label(""),
            };
            ui.label(RichText::new(&l.note).color(p.muted));
            ui.end_row();
        }
    });
}

/// All the state of the program.
pub struct App {
    recipes: Vec<Recipe>,
    settings: Settings,
    plan: Plan,
    tab: Tab,

    // --- the "Plan a bake" form ---
    recipe_idx: usize,
    inputs: Inputs,
    /// Manual step durations chosen in Advanced mode (step index -> minutes).
    overrides: BTreeMap<usize, i64>,
    mode: Mode,
    /// 0 = today, 1 = tomorrow ...
    day_offset: i64,
    hour: u32,
    minute: u32,

    // --- the Water tab ---
    ddt_c: f64,
    use_preferment: bool,
    preferment_c: f64,
    // calibration helper
    cal_dough_c: f64,
    cal_water_c: f64,

    // --- Sources tab ---
    source_idx: usize,

    // --- Timeline tab ---
    plan_name: String,
    /// A bakesheet window opened from the Timeline (index into plan.batches).
    sheet_window: Option<usize>,

    // --- alarms ---
    ringing: Option<Ringing>,
    /// Event key -> time before which a snoozed alarm stays quiet.
    snoozed: HashMap<String, NaiveDateTime>,

    // --- windows and messages ---
    palette_open: bool,
    palette_query: String,
    settings_open: bool,
    help_open: bool,
    status: String,

    /// Remembers which look is applied so we only rebuild the style when it changes.
    applied_look: Option<(bool, i32, bool)>,
}

impl App {
    pub fn new() -> Self {
        let recipes = build_recipes();
        let settings = Settings::load();
        // Reopen the plan from last time (an unreadable file simply gives an empty plan).
        let plan = std::fs::read_to_string(crate::settings::data_dir().join("current_plan.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let first = &recipes[0];
        App {
            inputs: Inputs::defaults(first),
            ddt_c: first.ddt_c,
            use_preferment: first.preferment.is_some(),
            preferment_c: 24.0,
            recipes,
            settings,
            plan,
            // `--screen timeline|water|sources` opens that screen first (handy for screenshots).
            tab: match std::env::args().skip_while(|a| a != "--screen").nth(1).as_deref() {
                Some("timeline") => Tab::Timeline,
                Some("water") => Tab::Water,
                Some("sources") => Tab::Sources,
                _ => Tab::Plan,
            },
            recipe_idx: 0,
            overrides: BTreeMap::new(),
            mode: Mode::Finish,
            day_offset: 1,
            hour: 10,
            minute: 0,
            cal_dough_c: 25.0,
            cal_water_c: 30.0,
            source_idx: 0,
            plan_name: "my-bake".into(),
            sheet_window: None,
            ringing: None,
            snoozed: HashMap::new(),
            palette_open: false,
            palette_query: String::new(),
            settings_open: false,
            help_open: false,
            status: "Welcome. Pick a bake, then press Ctrl+K any time for commands.".into(),
            applied_look: None,
        }
    }

    // -----------------------------------------------------------------------
    // Look and feel
    // -----------------------------------------------------------------------

    /// Is the dark theme in use right now?
    fn is_dark(&self, ctx: &egui::Context) -> bool {
        match self.settings.theme {
            ThemeChoice::Dark => true,
            ThemeChoice::Light => false,
            ThemeChoice::System => ctx.system_theme().map(|t| t == egui::Theme::Dark).unwrap_or(true),
        }
    }

    /// Apply colours, text size and animation settings (only when they changed).
    fn apply_look(&mut self, ctx: &egui::Context) {
        let dark = self.is_dark(ctx);
        let look = (dark, (self.settings.font_size * 10.0) as i32, self.settings.reduce_motion);
        if self.applied_look == Some(look) {
            return;
        }
        self.applied_look = Some(look);
        let p = palette(dark);
        let theme = if dark { egui::Theme::Dark } else { egui::Theme::Light };
        ctx.set_theme(theme);

        let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
        v.panel_fill = p.bg;
        v.window_fill = p.surface;
        v.extreme_bg_color = p.surface; // background of text boxes
        v.faint_bg_color = p.surface2;
        v.override_text_color = Some(p.text);
        v.hyperlink_color = p.accent;
        v.selection.bg_fill = p.select;
        v.window_stroke = Stroke::new(1.0_f32, p.border);
        v.window_corner_radius = CornerRadius::same(10);
        let radius = CornerRadius::same(6);
        for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
            w.corner_radius = radius;
        }
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
        v.widgets.inactive.bg_fill = p.surface2;
        v.widgets.inactive.weak_bg_fill = p.surface2;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border);
        v.widgets.hovered.bg_stroke = Stroke::new(1.5_f32, p.accent);
        v.widgets.active.bg_stroke = Stroke::new(1.5_f32, p.accent);
        // A clear ring around whatever has keyboard focus (helps keyboard users).
        v.selection.stroke = Stroke::new(2.0_f32, p.accent);
        ctx.set_visuals_of(theme, v);

        let size = self.settings.font_size;
        let motion = if self.settings.reduce_motion { 0.0 } else { 1.0 / 12.0 };
        ctx.style_mut_of(theme, |s| {
            use egui::{FontFamily, FontId, TextStyle};
            let prop = FontFamily::Proportional;
            s.text_styles.insert(TextStyle::Heading, FontId::new(size * 1.45, prop.clone()));
            s.text_styles.insert(TextStyle::Body, FontId::new(size, prop.clone()));
            s.text_styles.insert(TextStyle::Button, FontId::new(size, prop.clone()));
            s.text_styles.insert(TextStyle::Small, FontId::new(size * 0.82, prop));
            s.text_styles.insert(TextStyle::Monospace, FontId::new(size, FontFamily::Monospace));
            s.spacing.item_spacing = egui::vec2(8.0, 7.0);
            s.spacing.button_padding = egui::vec2(10.0, 5.0);
            s.animation_time = motion;
        });
    }

    fn pal(&self, ctx: &egui::Context) -> Palette {
        palette(self.is_dark(ctx))
    }

    // -----------------------------------------------------------------------
    // Small helpers
    // -----------------------------------------------------------------------

    fn recipe(&self) -> Recipe {
        self.recipes[self.recipe_idx].clone()
    }

    fn lookup(&self, key: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|r| r.key == key)
    }

    /// The anchor time chosen in the form (today + day offset, at hour:minute).
    fn anchor(&self) -> NaiveDateTime {
        let day = Local::now().date_naive() + Duration::days(self.day_offset);
        day.and_hms_opt(self.hour, self.minute, 0).unwrap_or_default()
    }

    /// Save the plan to the automatic file so it reopens next time.
    fn persist_plan(&mut self) {
        match serde_json::to_string_pretty(&self.plan) {
            Ok(text) => {
                if let Err(e) = std::fs::write(crate::settings::data_dir().join("current_plan.json"), text) {
                    self.status = format!("Could not save the plan automatically: {e}");
                }
            }
            Err(e) => self.status = format!("Could not save the plan automatically: {e}"),
        }
    }

    fn save_settings(&mut self) {
        if let Err(e) = self.settings.save() {
            self.status = e;
        }
    }

    /// All events of the plan, merged and sorted.
    fn events(&self) -> Vec<Event> {
        schedule::merge_timelines(&self.plan.batches, |k| self.lookup(k))
    }

    /// Add the bake from the form to the plan.
    fn add_to_plan(&mut self) {
        let r = self.recipe();
        if let Err(e) = calculate(&r, &self.inputs) {
            self.status = e;
            return;
        }
        self.plan.batches.push(Batch {
            recipe_key: r.key.to_string(),
            label: format!("{} x{}", r.name, self.inputs.pieces),
            pieces: self.inputs.pieces,
            piece_weight: self.inputs.piece_weight,
            hydration: self.inputs.hydration,
            yeast_type: self.inputs.yeast_type.clone(),
            anchor: self.anchor(),
            mode: self.mode,
            room_c: self.settings.room_c,
            overrides: self.overrides.clone(),
        });
        self.persist_plan();
        self.status = format!("Added {} to the plan. See the Timeline tab.", r.name);
        self.tab = Tab::Timeline;
    }

    /// Write a text/CSV export and report where it went.
    fn export(&mut self, csv: bool) {
        let events = self.events();
        if events.is_empty() {
            self.status = "Nothing to export yet - add a bake to the plan first.".into();
            return;
        }
        let stamp = Local::now().format("%Y%m%d-%H%M");
        let (name, body) = if csv {
            (format!("bake-timeline-{stamp}.csv"), schedule::timeline_as_csv(&events))
        } else {
            let mut text = schedule::timeline_as_text(&events);
            for b in &self.plan.batches {
                if let Some(sheet) = self.sheet_for(b) {
                    text.push_str("\n\n==============================\n");
                    text.push_str(&sheet_as_text(&sheet));
                }
            }
            (format!("bake-sheet-{stamp}.txt"), text)
        };
        let path = export_dir().join(name);
        self.status = match std::fs::write(&path, body) {
            Ok(()) => format!("Saved {} (in your Documents/BakeCraft folder).", path.file_name().and_then(|n| n.to_str()).unwrap_or("export")),
            Err(e) => format!("Could not write the export: {e}"),
        };
    }

    /// Rebuild the bakesheet for a batch in the plan.
    fn sheet_for(&self, b: &Batch) -> Option<Bakesheet> {
        let r = self.lookup(&b.recipe_key)?;
        let inputs = Inputs {
            pieces: b.pieces,
            piece_weight: b.piece_weight,
            hydration: b.hydration,
            yeast_type: b.yeast_type.clone(),
            waste_pct: 1.0,
        };
        calculate(r, &inputs).ok()
    }

    /// Run one palette command.
    fn run(&mut self, cmd: Command) {
        match cmd {
            Command::Go(t) => self.tab = t,
            Command::ToggleTheme => {
                // From "System" the first press picks the opposite of what you see now.
                self.settings.theme = if self.applied_look.map(|l| l.0).unwrap_or(true) { ThemeChoice::Light } else { ThemeChoice::Dark };
                self.save_settings();
            }
            Command::ToggleAdvanced => {
                self.settings.advanced = !self.settings.advanced;
                self.save_settings();
            }
            Command::AddToPlan => self.add_to_plan(),
            Command::ExportText => self.export(false),
            Command::ExportCsv => self.export(true),
            Command::OpenExportFolder => {
                let _ = webbrowser::open(&export_dir().to_string_lossy());
            }
            Command::TestAlarm => {
                self.ringing = Some(Ringing {
                    key: None,
                    title: "Test alarm".into(),
                    how: "This is what an alarm looks like.".into(),
                    done_when: "Press Enter for Done, or Esc to snooze.".into(),
                })
            }
            Command::Settings => self.settings_open = true,
            Command::Help => self.help_open = true,
        }
    }

    // -----------------------------------------------------------------------
    // Alarms
    // -----------------------------------------------------------------------

    /// Look for an event whose alarm should be ringing right now.
    fn check_alarms(&mut self, ctx: &egui::Context) {
        if self.ringing.is_some() {
            return;
        }
        let now = Local::now().naive_local();
        let events = self.events();
        for e in &events {
            let key = e.key();
            // ring only for events that started in the last 90 minutes
            // (so reopening an old plan does not ring for yesterday)
            let due = e.alarm && now >= e.start && now - e.start <= Duration::minutes(90);
            let quiet = self.snoozed.get(&key).is_some_and(|until| *until > now);
            if due && !quiet && !self.plan.done.contains(&key) {
                self.ringing = Some(Ringing { key: Some(key), title: format!("{}: {}", e.batch, e.name.trim()), how: e.how.clone(), done_when: e.done_when.clone() });
                // asks the operating system to flash the taskbar
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
                break;
            }
        }
    }

    /// The alarm pop-up. Done (Enter) or Snooze (Esc / close).
    fn alarm_window(&mut self, ctx: &egui::Context) {
        let Some(r) = &self.ringing else { return };
        let (title, how, done_when, key) = (r.title.clone(), r.how.clone(), r.done_when.clone(), r.key.clone());
        let p = self.pal(ctx);
        let mut action: Option<bool> = None; // Some(true) = done, Some(false) = snooze
        egui::Window::new("Time for the next step")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(380.0);
                ui.label(RichText::new(&title).heading().color(p.accent));
                if !how.is_empty() {
                    ui.label(&how);
                }
                if !done_when.is_empty() {
                    ui.label(RichText::new(format!("Done when: {done_when}")).color(p.muted));
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Done  (Enter)").strong()).clicked() {
                        action = Some(true);
                    }
                    if ui.button(format!("Snooze {} min  (Esc)", self.settings.snooze_minutes)).clicked() {
                        action = Some(false);
                    }
                });
            });
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Enter) {
                action = Some(true);
            } else if i.key_pressed(egui::Key::Escape) {
                action = Some(false);
            }
        });
        if let Some(done) = action {
            if let Some(k) = key {
                if done {
                    self.plan.done.push(k);
                    self.persist_plan();
                } else {
                    let until = Local::now().naive_local() + Duration::minutes(self.settings.snooze_minutes);
                    self.snoozed.insert(k, until);
                }
            }
            self.ringing = None;
        }
    }

    // -----------------------------------------------------------------------
    // Drawing helpers
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Screens
    // -----------------------------------------------------------------------

    fn header(&mut self, ctx: &egui::Context) {
        let p = self.pal(ctx);
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("BakeCraft").heading().color(p.accent));
                ui.add_space(14.0);
                for (tab, name) in [(Tab::Plan, "Plan a bake"), (Tab::Timeline, "Timeline & alarms"), (Tab::Water, "Water"), (Tab::Sources, "Sources")] {
                    ui.selectable_value(&mut self.tab, tab, name);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("?").on_hover_text("Help and shortcuts").clicked() {
                        self.help_open = true;
                    }
                    if ui.button("Settings").clicked() {
                        self.settings_open = true;
                    }
                    if ui.button("Commands  Ctrl+K").clicked() {
                        self.palette_open = true;
                        self.palette_query.clear();
                    }
                    let mut adv = self.settings.advanced;
                    if ui.toggle_value(&mut adv, "Advanced").on_hover_text("Show every dial: hydration, yeast type, step lengths").changed() {
                        self.settings.advanced = adv;
                        self.save_settings();
                    }
                });
            });
            ui.add_space(6.0);
        });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        let p = self.pal(ctx);
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&self.status).color(p.muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("v{}", version())).small().color(p.muted));
                });
            });
        });
    }

    /// Screen 1: choose a bake and see the grams.
    fn plan_tab(&mut self, ctx: &egui::Context) {
        let p = self.pal(ctx);
        let fs = self.settings.font_size;
        let recipe = self.recipe();
        let advanced = self.settings.advanced;

        egui::SidePanel::left("inputs").resizable(true).default_width(360.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                ui.add_space(8.0);
                // ---- 1. What are you making? ----
                card(&p, fs, ui, "1  What are you baking?", |ui| {
                    let before = self.recipe_idx;
                    egui::ComboBox::from_id_salt("recipe").width(ui.available_width() - 8.0).selected_text(recipe.name).show_ui(ui, |ui| {
                        for (i, r) in self.recipes.iter().enumerate() {
                            ui.selectable_value(&mut self.recipe_idx, i, r.name);
                        }
                    });
                    if self.recipe_idx != before {
                        // New recipe: reset the form to that recipe's own defaults.
                        let r = self.recipes[self.recipe_idx].clone();
                        let yeast = self.inputs.yeast_type.clone();
                        self.inputs = Inputs::defaults(&r);
                        self.inputs.yeast_type = yeast;
                        self.overrides.clear();
                        self.ddt_c = r.ddt_c;
                        self.use_preferment = r.preferment.is_some();
                    }
                    ui.label(RichText::new(recipe.description).color(p.muted));
                });

                // ---- 2. How many, how big? ----
                card(&p, fs, ui, "2  How many, how big?", |ui| {
                    egui::Grid::new("howmany").num_columns(2).show(ui, |ui| {
                        ui.label("How many");
                        ui.add(egui::DragValue::new(&mut self.inputs.pieces).range(1..=200));
                        ui.end_row();
                        ui.label("Raw dough per piece");
                        ui.add(egui::DragValue::new(&mut self.inputs.piece_weight).range(20.0..=5000.0).speed(5.0).suffix(" g"));
                        ui.end_row();
                    });
                    if advanced {
                        ui.separator();
                        ui.label(recipe.liquid_label);
                        ui.add(egui::Slider::new(&mut self.inputs.hydration, recipe.hydration_range.0..=recipe.hydration_range.1).suffix(" %"));
                        if let Some((lo, hi)) = recipe.school_range() {
                            ui.label(RichText::new(format!("School formulas range {lo:.0}-{hi:.0} %. The default is their average ({} %).", recipe.hydration)).small().color(p.muted));
                        }
                        if ui.small_button("Reset to the average").clicked() {
                            self.inputs.hydration = recipe.hydration;
                        }
                        if recipe.yeast_instant > 0.0 || recipe.preferment == Some("poolish") {
                            ui.horizontal(|ui| {
                                ui.label("Yeast");
                                egui::ComboBox::from_id_salt("yeast").selected_text(self.inputs.yeast_type.clone()).show_ui(ui, |ui| {
                                    for (name, _) in YEAST_TYPES {
                                        ui.selectable_value(&mut self.inputs.yeast_type, name.to_string(), name);
                                    }
                                });
                            });
                        }
                        ui.horizontal(|ui| {
                            ui.label("Bowl loss");
                            ui.add(egui::DragValue::new(&mut self.inputs.waste_pct).range(0.0..=10.0).speed(0.1).suffix(" %"));
                        });
                    }
                });

                // ---- 3. When? ----
                card(&p, fs, ui, "3  When?", |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.mode, Mode::Finish, "Ready by");
                        ui.selectable_value(&mut self.mode, Mode::Start, "Start at");
                    });
                    let when = self.anchor();
                    ui.horizontal(|ui| {
                        ui.label("In");
                        ui.add(egui::DragValue::new(&mut self.day_offset).range(0..=30).suffix(" day(s)"));
                        ui.label("at");
                        ui.add(egui::DragValue::new(&mut self.hour).range(0..=23).custom_formatter(|n, _| format!("{:02}", n as i64)));
                        ui.label(":");
                        ui.add(egui::DragValue::new(&mut self.minute).range(0..=59).speed(1.0).custom_formatter(|n, _| format!("{:02}", n as i64)));
                    });
                    ui.label(RichText::new(when.format("%A %d %B, %H:%M").to_string()).strong());
                    ui.horizontal(|ui| {
                        ui.label("Kitchen temperature");
                        if ui.add(egui::DragValue::new(&mut self.settings.room_c).range(10.0..=35.0).suffix(" C")).changed() {
                            self.save_settings();
                        }
                    });
                    ui.label(RichText::new("Warmer kitchen = shorter ferments. Colder = longer.").small().color(p.muted));
                });

                if advanced {
                    card(&p, fs, ui, "Step lengths (at the recipe's own temperature)", |ui| {
                        for (i, s) in recipe.steps.iter().enumerate() {
                            if s.max_minutes == 0 {
                                continue;
                            }
                            let mut mins = self.overrides.get(&i).copied().unwrap_or(s.minutes);
                            ui.horizontal(|ui| {
                                if ui.add(egui::DragValue::new(&mut mins).range(s.min_minutes..=s.max_minutes).suffix(" min")).changed() {
                                    self.overrides.insert(i, mins);
                                }
                                ui.label(s.name);
                            });
                        }
                    });
                }

                if ui.add_sized([ui.available_width(), 36.0], egui::Button::new(RichText::new("Add to plan   (Ctrl+Enter)").strong())).clicked() {
                    self.add_to_plan();
                }
                ui.add_space(12.0);
            });
        });

        // ---- the bakesheet ----
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                ui.add_space(8.0);
                match calculate(&recipe, &self.inputs) {
                    Err(e) => {
                        ui.colored_label(p.accent, e);
                    }
                    Ok(sheet) => {
                        ui.label(RichText::new(format!("{} x {} @ {} g", sheet.pieces, recipe.piece_name, sheet.piece_weight)).heading());
                        ui.label(
                            RichText::new(format!("Total flour {:.0} g  ·  total dough {:.0} g (includes {} % bowl loss)", sheet.total_flour, sheet.total_dough, self.inputs.waste_pct))
                                .color(p.muted),
                        );
                        ui.add_space(8.0);
                        if !sheet.preferment.is_empty() {
                            let title = if recipe.preferment == Some("levain") { "Step 1  Build the levain (the day before)" } else { "Step 1  Mix the poolish (the night before)" };
                            card(&p, fs, ui, title, |ui| lines_table(&p, ui, "pref", &sheet.preferment, false));
                        }
                        let title = if sheet.preferment.is_empty() { "Weigh for mix day" } else { "Step 2  Weigh for mix day" };
                        card(&p, fs, ui, title, |ui| lines_table(&p, ui, "final", &sheet.final_dough, false));
                        if !sheet.lamination.is_empty() {
                            card(&p, fs, ui, "Lamination", |ui| lines_table(&p, ui, "lam", &sheet.lamination, false));
                        }
                        card(&p, fs, ui, "Divide", |ui| lines_table(&p, ui, "div", &sheet.divide, false));
                        card(&p, fs, ui, "Bake", |ui| {
                            ui.label(recipe.bake);
                            if !recipe.notes.is_empty() {
                                ui.add_space(4.0);
                                ui.label(RichText::new(recipe.notes).color(p.muted));
                            }
                        });
                        if advanced {
                            card(&p, fs, ui, "Check: the whole formula (baker's %)", |ui| {
                                lines_table(&p, ui, "overall", &sheet.overall, true);
                                ui.label(RichText::new("The levain/poolish water is counted here, so the hydration is the true figure.").small().color(p.muted));
                            });
                        }
                        ui.label(RichText::new("Times and temperatures are estimates. When the dough and the clock disagree, go with the dough.").small().color(p.muted));
                    }
                }
            });
        });
    }

    /// Screen 2: the merged timeline.
    fn timeline_tab(&mut self, ctx: &egui::Context) {
        let p = self.pal(ctx);
        let fs = self.settings.font_size;
        let events = self.events();
        let clashes = schedule::find_clashes(&events, 10);
        let clash_set: std::collections::HashSet<usize> = clashes.iter().flat_map(|(a, b)| [*a, *b]).collect();
        let now = Local::now().naive_local();

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                ui.add_space(8.0);

                // ---- the bakes in the plan ----
                card(&p, fs, ui, "Bakes in this plan", |ui| {
                    if self.plan.batches.is_empty() {
                        ui.label("Nothing here yet. Choose a bake on the Plan tab and press Add to plan.");
                    }
                    let mut remove: Option<usize> = None;
                    let mut nudge: Option<(usize, i64)> = None;
                    let mut sheet: Option<usize> = None;
                    for (i, b) in self.plan.batches.iter().enumerate() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(&b.label).strong());
                            let what = if b.mode == Mode::Finish { "ready by" } else { "starts" };
                            ui.label(RichText::new(format!("{what} {}", b.anchor.format("%a %d %b %H:%M"))).color(p.muted));
                            if ui.small_button("-15 min").clicked() {
                                nudge = Some((i, -15));
                            }
                            if ui.small_button("+15 min").clicked() {
                                nudge = Some((i, 15));
                            }
                            if ui.small_button("Bakesheet").clicked() {
                                sheet = Some(i);
                            }
                            if ui.small_button("Remove").clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some((i, m)) = nudge {
                        self.plan.batches[i].anchor += Duration::minutes(m);
                        self.persist_plan();
                    }
                    if let Some(i) = sheet {
                        self.sheet_window = Some(i);
                    }
                    if let Some(i) = remove {
                        self.plan.batches.remove(i);
                        // "done" keys use batch numbers, so clear them to avoid ticking the wrong rows
                        self.plan.done.clear();
                        self.persist_plan();
                    }
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Plan name");
                        ui.add(egui::TextEdit::singleline(&mut self.plan_name).desired_width(140.0));
                        if ui.button("Save").clicked() {
                            self.status = match self.plan.save(&self.plan_name) {
                                Ok(_) => format!("Saved plan '{}'.", Plan::safe_name(&self.plan_name)),
                                Err(e) => e,
                            };
                        }
                        egui::ComboBox::from_id_salt("openplan").selected_text("Open saved plan").show_ui(ui, |ui| {
                            for name in Plan::saved_names() {
                                if ui.selectable_label(false, &name).clicked() {
                                    match Plan::load(&name) {
                                        Ok(pl) => {
                                            self.plan = pl;
                                            self.plan_name = name.clone();
                                            self.persist_plan();
                                            self.status = format!("Opened plan '{name}'.");
                                        }
                                        Err(e) => self.status = e,
                                    }
                                }
                            }
                        });
                        if ui.button("Export .txt").clicked() {
                            self.export(false);
                        }
                        if ui.button("Export .csv").clicked() {
                            self.export(true);
                        }
                        if ui.button("Clear plan").clicked() {
                            self.plan = Plan::default();
                            self.persist_plan();
                        }
                    });
                });

                if !clashes.is_empty() {
                    ui.colored_label(p.accent, format!("{} busy moment(s): hands-on jobs from different bakes land within 10 minutes of each other (amber rows). Nudge one bake by 15 minutes.", clashes.len()));
                    ui.add_space(6.0);
                }

                // ---- the timeline, one heading per day ----
                let mut day = String::new();
                let mut toggle: Option<String> = None;
                for (idx, e) in events.iter().enumerate() {
                    let d = e.start.format("%A %d %B").to_string();
                    if d != day {
                        ui.add_space(8.0);
                        ui.label(RichText::new(d.to_uppercase()).strong().color(p.accent));
                        day = d;
                    }
                    let key = e.key();
                    let done = self.plan.done.contains(&key);
                    let is_now = now >= e.start && now < e.end;
                    let fill = if is_now {
                        p.now
                    } else if clash_set.contains(&idx) {
                        p.clash
                    } else {
                        p.surface
                    };
                    egui::Frame::new().fill(fill).stroke(Stroke::new(1.0_f32, p.border)).corner_radius(CornerRadius::same(6)).inner_margin(Margin::symmetric(10, 4)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let mut d2 = done;
                            if ui.checkbox(&mut d2, "").on_hover_text("Mark as done").changed() {
                                toggle = Some(key.clone());
                            }
                            let mut title = format!("{} - {}   {}", e.start.format("%H:%M"), e.end.format("%H:%M"), e.name.trim());
                            if !e.is_fold {
                                title.push_str(&format!("   ({} min)", e.minutes()));
                            }
                            let mut text = RichText::new(title);
                            if done {
                                text = text.color(p.muted).strikethrough();
                            } else if !e.is_fold {
                                text = text.strong();
                            }
                            if e.how.is_empty() && e.done_when.is_empty() {
                                ui.label(text);
                            } else {
                                egui::CollapsingHeader::new(text).id_salt(&key).show(ui, |ui| {
                                    if !e.how.is_empty() {
                                        ui.label(&e.how);
                                    }
                                    if !e.done_when.is_empty() {
                                        ui.label(RichText::new(format!("Done when: {}", e.done_when)).color(p.muted));
                                    }
                                });
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new(&e.batch).small().color(p.muted));
                                if is_now {
                                    ui.label(RichText::new("NOW").small().strong().color(p.good));
                                }
                            });
                        });
                    });
                }
                if let Some(k) = toggle {
                    if let Some(pos) = self.plan.done.iter().position(|x| *x == k) {
                        self.plan.done.remove(pos);
                    } else {
                        self.plan.done.push(k);
                    }
                    self.persist_plan();
                }
                ui.add_space(12.0);
                ui.label(RichText::new("Alarms ring while BakeCraft is open (minimised is fine). Done dismisses; Snooze rings again later.").small().color(p.muted));
            });
        });
    }

    /// Screen 3: water temperature.
    fn water_tab(&mut self, ctx: &egui::Context) {
        let p = self.pal(ctx);
        let fs = self.settings.font_size;
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("What temperature should the water be?").heading());
                ui.label(RichText::new("Every timing in a recipe assumes the dough comes off the mixer at a set temperature. Water is the easiest thing to adjust.").color(p.muted));
                ui.add_space(8.0);

                let friction = self.settings.custom_friction.unwrap_or_else(|| water::friction_for(&self.settings.mixing));
                card(&p, fs, ui, "Your numbers", |ui| {
                    egui::Grid::new("waterin").num_columns(2).show(ui, |ui| {
                        ui.label("Desired dough temperature");
                        ui.add(egui::DragValue::new(&mut self.ddt_c).range(15.0..=35.0).suffix(" C"));
                        ui.end_row();
                        ui.label("Flour temperature");
                        if ui.add(egui::DragValue::new(&mut self.settings.flour_c).range(0.0..=40.0).suffix(" C")).changed() {
                            self.save_settings();
                        }
                        ui.end_row();
                        ui.label("Room temperature");
                        if ui.add(egui::DragValue::new(&mut self.settings.room_c).range(0.0..=45.0).suffix(" C")).changed() {
                            self.save_settings();
                        }
                        ui.end_row();
                        ui.label("Levain / poolish in this dough?");
                        ui.checkbox(&mut self.use_preferment, "yes");
                        ui.end_row();
                        if self.use_preferment {
                            ui.label("Levain / poolish temperature");
                            ui.add(egui::DragValue::new(&mut self.preferment_c).range(5.0..=40.0).suffix(" C"));
                            ui.end_row();
                        }
                        ui.label("How you mix");
                        egui::ComboBox::from_id_salt("mixing").selected_text(self.settings.mixing.clone()).show_ui(ui, |ui| {
                            for (name, f) in MIXING {
                                if ui.selectable_value(&mut self.settings.mixing, name.to_string(), format!("{name}  (friction {f} C)")).changed() {
                                    self.settings.custom_friction = None;
                                    self.save_settings();
                                }
                            }
                        });
                        ui.end_row();
                    });
                });

                let pref = if self.use_preferment { Some(self.preferment_c) } else { None };
                let w = water::water_temperature(self.ddt_c, self.settings.flour_c, self.settings.room_c, pref, friction);
                card(&p, fs, ui, "Use water at", |ui| {
                    ui.label(RichText::new(format!("{w:.0} C")).size(self.settings.font_size * 3.0).strong().color(p.accent));
                    ui.label(water::advice(w));
                    ui.label(RichText::new(format!("Friction used: {friction:.1} C{}", if self.settings.custom_friction.is_some() { " (your own calibration)" } else { "" })).small().color(p.muted));
                });

                card(&p, fs, ui, "Calibrate your mixer (optional)", |ui| {
                    ui.label("After a normal mix, probe the dough and enter what you actually used. BakeCraft works out YOUR friction number.");
                    egui::Grid::new("cal").num_columns(2).show(ui, |ui| {
                        ui.label("Dough temperature after mixing");
                        ui.add(egui::DragValue::new(&mut self.cal_dough_c).range(10.0..=40.0).suffix(" C"));
                        ui.end_row();
                        ui.label("Water temperature you used");
                        ui.add(egui::DragValue::new(&mut self.cal_water_c).range(0.0..=50.0).suffix(" C"));
                        ui.end_row();
                    });
                    let f = water::friction_from_mix(self.cal_dough_c, self.settings.flour_c, self.settings.room_c, self.cal_water_c, pref);
                    ui.label(format!("Your friction: {f:.1} C"));
                    ui.horizontal(|ui| {
                        if ui.button("Use this number").clicked() {
                            self.settings.custom_friction = Some((f * 10.0).round() / 10.0);
                            self.save_settings();
                            self.status = "Saved your mixer friction.".into();
                        }
                        if self.settings.custom_friction.is_some() && ui.button("Go back to the default").clicked() {
                            self.settings.custom_friction = None;
                            self.save_settings();
                        }
                    });
                });
            });
        });
    }

    /// Screen 4: where the numbers come from.
    fn sources_tab(&mut self, ctx: &egui::Context) {
        let p = self.pal(ctx);
        let fs = self.settings.font_size;
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("Where the recipes come from").heading());
                ui.label(RichText::new("Each recipe is the average of published formulas from professional schools and bakeries. A reference book is shown for comparison but never averaged. The formulas are factual ratios with their sources cited - please buy the books.").color(p.muted));
                ui.add_space(8.0);

                // Choose a recipe family via the recipes list (two sourdoughs share a family).
                egui::ComboBox::from_id_salt("srcrecipe").selected_text(self.recipes[self.source_idx].name).width(320.0).show_ui(ui, |ui| {
                    for (i, r) in self.recipes.iter().enumerate() {
                        ui.selectable_value(&mut self.source_idx, i, r.name);
                    }
                });
                let r = self.recipes[self.source_idx].clone();
                let main_key = if matches!(r.family, "brioche" | "croissant") { Key::Milk } else { Key::Hydration };
                ui.add_space(6.0);
                let avg_txt = sources::average(r.family, main_key).map(|a| format!("{a:.1} %")).unwrap_or_else(|| "n/a".into());
                let range_txt = sources::spread(r.family, main_key).map(|(lo, hi)| format!("{lo:.1}-{hi:.1} %")).unwrap_or_else(|| "n/a".into());
                ui.label(RichText::new(format!("{}: school average {avg_txt}  (range {range_txt})", r.liquid_label)).strong());
                ui.add_space(6.0);

                for s in sources::sources_for(r.family, true) {
                    card(&p, fs, ui, "", |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(s.name).strong());
                            if s.reference_only {
                                ui.label(RichText::new("reference only - not averaged").small().color(p.accent));
                            }
                        });
                        ui.label(RichText::new(s.origin).color(p.muted));
                        let mut nums = vec![];
                        for (label, key) in [
                            ("liquid/hydration", Key::Hydration),
                            ("milk", Key::Milk),
                            ("eggs", Key::Eggs),
                            ("salt", Key::Salt),
                            ("yeast", Key::YeastInstant),
                            ("sugar", Key::Sugar),
                            ("butter", Key::Butter),
                            ("oil", Key::Oil),
                            ("roll-in", Key::RollinButter),
                            ("prefermented flour", Key::Pff),
                        ] {
                            if let Some(v) = s.values.get(key) {
                                nums.push(format!("{label} {v} %"));
                            }
                        }
                        if let Some(h) = s.values.pref_hydration {
                            nums.push(format!("preferment hydration {h} %"));
                        }
                        ui.label(nums.join("  ·  "));
                        ui.label(RichText::new(s.citation).small().color(p.muted));
                        if !s.notes.is_empty() {
                            ui.label(RichText::new(s.notes).small());
                        }
                        if let Some(url) = s.url {
                            ui.hyperlink_to("Open the source", url);
                        }
                    });
                }
            });
        });
    }

    // -----------------------------------------------------------------------
    // Pop-up windows
    // -----------------------------------------------------------------------

    fn palette_window(&mut self, ctx: &egui::Context) {
        if !self.palette_open {
            return;
        }
        let mut chosen: Option<Command> = None;
        let mut close = false;
        egui::Window::new("Commands")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
            .show(ctx, |ui| {
                ui.set_width(440.0);
                let edit = ui.add(egui::TextEdit::singleline(&mut self.palette_query).hint_text("Type a command...").desired_width(f32::INFINITY));
                edit.request_focus(); // always ready to type
                let q = self.palette_query.to_lowercase();
                let matches: Vec<_> = commands().into_iter().filter(|(name, _)| name.to_lowercase().contains(&q)).collect();
                ui.add_space(4.0);
                for (name, cmd) in &matches {
                    if ui.selectable_label(false, *name).clicked() {
                        chosen = Some(*cmd);
                    }
                }
                if matches.is_empty() {
                    ui.label("No matching command.");
                }
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    chosen = matches.first().map(|(_, c)| *c);
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
            });
        if let Some(c) = chosen {
            self.palette_open = false;
            self.run(c);
        } else if close {
            self.palette_open = false;
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        let mut open = self.settings_open;
        let mut changed = false;
        egui::Window::new("Settings").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.label(RichText::new("Appearance").strong());
            ui.horizontal(|ui| {
                changed |= ui.selectable_value(&mut self.settings.theme, ThemeChoice::System, "Follow system").changed();
                changed |= ui.selectable_value(&mut self.settings.theme, ThemeChoice::Dark, "Dark").changed();
                changed |= ui.selectable_value(&mut self.settings.theme, ThemeChoice::Light, "Light").changed();
            });
            changed |= ui.add(egui::Slider::new(&mut self.settings.font_size, 12.0..=22.0).text("Text size")).changed();
            changed |= ui.checkbox(&mut self.settings.reduce_motion, "Reduce motion (no animations)").changed();
            ui.separator();
            ui.label(RichText::new("Kitchen defaults").strong());
            changed |= ui.add(egui::DragValue::new(&mut self.settings.room_c).range(5.0..=40.0).prefix("Room ").suffix(" C")).changed();
            changed |= ui.add(egui::DragValue::new(&mut self.settings.flour_c).range(0.0..=40.0).prefix("Flour ").suffix(" C")).changed();
            changed |= ui.add(egui::DragValue::new(&mut self.settings.snooze_minutes).range(1..=60).prefix("Snooze ").suffix(" min")).changed();
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Open the data folder").clicked() {
                    let _ = webbrowser::open(&crate::settings::data_dir().to_string_lossy());
                }
                if ui.button("Reset all settings").clicked() {
                    self.settings = Settings::default();
                    changed = true;
                }
            });
            ui.label("Settings and plans are stored only on this computer.");
        });
        self.settings_open = open;
        if changed {
            self.save_settings();
        }
    }

    fn help_window(&mut self, ctx: &egui::Context) {
        let mut open = self.help_open;
        egui::Window::new("Help").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
            ui.label(RichText::new("How it works").strong());
            ui.label("1. Plan a bake: pick a recipe, how many, and when. Every gram is worked out for you.");
            ui.label("2. Add it to the plan. Add more bakes if you like - they share one timeline.");
            ui.label("3. Timeline: follow the steps. Alarms ring while the app is open.");
            ui.separator();
            ui.label(RichText::new("Keyboard").strong());
            for (k, what) in [
                ("Ctrl+K", "command palette (Cmd+K on a Mac)"),
                ("Ctrl+1 .. 4", "switch screen"),
                ("Ctrl+Enter", "add the bake to the plan"),
                ("Tab / Shift+Tab", "move between controls"),
                ("Enter / Esc", "alarm: Done / Snooze"),
            ] {
                ui.label(format!("{k:<16} {what}"));
            }
            ui.separator();
            ui.label("Times are estimates. Fermentation follows temperature, flour and starter strength, not the clock. Every step says what the dough should look like - go with the dough.");
        });
        self.help_open = open;
    }

    fn sheet_window_ui(&mut self, ctx: &egui::Context) {
        let Some(i) = self.sheet_window else { return };
        let text = self.plan.batches.get(i).and_then(|b| self.sheet_for(b)).map(|s| sheet_as_text(&s));
        let mut open = true;
        egui::Window::new("Bakesheet").open(&mut open).default_size([640.0, 480.0]).show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| match &text {
                Some(t) => {
                    ui.label(RichText::new(t).monospace());
                }
                None => {
                    ui.label("That bake is no longer in the plan.");
                }
            });
        });
        if !open {
            self.sheet_window = None;
        }
    }

    /// Keyboard shortcuts that work on every screen.
    fn shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};
        let (k, one, two, three, four, enter) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::COMMAND, Key::K),
                i.consume_key(Modifiers::COMMAND, Key::Num1),
                i.consume_key(Modifiers::COMMAND, Key::Num2),
                i.consume_key(Modifiers::COMMAND, Key::Num3),
                i.consume_key(Modifiers::COMMAND, Key::Num4),
                i.consume_key(Modifiers::COMMAND, Key::Enter),
            )
        });
        if k {
            self.palette_open = !self.palette_open;
            self.palette_query.clear();
        }
        if one {
            self.tab = Tab::Plan;
        }
        if two {
            self.tab = Tab::Timeline;
        }
        if three {
            self.tab = Tab::Water;
        }
        if four {
            self.tab = Tab::Sources;
        }
        if enter && self.tab == Tab::Plan {
            self.add_to_plan();
        }
    }
}

impl eframe::App for App {
    /// Called by egui for every frame. We redraw everything from our state.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_look(ctx);
        self.shortcuts(ctx);
        self.check_alarms(ctx);

        self.header(ctx);
        self.status_bar(ctx);
        match self.tab {
            Tab::Plan => self.plan_tab(ctx),
            Tab::Timeline => self.timeline_tab(ctx),
            Tab::Water => self.water_tab(ctx),
            Tab::Sources => self.sources_tab(ctx),
        }

        self.palette_window(ctx);
        self.settings_window(ctx);
        self.help_window(ctx);
        self.sheet_window_ui(ctx);
        self.alarm_window(ctx);

        // Wake up once a second so alarms and the "NOW" marker stay current.
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
}
