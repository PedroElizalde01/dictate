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

Local Whisper dictation for Linux X11 and Windows 10/11
</div>

---

Press a global hotkey, speak, and the transcription is typed at your cursor in the app you were using.
Runs entirely on your machine. No cloud, no API keys, no telemetry.

## Features

- Global hotkey toggle, default `Ctrl+Shift+D`
- Cancel hotkey, default `Ctrl+Shift+X`
- Open-settings hotkey, default `Ctrl+Shift+,`
- Animated overlay with live audio waveform
- Local Whisper via `whisper.cpp` with optional Vulkan build
- Model sizes: `tiny`, `base`, `small`, `medium`
- Multilingual: auto, English, Spanish
- Optional basic post-processing
- System tray and autostart on login
- Tauri 2, React, Rust

## Platform Status

| Platform | Status | Paste backend |
| --- | --- | --- |
| Windows 10/11 | Supported | Win32 `SendInput` |
| Linux X11 | Supported | `xdotool` |
| Linux Wayland | Pending | Not implemented |
| macOS | Pending | Not implemented |

## Windows Install

Open PowerShell and run:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

The installer checks for Git, Rust, Node.js, CMake, and Visual Studio Build Tools with the C++ workload. If `winget` is available, missing tools are installed automatically.

Optional Vulkan build:

```powershell
$env:VULKAN = "1"
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

Manual Windows build:

```powershell
git clone https://github.com/USERNAME/dictate.git
cd dictate
powershell -ExecutionPolicy Bypass -File .\scripts\build-whisper.ps1
npm.cmd install
npm.cmd run tauri dev
```

Production bundle:

```powershell
npm.cmd run tauri build
```

Windows installers are produced under `src-tauri\target\release\bundle`.

## Linux Install

One-line install:

```bash
curl -fsSL https://raw.githubusercontent.com/USERNAME/dictate/main/install.sh | bash
```

Replace `USERNAME` with the GitHub user or org hosting the repo. The script:

1. Detects your distro and installs system dependencies with `apt`, `dnf`, or `pacman`.
2. Installs `rustup` and Node.js via `nvm` if missing.
3. Clones the repo to `~/.local/share/dictate-src`.
4. Builds the `whisper.cpp` binary.
5. Builds the Tauri app and installs it.

Set `INSTALL_DIR` to override the source location:

```bash
INSTALL_DIR=$HOME/code/dictate curl -fsSL ... | bash
```

Manual Linux build:

```bash
sudo apt install -y xdotool build-essential cmake pkg-config git \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libasound2-dev libpulse-dev libssl-dev

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
nvm install --lts

git clone https://github.com/USERNAME/dictate.git
cd dictate
./scripts/build-whisper.sh
npm install
npm run tauri dev
```

## First Run

The settings window opens automatically. Pick a microphone, download a Whisper model, set your hotkeys, then click Hide. The app keeps running in the system tray.

## Usage

| Action | Default shortcut |
| --- | --- |
| Toggle dictation | `Ctrl+Shift+D` |
| Cancel without paste | `Ctrl+Shift+X` |
| Open settings | `Ctrl+Shift+,` |

Press the toggle hotkey, speak, then press it again. The transcription is typed wherever your cursor was when recording started. Use cancel to abort mid-recording without transcribing or pasting.

## Where Files Live

| Platform | Path | Purpose |
| --- | --- | --- |
| Windows | `%APPDATA%\dictate\settings.json` | User settings |
| Windows | `%LOCALAPPDATA%\dictate\models\` | Downloaded Whisper models |
| Linux | `~/.config/dictate/settings.json` | User settings |
| Linux | `~/.local/share/dictate/models/` | Downloaded Whisper models |
| Source checkout | `src-tauri/binaries/whisper-cli.exe` or `src-tauri/binaries/whisper-cli` | Compiled Whisper binary |

## Troubleshooting

- No paste on Windows: test in Notepad first. Dictate temporarily uses the clipboard and sends `Ctrl+V`; elevated apps and secure desktops can reject input from a non-elevated app.
- No paste on Linux: ensure X11. `echo $XDG_SESSION_TYPE` should print `x11`.
- `whisper-cli binary not found`: run `scripts\build-whisper.ps1` on Windows or `./scripts/build-whisper.sh` on Linux.
- Empty transcription: model may be too small for the language. Try `small` or `medium`.
- Visual Studio build errors on Windows: install Visual Studio 2022 Build Tools with the C++ workload, then rerun `scripts\build-whisper.ps1`.

## Roadmap

- Linux Wayland paste backend
- macOS paste backend
- GPU acceleration toggle in the UI
- More languages

## License

MIT
