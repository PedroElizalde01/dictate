<div align="center">
<pre>
 ██████████   █████   █████████  ███████████   █████████   ███████████ ██████████
░░███░░░░███ ░░███   ███░░░░░███░█░░░███░░░█  ███░░░░░███ ░█░░░███░░░█░░███░░░░░█
 ░███   ░░███ ░███  ███     ░░░ ░   ░███  ░  ░███    ░███ ░   ░███  ░  ░███  █ ░ 
 ░███    ░███ ░███ ░███             ░███     ░███████████     ░███     ░██████   
 ░███    ░███ ░███ ░███             ░███     ░███░░░░░███     ░███     ░███░░█   
 ░███    ███  ░███ ░░███     ███    ░███     ░███    ░███     ░███     ░███ ░   █
 ██████████   █████ ░░█████████     █████    █████   █████    █████    ██████████
░░░░░░░░░░   ░░░░░   ░░░░░░░░░     ░░░░░    ░░░░░   ░░░░░    ░░░░░    ░░░░░░░░░░ 
       </pre>
</div>

Local Whisper dictation for Linux. Press a global hotkey, speak, and the transcription is typed at your cursor — in any focused application.

Runs entirely on your machine. No cloud, no API keys, no telemetry.

## Features

- Global hotkey toggle (default `Ctrl+Shift+D`)
- Cancel hotkey to abort without transcribing (default `Ctrl+Shift+X`)
- Open-settings hotkey (default `Ctrl+Shift+,`)
- Animated overlay pill with live audio waveform
- Local Whisper via `whisper.cpp` (CPU, optional Vulkan/iGPU)
- Choose model size: `tiny`, `base`, `small`, `medium`
- Multilingual: Auto / English / Spanish
- Optional basic post-processing (capitalization, spacing, trailing punctuation)
- System tray + autostart on login
- Tauri 2 + React + Rust — small footprint, native feel

Linux X11 only for now (auto-paste uses `xdotool`). Mac/Windows planned.

## Install

One-line install:

```bash
curl -fsSL https://raw.githubusercontent.com/USERNAME/dictate/main/install.sh | bash
```

Replace `USERNAME` with the GitHub user/org hosting the repo. The script:

1. Detects your distro and installs system dependencies (`apt`, `dnf`, or `pacman`).
2. Installs `rustup` and Node.js (via `nvm`) if missing.
3. Clones the repo to `~/.local/share/dictate-src`.
4. Builds the `whisper.cpp` sidecar binary.
5. Builds the Tauri app and installs it.

Set `INSTALL_DIR` to override the source location:

```bash
INSTALL_DIR=$HOME/code/dictate curl -fsSL ... | bash
```

## Manual install

```bash
# 1. System deps (Ubuntu/Pop!_OS/Debian)
sudo apt install -y xdotool build-essential cmake pkg-config git \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libasound2-dev libpulse-dev libssl-dev

# 2. Toolchains (skip if already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
nvm install --lts

# 3. Clone
git clone https://github.com/USERNAME/dictate.git
cd dictate

# 4. Build whisper.cpp sidecar (CPU AVX2; or VULKAN=1 for iGPU)
./scripts/build-whisper.sh

# 5. JS deps + run
npm install
npm run tauri dev          # development
# or
npm run tauri build        # production bundle in src-tauri/target/release
```

## First run

The settings window opens automatically. Pick a microphone, click **Get small** (or another size) to download a Whisper model, set your hotkeys, then click **Hide**. The app keeps running in the system tray.

## Usage

| Action            | Default shortcut    |
|-------------------|---------------------|
| Toggle dictation  | `Ctrl+Shift+D`      |
| Cancel (no paste) | `Ctrl+Shift+X`      |
| Open settings     | `Ctrl+Shift+,`      |

Press the toggle hotkey, speak, press it again — the transcription is typed wherever your cursor is. Use the cancel hotkey to abort mid-recording without transcribing or pasting.

## Where files live

| Path                                 | Purpose                       |
|--------------------------------------|-------------------------------|
| `~/.config/dictate/settings.json`    | User settings                 |
| `~/.local/share/dictate/models/`     | Downloaded Whisper models     |
| `src-tauri/binaries/whisper-cli`     | Compiled whisper.cpp binary   |

## Troubleshooting

- **No paste**: ensure X11 (`echo $XDG_SESSION_TYPE` should print `x11`). Wayland support pending — swap `xdotool` for `wtype` in `src-tauri/src/paste.rs`.
- **Tray icon stuck**: GNOME caches AppIndicator icons. Toggle the extension:
  ```bash
  gnome-extensions list | grep -i indicator
  gnome-extensions disable <id>
  gnome-extensions enable <id>
  ```
- **`whisper-cli binary not found`**: run `./scripts/build-whisper.sh` from the repo root.
- **Empty transcription**: model may be too small for the language; try `small` or `medium`.

## Roadmap

- Wayland support (`wtype`/`ydotool`)
- macOS + Windows builds
- GPU acceleration toggle in UI
- More languages

## License

MIT
