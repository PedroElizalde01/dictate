use anyhow::{anyhow, Result};
use serde::Serialize;
use std::path::PathBuf;
use tauri::Manager;

use crate::config::models_dir;

#[derive(Debug, Serialize)]
pub struct ModelFile {
    pub name: String,
    pub path: String,
    pub size_mb: u64,
}

pub fn list_models() -> Vec<ModelFile> {
    let mut out = Vec::new();
    let dir = models_dir();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("bin") {
                let size_mb = p.metadata().map(|m| m.len() / 1_048_576).unwrap_or(0);
                out.push(ModelFile {
                    name: p.file_name().unwrap_or_default().to_string_lossy().into(),
                    path: p.to_string_lossy().into(),
                    size_mb,
                });
            }
        }
    }
    out
}

pub fn download_model(size: &str) -> Result<PathBuf> {
    let allowed = ["tiny", "base", "small", "medium"];
    if !allowed.contains(&size) {
        return Err(anyhow!("invalid model size"));
    }
    let filename = format!("ggml-{size}.bin");
    let url = format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{filename}"
    );
    let dest = models_dir().join(&filename);
    if dest.exists() {
        return Ok(dest);
    }
    let tmp = dest.with_extension("bin.part");
    let resp = reqwest::blocking::get(&url)?;
    if !resp.status().is_success() {
        return Err(anyhow!("download failed: {}", resp.status()));
    }
    let bytes = resp.bytes()?;
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, &dest)?;
    Ok(dest)
}

pub fn whisper_binary_path(app: &tauri::AppHandle) -> PathBuf {
    let binary_name = format!("whisper-cli{}", std::env::consts::EXE_SUFFIX);
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dev_bin = manifest_dir.join("binaries").join(&binary_name);
    if dev_bin.exists() {
        return dev_bin;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidates = [
                parent.join(&binary_name),
                parent.join("binaries").join(&binary_name),
            ];
            for c in candidates {
                if c.exists() {
                    return c;
                }
            }
        }
    }
    if let Ok(resource) = app.path().resource_dir() {
        let p = resource.join("binaries").join(&binary_name);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from(binary_name)
}

