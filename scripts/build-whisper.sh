#!/usr/bin/env bash
# Build whisper.cpp -> src-tauri/binaries/whisper-cli (CPU + AVX2).
# AMD Ryzen 7 -> use AVX2. iGPU Vulkan optional via VULKAN=1 env.

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="$ROOT/.whisper-build"
BIN_DIR="$ROOT/src-tauri/binaries"
VULKAN="${VULKAN:-0}"

mkdir -p "$BIN_DIR"

if [ ! -d "$BUILD_DIR/whisper.cpp" ]; then
  mkdir -p "$BUILD_DIR"
  git clone --depth 1 https://github.com/ggerganov/whisper.cpp "$BUILD_DIR/whisper.cpp"
fi

cd "$BUILD_DIR/whisper.cpp"
git pull --ff-only || true

CMAKE_FLAGS=(-DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_EXAMPLES=ON)
if [ "$VULKAN" = "1" ]; then
  CMAKE_FLAGS+=(-DGGML_VULKAN=ON)
fi

cmake -B build "${CMAKE_FLAGS[@]}"
cmake --build build -j --config Release

# Find the produced binary (older versions: 'main', newer: 'whisper-cli')
SRC=""
for cand in build/bin/whisper-cli build/bin/main build/whisper-cli; do
  if [ -x "$cand" ]; then SRC="$cand"; break; fi
done
if [ -z "$SRC" ]; then
  echo "whisper-cli binary not found after build" >&2
  exit 1
fi

cp "$SRC" "$BIN_DIR/whisper-cli"
chmod +x "$BIN_DIR/whisper-cli"
echo "Installed: $BIN_DIR/whisper-cli"

# Tauri sidecar naming: whisper-cli-<target-triple>
TARGET_TRIPLE="$(rustc -vV | sed -n 's|host: ||p')"
cp "$BIN_DIR/whisper-cli" "$BIN_DIR/whisper-cli-${TARGET_TRIPLE}"
echo "Sidecar: $BIN_DIR/whisper-cli-${TARGET_TRIPLE}"
