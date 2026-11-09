//! Sound + transport. The sequencer runs inside the audio callback so timing is
//! sample-accurate; the UI only sends commands and reads the playhead.

use crate::model::{Hand, Note};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

#[derive(Debug)]
pub enum Cmd {
    Load { notes: Vec<Note>, bar_len: f64, pickup: f64, click_every: f64, end: f64 },
    Play,
    Pause,
    Seek(f64),
    Tempo(f64),
    /// Loop range in beats and how many times to play it (0 = forever).
    Loop(Option<(f64, f64)>, u32),
    Metronome(bool),
    CountIn(bool),
    /// Which hands the app plays out loud.
    Sound { rh: bool, lh: bool },
    Volume(f32),
    Wait { enabled: bool, rh: bool, lh: bool },
    KeyDown(u8, u8),
    KeyUp(u8),
    Echo(bool),
    Preview(u8),
}

#[derive(Default)]
pub struct Shared {
    pos: AtomicU64,
    pub playing: AtomicBool,
    pub loops_done: AtomicU32,
    pub finished: AtomicBool,
    pub waiting: AtomicBool,
    pub counting_in: AtomicBool,
    pub has_device: AtomicBool,
}

impl Shared {
    pub fn pos(&self) -> f64 {
        f64::from_bits(self.pos.load(Ordering::Relaxed))
    }
    fn set_pos(&self, p: f64) {
        self.pos.store(p.to_bits(), Ordering::Relaxed);
    }
}

#[derive(Clone, Copy)]
struct Ev {
    t: f64,
    pitch: u8,
    on: bool,
    hand: Hand,
}

struct Voice {
    pitch: u8,
    freq: f32,
    phase: [f32; 6],
    age: f32,
    amp: f32,
    release: Option<f32>,
    source: u8, // 0 = sequence, 1 = live/preview
    click: bool,
}

struct Core {
    sr: f32,
    rx: Receiver<Cmd>,
    shared: Arc<Shared>,
    events: Vec<Ev>,
    idx: usize,
    pos: f64,
    bpm: f64,
    bar_len: f64,
    pickup: f64,
    click_every: f64,
    end: f64,
    playing: bool,
    loop_range: Option<(f64, f64)>,
    loop_reps: u32,
    loops_done: u32,
    metronome: bool,
    count_in: bool,
    preroll_until: Option<f64>,
    last_click: i64,
    sound_rh: bool,
    sound_lh: bool,
    volume: f32,
    wait: bool,
    wait_rh: bool,
    wait_lh: bool,
    pressed: HashSet<u8>,
    hit_since: HashSet<u8>,
    echo: bool,
    voices: Vec<Voice>,
}

impl Core {
    fn new(sr: f32, rx: Receiver<Cmd>, shared: Arc<Shared>) -> Self {
        Self {
            sr, rx, shared, events: vec![], idx: 0, pos: 0.0, bpm: 90.0, bar_len: 4.0, pickup: 0.0,
            click_every: 1.0, end: 0.0, playing: false, loop_range: None, loop_reps: 0, loops_done: 0,
            metronome: false, count_in: true, preroll_until: None, last_click: i64::MIN, sound_rh: true,
            sound_lh: true, volume: 0.6, wait: false, wait_rh: true, wait_lh: true, pressed: HashSet::new(),
            hit_since: HashSet::new(), echo: false, voices: Vec::with_capacity(64),
        }
    }

    fn seek(&mut self, p: f64) {
        self.pos = p;
        self.idx = self.events.partition_point(|e| e.t < p - 1e-9);
        self.release_source(0);
        self.hit_since.clear();
        self.reset_click();
        self.shared.set_pos(p);
    }

    fn release_source(&mut self, src: u8) {
        for v in &mut self.voices {
            if v.source == src && v.release.is_none() {
                v.release = Some(v.age);
            }
        }
    }

    fn note_on(&mut self, pitch: u8, vel: f32, source: u8) {
        if self.voices.len() > 56 {
            // drop the oldest released (or oldest) voice
            let i = self.voices.iter().position(|v| v.release.is_some()).unwrap_or(0);
            self.voices.remove(i);
        }
        let freq = 440.0 * 2f32.powf((pitch as f32 - 69.0) / 12.0);
        self.voices.push(Voice { pitch, freq, phase: [0.0; 6], age: 0.0, amp: vel, release: None, source, click: false });
    }

    fn note_off(&mut self, pitch: u8, source: u8) {
        for v in &mut self.voices {
            if v.pitch == pitch && v.source == source && v.release.is_none() && !v.click {
                v.release = Some(v.age);
            }
        }
    }

    fn click(&mut self, accent: bool) {
        let freq = if accent { 1760.0 } else { 1175.0 };
        self.voices.push(Voice { pitch: 0, freq, phase: [0.0; 6], age: 0.0, amp: if accent { 0.55 } else { 0.35 }, release: Some(0.0), source: 2, click: true });
    }

    fn handle(&mut self, c: Cmd) {
        match c {
            Cmd::Load { notes, bar_len, pickup, click_every, end } => {
                let mut ev = Vec::with_capacity(notes.len() * 2);
                for n in &notes {
                    ev.push(Ev { t: n.start, pitch: n.pitch, on: true, hand: n.hand });
                    ev.push(Ev { t: n.start + n.dur * 0.97, pitch: n.pitch, on: false, hand: n.hand });
                }
                ev.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap().then(a.on.cmp(&b.on)));
                self.events = ev;
                self.bar_len = bar_len;
                self.pickup = pickup;
                self.click_every = click_every;
                self.end = end;
                let p = self.pos.min(end);
                self.seek(p);
            }
            Cmd::Play => {
                if !self.playing {
                    self.playing = true;
                    self.loops_done = 0;
                    self.shared.loops_done.store(0, Ordering::Relaxed);
                    self.shared.finished.store(false, Ordering::Relaxed);
                    if self.pos >= self.end - 1e-6 {
                        let s = self.loop_range.map(|l| l.0).unwrap_or(0.0);
                        self.seek(s);
                    }
                    if self.count_in {
                        let start = self.pos;
                        self.preroll_until = Some(start);
                        self.pos = start - self.bar_len;
                        self.reset_click();
                    }
                }
            }
            Cmd::Pause => {
                self.playing = false;
                self.preroll_until = None;
                self.release_source(0);
                if self.pos < 0.0 {
                    self.pos = 0.0;
                }
            }
            Cmd::Seek(p) => {
                self.preroll_until = None;
                self.seek(p);
            }
            Cmd::Tempo(b) => self.bpm = b.clamp(10.0, 300.0),
            Cmd::Loop(r, reps) => {
                self.loop_range = r;
                self.loop_reps = reps;
            }
            Cmd::Metronome(m) => self.metronome = m,
            Cmd::CountIn(c) => self.count_in = c,
            Cmd::Sound { rh, lh } => {
                self.sound_rh = rh;
                self.sound_lh = lh;
                self.release_source(0);
            }
            Cmd::Volume(v) => self.volume = v,
            Cmd::Wait { enabled, rh, lh } => {
                self.wait = enabled;
                self.wait_rh = rh;
                self.wait_lh = lh;
            }
            Cmd::KeyDown(p, vel) => {
                self.pressed.insert(p);
                self.hit_since.insert(p);
                if self.echo {
                    self.note_on(p, vel as f32 / 127.0, 1);
                }
            }
            Cmd::KeyUp(p) => {
                self.pressed.remove(&p);
                self.note_off(p, 1);
            }
            Cmd::Echo(e) => self.echo = e,
            Cmd::Preview(p) => {
                self.note_on(p, 0.7, 1);
                // auto-release handled by natural decay; schedule release after a bit
                if let Some(v) = self.voices.last_mut() {
                    v.release = Some(0.6);
                }
            }
        }
    }

    /// Notes the student must press before we may pass beat `t`.
    fn wait_satisfied(&self, t: f64) -> bool {
        let mut i = self.idx;
        while i < self.events.len() && (self.events[i].t - t).abs() < 1e-6 {
            let e = self.events[i];
            if e.on {
                let mine = (e.hand == Hand::Right && self.wait_rh) || (e.hand == Hand::Left && self.wait_lh);
                if mine && !self.hit_since.contains(&e.pitch) {
                    return false;
                }
            }
            i += 1;
        }
        true
    }

    fn step_sequencer(&mut self, dt_beats: f64) {
        if !self.playing {
            return;
        }
        let next = self.pos + dt_beats;
        // pre-roll (count-in): only clicks
        if let Some(st) = self.preroll_until {
            self.metronome_tick(next, true);
            self.pos = next;
            if self.pos >= st {
                self.pos = st;
                self.preroll_until = None;
                self.shared.counting_in.store(false, Ordering::Relaxed);
            } else {
                self.shared.counting_in.store(true, Ordering::Relaxed);
            }
            return;
        }
        // wait mode: hold at the next onset until the right keys are pressed
        if self.wait {
            if let Some(e) = self.events.get(self.idx) {
                if e.on && e.t <= next && !self.wait_satisfied(e.t) {
                    self.pos = e.t;
                    self.shared.waiting.store(true, Ordering::Relaxed);
                    return;
                }
            }
            self.shared.waiting.store(false, Ordering::Relaxed);
        }
        while self.idx < self.events.len() && self.events[self.idx].t <= next {
            let e = self.events[self.idx];
            let audible = match e.hand {
                Hand::Right => self.sound_rh,
                Hand::Left => self.sound_lh,
            };
            if e.on {
                // starting a new onset group clears the "hit" memory
                if self.wait && (self.idx == 0 || self.events[self.idx - 1].t < e.t - 1e-6) {
                    self.hit_since.retain(|p| self.pressed.contains(p));
                }
                if audible {
                    self.note_on(e.pitch, 0.55, 0);
                }
            } else {
                self.note_off(e.pitch, 0);
            }
            self.idx += 1;
        }
        self.metronome_tick(next, false);
        self.pos = next;
        let stop_at = self.loop_range.map(|l| l.1).unwrap_or(self.end);
        if self.pos >= stop_at {
            if let Some((a, _)) = self.loop_range {
                self.loops_done += 1;
                self.shared.loops_done.store(self.loops_done, Ordering::Relaxed);
                if self.loop_reps > 0 && self.loops_done >= self.loop_reps {
                    self.playing = false;
                    self.shared.finished.store(true, Ordering::Relaxed);
                }
                self.seek(a);
            } else {
                self.playing = false;
                self.shared.finished.store(true, Ordering::Relaxed);
                self.release_source(0);
                self.pos = self.end;
            }
        }
    }

    fn reset_click(&mut self) {
        self.last_click = ((self.pos - self.pickup) / self.click_every - 1e-9).ceil() as i64 - 1;
    }

    fn metronome_tick(&mut self, next: f64, force: bool) {
        let k = ((next - self.pickup) / self.click_every + 1e-9).floor() as i64;
        if k > self.last_click {
            self.last_click = k;
            if self.metronome || force {
                let at = self.pickup + k as f64 * self.click_every;
                let in_bar = ((at - self.pickup) / self.bar_len).rem_euclid(1.0);
                self.click(in_bar < 1e-6 || in_bar > 1.0 - 1e-6);
            }
        }
    }

    fn render(&mut self, out: &mut [f32], channels: usize) {
        while let Ok(c) = self.rx.try_recv() {
            self.handle(c);
        }
        let frames = out.len() / channels;
        let dt = 1.0 / self.sr;
        // step sequencer in 64-sample chunks for good timing
        let chunk = 64usize;
        let mut f = 0;
        while f < frames {
            let n = chunk.min(frames - f);
            let beats = n as f64 * self.bpm / 60.0 / self.sr as f64;
            self.step_sequencer(beats);
            for i in 0..n {
                let mut s = 0.0f32;
                for v in &mut self.voices {
                    s += voice_sample(v, dt);
                }
                let s = (s * self.volume * 0.35).tanh();
                for c in 0..channels {
                    out[(f + i) * channels + c] = s;
                }
            }
            f += n;
        }
        self.voices.retain(|v| !voice_dead(v));
        self.shared.set_pos(self.pos);
        self.shared.playing.store(self.playing, Ordering::Relaxed);
    }
}

fn voice_dead(v: &Voice) -> bool {
    if v.click {
        return v.age > 0.06;
    }
    match v.release {
        Some(r) => v.age > r + 0.25,
        None => v.age > 12.0,
    }
}

const TAU: f32 = std::f32::consts::TAU;

fn voice_sample(v: &mut Voice, dt: f32) -> f32 {
    v.age += dt;
    if v.click {
        v.phase[0] = (v.phase[0] + v.freq * dt).fract();
        return (v.phase[0] * TAU).sin() * v.amp * (-v.age * 90.0).exp();
    }
    // simple piano-ish additive tone: bright attack, upper partials decay faster
    let low = ((88.0 - v.pitch as f32) / 60.0).clamp(0.2, 1.3); // low notes ring longer
    let mut s = 0.0;
    const H: [f32; 6] = [1.0, 0.45, 0.22, 0.12, 0.07, 0.04];
    for k in 0..6 {
        let fk = v.freq * (k as f32 + 1.0) * (1.0 + 0.0004 * (k * k) as f32);
        if fk > 9000.0 {
            break;
        }
        v.phase[k] = (v.phase[k] + fk * dt).fract();
        let decay = (-v.age * (0.9 + k as f32 * 1.1) / low).exp();
        s += (v.phase[k] * TAU).sin() * H[k] * decay;
    }
    let attack = (v.age / 0.004).min(1.0);
    let rel = match v.release {
        Some(r) if v.age > r => (-(v.age - r) / 0.07).exp(),
        _ => 1.0,
    };
    s * v.amp * attack * rel
}

pub struct Audio {
    pub tx: Sender<Cmd>,
    pub shared: Arc<Shared>,
    _stream: Option<cpal::Stream>,
    pub device_name: String,
}

impl Audio {
    pub fn start() -> Audio {
        let (tx, rx) = channel();
        let shared = Arc::new(Shared::default());
        match open_stream(rx, shared.clone()) {
            Ok((stream, name)) => {
                shared.has_device.store(true, Ordering::Relaxed);
                Audio { tx, shared, _stream: Some(stream), device_name: name }
            }
            Err(err) => {
                let (tx, rx) = channel();
                // no sound card: keep the transport running silently so practice still works
                let sh = shared.clone();
                std::thread::spawn(move || {
                    let mut core = Core::new(44100.0, rx, sh);
                    let mut buf = vec![0.0f32; 441];
                    loop {
                        core.render(&mut buf, 1);
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                });
                Audio { tx, shared, _stream: None, device_name: format!("No audio output ({err})") }
            }
        }
    }

    pub fn send(&self, c: Cmd) {
        let _ = self.tx.send(c);
    }
}

fn open_stream(rx: Receiver<Cmd>, shared: Arc<Shared>) -> Result<(cpal::Stream, String), String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();
    let Some(device) = host.default_output_device() else { return Err("no output device".into()) };
    let config = match device.default_output_config() {
        Ok(c) => c,
        Err(e) => return Err(e.to_string()),
    };
    let name = device.name().unwrap_or_else(|_| "Default output".into());
    let sr = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;
    let fmt = config.sample_format();
    let cfg: cpal::StreamConfig = config.into();
    let mut core = Core::new(sr, rx, shared);
    let err_fn = |e| eprintln!("audio error: {e}");
    let stream = match fmt {
        cpal::SampleFormat::F32 => device.build_output_stream(&cfg, move |d: &mut [f32], _| core.render(d, channels), err_fn, None),
        cpal::SampleFormat::I16 => {
            let mut tmp = Vec::new();
            device.build_output_stream(
                &cfg,
                move |d: &mut [i16], _| {
                    tmp.resize(d.len(), 0.0);
                    core.render(&mut tmp, channels);
                    for (o, s) in d.iter_mut().zip(&tmp) {
                        *o = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                    }
                },
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let mut tmp = Vec::new();
            device.build_output_stream(
                &cfg,
                move |d: &mut [u16], _| {
                    tmp.resize(d.len(), 0.0);
                    core.render(&mut tmp, channels);
                    for (o, s) in d.iter_mut().zip(&tmp) {
                        *o = ((s.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32) as u16;
                    }
                },
                err_fn,
                None,
            )
        }
        other => return Err(format!("unsupported sample format {other:?}")),
    };
    match stream {
        Ok(s) => {
            if let Err(e) = s.play() {
                return Err(e.to_string());
            }
            Ok((s, name))
        }
        Err(e) => Err(e.to_string()),
    }
}
