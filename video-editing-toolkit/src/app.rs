//! The window: three tabs (Compress, Audio, Frames) sharing one file list.

use crate::engine::{self, AudioFormat, AudioOpts, Codec, CompressOpts, FramesOpts, ImageFormat, Job, Tools};
use crate::runner::{self, Event};
use egui::{Color32, RichText};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Compress,
    Audio,
    Frames,
}

struct LogLine {
    ok: bool,
    text: String,
}

pub struct VideoToolkitApp {
    tab: Tab,
    tools: Result<Tools, String>,
    inputs: Vec<PathBuf>,
    path_box: String,
    out_dir: String,
    compress: CompressOpts,
    audio: AudioOpts,
    frames: FramesOpts,
    // running state
    rx: Option<Receiver<Event>>,
    cancel: Arc<AtomicBool>,
    progress: f32,
    current: String,
    position: (usize, usize),
    log: Vec<LogLine>,
    status: String,
    update_msg: Option<Result<Option<crate::updater::Release>, String>>,
}

impl VideoToolkitApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            tab: Tab::Compress,
            tools: engine::find_tools(),
            inputs: Vec::new(),
            path_box: String::new(),
            out_dir: String::new(),
            compress: CompressOpts::default(),
            audio: AudioOpts::default(),
            frames: FramesOpts::default(),
            rx: None,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: 0.0,
            current: String::new(),
            position: (0, 0),
            log: Vec::new(),
            status: "Drop videos or folders onto the window to begin.".into(),
            update_msg: None,
        }
    }

    fn running(&self) -> bool {
        self.rx.is_some()
    }

    fn add_paths(&mut self, paths: &[PathBuf]) {
        let found = engine::collect_inputs(paths);
        let before = self.inputs.len();
        for p in found {
            if !self.inputs.contains(&p) {
                self.inputs.push(p);
            }
        }
        let added = self.inputs.len() - before;
        self.status = if added == 0 { "No new video files in that.".into() } else { format!("Added {added} video{}.", if added == 1 { "" } else { "s" }) };
    }

    fn start(&mut self) {
        let Ok(tools) = self.tools.clone() else { return };
        if self.inputs.is_empty() {
            self.status = "Add at least one video first.".into();
            return;
        }
        let job = match self.tab {
            Tab::Compress => Job::Compress(self.compress.clone()),
            Tab::Audio => Job::Audio(self.audio.clone()),
            Tab::Frames => {
                if !(self.frames.interval > 0.0) {
                    self.status = "The interval must be above zero.".into();
                    return;
                }
                Job::Frames(self.frames.clone())
            }
        };
        let out_dir = if self.out_dir.trim().is_empty() { None } else { Some(PathBuf::from(self.out_dir.trim())) };
        let (tx, rx) = mpsc::channel();
        self.cancel = Arc::new(AtomicBool::new(false));
        self.rx = Some(rx);
        self.progress = 0.0;
        self.log.clear();
        self.status = format!("{}…", job.name());
        let (inputs, cancel) = (self.inputs.clone(), self.cancel.clone());
        std::thread::spawn(move || runner::run_batch(tools, job, inputs, out_dir, cancel, tx));
    }

    fn poll(&mut self) {
        let Some(rx) = &self.rx else { return };
        let mut finished = false;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                Event::FileStart { index, total, name } => {
                    self.position = (index, total);
                    self.current = name;
                    self.progress = 0.0;
                }
                Event::Progress(p) => self.progress = p,
                Event::FileDone { name, ok, message } => self.log.push(LogLine { ok, text: format!("{name}: {message}") }),
                Event::Finished { done, failed, cancelled } => {
                    self.status = format!("{done} done, {failed} failed{}.", if cancelled { ", cancelled" } else { "" });
                    finished = true;
                }
            }
        }
        if finished {
            self.rx = None;
            self.current.clear();
        }
    }
}

const RED: Color32 = Color32::from_rgb(200, 90, 90);

impl eframe::App for VideoToolkitApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        if self.running() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() && !self.running() {
            self.add_paths(&dropped);
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("Video Editing Toolkit");
                ui.label(RichText::new(crate::updater::version_label()).weak().small());
                ui.separator();
                for (t, label) in [(Tab::Compress, "Compress"), (Tab::Audio, "Extract audio"), (Tab::Frames, "Extract frames")] {
                    if ui.selectable_label(self.tab == t, label).clicked() && !self.running() {
                        self.tab = t;
                    }
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
                        ui.label(RichText::new(e).color(RED));
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
                        if ui.button("Install Video Editing Toolkit").clicked() {
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

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Err(e) = &self.tools {
                ui.add_space(8.0);
                ui.colored_label(RED, e);
                if ui.button("Look again").clicked() {
                    self.tools = engine::find_tools();
                }
                return;
            }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                self.ui_files(ui);
                ui.add_space(10.0);
                ui.separator();
                ui.add_enabled_ui(!self.running(), |ui| {
                    self.ui_settings(ui);
                    ui.add_space(6.0);
                    self.ui_output(ui);
                });
                ui.add_space(10.0);
                self.ui_run(ui);
            });
        });
    }
}

impl VideoToolkitApp {
    fn ui_files(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.strong("Videos");
            ui.label(RichText::new(format!("{} selected", self.inputs.len())).weak());
        });
        ui.add_enabled_ui(!self.running(), |ui| {
            ui.horizontal(|ui| {
                #[cfg(windows)]
                {
                    if ui.button("Add files…").clicked() {
                        if let Some(files) = rfd::FileDialog::new().add_filter("Video", engine::VIDEO_EXTS).pick_files() {
                            self.add_paths(&files);
                        }
                    }
                    if ui.button("Add folder…").clicked() {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            self.add_paths(&[dir]);
                        }
                    }
                }
                let resp = ui.add(egui::TextEdit::singleline(&mut self.path_box).hint_text("or paste a file or folder path").desired_width(320.0));
                let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (ui.button("Add").clicked() || enter) && !self.path_box.trim().is_empty() {
                    let p = PathBuf::from(self.path_box.trim().trim_matches('"'));
                    self.add_paths(&[p]);
                    self.path_box.clear();
                }
                if !self.inputs.is_empty() && ui.button("Clear").clicked() {
                    self.inputs.clear();
                }
            });
        });
        ui.add_space(4.0);
        if self.inputs.is_empty() {
            ui.label(RichText::new("Nothing yet. Drag videos or whole folders onto this window.").weak());
            return;
        }
        let running = self.running();
        let mut remove: Option<usize> = None;
        egui::ScrollArea::vertical().id_salt("files").max_height(170.0).show(ui, |ui| {
            for (i, p) in self.inputs.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui.add_enabled(!running, egui::Button::new("✕").small()).clicked() {
                        remove = Some(i);
                    }
                    let size = std::fs::metadata(p).map(|m| engine::human_size(m.len())).unwrap_or_default();
                    ui.label(p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
                    ui.label(RichText::new(size).weak().small());
                    ui.label(RichText::new(p.parent().map(|d| d.display().to_string()).unwrap_or_default()).weak().small());
                });
            }
        });
        if let Some(i) = remove {
            self.inputs.remove(i);
        }
    }

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        match self.tab {
            Tab::Compress => {
                let o = &mut self.compress;
                ui.strong("Compress");
                ui.label(RichText::new("Re-encodes to MP4. Quality is a CRF number: lower keeps more detail and makes a bigger file.").weak().small());
                egui::Grid::new("compress").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("Codec");
                    egui::ComboBox::from_id_salt("codec").selected_text(o.codec.label()).show_ui(ui, |ui| {
                        ui.selectable_value(&mut o.codec, Codec::H265, Codec::H265.label());
                        ui.selectable_value(&mut o.codec, Codec::H264, Codec::H264.label());
                    });
                    ui.end_row();
                    ui.label("Quality (CRF)");
                    ui.add(egui::Slider::new(&mut o.crf, 18..=35));
                    ui.end_row();
                    ui.label("Speed");
                    egui::ComboBox::from_id_salt("preset").selected_text(o.preset.clone()).show_ui(ui, |ui| {
                        for p in engine::PRESETS {
                            ui.selectable_value(&mut o.preset, p.to_string(), *p);
                        }
                    });
                    ui.end_row();
                    ui.label("Largest width");
                    let label = o.max_width.map(|w| format!("{w} px")).unwrap_or_else(|| "Keep original".into());
                    egui::ComboBox::from_id_salt("width").selected_text(label).show_ui(ui, |ui| {
                        ui.selectable_value(&mut o.max_width, None, "Keep original");
                        for w in [1920u32, 1280, 854] {
                            ui.selectable_value(&mut o.max_width, Some(w), format!("{w} px"));
                        }
                    });
                    ui.end_row();
                    ui.label("Audio");
                    egui::ComboBox::from_id_salt("abr").selected_text(format!("{} kbit/s", o.audio_kbps)).show_ui(ui, |ui| {
                        for k in [64u32, 96, 128, 192, 256] {
                            ui.selectable_value(&mut o.audio_kbps, k, format!("{k} kbit/s"));
                        }
                    });
                    ui.end_row();
                    ui.label("File name ends");
                    ui.add_enabled(!o.in_place, egui::TextEdit::singleline(&mut o.suffix).desired_width(140.0));
                    ui.end_row();
                });
                ui.checkbox(&mut o.in_place, "Replace the original, but only when the new file is smaller");
                ui.checkbox(&mut o.overwrite, "Redo files that already have a compressed copy");
            }
            Tab::Audio => {
                let o = &mut self.audio;
                ui.strong("Extract audio");
                ui.label(RichText::new("Saves each video's sound as its own file. Handy for transcripts and podcasts.").weak().small());
                egui::Grid::new("audio").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("Format");
                    egui::ComboBox::from_id_salt("afmt").selected_text(o.format.ext()).show_ui(ui, |ui| {
                        for f in AudioFormat::ALL {
                            ui.selectable_value(&mut o.format, f, f.ext());
                        }
                    });
                    ui.end_row();
                    ui.label("Bitrate");
                    ui.add_enabled_ui(o.format != AudioFormat::Wav, |ui| {
                        egui::ComboBox::from_id_salt("akbps").selected_text(format!("{} kbit/s", o.kbps)).show_ui(ui, |ui| {
                            for k in [32u32, 64, 96, 128, 192, 256] {
                                ui.selectable_value(&mut o.kbps, k, format!("{k} kbit/s"));
                            }
                        });
                    });
                    ui.end_row();
                });
                ui.checkbox(&mut o.mono, "Mono (half the size, fine for speech)");
                ui.checkbox(&mut o.overwrite, "Replace files that already exist");
            }
            Tab::Frames => {
                let o = &mut self.frames;
                ui.strong("Extract frames");
                ui.label(RichText::new("Saves a still every few seconds into a folder beside the video, named by time (clip_t00m10s.jpg).").weak().small());
                egui::Grid::new("frames").num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("One frame every");
                    ui.add(egui::DragValue::new(&mut o.interval).speed(0.25).range(0.1..=3600.0).suffix(" s"));
                    ui.end_row();
                    ui.label("Start at");
                    ui.add(egui::DragValue::new(&mut o.start).speed(0.5).range(0.0..=86400.0).suffix(" s"));
                    ui.end_row();
                    ui.label("Largest width");
                    egui::ComboBox::from_id_salt("fw").selected_text(format!("{} px", o.max_width)).show_ui(ui, |ui| {
                        for w in [3840u32, 1920, 1280, 854, 640] {
                            ui.selectable_value(&mut o.max_width, w, format!("{w} px"));
                        }
                    });
                    ui.end_row();
                    ui.label("Image type");
                    egui::ComboBox::from_id_salt("ff").selected_text(o.format.ext()).show_ui(ui, |ui| {
                        ui.selectable_value(&mut o.format, ImageFormat::Jpg, "jpg");
                        ui.selectable_value(&mut o.format, ImageFormat::Png, "png");
                    });
                    ui.end_row();
                    ui.label("JPEG quality");
                    ui.add_enabled(o.format == ImageFormat::Jpg, egui::Slider::new(&mut o.quality, 2..=20).text("2 best"));
                    ui.end_row();
                });
                ui.checkbox(&mut o.dedupe, "Skip frames that look the same as the last one (slides, still shots)");
                ui.checkbox(&mut o.overwrite, "Replace frames that already exist");
            }
        }
    }

    fn ui_output(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Save to");
            ui.add(egui::TextEdit::singleline(&mut self.out_dir).hint_text("next to each video").desired_width(360.0));
            #[cfg(windows)]
            if ui.button("Choose…").clicked() {
                if let Some(d) = rfd::FileDialog::new().pick_folder() {
                    self.out_dir = d.display().to_string();
                }
            }
        });
    }

    fn ui_run(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.running() {
                if ui.button("Cancel").clicked() {
                    self.cancel.store(true, Ordering::Relaxed);
                    self.status = "Cancelling…".into();
                }
                ui.label(format!("{} of {}: {}", self.position.0, self.position.1, self.current));
            } else {
                let label = match self.tab {
                    Tab::Compress => "Compress",
                    Tab::Audio => "Extract audio",
                    Tab::Frames => "Extract frames",
                };
                if ui.add_enabled(!self.inputs.is_empty(), egui::Button::new(RichText::new(label).strong())).clicked() {
                    self.start();
                }
            }
        });
        if self.running() {
            ui.add(egui::ProgressBar::new(self.progress).show_percentage());
        }
        if !self.log.is_empty() {
            ui.add_space(6.0);
            for l in &self.log {
                let color = if l.ok { ui.visuals().text_color() } else { RED };
                ui.label(RichText::new(format!("{} {}", if l.ok { "✓" } else { "✗" }, l.text)).color(color));
            }
        }
    }
}
