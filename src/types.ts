export type Language = "auto" | "en" | "es";
export type PostProcess = "raw" | "cleanup";

export interface DictEntry {
  from: string;
  to: string;
}

export interface Settings {
  micDevice: string | null;
  modelPath: string | null;
  hotkey: string;
  cancelHotkey: string;
  settingsHotkey: string;
  language: Language;
  postProcess: PostProcess;
  autostart: boolean;
  dictionary: DictEntry[];
  reviewMode: boolean;
  confirmKey: string;
}

export interface MicDevice {
  name: string;
  is_default: boolean;
}

export interface HistoryEntry {
  id: number;
  text: string;
  timestampMs: number;
}

export interface ModelFile {
  name: string;
  path: string;
  size_mb: number;
}

export const DEFAULT_SETTINGS: Settings = {
  micDevice: null,
  modelPath: null,
  hotkey: "CmdOrCtrl+Shift+D",
  cancelHotkey: "CmdOrCtrl+Shift+X",
  settingsHotkey: "CmdOrCtrl+Shift+,",
  language: "auto",
  postProcess: "cleanup",
  autostart: false,
  dictionary: [],
  reviewMode: false,
  confirmKey: "Tab",
};
