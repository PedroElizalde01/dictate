import { invoke } from "@tauri-apps/api/core";
import type { MicDevice, ModelFile, Settings } from "./types";

export const api = {
  listMics: () => invoke<MicDevice[]>("list_mics"),
  listModels: () => invoke<ModelFile[]>("list_models"),
  downloadModel: (size: string) => invoke<string>("download_model", { size }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  applyHotkey: (combo: string) => invoke<void>("apply_hotkey", { combo }),
  applyCancelHotkey: (combo: string) => invoke<void>("apply_cancel_hotkey", { combo }),
  applySettingsHotkey: (combo: string) => invoke<void>("apply_settings_hotkey", { combo }),
  applyAutostart: (enabled: boolean) => invoke<void>("apply_autostart", { enabled }),
  hideMain: () => invoke<void>("hide_main"),
  toggleDictate: () => invoke<void>("toggle_dictate"),
};
