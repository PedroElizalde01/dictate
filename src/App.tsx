import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { DEFAULT_SETTINGS, Language, PostProcess, Settings, MicDevice, ModelFile } from "./types";

const MODEL_SIZES: { id: string; size: string }[] = [
  { id: "tiny", size: "75 MB" },
  { id: "base", size: "142 MB" },
  { id: "small", size: "466 MB" },
  { id: "medium", size: "1.5 GB" },
];

const LangOptions: { v: Language; label: string }[] = [
  { v: "auto", label: "Auto" },
  { v: "en", label: "English" },
  { v: "es", label: "Español" },
];

const PostOptions: { v: PostProcess; label: string }[] = [
  { v: "raw", label: "Raw" },
  { v: "cleanup", label: "Clean" },
];

function Icon({ name }: { name: string }) {
  const stroke = "currentColor";
  const sw = 1.6;
  const map: Record<string, JSX.Element> = {
    mic: (
      <>
        <rect x="9" y="2" width="6" height="12" rx="3" stroke={stroke} strokeWidth={sw} />
        <path d="M5 10v1a7 7 0 0014 0v-1M12 18v3" stroke={stroke} strokeWidth={sw} strokeLinecap="round" />
      </>
    ),
    cpu: (
      <>
        <rect x="5" y="5" width="14" height="14" rx="2" stroke={stroke} strokeWidth={sw} />
        <rect x="9" y="9" width="6" height="6" rx="1" stroke={stroke} strokeWidth={sw} />
        <path d="M9 2v3M15 2v3M9 19v3M15 19v3M2 9h3M2 15h3M19 9h3M19 15h3" stroke={stroke} strokeWidth={sw} strokeLinecap="round" />
      </>
    ),
    sparkle: (
      <path d="M12 3l1.7 4.6L18 9l-4.3 1.4L12 15l-1.7-4.6L6 9l4.3-1.4L12 3z" stroke={stroke} strokeWidth={sw} strokeLinejoin="round" />
    ),
    power: (
      <>
        <path d="M12 3v9" stroke={stroke} strokeWidth={sw} strokeLinecap="round" />
        <path d="M5.5 8a8 8 0 1013 0" stroke={stroke} strokeWidth={sw} strokeLinecap="round" />
      </>
    ),
    wave: (
      <path d="M3 12h2l2-7 3 14 3-10 3 6 2-3h3" stroke={stroke} strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round" fill="none" />
    ),
  };
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" aria-hidden>
      {map[name]}
    </svg>
  );
}

function BrandMark() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden>
      <path d="M4 12h2M8 6v12M12 3v18M16 7v10M20 11v2" stroke="white" strokeWidth="2" strokeLinecap="round" />
    </svg>
  );
}

function parseHotkey(combo: string): string[] {
  return combo.split("+").map((s) => s.trim()).filter(Boolean);
}

function HotkeyRow(props: {
  label: string;
  value: string;
  editing: boolean;
  buffer: string;
  onEdit: () => void;
  onCancel: () => void;
  onSave: () => void;
  onKeyDown: (e: React.KeyboardEvent) => void;
}) {
  const { label, value, editing, buffer, onEdit, onCancel, onSave, onKeyDown } = props;
  return (
    <div className="hotkey-row">
      <div className="hotkey-label">{label}</div>
      {editing ? (
        <div className="hotkey-edit">
          <input
            autoFocus
            readOnly
            placeholder="Press combo…"
            value={buffer}
            onKeyDown={onKeyDown}
          />
          <button className="primary" onClick={onSave} disabled={!buffer}>Save</button>
          <button onClick={onCancel}>Cancel</button>
        </div>
      ) : (
        <div className="hotkey-show">
          <div className="keys">
            {parseHotkey(value).map((k, i, arr) => (
              <span key={i} className="row" style={{ gap: 4 }}>
                <span className="kbd">{k}</span>
                {i < arr.length - 1 && <span style={{ color: "var(--text-subtle)" }}>+</span>}
              </span>
            ))}
          </div>
          <button onClick={onEdit}>Change</button>
        </div>
      )}
    </div>
  );
}

export default function App() {
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [mics, setMics] = useState<MicDevice[]>([]);
  const [models, setModels] = useState<ModelFile[]>([]);
  type HotkeyKind = "dictate" | "cancel" | "settings";
  const [hotkeyEdit, setHotkeyEdit] = useState<HotkeyKind | null>(null);
  const [hotkeyBuffer, setHotkeyBuffer] = useState("");
  const [downloading, setDownloading] = useState<string | null>(null);
  const [toast, setToast] = useState<{ msg: string; kind: "info" | "error" } | null>(null);

  const refresh = async () => {
    setMics(await api.listMics());
    setModels(await api.listModels());
  };

  useEffect(() => {
    (async () => {
      setSettings(await api.getSettings());
      await refresh();
    })();
    const un = listen<string>("dictate-error", (e) => {
      flash(e.payload, "error");
    });
    return () => {
      un.then((f) => f());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const flash = (msg: string, kind: "info" | "error" = "info") => {
    setToast({ msg, kind });
    setTimeout(() => setToast(null), 3200);
  };

  const update = async <K extends keyof Settings>(key: K, value: Settings[K]) => {
    const next = { ...settings, [key]: value };
    setSettings(next);
    await api.saveSettings(next);
  };

  const onHotkeyKeyDown = (e: React.KeyboardEvent) => {
    e.preventDefault();
    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.shiftKey) parts.push("Shift");
    if (e.altKey) parts.push("Alt");
    if (e.metaKey) parts.push("Super");
    const k = e.key;
    if (k.length === 1 || (k.length > 1 && !["Control", "Shift", "Alt", "Meta"].includes(k))) {
      parts.push(k.length === 1 ? k.toUpperCase() : k);
      setHotkeyBuffer(parts.join("+"));
    }
  };

  const saveHotkey = async (kind: HotkeyKind) => {
    if (!hotkeyBuffer) return;
    try {
      if (kind === "dictate") {
        await api.applyHotkey(hotkeyBuffer);
        await update("hotkey", hotkeyBuffer);
      } else if (kind === "cancel") {
        await api.applyCancelHotkey(hotkeyBuffer);
        await update("cancelHotkey", hotkeyBuffer);
      } else {
        await api.applySettingsHotkey(hotkeyBuffer);
        await update("settingsHotkey", hotkeyBuffer);
      }
      flash(`${kind} hotkey set to ${hotkeyBuffer}`);
    } catch (e) {
      flash(`Hotkey error: ${e}`, "error");
    }
    setHotkeyEdit(null);
    setHotkeyBuffer("");
  };

  const downloadModel = async (size: string) => {
    setDownloading(size);
    flash(`Downloading ${size} model…`);
    try {
      const path = await api.downloadModel(size);
      flash(`Downloaded ${size}`);
      await refresh();
      if (!settings.modelPath) await update("modelPath", path);
    } catch (e) {
      flash(`Download error: ${e}`, "error");
    }
    setDownloading(null);
  };

  const installedSet = new Set(models.map((m) => m.name.replace(/^ggml-|\.bin$/g, "")));

  return (
    <div className="shell single">
      <main className="main">
        <div className="topbar">
          <div className="brand inline">
            <div className="brand-mark"><BrandMark /></div>
            <div className="brand-name">Dictate</div>
            <span className="dot" title="Ready · listening for hotkey" />
          </div>
          <div className="topbar-right">
            <span className="kbd">{parseHotkey(settings.hotkey).join(" + ") || "—"}</span>
            <button className="ghost" onClick={() => api.hideMain()}>Hide</button>
          </div>
        </div>

        <div className="content">
          <header className="page-head">
            <h1>Dictation settings</h1>
            <p>Local Whisper transcription. Press your hotkey to toggle recording — text is typed at the cursor.</p>
          </header>

          <section className="section">
            <div className="section-meta">
              <h3>Microphone</h3>
              <p>Input device used to capture audio. cpal enumerates host devices.</p>
            </div>
            <div className="section-body">
              <label className="field">
                <span>Device</span>
                <select
                  value={settings.micDevice ?? ""}
                  onChange={(e) => update("micDevice", e.target.value || null)}
                >
                  <option value="">System default</option>
                  {mics.map((m) => (
                    <option key={m.name} value={m.name}>
                      {m.name}{m.is_default ? " — default" : ""}
                    </option>
                  ))}
                </select>
              </label>
            </div>
          </section>

          <section className="section">
            <div className="section-meta">
              <h3>Whisper model</h3>
              <p>Stored in <code>~/.local/share/dictate/models</code>. Larger models are slower but more accurate.</p>
            </div>
            <div className="section-body">
              <label className="field">
                <span>Active model</span>
                <select
                  value={settings.modelPath ?? ""}
                  onChange={(e) => update("modelPath", e.target.value || null)}
                >
                  <option value="">— none selected —</option>
                  {models.map((m) => (
                    <option key={m.path} value={m.path}>
                      {m.name} · {m.size_mb} MB
                    </option>
                  ))}
                </select>
              </label>
              <div className="row" style={{ gap: 6, flexWrap: "wrap" }}>
                {MODEL_SIZES.map((m) => {
                  const installed = installedSet.has(m.id);
                  const isDl = downloading === m.id;
                  return (
                    <button
                      key={m.id}
                      className="chip"
                      disabled={downloading !== null || installed}
                      onClick={() => downloadModel(m.id)}
                      title={installed ? "Already installed" : `Download ${m.id} (${m.size})`}
                    >
                      {isDl && <span className="pulse" />}
                      <span style={{ textTransform: "capitalize" }}>{m.id}</span>
                      <span style={{ color: "var(--text-subtle)" }}>{installed ? "installed" : m.size}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          </section>

          <section className="section">
            <div className="section-meta">
              <h3>Language</h3>
              <p>Auto-detect or constrain to a single language for better accuracy.</p>
            </div>
            <div className="section-body">
              <div className="segmented" role="tablist">
                {LangOptions.map((o) => (
                  <button
                    key={o.v}
                    role="tab"
                    className={settings.language === o.v ? "active" : ""}
                    onClick={() => update("language", o.v)}
                  >
                    {o.label}
                  </button>
                ))}
              </div>
            </div>
          </section>

          <section className="section">
            <div className="section-meta">
              <h3>Post-processing</h3>
              <p>Raw passes Whisper output unchanged. Clean capitalizes sentences and normalizes spacing.</p>
            </div>
            <div className="section-body">
              <div className="segmented" role="tablist">
                {PostOptions.map((o) => (
                  <button
                    key={o.v}
                    role="tab"
                    className={settings.postProcess === o.v ? "active" : ""}
                    onClick={() => update("postProcess", o.v)}
                  >
                    {o.label}
                  </button>
                ))}
              </div>
            </div>
          </section>

          <section className="section">
            <div className="section-meta">
              <h3>Global shortcuts</h3>
              <p>Triggered system-wide. Cancel stops dictation immediately without transcribing or pasting.</p>
            </div>
            <div className="section-body">
              <HotkeyRow
                label="Toggle dictation"
                value={settings.hotkey}
                editing={hotkeyEdit === "dictate"}
                buffer={hotkeyBuffer}
                onEdit={() => { setHotkeyEdit("dictate"); setHotkeyBuffer(""); }}
                onCancel={() => { setHotkeyEdit(null); setHotkeyBuffer(""); }}
                onKeyDown={onHotkeyKeyDown}
                onSave={() => saveHotkey("dictate")}
              />
              <HotkeyRow
                label="Cancel dictation"
                value={settings.cancelHotkey}
                editing={hotkeyEdit === "cancel"}
                buffer={hotkeyBuffer}
                onEdit={() => { setHotkeyEdit("cancel"); setHotkeyBuffer(""); }}
                onCancel={() => { setHotkeyEdit(null); setHotkeyBuffer(""); }}
                onKeyDown={onHotkeyKeyDown}
                onSave={() => saveHotkey("cancel")}
              />
              <HotkeyRow
                label="Open settings"
                value={settings.settingsHotkey}
                editing={hotkeyEdit === "settings"}
                buffer={hotkeyBuffer}
                onEdit={() => { setHotkeyEdit("settings"); setHotkeyBuffer(""); }}
                onCancel={() => { setHotkeyEdit(null); setHotkeyBuffer(""); }}
                onKeyDown={onHotkeyKeyDown}
                onSave={() => saveHotkey("settings")}
              />
            </div>
          </section>

          <section className="section">
            <div className="section-meta">
              <h3>System</h3>
              <p>Launch on login so your hotkey is always available.</p>
            </div>
            <div className="section-body">
              <div className="row between">
                <div className="row" style={{ gap: 10 }}>
                  <Icon name="power" />
                  <div>
                    <div style={{ fontSize: 13 }}>Start at login</div>
                    <div style={{ fontSize: 11.5, color: "var(--text-subtle)" }}>
                      Adds a desktop entry to autostart.
                    </div>
                  </div>
                </div>
                <div
                  className={`switch ${settings.autostart ? "on" : ""}`}
                  role="switch"
                  aria-checked={settings.autostart}
                  tabIndex={0}
                  onClick={async () => {
                    const next = !settings.autostart;
                    try {
                      await api.applyAutostart(next);
                      await update("autostart", next);
                    } catch (e) {
                      flash(`Autostart error: ${e}`, "error");
                    }
                  }}
                />
              </div>
            </div>
          </section>
        </div>
      </main>

      {toast && (
        <div className={`toast ${toast.kind === "error" ? "error" : ""}`}>
          <span className="icon-dot" />
          <span>{toast.msg}</span>
        </div>
      )}
    </div>
  );
}
