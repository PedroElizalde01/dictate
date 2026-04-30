mod audio;
mod cleanup;
mod config;
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

struct AppState {
    audio: AudioController,
    is_recording: Mutex<bool>,
    target_window: Mutex<Option<String>>,
    settings: Mutex<Settings>,
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
            let result = finish_dictation(&app2, &state2);
            if let Err(e) = result {
                let msg = format!("{e}");
                eprintln!("dictation error: {msg}");
                let _ = app2.emit("dictate-error", msg);
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


fn finish_dictation(app: &AppHandle, state: &Arc<AppState>) -> anyhow::Result<()> {
    let settings = state.settings.lock().clone();
    let model_path = settings
        .model_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("no model selected"))?;
    let bin = models::whisper_binary_path(app);
    if !bin.exists() {
        return Err(anyhow::anyhow!(
            "whisper-cli binary not found at {}. Run ./scripts/build-whisper.sh",
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

    let text = whisper::transcribe(
        &bin,
        std::path::Path::new(model_path),
        &tmp,
        &settings.language,
    )?;
    let _ = std::fs::remove_file(&tmp);
    eprintln!("transcribed ({dur_s:.2}s, {} chars): {text:?}", text.len());

    let final_text = if settings.post_process == "cleanup" {
        cleanup::basic_cleanup(&text)
    } else {
        text
    };
    if final_text.is_empty() {
        let _ = app.emit("dictate-error", "empty transcription");
        return Ok(());
    }
    let target = state.target_window.lock().clone();
    let _ = app.emit("dictate-phase", PhasePayload { phase: "exit" });
    std::thread::sleep(std::time::Duration::from_millis(220));
    hide_overlay(app);
    std::thread::sleep(std::time::Duration::from_millis(60));
    eprintln!("paste -> target {target:?}, {} chars", final_text.len());
    paste::type_text(&final_text, target.as_deref())?;
    Ok(())
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

fn show_overlay(app: &AppHandle) {
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = h.get_webview_window("overlay") {
            if let Some(monitor) = w.current_monitor().ok().flatten() {
                let size = monitor.size();
                let scale = monitor.scale_factor();
                let win_w = (220.0 * scale) as i32;
                let win_h = (60.0 * scale) as i32;
                let x = (size.width as i32 - win_w) / 2;
                let y = size.height as i32 - win_h - (40.0 * scale) as i32;
                let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
            }
            let _ = w.show();
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
