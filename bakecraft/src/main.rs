//! BakeCraft - a bake planner.
//!
//! Pick a recipe, how many and how big, and get every gram to weigh and a
//! clock-time timeline with alarms. Recipes are averages of published school
//! and bakery formulas.
//!
//! Where to read next (in this order if you are learning):
//!   sources.rs     the research data
//!   formulas.rs    recipes and their steps
//!   calculator.rs  baker's percentages -> grams
//!   schedule.rs    clock times, temperature adjustment, clashes
//!   water.rs       dough-temperature water calculator
//!   settings.rs    saved choices and plans
//!   app.rs         the window

// On Windows, don't open a black console window behind the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod calculator;
mod formulas;
mod schedule;
mod settings;
mod sources;
mod water;

fn main() -> eframe::Result {
    // `bakecraft --version` prints the version and exits (handy for checking a download).
    // (With windows_subsystem = "windows" there is no console, so this is mainly for Linux.)
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("BakeCraft {}", app::version());
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("BakeCraft")
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([860.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native("BakeCraft", options, Box::new(|_cc| Ok(Box::new(app::App::new()))))
}
