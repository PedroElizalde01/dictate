import { invoke } from "@tauri-apps/api/core";
import type { HistoryEntry, MicDevice, ModelFile, Settings } from "./types";

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
  getHistory: () => invoke<HistoryEntry[]>("get_history"),
  deleteHistoryEntry: (id: number) => invoke<void>("delete_history_entry", { id }),
  clearHistory: () => invoke<void>("clear_history"),
  hideMain: () => invoke<void>("hide_main"),
  toggleDictate: () => invoke<void>("toggle_dictate"),
};
