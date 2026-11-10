//! Background conversion of PDFs into playable pieces: one page, a whole book, or every PDF
//! in a folder.
//!
//! * Typeset PDFs (MuseScore, Sibelius, Dorico, Finale…) are read directly by the built-in
//!   reader (`vecscore`): every page, in seconds, no extra software.
//! * Scanned books (pictures of pages) need optical music recognition: Audiveris, run on the
//!   whole book in one go, if it's installed.
//! * PDFs with no music in them (letters, receipts, theory text) are skipped, untouched.
//!
//! Files are only ever read. Nothing is written next to your PDFs.

use crate::curriculum::data_dir;
use crate::import::{import_musicxml, title_case, unzip_for_omr};
use crate::model::{Origin, Piece};
use crate::vecscore;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Job {
    pub pdf: PathBuf,
    /// 1-based inclusive page range; None = the whole PDF.
    pub pages: Option<(usize, usize)>,
}

pub enum ConvMsg {
    /// Now working on job `index` (0-based).
    Working { index: usize, what: String },
    Piece(Box<Piece>),
    /// A PDF that couldn't be converted, and why (in plain words).
    Skipped { pdf: PathBuf, why: String },
    Finished,
}

pub struct Converter {
    pub rx: Receiver<ConvMsg>,
    cancel: Arc<AtomicBool>,
    pub total: usize,
    pub index: usize,
    pub what: String,
    pub made: usize,
    pub skipped: Vec<(PathBuf, String)>,
    pub finished: bool,
}

impl Converter {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

pub fn start(jobs: Vec<Job>, audiveris: Option<PathBuf>) -> Converter {
    let (tx, rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let c2 = cancel.clone();
    let total = jobs.len();
    std::thread::spawn(move || {
        for (i, job) in jobs.into_iter().enumerate() {
            if c2.load(Ordering::Relaxed) {
                break;
            }
            let name = job.pdf.file_name().and_then(|n| n.to_str()).unwrap_or("PDF").to_string();
            let _ = tx.send(ConvMsg::Working { index: i, what: format!("Reading {name}…") });
            // a damaged or unusual PDF must never take the app down with it
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| convert_one(&job, audiveris.as_deref(), &tx, &c2, i)));
            match res {
                Ok(Ok(n)) if n > 0 => {}
                Ok(Ok(_)) => {
                    let _ = tx.send(ConvMsg::Skipped { pdf: job.pdf.clone(), why: "no notes found".into() });
                }
                Ok(Err(why)) => {
                    let _ = tx.send(ConvMsg::Skipped { pdf: job.pdf.clone(), why });
                }
                Err(_) => {
                    let _ = tx.send(ConvMsg::Skipped { pdf: job.pdf.clone(), why: "PianoCraft couldn't read this PDF".into() });
                }
            }
        }
        let _ = tx.send(ConvMsg::Finished);
    });
    Converter { rx, cancel, total, index: 0, what: String::new(), made: 0, skipped: vec![], finished: false }
}

/// Id for a converted piece: stable for the same file, so converting again replaces it.
pub fn piece_id(pdf: &Path, pages: Option<(usize, usize)>, part: usize) -> String {
    let stem: String = pdf.file_stem().and_then(|s| s.to_str()).unwrap_or("pdf").to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
    let stem: String = stem.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-").chars().take(48).collect();
    let pg = match pages {
        Some((a, b)) if a == b => format!("-p{a}"),
        Some((a, b)) => format!("-p{a}-{b}"),
        None => String::new(),
    };
    let part = if part > 0 { format!("-{}", part + 1) } else { String::new() };
    format!("pdf-{stem}{pg}{part}")
}

/// A readable title from a file name like "violet-evergarden-2021-live.pdf".
pub fn nice_title(pdf: &Path) -> String {
    let stem = pdf.file_stem().and_then(|s| s.to_str()).unwrap_or("PDF");
    // drop download-site junk in brackets: "Title (z-library…)"
    let stem = stem.split(" (").next().unwrap_or(stem);
    title_case(&stem.replace(['-', '_'], " "))
}

fn convert_one(job: &Job, audiveris: Option<&Path>, tx: &Sender<ConvMsg>, cancel: &AtomicBool, index: usize) -> Result<usize, String> {
    let bytes = std::fs::read(&job.pdf).map_err(|e| format!("couldn't open the file ({e})"))?;
    let title = nice_title(&job.pdf);
    let title = match job.pages {
        Some((a, b)) if a == b => format!("{title} p.{a}"),
        Some((a, b)) => format!("{title} pp.{a}–{b}"),
        None => title,
    };
    match vecscore::read_pdf_pages(bytes, &title, job.pages) {
        Ok((mut p, r)) => {
            p.id = piece_id(&job.pdf, job.pages, 0);
            p.origin = Origin::Imported(job.pdf.display().to_string());
            let pct = 100.0 * r.clean_measures as f64 / r.measures.max(1) as f64;
            let check = if pct >= 95.0 {
                "Every bar adds up.".to_string()
            } else {
                format!("{pct:.0}% of bars add up exactly; listen through and compare the rest with the page.")
            };
            p.notes_text = format!("{} {check}", p.notes_text);
            let _ = tx.send(ConvMsg::Piece(Box::new(p)));
            Ok(1)
        }
        Err(e) if e == vecscore::NOT_TYPESET_SCAN => {
            let Some(exe) = audiveris else {
                return Err("scanned book – needs Audiveris (Settings → Make pages playable)".into());
            };
            let what = match job.pages {
                Some((a, b)) if a == b => format!("page {a}"),
                _ => "the whole book".to_string(),
            };
            let name = job.pdf.file_name().and_then(|n| n.to_str()).unwrap_or("PDF");
            let _ = tx.send(ConvMsg::Working { index, what: format!("Audiveris is reading {what} of {name} (about a minute per page)…") });
            let files = run_audiveris(exe, &job.pdf, job.pages, cancel)?;
            let mut n = 0;
            let mut errors = vec![];
            for (k, f) in files.iter().enumerate() {
                let xml = if f.extension().is_some_and(|x| x == "mxl") {
                    std::fs::read(f).map_err(|e| e.to_string()).and_then(|b| unzip_for_omr(&b))
                } else {
                    std::fs::read_to_string(f).map_err(|e| e.to_string())
                };
                match xml.and_then(|x| import_musicxml(&x, &title)) {
                    Ok(mut p) if !p.notes.is_empty() => {
                        p.id = piece_id(&job.pdf, job.pages, k);
                        p.title = if files.len() > 1 { format!("{title} – part {}", k + 1) } else { title.clone() };
                        p.notes_text = "Recognised from your scanned book by Audiveris. Old scans can have mistakes – compare with the page.".into();
                        p.origin = Origin::Imported(job.pdf.display().to_string());
                        let _ = tx.send(ConvMsg::Piece(Box::new(p)));
                        n += 1;
                    }
                    Ok(_) => {}
                    Err(e) => errors.push(e),
                }
            }
            if n == 0 {
                return Err(match errors.first() {
                    Some(e) => format!("Audiveris's result couldn't be read: {e}"),
                    None => "Audiveris didn't recognise any notes (very faint or skewed scans can do this)".into(),
                });
            }
            Ok(n)
        }
        Err(e) if e == vecscore::NOT_MUSIC => Err("no sheet music in it".into()),
        Err(e) => Err(e),
    }
}

/// Run Audiveris in batch mode and return the MusicXML files it wrote.
pub fn run_audiveris(exe: &Path, pdf: &Path, pages: Option<(usize, usize)>, cancel: &AtomicBool) -> Result<Vec<PathBuf>, String> {
    let stem: String = pdf.file_stem().and_then(|s| s.to_str()).unwrap_or("book").chars().filter(|c| c.is_alphanumeric()).take(24).collect();
    let tag = match pages {
        Some((a, b)) => format!("{stem}-p{a}-{b}"),
        None => format!("{stem}-all"),
    };
    let out = data_dir().join("converted").join(tag);
    let _ = std::fs::remove_dir_all(&out);
    let _ = std::fs::create_dir_all(&out);
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("-batch").arg("-export").arg("-output").arg(&out);
    if let Some((a, b)) = pages {
        cmd.arg("-sheets").arg(if a == b { a.to_string() } else { format!("{a}-{b}") });
    }
    cmd.arg("--").arg(pdf);
    cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // no console window
    }
    let mut child = cmd.spawn().map_err(|e| format!("couldn't start Audiveris ({e})"))?;
    // keep the last lines of its log for the error message
    let tail = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    if let Some(err) = child.stderr.take() {
        let tail = tail.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in std::io::BufReader::new(err).lines().map_while(Result::ok) {
                let mut t = tail.lock().unwrap();
                t.push(line);
                if t.len() > 20 {
                    t.remove(0);
                }
            }
        });
    }
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            return Err("cancelled".into());
        }
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(250)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let mut files = vec![];
    fn walk(d: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(rd) = std::fs::read_dir(d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "mxl" || x == "musicxml" || x == "xml") {
                    out.push(p);
                }
            }
        }
    }
    walk(&out, &mut files);
    files.sort();
    if files.is_empty() {
        let t = tail.lock().unwrap();
        let last = t.iter().rev().find(|l| l.contains("WARN") || l.contains("ERROR") || l.contains("Exception")).or(t.last()).cloned().unwrap_or_default();
        return Err(format!("Audiveris didn't recognise any music. {last}").trim().to_string());
    }
    Ok(files)
}
