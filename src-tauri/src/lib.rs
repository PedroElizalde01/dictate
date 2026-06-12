mod audio;
mod cleanup;
mod config;
mod dictionary;
mod history;
mod models;
mod paste;
mod whisper;

use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use tauri::{
    include_image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::{ManagerExt, MacosLauncher};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use std::str::FromStr;

use crate::audio::AudioController;
use crate::config::Settings;

struct PendingReview {
    original: String,
    target: Option<String>,
}

struct AppState {
    audio: AudioController,
    is_recording: Mutex<bool>,
    target_window: Mutex<Option<String>>,
    settings: Mutex<Settings>,
    pending_review: Mutex<Option<PendingReview>>,
    dictate_sc: Mutex<Option<Shortcut>>,
    cancel_sc: Mutex<Option<Shortcut>>,
    settings_sc: Mutex<Option<Shortcut>>,
}

fn rebind(
    app: &AppHandle,
    slot: &Mutex<Option<Shortcut>>,
    combo: &str,
) -> Result<(), String> {
    let gs = app.global_shortcut();
    if let Some(prev) = slot.lock().take() {
        let _ = gs.unregister(prev);
    }
    let sc = Shortcut::from_str(combo).map_err(|e| format!("parse: {e}"))?;
    gs.register(sc.clone()).map_err(|e| format!("register: {e}"))?;
    *slot.lock() = Some(sc);
    Ok(())
}

#[derive(Serialize, Clone)]
struct PhasePayload {
    phase: &'static str,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ReviewPayload {
    text: String,
    confirm_key: String,
}

#[tauri::command]
fn list_mics() -> Result<Vec<audio::MicDevice>, String> {
    audio::list_input_devices().map_err(|e| e.to_string())
}

#[tauri::command]
fn list_models() -> Vec<models::ModelFile> {
    models::list_models()
}

#[tauri::command]
fn download_model(size: String) -> Result<String, String> {
    models::download_model(&size)
        .map(|p| p.to_string_lossy().into())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(state: tauri::State<Arc<AppState>>) -> Settings {
    state.settings.lock().clone()
}

#[tauri::command]
fn save_settings(
    settings: Settings,
    state: tauri::State<Arc<AppState>>,
) -> Result<(), String> {
    *state.settings.lock() = settings.clone();
    config::save_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn apply_hotkey(combo: String, app: AppHandle, state: tauri::State<Arc<AppState>>) -> Result<(), String> {
    rebind(&app, &state.dictate_sc, &combo)
}

#[tauri::command]
fn apply_cancel_hotkey(combo: String, app: AppHandle, state: tauri::State<Arc<AppState>>) -> Result<(), String> {
    rebind(&app, &state.cancel_sc, &combo)
}

#[tauri::command]
fn apply_settings_hotkey(combo: String, app: AppHandle, state: tauri::State<Arc<AppState>>) -> Result<(), String> {
    rebind(&app, &state.settings_sc, &combo)
}

#[tauri::command]
fn apply_autostart(enabled: bool, app: AppHandle) -> Result<(), String> {
    let mgr = app.autolaunch();
    if enabled {
        mgr.enable().map_err(|e| e.to_string())
    } else {
        mgr.disable().map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn get_history() -> Vec<history::HistoryEntry> {
    history::load_history()
}

#[tauri::command]
fn delete_history_entry(id: u64) -> Result<(), String> {
    history::delete_entry(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_history() -> Result<(), String> {
    history::clear().map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_main(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

#[tauri::command]
fn show_main(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[tauri::command]
fn toggle_dictate(app: AppHandle, state: tauri::State<Arc<AppState>>) {
    do_toggle(app, state.inner().clone());
}

fn do_toggle(app: AppHandle, state: Arc<AppState>) {
    let mut flag = state.is_recording.lock();
    if *flag {
        *flag = false;
        drop(flag);
        let _ = app.emit("dictate-phase", PhasePayload { phase: "transcribing" });
        let app2 = app.clone();
        let state2 = state.clone();
        std::thread::spawn(move || {
            match finish_dictation(&app2, &state2) {
                // review pending: overlay stays open with the editable text
                Ok(false) => return,
                Ok(true) => {}
                Err(e) => {
                    let msg = format!("{e}");
                    eprintln!("dictation error: {msg}");
                    let _ = app2.emit("dictate-error", msg);
                }
            }
            let _ = app2.emit("dictate-phase", PhasePayload { phase: "exit" });
            std::thread::sleep(std::time::Duration::from_millis(260));
            hide_overlay(&app2);
        });
    } else {
        let settings = state.settings.lock().clone();
        let target = paste::active_window_id();
        eprintln!("captured target window: {target:?}");
        *state.target_window.lock() = target;
        match state.audio.start(settings.mic_device.clone()) {
            Ok(()) => {
                *flag = true;
                drop(flag);
                show_overlay(&app);
                let _ = app.emit("dictate-phase", PhasePayload { phase: "recording" });
            }
            Err(e) => {
                let msg = format!("recorder start failed: {e}");
                eprintln!("{msg}");
                let _ = app.emit("dictate-error", msg);
            }
        }
    }
}


/// Returns Ok(false) when review mode kept the overlay open, Ok(true) when done.
fn finish_dictation(app: &AppHandle, state: &Arc<AppState>) -> anyhow::Result<bool> {
    let settings = state.settings.lock().clone();
    let model_path = settings
        .model_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no model selected"))?;
    let bin = models::whisper_binary_path(app);
    if !bin.exists() {
        return Err(anyhow::anyhow!(
            "whisper-cli binary not found at {}. Run scripts/build-whisper.ps1 on Windows or ./scripts/build-whisper.sh on Linux.",
            bin.display()
        ));
    }
    if !std::path::Path::new(model_path).exists() {
        return Err(anyhow::anyhow!("model file missing: {model_path}"));
    }

    let (samples, src_rate) = state.audio.stop()?;
    let dur_s = samples.len() as f32 / src_rate as f32;
    if samples.is_empty() || dur_s < 0.3 {
        return Err(anyhow::anyhow!("recording too short ({dur_s:.2}s)"));
    }

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let tmp = config::data_dir().join(format!("rec-{stamp}.wav"));
    audio::write_wav_16k(&tmp, &samples, src_rate)?;

    let prompt = dictionary::vocab_prompt(&settings.dictionary);
    let text = whisper::transcribe(
        &bin,
        std::path::Path::new(model_path),
        &tmp,
        &settings.language,
        prompt.as_deref(),
    )?;
    let _ = std::fs::remove_file(&tmp);
    eprintln!("transcribed ({dur_s:.2}s, {} chars): {text:?}", text.len());

    let final_text = if settings.post_process == "cleanup" {
        cleanup::basic_cleanup(&text)
    } else {
        text
    };
    let final_text = dictionary::apply(&final_text, &settings.dictionary);
    if final_text.is_empty() {
        let _ = app.emit("dictate-error", "empty transcription");
        return Ok(true);
    }

    if settings.review_mode {
        *state.pending_review.lock() = Some(PendingReview {
            original: final_text.clone(),
            target: state.target_window.lock().clone(),
        });
        layout_overlay(app, 480.0, 200.0, true);
        let _ = app.emit("dictate-phase", PhasePayload { phase: "review" });
        let _ = app.emit(
            "review-text",
            ReviewPayload { text: final_text, confirm_key: settings.confirm_key.clone() },
        );
        return Ok(false);
    }

    if let Err(e) = history::add_entry(&final_text) {
        eprintln!("history save failed: {e}");
    }
    let _ = app.emit("history-updated", ());
    let target = state.target_window.lock().clone();
    let _ = app.emit("dictate-phase", PhasePayload { phase: "exit" });
    std::thread::sleep(std::time::Duration::from_millis(220));
    hide_overlay(app);
    std::thread::sleep(std::time::Duration::from_millis(60));
    eprintln!("paste -> target {target:?}, {} chars", final_text.len());
    paste::type_text(&final_text, target.as_deref())?;
    Ok(true)
}

#[tauri::command]
fn confirm_review(text: String, app: AppHandle, state: tauri::State<Arc<AppState>>) {
    let Some(pending) = state.pending_review.lock().take() else { return };
    let state2 = state.inner().clone();
    std::thread::spawn(move || {
        let edited = text.trim().to_string();

        // learn replacement pairs from the user's edits
        let learned = dictionary::learn(&pending.original, &edited);
        if !learned.is_empty() {
            let mut s = state2.settings.lock();
            let mut changed = false;
            for e in learned {
                eprintln!("dictionary learned: {} -> {}", e.from, e.to);
                changed |= dictionary::merge_entry(&mut s.dictionary, e);
            }
            let snapshot = s.clone();
            drop(s);
            if changed {
                let _ = config::save_settings(&snapshot);
                let _ = app.emit("settings-updated", ());
            }
        }

        if !edited.is_empty() {
            if let Err(e) = history::add_entry(&edited) {
                eprintln!("history save failed: {e}");
            }
            let _ = app.emit("history-updated", ());
        }

        let _ = app.emit("dictate-phase", PhasePayload { phase: "exit" });
        std::thread::sleep(std::time::Duration::from_millis(220));
        hide_overlay(&app);
        layout_overlay(&app, 220.0, 60.0, false);
        std::thread::sleep(std::time::Duration::from_millis(60));
        if !edited.is_empty() {
            eprintln!("paste -> target {:?}, {} chars", pending.target, edited.len());
            if let Err(e) = paste::type_text(&edited, pending.target.as_deref()) {
                let _ = app.emit("dictate-error", format!("paste failed: {e}"));
            }
        }
    });
}

#[tauri::command]
fn cancel_review(app: AppHandle, state: tauri::State<Arc<AppState>>) {
    if state.pending_review.lock().take().is_none() {
        return;
    }
    std::thread::spawn(move || {
        let _ = app.emit("dictate-phase", PhasePayload { phase: "exit" });
        std::thread::sleep(std::time::Duration::from_millis(220));
        hide_overlay(&app);
        layout_overlay(&app, 220.0, 60.0, false);
        eprintln!("review discarded");
    });
}

fn do_cancel(app: AppHandle, state: Arc<AppState>) {
    let mut flag = state.is_recording.lock();
    if !*flag {
        return;
    }
    *flag = false;
    drop(flag);
    let _ = state.audio.stop();
    let _ = app.emit("dictate-phase", PhasePayload { phase: "exit" });
    let app2 = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(260));
        hide_overlay(&app2);
    });
    eprintln!("dictation cancelled");
}

fn do_show_settings(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Monitor the overlay should appear on: under the cursor first (the overlay
/// keeps its last position, so current_monitor() points at the wrong screen
/// after the user moves to another display), then current, then primary.
fn target_monitor(h: &AppHandle, w: &tauri::WebviewWindow) -> Option<tauri::Monitor> {
    h.cursor_position()
        .ok()
        .and_then(|p| h.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| w.current_monitor().ok().flatten())
        .or_else(|| w.primary_monitor().ok().flatten())
}

/// Bottom-center the overlay on `monitor` given its logical size.
fn position_overlay(w: &tauri::WebviewWindow, monitor: &tauri::Monitor, w_log: f64, h_log: f64) {
    let size = monitor.size();
    let pos = monitor.position();
    let scale = monitor.scale_factor();
    let win_w = (w_log * scale) as i32;
    let win_h = (h_log * scale) as i32;
    let x = pos.x + (size.width as i32 - win_w) / 2;
    let y = pos.y + size.height as i32 - win_h - (40.0 * scale) as i32;
    let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
}

fn show_overlay(app: &AppHandle) {
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = h.get_webview_window("overlay") {
            if let Some(monitor) = target_monitor(&h, &w) {
                position_overlay(&w, &monitor, 220.0, 60.0);
            }
            let _ = w.show();
        }
    });
}

/// Resize + reposition the overlay (logical units, bottom-center).
/// `focus` grabs keyboard focus, needed for review editing.
fn layout_overlay(app: &AppHandle, w_log: f64, h_log: f64, focus: bool) {
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = h.get_webview_window("overlay") {
            let _ = w.set_size(tauri::LogicalSize::new(w_log, h_log));
            if let Some(monitor) = target_monitor(&h, &w) {
                position_overlay(&w, &monitor, w_log, h_log);
            }
            if focus {
                let _ = w.set_focus();
            }
        }
    });
}

fn hide_overlay(app: &AppHandle) {
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = h.get_webview_window("overlay") {
            let _ = w.hide();
        }
    });
}

pub fn run() {
    let initial = config::load_settings();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let Some(state) = app.try_state::<Arc<AppState>>() else { return };
                    let d = state.dictate_sc.lock().clone();
                    let c = state.cancel_sc.lock().clone();
                    let s = state.settings_sc.lock().clone();
                    let recv = Some(shortcut);
                    if recv == d.as_ref() {
                        do_toggle(app.clone(), state.inner().clone());
                    } else if recv == c.as_ref() {
                        do_cancel(app.clone(), state.inner().clone());
                    } else if recv == s.as_ref() {
                        do_show_settings(app.clone());
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            list_mics,
            list_models,
            download_model,
            get_settings,
            save_settings,
            apply_hotkey,
            apply_cancel_hotkey,
            apply_settings_hotkey,
            apply_autostart,
            get_history,
            delete_history_entry,
            clear_history,
            confirm_review,
            cancel_review,
            hide_main,
            show_main,
            toggle_dictate,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let audio = AudioController::spawn(handle.clone());
            let state = Arc::new(AppState {
                audio,
                is_recording: Mutex::new(false),
                target_window: Mutex::new(None),
                settings: Mutex::new(initial.clone()),
                pending_review: Mutex::new(None),
                dictate_sc: Mutex::new(None),
                cancel_sc: Mutex::new(None),
                settings_sc: Mutex::new(None),
            });
            app.manage(state.clone());

            for (label, combo, slot) in [
                ("dictate", &initial.hotkey, &state.dictate_sc),
                ("cancel", &initial.cancel_hotkey, &state.cancel_sc),
                ("settings", &initial.settings_hotkey, &state.settings_sc),
            ] {
                if let Err(e) = rebind(&handle, slot, combo) {
                    eprintln!("{label} hotkey register failed ({combo}): {e}");
                }
            }

            let first_run = !config::settings_path().exists();
            if first_run {
                if let Some(w) = handle.get_webview_window("main") {
                    let _ = w.show();
                }
                let _ = config::save_settings(&state.settings.lock());
            }

            let show_item = MenuItem::with_id(app, "show", "Settings", true, None::<&str>)?;
            let toggle_item = MenuItem::with_id(app, "toggle", "Toggle dictate", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &toggle_item, &quit_item])?;
            let state_for_tray = state.clone();
            let _tray = TrayIconBuilder::with_id("main")
                .icon(include_image!("icons/tray.png"))
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .tooltip("Dictate")
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "toggle" => do_toggle(app.clone(), state_for_tray.clone()),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            if let Some(main) = handle.get_webview_window("main") {
                let h2 = handle.clone();
                main.on_window_event(move |e| {
                    if let WindowEvent::CloseRequested { api, .. } = e {
                        api.prevent_close();
                        if let Some(w) = h2.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
