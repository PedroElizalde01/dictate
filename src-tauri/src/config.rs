use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictEntry {
    /// Misheard word to replace (empty = vocabulary hint only).
    #[serde(default)]
    pub from: String,
    /// Correct word. Also fed to whisper as a vocabulary bias prompt.
    #[serde(default)]
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub mic_device: Option<String>,
    pub model_path: Option<String>,
    pub hotkey: String,
    #[serde(default = "default_cancel")]
    pub cancel_hotkey: String,
    #[serde(default = "default_settings_sc")]
    pub settings_hotkey: String,
    pub language: String,
    pub post_process: String,
    pub autostart: bool,
    #[serde(default)]
    pub dictionary: Vec<DictEntry>,
    #[serde(default)]
    pub review_mode: bool,
    #[serde(default = "default_confirm_key")]
    pub confirm_key: String,
}

fn default_cancel() -> String { "CmdOrCtrl+Shift+X".into() }
fn default_settings_sc() -> String { "CmdOrCtrl+Shift+,".into() }
fn default_confirm_key() -> String { "Tab".into() }

impl Default for Settings {
    fn default() -> Self {
        Self {
            mic_device: None,
            model_path: None,
            hotkey: "CmdOrCtrl+Shift+D".into(),
            cancel_hotkey: default_cancel(),
            settings_hotkey: default_settings_sc(),
            language: "auto".into(),
            post_process: "cleanup".into(),
            autostart: false,
            dictionary: Vec::new(),
            review_mode: false,
            confirm_key: default_confirm_key(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    let dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let p = dir.join("dictate");
    std::fs::create_dir_all(&p).ok();
    p
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn data_dir() -> PathBuf {
    let dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    let p = dir.join("dictate");
    std::fs::create_dir_all(&p).ok();
    p
}

pub fn models_dir() -> PathBuf {
    let p = data_dir().join("models");
    std::fs::create_dir_all(&p).ok();
    p
}

pub fn load_settings() -> Settings {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(s: &Settings) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(s).unwrap();
    std::fs::write(settings_path(), json)
}
