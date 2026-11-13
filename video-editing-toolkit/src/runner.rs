//! Runs a job over a list of files on a worker thread and reports back through a channel.
//! The window and the command line both listen to the same events.

use crate::engine::{self, Cancel, Job, Tools};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::atomic::Ordering;

pub enum Event {
    FileStart { index: usize, total: usize, name: String },
    Progress(f32),
    FileDone { name: String, ok: bool, message: String },
    Finished { done: usize, failed: usize, cancelled: bool },
}

pub fn run_batch(tools: Tools, job: Job, inputs: Vec<PathBuf>, out_dir: Option<PathBuf>, cancel: Cancel, tx: Sender<Event>) {
    let total = inputs.len();
    let (mut done, mut failed, mut cancelled) = (0usize, 0usize, false);
    for (i, input) in inputs.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let name = input.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let _ = tx.send(Event::FileStart { index: i + 1, total, name: name.clone() });
        let mut last = -1.0f32;
        let mut on_progress = |p: f32| {
            if p - last >= 0.005 || p >= 1.0 {
                last = p;
                let _ = tx.send(Event::Progress(p));
            }
        };
        match engine::process_file(&tools, &job, input, &out_dir, &cancel, &mut on_progress) {
            Ok(o) => {
                done += 1;
                let _ = tx.send(Event::FileDone { name, ok: true, message: o.message });
            }
            Err(e) if e == "Cancelled" => {
                cancelled = true;
                let _ = tx.send(Event::FileDone { name, ok: false, message: "cancelled".into() });
                break;
            }
            Err(e) => {
                failed += 1;
                let _ = tx.send(Event::FileDone { name, ok: false, message: e });
            }
        }
    }
    let _ = tx.send(Event::Finished { done, failed, cancelled });
}
