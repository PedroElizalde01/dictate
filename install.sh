#!/usr/bin/env bash
# Dictate one-line installer for Linux.
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/USERNAME/dictate/main/install.sh | bash
#
# Env vars:
#   REPO_URL    Override git remote (default: https://github.com/USERNAME/dictate.git)
#   INSTALL_DIR Override source checkout dir (default: $HOME/.local/share/dictate-src)
#   BRANCH      Branch to clone (default: main)
#   VULKAN      1 to build whisper.cpp with Vulkan/iGPU acceleration

set -euo pipefail

REPO_URL="${REPO_URL:-https://github.com/USERNAME/dictate.git}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/share/dictate-src}"
BRANCH="${BRANCH:-main}"
VULKAN="${VULKAN:-0}"

c_blue=$'\033[1;34m'; c_green=$'\033[1;32m'; c_yellow=$'\033[1;33m'; c_red=$'\033[1;31m'; c_dim=$'\033[2m'; c_off=$'\033[0m'
log() { echo "${c_blue}==>${c_off} $*"; }
ok()  { echo "${c_green}✓${c_off} $*"; }
warn(){ echo "${c_yellow}!${c_off} $*"; }
die() { echo "${c_red}✗${c_off} $*" >&2; exit 1; }

case "$(uname -s)" in
  Linux) ;;
  MINGW*|MSYS*|CYGWIN*)
    die "Use the Windows installer instead: powershell -ExecutionPolicy Bypass -File .\\install.ps1"
    ;;
  *)
    die "This installer is for Linux. Windows 10/11 users should run install.ps1."
    ;;
esac
[ "${XDG_SESSION_TYPE:-}" = "x11" ] || warn "Session is '${XDG_SESSION_TYPE:-unknown}'. Auto-paste needs X11; Wayland support is pending."

# --- Detect package manager ---
if   command -v apt-get >/dev/null 2>&1; then PM=apt
elif command -v dnf     >/dev/null 2>&1; then PM=dnf
elif command -v pacman  >/dev/null 2>&1; then PM=pacman
else die "Unsupported package manager. Install deps manually (see README)."
fi
log "Detected package manager: $PM"

# --- System deps ---
install_apt() {
  sudo apt-get update -y
  sudo apt-get install -y \
    xdotool build-essential cmake pkg-config git curl ca-certificates \
    libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
    librsvg2-dev libasound2-dev libpulse-dev libssl-dev
}
install_dnf() {
  sudo dnf install -y \
    xdotool gcc-c++ make cmake pkgconf-pkg-config git curl \
    webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel \
    librsvg2-devel alsa-lib-devel pulseaudio-libs-devel openssl-devel
}
install_pacman() {
  sudo pacman -Sy --needed --noconfirm \
    xdotool base-devel cmake pkgconf git curl \
    webkit2gtk-4.1 gtk3 libayatana-appindicator librsvg \
    alsa-lib libpulse openssl
}
log "Installing system dependencies (sudo required)…"
case "$PM" in
  apt)    install_apt ;;
  dnf)    install_dnf ;;
  pacman) install_pacman ;;
esac
ok "System deps installed."

# --- Rust ---
if ! command -v cargo >/dev/null 2>&1; then
  log "Installing Rust toolchain via rustup…"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
else
  ok "Rust already installed: $(rustc --version)"
fi

# --- Node ---
if ! command -v node >/dev/null 2>&1; then
  log "Installing Node.js via nvm…"
  export NVM_DIR="$HOME/.nvm"
  if [ ! -d "$NVM_DIR" ]; then
    curl -fsSL https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
  fi
  # shellcheck disable=SC1091
  . "$NVM_DIR/nvm.sh"
  nvm install --lts
  nvm use --lts
else
  ok "Node already installed: $(node --version)"
fi

# --- Source checkout ---
if [ -d "$INSTALL_DIR/.git" ]; then
  log "Updating existing checkout at $INSTALL_DIR"
  git -C "$INSTALL_DIR" fetch --depth 1 origin "$BRANCH"
  git -C "$INSTALL_DIR" checkout "$BRANCH"
  git -C "$INSTALL_DIR" reset --hard "origin/$BRANCH"
else
  log "Cloning $REPO_URL → $INSTALL_DIR"
  mkdir -p "$(dirname "$INSTALL_DIR")"
  git clone --depth 1 --branch "$BRANCH" "$REPO_URL" "$INSTALL_DIR"
fi
ok "Source ready."

cd "$INSTALL_DIR"

# --- whisper.cpp sidecar ---
log "Building whisper.cpp sidecar (this takes a few minutes)…"
VULKAN="$VULKAN" ./scripts/build-whisper.sh
ok "whisper-cli built."

# --- JS deps ---
log "Installing JS dependencies…"
npm install --no-audit --no-fund
ok "JS deps installed."

# --- Build app ---
log "Building Dictate (release)…"
npm run tauri build
ok "Build complete."

# --- Install bundle ---
DEB="$(find src-tauri/target/release/bundle/deb -maxdepth 2 -name 'dictate*.deb' 2>/dev/null | head -n1 || true)"
RPM="$(find src-tauri/target/release/bundle/rpm -maxdepth 2 -name 'dictate*.rpm' 2>/dev/null | head -n1 || true)"
APPIMG="$(find src-tauri/target/release/bundle/appimage -maxdepth 2 -name 'dictate*.AppImage' 2>/dev/null | head -n1 || true)"

if [ -n "$DEB" ] && [ "$PM" = "apt" ]; then
  log "Installing $DEB"
  sudo apt-get install -y "$DEB"
elif [ -n "$RPM" ] && [ "$PM" = "dnf" ]; then
  log "Installing $RPM"
  sudo dnf install -y "$RPM"
elif [ -n "$APPIMG" ]; then
  mkdir -p "$HOME/.local/bin"
  cp "$APPIMG" "$HOME/.local/bin/dictate"
  chmod +x "$HOME/.local/bin/dictate"
  ok "AppImage installed at ~/.local/bin/dictate"
else
  warn "No bundle produced; run from source: cd $INSTALL_DIR && npm run tauri dev"
fi

cat <<EOF

${c_green}Dictate installed.${c_off}

Next steps:
  1. Launch:        ${c_dim}dictate${c_off}   (or run from your app launcher)
  2. Pick a mic and download a Whisper model in the settings window.
  3. Press your hotkey to dictate. Default: ${c_yellow}Ctrl+Shift+D${c_off}

Source:    $INSTALL_DIR
Settings:  ~/.config/dictate/settings.json
Models:    ~/.local/share/dictate/models/
EOF
