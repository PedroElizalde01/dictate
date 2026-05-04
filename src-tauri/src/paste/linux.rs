use anyhow::{anyhow, Result};
use std::process::Command;
use std::time::Duration;

pub fn active_window_id() -> Option<String> {
    let out = Command::new("xdotool").arg("getactivewindow").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

fn raise_window(id: &str) {
    let _ = Command::new("xdotool")
        .arg("windowactivate")
        .arg("--sync")
        .arg(id)
        .status();
}

pub fn type_text(text: &str, target_window: Option<&str>) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }
    if let Some(id) = target_window {
        raise_window(id);
        std::thread::sleep(Duration::from_millis(80));
    }
    let out = Command::new("xdotool")
        .arg("type")
        .arg("--clearmodifiers")
        .arg("--delay")
        .arg("4")
        .arg("--")
        .arg(text)
        .output()
        .map_err(|e| anyhow!("xdotool spawn failed: {e}. Install xdotool."))?;
    if !out.status.success() {
        return Err(anyhow!(
            "xdotool type exit {}: stderr={}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}
