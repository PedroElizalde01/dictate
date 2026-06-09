use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config;

const MAX_ENTRIES: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: u64,
    pub text: String,
    pub timestamp_ms: u64,
}

fn history_path() -> PathBuf {
    config::data_dir().join("history.json")
}

pub fn load_history() -> Vec<HistoryEntry> {
    std::fs::read_to_string(history_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_history(entries: &[HistoryEntry]) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(entries).unwrap();
    std::fs::write(history_path(), json)
}

pub fn add_entry(text: &str) -> std::io::Result<HistoryEntry> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let entry = HistoryEntry {
        id: now,
        text: text.to_string(),
        timestamp_ms: now,
    };
    let mut entries = load_history();
    entries.insert(0, entry.clone());
    entries.truncate(MAX_ENTRIES);
    save_history(&entries)?;
    Ok(entry)
}

pub fn delete_entry(id: u64) -> std::io::Result<()> {
    let mut entries = load_history();
    entries.retain(|e| e.id != id);
    save_history(&entries)
}

pub fn clear() -> std::io::Result<()> {
    save_history(&[])
}
