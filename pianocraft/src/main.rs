#![cfg_attr(windows, windows_subsystem = "windows")]

mod audio;
mod books;
mod builtin;
mod curriculum;
mod generate;
mod import;
mod install;
mod midi_in;
mod model;
mod notation;
mod repertoire;
mod startlog;
mod transform;
mod ui;
mod updater;
#[allow(dead_code)]
mod vecscore;
#[cfg(test)]
mod tests;

fn main() -> eframe::Result<()> {
    // `pianocraft --inspect file.mid` prints what the importer sees (handy for checking files)
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--inspect" {
        match import::import_file(std::path::Path::new(&args[2])) {
            Ok(p) => {
                let d = transform::rate(&p);
                println!("{} | {} | {}/{} | bpm {:.0} | bars {} | notes RH {} LH {} | difficulty {:.1} | easiest transpose {:+}",
                    p.title, p.key_name(), p.ts_num, p.ts_den, p.bpm, p.bar_count(),
                    p.notes.iter().filter(|n| n.hand == model::Hand::Right).count(),
                    p.notes.iter().filter(|n| n.hand == model::Hand::Left).count(), d.score, transform::easiest_transposition(&p));
                for n in p.notes.iter().take(12) {
                    println!("  {:>6.2} {:>5.2} {:<4} {:?}", n.start, n.dur, model::pitch_name(n.pitch, false), n.hand);
                }
            }
            Err(e) => println!("ERROR: {e}"),
        }
        return Ok(());
    }
    match args.get(1).map(|s| s.as_str()) {
        Some("--readpdf") if args.len() >= 3 => {
            let bytes = std::fs::read(&args[2]).unwrap_or_default();
            if args.iter().any(|a| a == "--bars") { vecscore::debug_bars(bytes.clone()); return Ok(()); }
            if args.iter().any(|a| a == "--dump") {
                vecscore::debug_dump(bytes.clone());
            }
            match vecscore::read_pdf(bytes, "test") {
                Ok((p, r)) => {
                    println!("pages {} systems {} measures {} clean {} ({:.0}%) notes {} | {} {}/{} bars {} RH {} LH {}",
                        r.pages, r.systems, r.measures, r.clean_measures, 100.0 * r.clean_measures as f64 / r.measures.max(1) as f64, r.notes,
                        p.key_name(), p.ts_num, p.ts_den, p.bar_count(),
                        p.notes.iter().filter(|n| n.hand == model::Hand::Right).count(), p.notes.iter().filter(|n| n.hand == model::Hand::Left).count());
                    if let Some(out) = args.get(3).filter(|a| !a.starts_with("--")) {
                        let _ = std::fs::write(out, serde_json::to_string(&p).unwrap_or_default());
                    }
                }
                Err(e) => println!("ERROR: {e}"),
            }
            return Ok(());
        }
        Some("--version") => {
            println!("PianoCraft {}", updater::version_label());
            return Ok(());
        }
        Some("--install") => {
            match install::install(args.iter().any(|a| a == "--desktop")) {
                Ok(p) => println!("Installed to {}", p.display()),
                Err(e) => println!("Install failed: {e}"),
            }
            return Ok(());
        }
        Some("--uninstall") => {
            let msg = install::uninstall().unwrap_or_else(|e| e);
            println!("{msg}");
            #[cfg(windows)]
            rfd::MessageDialog::new().set_title("PianoCraft").set_description(&msg).show();
            return Ok(());
        }
        Some("--update") => {
            match updater::check() {
                Ok(Some(r)) => match updater::install(&r) {
                    Ok(_) => println!("Updated to {}", r.title),
                    Err(e) => println!("Update failed: {e}"),
                },
                Ok(None) => println!("PianoCraft {} is up to date.", updater::version_label()),
                Err(e) => println!("{e}"),
            }
            return Ok(());
        }
        _ => {}
    }
    startlog::log(&format!("launch {}", updater::version_label()));
    updater::cleanup();
    // registry/shortcut upkeep can be slow (PowerShell): never block the window on it
    std::thread::spawn(install::refresh_registration);
    let icon = egui::IconData { rgba: include_bytes!("../assets/icon64.rgba").to_vec(), width: 64, height: 64 };
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(std::sync::Arc::new(icon))
            .with_title("PianoCraft")
            .with_inner_size([1380.0, 900.0])
            .with_min_inner_size([900.0, 600.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    let r = eframe::run_native("PianoCraft", opts, Box::new(|cc| Ok(Box::new(ui::app::App::new(cc)))));
    if let Err(e) = &r {
        startlog::log(&format!("window error: {e}"));
    }
    // make sure nothing lingers in the background after the window closes
    std::process::exit(if r.is_ok() { 0 } else { 1 });
}
