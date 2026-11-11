//! settings.rs - your choices, saved between sessions.
//!
//! Two small JSON files live in a per-user folder that the operating system
//! picks (found with the `dirs` crate):
//!
//! * `settings.json`     - theme, layout, text size, kitchen defaults
//! * `current_plan.json` - the bakes you have added, reopened next time
//!
//! Nothing is sent anywhere. There is no account and no tracking.

use crate::schedule::Batch;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Which colours to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    /// Follow the operating system.
    System,
    Dark,
    Light,
}

/// Everything the Settings window can change.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)] // a missing field in an older file falls back to its default
pub struct Settings {
    pub theme: ThemeChoice,
    /// false = Simple layout (the essentials), true = Advanced (every dial).
    pub advanced: bool,
    /// Text size in points.
    pub font_size: f32,
    /// Turns off animations.
    pub reduce_motion: bool,
    pub room_c: f64,
    pub flour_c: f64,
    /// "by hand", "stand mixer" or "spiral mixer".
    pub mixing: String,
    /// Minutes before a snoozed alarm rings again.
    pub snooze_minutes: i64,
    /// Your own mixer friction (deg C) from the Water tab's calibration, if you set one.
    pub custom_friction: Option<f64>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: ThemeChoice::System,
            advanced: false,
            font_size: 15.0,
            reduce_motion: false,
            room_c: 21.0,
            flour_c: 20.0,
            mixing: "by hand".into(),
            snooze_minutes: 5,
            custom_friction: None,
        }
    }
}

/// The app's private data folder (created on first use).
pub fn data_dir() -> PathBuf {
    let dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("bakecraft");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Where exported sheets go: the user's Documents folder if there is one.
pub fn export_dir() -> PathBuf {
    let dir = dirs::document_dir().unwrap_or_else(data_dir).join("BakeCraft");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

impl Settings {
    /// Load settings; any problem simply gives the defaults.
    pub fn load() -> Self {
        std::fs::read_to_string(data_dir().join("settings.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Save settings. Returns an error message to show in the status bar if it fails.
    pub fn save(&self) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(data_dir().join("settings.json"), text).map_err(|e| format!("Could not save settings: {e}"))
    }
}

/// The plan: a list of bakes plus which steps are ticked off.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Plan {
    pub batches: Vec<Batch>,
    /// Keys of events marked done (see `Event::key`).
    #[serde(default)]
    pub done: Vec<String>,
}

impl Plan {
    /// A plan file name must not contain odd characters - keep letters, digits, - and _.
    pub fn safe_name(name: &str) -> String {
        let cleaned: String = name.trim().chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
        if cleaned.is_empty() { "plan".into() } else { cleaned }
    }

    fn path(name: &str) -> PathBuf {
        data_dir().join(format!("{}.json", Plan::safe_name(name)))
    }

    pub fn save(&self, name: &str) -> Result<PathBuf, String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let path = Plan::path(name);
        std::fs::write(&path, text).map_err(|e| format!("Could not save the plan: {e}"))?;
        Ok(path)
    }

    pub fn load(name: &str) -> Result<Plan, String> {
        let text = std::fs::read_to_string(Plan::path(name)).map_err(|_| format!("No saved plan called '{name}'."))?;
        serde_json::from_str(&text).map_err(|e| format!("That plan file could not be read: {e}"))
    }

    /// Names of all saved plans (not counting the automatic one).
    pub fn saved_names() -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(data_dir())
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter_map(|e| e.path().file_stem().and_then(|s| s.to_str()).map(String::from).filter(|_| e.path().extension().is_some_and(|x| x == "json")))
                    .filter(|n| n != "settings" && n != "current_plan")
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }
}
