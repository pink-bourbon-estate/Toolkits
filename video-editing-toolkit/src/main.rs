#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cli;
mod engine;
mod install;
mod runner;
mod updater;

#[cfg(windows)]
fn attach_parent_console() {
    // The release build is a GUI program; hook it up to the terminal it was started from so the
    // command line prints something. Does nothing when started from the Start menu.
    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    unsafe {
        AttachConsole(0xFFFF_FFFF);
    }
}
#[cfg(not(windows))]
fn attach_parent_console() {}

fn main() -> eframe::Result<()> {
    startlog();
    updater::cleanup();
    std::thread::spawn(install::refresh_registration);

    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("--install") => {
            attach_parent_console();
            match install::install(args.iter().any(|a| a == "--desktop")) {
                Ok(p) => println!("Installed to {}", p.display()),
                Err(e) => eprintln!("{e}"),
            }
            return Ok(());
        }
        Some("--uninstall") => {
            attach_parent_console();
            let msg = install::uninstall().unwrap_or_else(|e| e);
            println!("{msg}");
            return Ok(());
        }
        Some("--update") => {
            attach_parent_console();
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
        Some("--version") | Some("-V") => {
            attach_parent_console();
            println!("Video Editing Toolkit {}", updater::version_label());
            return Ok(());
        }
        Some("compress") | Some("audio") | Some("frames") | Some("-h") | Some("--help") | Some("help") => {
            attach_parent_console();
            use clap::Parser;
            let code = cli::run(cli::Cli::parse());
            std::process::exit(code);
        }
        _ => {}
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Video Editing Toolkit")
            .with_inner_size([960.0, 720.0])
            .with_min_inner_size([720.0, 560.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native("Video Editing Toolkit", native_options, Box::new(|cc| Ok(Box::new(app::VideoToolkitApp::new(cc)))))
}

fn startlog() {
    // Build identity only; no telemetry, no network call, nothing persisted off-device.
    let build = option_env!("VIDEO_EDITING_TOOLKIT_BUILD").unwrap_or("dev");
    eprintln!("Video Editing Toolkit build {build} starting.");
}
