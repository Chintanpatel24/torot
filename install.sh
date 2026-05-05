#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
#  Torot v3 — Universal Security Agent
#  One-liner installer: macOS, Linux, Windows (WSL/Git Bash)
#
#  curl -fsSL https://raw.githubusercontent.com/Chintanpatel24/torot/main/install.sh | bash
#  OR:  chmod +x install.sh && ./install.sh
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

TOROT_VERSION="3.0.0"
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

info()   { echo -e "${CYAN}[torot]${RESET} $*"; }
ok()     { echo -e "${GREEN}[  ok ]${RESET} $*"; }
warn()   { echo -e "${YELLOW}[ warn]${RESET} $*"; }
err()    { echo -e "${RED}[error]${RESET} $*" >&2; exit 1; }
header() { echo -e "\n${BOLD}${CYAN}━━ $* ━━${RESET}"; }

# ── Detect where this script lives (the project root) ─────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
TOROT_DIR="$SCRIPT_DIR"

# ── Banner ────────────────────────────────────────────────────────────────────
echo -e "${CYAN}"
cat << 'BANNER'
 ████████╗ ██████╗ ██████╗  ██████╗ ████████╗
    ██╔══╝██╔═══██╗██╔══██╗██╔═══██╗╚══██╔══╝
    ██║   ██║   ██║██████╔╝██║   ██║   ██║
    ██║   ██║   ██║██╔══██╗██║   ██║   ██║
    ██║   ╚██████╔╝██║  ██║╚██████╔╝   ██║
    ╚═╝    ╚═════╝ ╚═╝  ╚═╝ ╚═════╝    ╚═╝
BANNER
echo -e "${RESET}"
echo -e "  ${BOLD}Universal Security Agent  |  v${TOROT_VERSION}${RESET}"
echo -e "  ${CYAN}Project directory: ${TOROT_DIR}${RESET}"
echo ""

# ── Validate project structure ────────────────────────────────────────────────
header "Validating project"
[[ -f "$TOROT_DIR/package.json" ]]                  || err "package.json not found in $TOROT_DIR"
[[ -f "$TOROT_DIR/src-tauri/tauri.conf.json" ]]     || err "src-tauri/tauri.conf.json not found"
[[ -f "$TOROT_DIR/src-tauri/Cargo.toml" ]]          || err "src-tauri/Cargo.toml not found"
ok "Project structure valid"

# ── Detect OS / Arch ──────────────────────────────────────────────────────────
OS="unknown"
case "$(uname -s 2>/dev/null || echo Unknown)" in
  Darwin*)               OS="macos" ;;
  Linux*)                OS="linux" ;;
  MINGW*|CYGWIN*|MSYS*) OS="windows" ;;
esac
[[ "$OS" == "unknown" ]] && err "Unsupported OS — use macOS, Linux, or Windows WSL"
info "OS: $OS / $(uname -m 2>/dev/null || echo unknown)"

# ── Node.js 18+ ───────────────────────────────────────────────────────────────
header "Node.js"
if ! command -v node &>/dev/null; then
  warn "Node.js not found. Installing..."
  if [[ "$OS" == "macos" ]] && command -v brew &>/dev/null; then
    brew install node &>/dev/null && ok "Node.js installed via brew"
  elif [[ "$OS" == "linux" ]]; then
    curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash - &>/dev/null 2>&1 || true
    sudo apt-get install -y nodejs &>/dev/null 2>&1 && ok "Node.js installed"
  else
    err "Install Node.js 18+ from https://nodejs.org then re-run install.sh"
  fi
fi
NODE_MAJOR=$(node -e "process.stdout.write(String(parseInt(process.versions.node)))" 2>/dev/null || echo "0")
[[ "$NODE_MAJOR" -ge 18 ]] || err "Node.js 18+ required. Found: $(node --version). Upgrade then retry."
ok "Node.js $(node --version)"

# ── Rust / Cargo ──────────────────────────────────────────────────────────────
header "Rust"
if ! command -v cargo &>/dev/null; then
  info "Installing Rust via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
  # Source cargo env for this shell session
  . "$HOME/.cargo/env" 2>/dev/null || export PATH="$HOME/.cargo/bin:$PATH"
  ok "Rust installed"
else
  # Make sure cargo is on PATH even if installed in ~/.cargo/bin
  export PATH="$HOME/.cargo/bin:$PATH"
  ok "Rust $(rustc --version 2>/dev/null | cut -d' ' -f2)"
fi

# Verify cargo is accessible
command -v cargo &>/dev/null || err "cargo not found after Rust install. Open a new terminal and re-run install.sh."

# ── System deps (Linux only) ──────────────────────────────────────────────────
if [[ "$OS" == "linux" ]]; then
  header "Linux system deps"
  info "Installing Tauri prerequisites..."
  sudo apt-get update -qq 2>/dev/null || true
  sudo apt-get install -y -qq \
    libwebkit2gtk-4.1-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    patchelf \
    2>/dev/null && ok "Linux deps installed" || warn "Some deps may have failed — continuing"
fi

# ── npm install ───────────────────────────────────────────────────────────────
header "npm install"
cd "$TOROT_DIR"
info "Installing JavaScript dependencies..."
npm install --silent 2>&1 | tail -3 || err "npm install failed"
ok "npm dependencies ready"

# ── Build frontend (vite) ─────────────────────────────────────────────────────
header "Frontend build"
info "Building React frontend..."
npm run build 2>&1 | tail -5 || err "Frontend build failed — check TypeScript errors above"
ok "Frontend built (dist/)"

# ── Build Tauri app ───────────────────────────────────────────────────────────
header "Tauri build"
info "Compiling Rust backend + bundling desktop app..."
info "This takes 5-15 minutes on first build (subsequent builds are fast)."
echo ""

# Run tauri build from the project root (where package.json lives)
# tauri CLI v2 finds src-tauri/ automatically
cd "$TOROT_DIR"
npx tauri build 2>&1 | grep -E "(Compiling|Finished|error|warning:|Bundling|Built)" | head -40 || {
  echo ""
  err "Tauri build failed. Run: cd $TOROT_DIR && npx tauri build  for full output."
}

# ── Find and install binary ───────────────────────────────────────────────────
header "Installing binary"
BINARY_NAME="torot"
[[ "$OS" == "windows" ]] && BINARY_NAME="torot.exe"

BUILT_BIN=$(find "$TOROT_DIR/src-tauri/target/release" -maxdepth 1 -name "$BINARY_NAME" 2>/dev/null | head -1)

if [[ -z "$BUILT_BIN" ]]; then
  warn "Binary not found at expected path."
  warn "Try running manually: cd $TOROT_DIR && npx tauri dev"
else
  ok "Binary built: $BUILT_BIN"
  INSTALL_BIN="$HOME/.local/bin"
  mkdir -p "$INSTALL_BIN"
  cp "$BUILT_BIN" "$INSTALL_BIN/torot"
  chmod +x "$INSTALL_BIN/torot"
  ok "Installed to $INSTALL_BIN/torot"

  # Add to PATH if not already
  if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    for RC in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile"; do
      [[ -f "$RC" ]] && echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$RC" && break
    done
    export PATH="$HOME/.local/bin:$PATH"
    warn "Added ~/.local/bin to PATH. Restart your shell or run: export PATH=\"\$HOME/.local/bin:\$PATH\""
  fi

  # macOS: also check for .app bundle
  if [[ "$OS" == "macos" ]]; then
    APP_BUNDLE=$(find "$TOROT_DIR/src-tauri/target/release/bundle/macos" -name "*.app" 2>/dev/null | head -1)
    if [[ -n "$APP_BUNDLE" ]]; then
      info "macOS app bundle: $APP_BUNDLE"
      info "To install to Applications: cp -r \"$APP_BUNDLE\" /Applications/"
    fi
  fi
fi

# ── Security tools ────────────────────────────────────────────────────────────
header "Security tools"
info "Installing available security tools (skipping failures)..."

INSTALLED_TOOLS=()
SKIPPED_TOOLS=()

try_pip() {
  local name="$1" pkg="$2" bin="$3"
  command -v "$bin" &>/dev/null && { INSTALLED_TOOLS+=("$name"); return 0; }
  pip3 install "$pkg" -q 2>/dev/null && INSTALLED_TOOLS+=("$name") || SKIPPED_TOOLS+=("$name")
}
try_npm_tool() {
  local name="$1" pkg="$2" bin="$3"
  command -v "$bin" &>/dev/null && { INSTALLED_TOOLS+=("$name"); return 0; }
  command -v npm &>/dev/null && npm install -g "$pkg" -q 2>/dev/null && INSTALLED_TOOLS+=("$name") || SKIPPED_TOOLS+=("$name")
}
try_cargo_tool() {
  local name="$1" pkg="$2" bin="$3"
  command -v "$bin" &>/dev/null && { INSTALLED_TOOLS+=("$name"); return 0; }
  command -v cargo &>/dev/null && cargo install "$pkg" -q 2>/dev/null && INSTALLED_TOOLS+=("$name") || SKIPPED_TOOLS+=("$name")
}
try_go_tool() {
  local name="$1" pkg="$2" bin="$3"
  command -v "$bin" &>/dev/null && { INSTALLED_TOOLS+=("$name"); return 0; }
  command -v go &>/dev/null && go install "$pkg" 2>/dev/null && INSTALLED_TOOLS+=("$name") || SKIPPED_TOOLS+=("$name")
}

# Python tools
try_pip "slither"    "slither-analyzer"    "slither"
try_pip "mythril"    "mythril"             "myth"
try_pip "halmos"     "halmos"              "halmos"
try_pip "semgrep"    "semgrep"             "semgrep"
try_pip "sqlmap"     "sqlmap"              "sqlmap"
try_pip "arjun"      "arjun"               "arjun"
try_pip "binwalk"    "binwalk"             "binwalk"
try_pip "checksec"   "checksec"            "checksec"
try_pip "eth-wake"   "eth-wake"            "wake"
try_pip "manticore"  "manticore"           "manticore"

# npm tools
try_npm_tool "solhint"    "solhint"           "solhint"
try_npm_tool "smartcheck" "smartcheck"        "smartcheck"

# Cargo tools
try_cargo_tool "aderyn"      "aderyn"        "aderyn"
try_cargo_tool "cargo-audit" "cargo-audit"   "cargo-audit"
try_cargo_tool "heimdall"    "heimdall-rs"   "heimdall"

# Go tools
try_go_tool "nuclei"     "github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest"   "nuclei"
try_go_tool "ffuf"       "github.com/ffuf/ffuf/v2@latest"                            "ffuf"
try_go_tool "gobuster"   "github.com/OJ/gobuster/v3@latest"                          "gobuster"
try_go_tool "dalfox"     "github.com/hahwul/dalfox/v2@latest"                        "dalfox"
try_go_tool "gitleaks"   "github.com/zricethezav/gitleaks/v8@latest"                 "gitleaks"
try_go_tool "trufflehog" "github.com/trufflesecurity/trufflehog/v3@latest"           "trufflehog"

# System tools (apt/brew)
if [[ "$OS" == "linux" ]] && command -v apt-get &>/dev/null; then
  for pkg in nikto radare2 ltrace strace; do
    command -v "$pkg" &>/dev/null && { INSTALLED_TOOLS+=("$pkg"); continue; }
    sudo apt-get install -y -qq "$pkg" &>/dev/null 2>&1 && INSTALLED_TOOLS+=("$pkg") || SKIPPED_TOOLS+=("$pkg")
  done
elif [[ "$OS" == "macos" ]] && command -v brew &>/dev/null; then
  for pkg in nikto radare2; do
    command -v "$pkg" &>/dev/null && { INSTALLED_TOOLS+=("$pkg"); continue; }
    brew install "$pkg" &>/dev/null 2>&1 && INSTALLED_TOOLS+=("$pkg") || SKIPPED_TOOLS+=("$pkg")
  done
fi

# ── Final summary ─────────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}${GREEN}━━ Installation Complete ━━${RESET}"
echo ""
echo -e "  ${GREEN}Security tools installed (${#INSTALLED_TOOLS[@]}):${RESET}"
[[ ${#INSTALLED_TOOLS[@]} -gt 0 ]] && echo "    ${INSTALLED_TOOLS[*]}" || echo "    none"
echo ""
[[ ${#SKIPPED_TOOLS[@]} -gt 0 ]] && echo -e "  ${YELLOW}Skipped (${#SKIPPED_TOOLS[@]}):${RESET}\n    ${SKIPPED_TOOLS[*]}\n"
echo -e "  ${BOLD}Launch Torot:${RESET}"
if command -v torot &>/dev/null 2>&1; then
  echo "    torot"
else
  echo "    cd $TOROT_DIR && npx tauri dev      # dev mode"
  echo "    or: $TOROT_DIR/src-tauri/target/release/torot"
fi
echo ""
echo -e "  ${BOLD}Update later:${RESET}"
echo "    cd $TOROT_DIR && ./update.sh"
echo ""
