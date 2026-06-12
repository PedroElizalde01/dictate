import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { DEFAULT_SETTINGS, Language, PostProcess, Settings, MicDevice, ModelFile, HistoryEntry, DictEntry } from "./types";

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
    clock: (
      <>
        <circle cx="12" cy="12" r="9" stroke={stroke} strokeWidth={sw} />
        <path d="M12 7v5l3 2" stroke={stroke} strokeWidth={sw} strokeLinecap="round" strokeLinejoin="round" />
      </>
    ),
    gear: (
      <>
        <circle cx="12" cy="12" r="3" stroke={stroke} strokeWidth={sw} />
        <path d="M12 2.5v3M12 18.5v3M21.5 12h-3M5.5 12h-3M18.7 5.3l-2.1 2.1M7.4 16.6l-2.1 2.1M18.7 18.7l-2.1-2.1M7.4 7.4L5.3 5.3" stroke={stroke} strokeWidth={sw} strokeLinecap="round" />
      </>
    ),
    copy: (
      <>
        <rect x="9" y="9" width="11" height="11" rx="2" stroke={stroke} strokeWidth={sw} />
        <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" stroke={stroke} strokeWidth={sw} />
      </>
    ),
    book: (
      <>
        <path d="M4 5a2 2 0 012-2h13v16H6a2 2 0 00-2 2V5z" stroke={stroke} strokeWidth={sw} strokeLinejoin="round" />
        <path d="M4 19a2 2 0 012-2h13M8 7h7" stroke={stroke} strokeWidth={sw} strokeLinecap="round" />
      </>
    ),
    trash: (
      <>
        <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13a1 1 0 001 1h8a1 1 0 001-1l1-13M9 7V4a1 1 0 011-1h4a1 1 0 011 1v3" stroke={stroke} strokeWidth={sw} strokeLinecap="round" strokeLinejoin="round" />
      </>
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

function formatStamp(ms: number): { date: string; time: string } {
  const d = new Date(ms);
  return {
    date: d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" }),
    time: d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" }),
  };
}

type View = "history" | "dictionary" | "settings";

export default function App() {
  const [view, setView] = useState<View>("settings");
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [mics, setMics] = useState<MicDevice[]>([]);
  const [models, setModels] = useState<ModelFile[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  type HotkeyKind = "dictate" | "cancel" | "settings";
  const [hotkeyEdit, setHotkeyEdit] = useState<HotkeyKind | null>(null);
  const [hotkeyBuffer, setHotkeyBuffer] = useState("");
  const [downloading, setDownloading] = useState<string | null>(null);
  const [confirmKeyEdit, setConfirmKeyEdit] = useState(false);
  const [toast, setToast] = useState<{ msg: string; kind: "info" | "error" } | null>(null);

  const refresh = async () => {
    setMics(await api.listMics());
    setModels(await api.listModels());
  };

  const refreshHistory = async () => {
    setHistory(await api.getHistory());
  };

  useEffect(() => {
    (async () => {
      setSettings(await api.getSettings());
      await refresh();
      await refreshHistory();
    })();
    const unErr = listen<string>("dictate-error", (e) => {
      flash(e.payload, "error");
    });
    const unHist = listen("history-updated", () => {
      refreshHistory();
    });
    const unSett = listen("settings-updated", async () => {
      setSettings(await api.getSettings());
    });
    return () => {
      unErr.then((f) => f());
      unHist.then((f) => f());
      unSett.then((f) => f());
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

  const copyEntry = async (text: string) => {
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        // webkit2gtk fallback when Clipboard API is unavailable
        const ta = document.createElement("textarea");
        ta.value = text;
        ta.style.position = "fixed";
        ta.style.opacity = "0";
        document.body.appendChild(ta);
        ta.select();
        document.execCommand("copy");
        ta.remove();
      }
      flash("Copied to clipboard");
    } catch (e) {
      flash(`Copy error: ${e}`, "error");
    }
  };

  const deleteEntry = async (id: number) => {
    await api.deleteHistoryEntry(id);
    await refreshHistory();
  };

  const clearAll = async () => {
    await api.clearHistory();
    await refreshHistory();
    flash("History cleared");
  };

  const setDict = (next: DictEntry[]) => update("dictionary", next);
  const addDictEntry = () => setDict([...settings.dictionary, { from: "", to: "" }]);
  const editDictEntry = (i: number, patch: Partial<DictEntry>) =>
    setDict(settings.dictionary.map((d, j) => (j === i ? { ...d, ...patch } : d)));
  const removeDictEntry = (i: number) =>
    setDict(settings.dictionary.filter((_, j) => j !== i));

  const dictionaryView = (
    <div className="content">
      <header className="page-head">
        <div className="row between">
          <h1>Dictionary</h1>
          <button className="primary" onClick={addDictEntry}>Add word</button>
        </div>
        <p>
          Fix words Whisper keeps mishearing. "Correct word" is also fed to the model as a
          vocabulary hint; leave "Heard as" empty to only hint without replacing.
        </p>
      </header>

      {settings.dictionary.length === 0 ? (
        <div className="history-empty">
          <Icon name="book" />
          <p>No entries yet. Example: heard as <span className="kbd">cloud</span> → correct word <span className="kbd">Claude</span>.</p>
        </div>
      ) : (
        <div className="dict-list">
          <div className="dict-row dict-head">
            <span>Heard as (optional)</span>
            <span>Correct word</span>
            <span />
          </div>
          {settings.dictionary.map((d, i) => (
            <div className="dict-row" key={i}>
              <input
                value={d.from}
                placeholder="cloud"
                onChange={(e) => editDictEntry(i, { from: e.target.value })}
              />
              <input
                value={d.to}
                placeholder="Claude"
                onChange={(e) => editDictEntry(i, { to: e.target.value })}
              />
              <button className="ghost icon-btn" title="Remove" onClick={() => removeDictEntry(i)}>
                <Icon name="trash" />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );

  const historyView = (
    <div className="content">
      <header className="page-head">
        <div className="row between">
          <h1>History</h1>
          {history.length > 0 && (
            <button className="ghost" onClick={clearAll}>Clear all</button>
          )}
        </div>
        <p>Everything you've dictated, newest first. Stored locally only.</p>
      </header>

      {history.length === 0 ? (
        <div className="history-empty">
          <Icon name="wave" />
          <p>No dictations yet. Press <span className="kbd">{parseHotkey(settings.hotkey).join(" + ")}</span> to start.</p>
        </div>
      ) : (
        <div className="history-list">
          {history.map((h) => {
            const { date, time } = formatStamp(h.timestampMs);
            return (
              <div key={h.id} className="history-item">
                <div className="history-head">
                  <span className="history-stamp">{date} · {time}</span>
                  <div className="history-actions">
                    <button className="ghost icon-btn" title="Copy" onClick={() => copyEntry(h.text)}>
                      <Icon name="copy" />
                    </button>
                    <button className="ghost icon-btn" title="Delete" onClick={() => deleteEntry(h.id)}>
                      <Icon name="trash" />
                    </button>
                  </div>
                </div>
                <div className="history-text">{h.text}</div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );

  const settingsView = (
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
              <h3>Review before paste</h3>
              <p>Show the transcript in the overlay for a quick edit before pasting. Corrections teach the dictionary automatically.</p>
            </div>
            <div className="section-body">
              <div className="row between">
                <div style={{ fontSize: 13 }}>Enable review mode</div>
                <div
                  className={`switch ${settings.reviewMode ? "on" : ""}`}
                  role="switch"
                  aria-checked={settings.reviewMode}
                  tabIndex={0}
                  onClick={() => update("reviewMode", !settings.reviewMode)}
                />
              </div>
              <div className="hotkey-row">
                <div className="hotkey-label">Confirm &amp; paste key</div>
                {confirmKeyEdit ? (
                  <div className="hotkey-edit">
                    <input
                      autoFocus
                      readOnly
                      placeholder="Press a key…"
                      value={settings.confirmKey}
                      onKeyDown={(e) => {
                        e.preventDefault();
                        if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
                        update("confirmKey", e.key);
                        setConfirmKeyEdit(false);
                      }}
                    />
                    <button onClick={() => setConfirmKeyEdit(false)}>Cancel</button>
                  </div>
                ) : (
                  <div className="hotkey-show">
                    <div className="keys">
                      <span className="kbd">{settings.confirmKey}</span>
                    </div>
                    <button onClick={() => setConfirmKeyEdit(true)}>Change</button>
                  </div>
                )}
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
  );

  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark"><BrandMark /></div>
          <div>
            <div className="brand-name">Dictate</div>
            <div className="brand-sub">Local Whisper</div>
          </div>
        </div>
        <nav className="nav">
          <div
            className={`nav-item ${view === "history" ? "active" : ""}`}
            onClick={() => setView("history")}
          >
            <Icon name="clock" />
            History
          </div>
          <div
            className={`nav-item ${view === "dictionary" ? "active" : ""}`}
            onClick={() => setView("dictionary")}
          >
            <Icon name="book" />
            Dictionary
          </div>
          <div
            className={`nav-item ${view === "settings" ? "active" : ""}`}
            onClick={() => setView("settings")}
          >
            <Icon name="gear" />
            Settings
          </div>
        </nav>
        <div className="sidebar-foot">
          <span className="dot" title="Ready · listening for hotkey" />
          Ready
        </div>
      </aside>

      <main className="main">
        <div className="topbar">
          <div className="crumbs">
            <strong>
              {view === "history" ? "History" : view === "dictionary" ? "Dictionary" : "Settings"}
            </strong>
          </div>
          <div className="topbar-right">
            <span className="kbd">{parseHotkey(settings.hotkey).join(" + ") || "—"}</span>
            <button className="ghost" onClick={() => api.hideMain()}>Hide</button>
          </div>
        </div>

        {view === "history" ? historyView : view === "dictionary" ? dictionaryView : settingsView}
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
