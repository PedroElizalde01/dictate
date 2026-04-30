use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;
pub fn transcribe(
    binary: &Path,
    model: &Path,
    wav: &Path,
    language: &str,
) -> Result<String> {
    let mut cmd = Command::new(binary);
    cmd.arg("-m").arg(model);
    cmd.arg("-f").arg(wav);
    cmd.arg("-l").arg(language);
    cmd.arg("-nt"); // no timestamps
    cmd.arg("-otxt");
    cmd.arg("-of").arg(wav.with_extension("").as_os_str());
    cmd.arg("--no-prints");

    let output = cmd.output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "whisper-cli failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let txt_path = wav.with_extension("txt");
    let text = std::fs::read_to_string(&txt_path).unwrap_or_else(|_| {
        String::from_utf8_lossy(&output.stdout).to_string()
    });
    let _ = std::fs::remove_file(&txt_path);
    Ok(text.trim().to_string())
}
