use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct FrecencyEntry {
    pub visits: u32,
    pub last_visit: u64, // days since epoch
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct FrecencyStore {
    entries: HashMap<String, FrecencyEntry>,
}

impl FrecencyStore {
    pub fn new() -> Self { Self::default() }

    pub fn record_visit(&mut self, path: &str) {
        let today = now_days();
        let entry = self.entries.entry(path.to_string()).or_insert(FrecencyEntry { visits: 0, last_visit: today });
        entry.visits += 1;
        entry.last_visit = today;
    }

    pub fn top_dirs(&self, n: usize) -> Vec<(String, f64)> {
        let today = now_days();
        let mut scored: Vec<(String, f64)> = self.entries.iter()
            .map(|(k, e)| {
                let days_since = today.saturating_sub(e.last_visit) as f64;
                let score = e.visits as f64 * (1.0 / (1.0 + days_since * 0.1));
                (k.clone(), score)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(n);
        scored
    }

    pub fn load() -> Self {
        let Some(path) = frecency_path() else { return Self::new() };
        std::fs::read_to_string(path).ok()
            .and_then(|c| toml::from_str(&c).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = frecency_path() else { return };
        if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
        if let Ok(content) = toml::to_string(self) { let _ = std::fs::write(path, content); }
    }
}

fn frecency_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("rem").join("frecency.toml"))
}
fn now_days() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() / 86400).unwrap_or(0)
}
