#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::{active_window_id, type_text};
#[cfg(target_os = "windows")]
pub use windows::{active_window_id, type_text};

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn active_window_id() -> Option<String> {
    None
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn type_text(_text: &str, _target_window: Option<&str>) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(
        "auto-paste is not supported on this operating system yet"
    ))
}
