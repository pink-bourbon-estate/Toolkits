#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod install;
mod model;
mod updater;

fn main() -> eframe::Result<()> {
    startlog();
    updater::cleanup();
    std::thread::spawn(install::refresh_registration);

    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("--install") => {
            match install::install(args.iter().any(|a| a == "--desktop")) {
                Ok(p) => println!("Installed to {}", p.display()),
                Err(e) => eprintln!("{e}"),
            }
            return Ok(());
        }
        Some("--uninstall") => {
            let msg = install::uninstall().unwrap_or_else(|e| e);
            println!("{msg}");
            return Ok(());
        }
        Some("--update") => {
            match updater::check() {
                Ok(Some(rel)) => match updater::install(&rel) {
                    Ok(exe) => {
                        println!("Updated to build {}.", rel.build);
                        let _ = updater::relaunch(&exe);
                    }
                    Err(e) => eprintln!("{e}"),
                },
                Ok(None) => println!("Already up to date."),
                Err(e) => eprintln!("{e}"),
            }
            return Ok(());
        }
        Some("--version") => {
            println!("ChessCraft {}", updater::version_label());
            return Ok(());
        }
        _ => {}
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ChessCraft")
            .with_inner_size([1150.0, 760.0])
            .with_min_inner_size([760.0, 560.0]),
        ..Default::default()
    };

    eframe::run_native(
        "ChessCraft",
        native_options,
        Box::new(|cc| Ok(Box::new(app::ChessCraftApp::new(cc)))),
    )
}

fn startlog() {
    // Build identity only; no telemetry, no network call, nothing persisted off-device.
    let build = option_env!("CHESSCRAFT_BUILD").unwrap_or("dev");
    eprintln!("ChessCraft build {build} starting.");
}
