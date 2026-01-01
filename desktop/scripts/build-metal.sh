#!/bin/bash
# Build macOS installer with Metal (Apple Silicon) support
set -e

# Load Apple notarization credentials if available
if [ -f "$(dirname "$0")/../.env.local" ]; then
    source "$(dirname "$0")/../.env.local"
    echo "Loaded Apple credentials from .env.local"
fi

echo "=== Building Meeting-Local with Metal Support ==="
echo ""

# === PREREQUISITE CHECKS ===
echo "[Prerequisites] Checking required tools..."

missing=()

# Check Rust
if ! command -v cargo &> /dev/null; then
    missing+=("Rust (https://rustup.rs)")
fi

# Check pnpm
if ! command -v pnpm &> /dev/null; then
    missing+=("pnpm (npm install -g pnpm)")
fi

# Check Xcode CLI tools
if ! xcode-select -p &> /dev/null; then
    missing+=("Xcode Command Line Tools (xcode-select --install)")
fi

# Check macOS
if [[ "$(uname)" != "Darwin" ]]; then
    echo "ERROR: This script must be run on macOS"
    exit 1
fi

if [ ${#missing[@]} -gt 0 ]; then
    echo ""
    echo "ERROR: Missing prerequisites:"
    for item in "${missing[@]}"; do
        echo "  - $item"
    done
    echo ""
    echo "Install missing tools and try again."
    exit 1
fi

echo "  Rust: OK"
echo "  pnpm: OK"
echo "  Xcode CLI: OK"

if [[ "$(uname -m)" == "arm64" ]]; then
    echo "  Architecture: Apple Silicon"
else
    echo "  Architecture: Intel (Metal performance may be limited)"
fi

# Navigate to desktop directory
cd "$(dirname "$0")/.."

# Clean old sidecar binaries to ensure fresh build
echo ""
echo "[0/5] Cleaning old sidecar binaries..."
rm -f src-tauri/binaries/llm-sidecar-*

# === BUILD STEPS ===
echo ""
echo "[1/5] Building frontend..."
pnpm build

echo ""
echo "[2/5] Building LLM sidecar with Metal..."
cd src-tauri
# Clean only sidecar packages (not whisper-rs which has complex builds)
echo "  Cleaning sidecar cached builds..."
cargo clean -p llm-sidecar 2>/dev/null || true
cargo clean -p mistralrs 2>/dev/null || true
cargo clean -p mistralrs-core 2>/dev/null || true
cargo build --release -p llm-sidecar --no-default-features --features metal
cd ..

echo ""
echo "[3/5] Copying and signing LLM sidecar for notarization..."
# Copy sidecar to binaries folder and sign it before Tauri bundles it
TARGET_TRIPLE="aarch64-apple-darwin"
SIDECAR_SRC="src-tauri/target/release/llm-sidecar"
SIDECAR_DEST="src-tauri/binaries/llm-sidecar-${TARGET_TRIPLE}"
SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:-}"
ENTITLEMENTS="src-tauri/entitlements.plist"

mkdir -p src-tauri/binaries
if [[ -f "$SIDECAR_SRC" ]]; then
    echo "  Copying sidecar to binaries/"
    cp "$SIDECAR_SRC" "$SIDECAR_DEST"

    if [[ -n "$SIGNING_IDENTITY" ]]; then
        echo "  Signing: $SIDECAR_DEST"
        codesign --sign "$SIGNING_IDENTITY" \
                 --options runtime \
                 --entitlements "$ENTITLEMENTS" \
                 --timestamp \
                 --force \
                 "$SIDECAR_DEST"
        echo "  Verifying signature..."
        codesign --verify --verbose=2 "$SIDECAR_DEST"
        echo "  Sidecar signed successfully"
    else
        echo "  Skipping signing (APPLE_SIGNING_IDENTITY not set)"
    fi
else
    echo "  ERROR: Sidecar not found at $SIDECAR_SRC"
    exit 1
fi

echo ""
echo "[4/5] Building Tauri app with Metal..."
pnpm tauri build --config '{"build":{"beforeBuildCommand":""}}' -- --features metal

echo ""
echo "[5/5] Renaming installer..."
BUNDLE_DIR="src-tauri/target/release/bundle/dmg"
ORIGINAL_DMG=$(ls "$BUNDLE_DIR"/*.dmg 2>/dev/null | head -1)
if [[ -n "$ORIGINAL_DMG" ]]; then
    NEW_NAME=$(basename "$ORIGINAL_DMG" | sed 's/meeting-local/meeting-local-Metal/')
    NEW_PATH="$BUNDLE_DIR/$NEW_NAME"
    mv "$ORIGINAL_DMG" "$NEW_PATH"
    echo ""
    echo "=== Build Complete ==="
    echo "Output: $NEW_PATH"
else
    echo "WARNING: Could not find installer to rename"
fi
