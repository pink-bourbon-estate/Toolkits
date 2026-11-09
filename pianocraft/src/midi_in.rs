//! Optional MIDI keyboard input (USB keyboards, or 5-pin MIDI keyboards via a USB-MIDI interface cable).

use crate::audio::Cmd;
use std::collections::HashSet;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub struct MidiIn {
    conn: Option<midir::MidiInputConnection<()>>,
    pub port_name: Option<String>,
    pub pressed: Arc<Mutex<HashSet<u8>>>,
    pub last_error: Option<String>,
}

impl Default for MidiIn {
    fn default() -> Self {
        Self { conn: None, port_name: None, pressed: Arc::new(Mutex::new(HashSet::new())), last_error: None }
    }
}

impl MidiIn {
    pub fn list_ports() -> Vec<String> {
        let Ok(mi) = midir::MidiInput::new("PianoCraft scan") else { return vec![] };
        mi.ports().iter().filter_map(|p| mi.port_name(p).ok()).collect()
    }

    pub fn connected(&self) -> bool {
        self.conn.is_some()
    }

    pub fn disconnect(&mut self) {
        if let Some(c) = self.conn.take() {
            c.close();
        }
        self.port_name = None;
        self.pressed.lock().unwrap().clear();
    }

    pub fn connect(&mut self, name: &str, tx: Sender<Cmd>) {
        self.disconnect();
        let mi = match midir::MidiInput::new("PianoCraft") {
            Ok(m) => m,
            Err(e) => {
                self.last_error = Some(e.to_string());
                return;
            }
        };
        let Some(port) = mi.ports().into_iter().find(|p| mi.port_name(p).ok().as_deref() == Some(name)) else {
            self.last_error = Some("That MIDI port is no longer available".into());
            return;
        };
        let pressed = self.pressed.clone();
        let tx = Mutex::new(tx);
        match mi.connect(
            &port,
            "pianocraft-in",
            move |_t, msg, _| {
                if msg.len() < 3 {
                    return;
                }
                let status = msg[0] & 0xF0;
                let (key, vel) = (msg[1], msg[2]);
                let tx = tx.lock().unwrap();
                if status == 0x90 && vel > 0 {
                    pressed.lock().unwrap().insert(key);
                    let _ = tx.send(Cmd::KeyDown(key, vel));
                } else if status == 0x80 || (status == 0x90 && vel == 0) {
                    pressed.lock().unwrap().remove(&key);
                    let _ = tx.send(Cmd::KeyUp(key));
                }
            },
            (),
        ) {
            Ok(c) => {
                self.conn = Some(c);
                self.port_name = Some(name.to_string());
                self.last_error = None;
            }
            Err(e) => self.last_error = Some(e.to_string()),
        }
    }
}
