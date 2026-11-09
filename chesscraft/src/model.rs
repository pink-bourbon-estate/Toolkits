//! The repertoire tree: loaded from the embedded, Stockfish-generated `repertoire.json`,
//! replayed move by move with `shakmaty` so every node carries a real, legal position.

use serde::{Deserialize, Serialize};
use shakmaty::san::San;
use shakmaty::{Chess, Color, Position};
use std::collections::HashMap;

#[derive(Deserialize, Clone)]
struct RawNode {
    ply: u32,
    san: String,
    color: String,
    parent: i64,
    user: bool,
    system: bool,
    plan_san: Option<String>,
    deviation_loss: i32,
    rank: u8,
    cp: Option<i32>,
    note: String,
}

#[derive(Deserialize)]
struct RawRep {
    side: String,
    nodes: Vec<RawNode>,
}

#[derive(Deserialize)]
struct RawFile {
    modern: RawRep,
    london: RawRep,
}

#[derive(Clone)]
pub struct Node {
    pub ply: u32,
    pub san: String,
    pub color: Color,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub user: bool,
    pub system: bool,
    pub plan_san: Option<String>,
    pub deviation_loss: i32,
    pub rank: u8,
    pub cp: Option<i32>,
    pub note: String,
    pub pos: Chess,
}

pub struct Repertoire {
    pub id: &'static str,
    pub title: &'static str,
    pub tagline: &'static str,
    pub side: Color,
    pub nodes: Vec<Node>,
    /// indices of ply-1 nodes (children of the start position)
    pub roots: Vec<usize>,
}

impl Repertoire {
    pub fn children(&self, idx: Option<usize>) -> &[usize] {
        match idx {
            None => &self.roots,
            Some(i) => &self.nodes[i].children,
        }
    }

    pub fn path_to(&self, idx: usize) -> Vec<usize> {
        let mut v = vec![idx];
        let mut cur = idx;
        while let Some(p) = self.nodes[cur].parent {
            v.push(p);
            cur = p;
        }
        v.reverse();
        v
    }

    pub fn position_after(&self, idx: Option<usize>) -> Chess {
        match idx {
            None => Chess::default(),
            Some(i) => self.nodes[i].pos.clone(),
        }
    }

    /// Every distinct position where it's our move, with whether it's still marked a system move
    /// the first time it's seen (mirrors the web app's "how often does each plan move appear"
    /// honesty check).
    pub fn user_move_count(&self) -> (usize, usize) {
        let mut seen: HashMap<String, bool> = HashMap::new();
        for n in &self.nodes {
            if n.user {
                let key = self.path_to(self.index_of(n)).iter().map(|i| self.nodes[*i].san.clone()).collect::<Vec<_>>().join(" ");
                seen.entry(key).or_insert(n.system);
            }
        }
        let total = seen.len();
        let system = seen.values().filter(|v| **v).count();
        (system, total)
    }

    fn index_of(&self, n: &Node) -> usize {
        // nodes are stored in a Vec where position == index; find by pointer comparison via ply+san+parent
        self.nodes.iter().position(|x| x.ply == n.ply && x.san == n.san && x.parent == n.parent).unwrap_or(0)
    }
}

fn build(raw: RawRep, side_color: Color) -> Repertoire {
    debug_assert_eq!(raw.side, if side_color == Color::White { "w" } else { "b" }, "repertoire.json side mismatch");
    let mut nodes: Vec<Node> = Vec::with_capacity(raw.nodes.len());
    let mut roots = Vec::new();
    for rn in &raw.nodes {
        let parent = if rn.parent < 0 { None } else { Some(rn.parent as usize) };
        let base_pos = match parent {
            None => Chess::default(),
            Some(p) => nodes[p].pos.clone(),
        };
        let pos = match San::from_ascii(rn.san.as_bytes()).ok().and_then(|s| s.to_move(&base_pos).ok()).and_then(|m| base_pos.clone().play(&m).ok()) {
            Some(p) => p,
            None => base_pos.clone(), // shouldn't happen: data was legality-checked at generation time
        };
        let idx = nodes.len();
        nodes.push(Node {
            ply: rn.ply,
            san: rn.san.clone(),
            color: if rn.color == "w" { Color::White } else { Color::Black },
            parent,
            children: Vec::new(),
            user: rn.user,
            system: rn.system,
            plan_san: rn.plan_san.clone(),
            deviation_loss: rn.deviation_loss,
            rank: rn.rank,
            cp: rn.cp,
            note: rn.note.clone(),
            pos,
        });
        match parent {
            None => roots.push(idx),
            Some(p) => nodes[p].children.push(idx),
        }
    }
    Repertoire { id: "", title: "", tagline: "", side: side_color, nodes, roots }
}

pub fn load() -> (Repertoire, Repertoire) {
    let raw: RawFile = serde_json::from_str(include_str!("../assets/repertoire.json")).expect("repertoire.json is embedded and was engine-validated at build time");
    let mut modern = build(raw.modern, Color::Black);
    modern.id = "modern";
    modern.title = "The Modern Defence";
    modern.tagline = "One kingside fianchetto against everything White does — generated and move-checked by Stockfish 19.";
    let mut london = build(raw.london, Color::White);
    london.id = "london";
    london.title = "The London System";
    london.tagline = "One set-up for White against almost any Black reply — generated and move-checked by Stockfish 19.";
    (modern, london)
}

// ---------------- spaced repetition + saved progress ----------------

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Card {
    pub reps: u32,
    pub ease: f32,
    pub interval_days: f32,
    pub due_unix: i64,
    pub lapses: u32,
}

impl Card {
    pub fn grade(&mut self, ok: bool, now: i64) {
        if ok {
            self.reps += 1;
            self.interval_days = if self.reps == 1 { 1.0 } else if self.reps == 2 { 3.0 } else { (self.interval_days.max(1.0)) * self.ease };
            self.ease = (self.ease + 0.05).min(3.0);
            self.due_unix = now + (self.interval_days as i64) * 86400;
        } else {
            self.reps = 0;
            self.lapses += 1;
            self.ease = (self.ease - 0.2).max(1.3);
            self.interval_days = 0.0;
            self.due_unix = now + 600; // 10 minutes
        }
    }
}

impl Default for Repertoire {
    fn default() -> Self {
        Repertoire { id: "", title: "", tagline: "", side: Color::White, nodes: Vec::new(), roots: Vec::new() }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Progress {
    /// keyed by "{rep_id}|{move path as space-joined SAN}"
    pub cards: HashMap<String, Card>,
    pub stage: HashMap<String, u8>,
}

pub fn data_dir() -> std::path::PathBuf {
    let d = dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("ChessCraft");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn load_progress() -> Progress {
    let p = data_dir().join("progress.json");
    std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save_progress(p: &Progress) {
    if let Ok(s) = serde_json::to_string_pretty(p) {
        let _ = std::fs::write(data_dir().join("progress.json"), s);
    }
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repertoire_loads_and_replays_legally() {
        let (modern, london) = load();
        assert_eq!(modern.side, Color::Black);
        assert_eq!(london.side, Color::White);
        assert!(modern.nodes.len() > 1000, "expected ~1316 modern nodes, got {}", modern.nodes.len());
        assert!(london.nodes.len() > 150, "expected ~221 london nodes, got {}", london.nodes.len());
        // Every node's san must have actually been applied (pos differs from its parent's).
        for n in &modern.nodes {
            let before = match n.parent {
                None => Chess::default(),
                Some(p) => modern.nodes[p].pos.clone(),
            };
            assert_ne!(before.board().clone(), n.pos.board().clone(), "node {} ({}) did not change the board — SAN failed to apply", n.ply, n.san);
        }
        let (sys, total) = modern.user_move_count();
        assert!(sys > 0 && sys <= total);
        let (sys2, total2) = london.user_move_count();
        assert!(sys2 > 0 && sys2 <= total2);
    }
}
