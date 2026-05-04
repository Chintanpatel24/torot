#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
#  Torot v3 — Universal Security Agent
#  One-liner installer for macOS, Linux, Windows (WSL/Git Bash)
#
#  Usage (curl):
#    curl -fsSL https://raw.githubusercontent.com/Chintanpatel24/torot/main/install.sh | bash
#
#  Or local:
#    chmod +x install.sh && ./install.sh
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail
IFS=$'\n\t'

TOROT_VERSION="3.0.0"
TOROT_REPO="https://github.com/Chintanpatel24/torot"
TOROT_DIR="$HOME/.torot-app"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

info()    { echo -e "${CYAN}[torot]${RESET} $*"; }
ok()      { echo -e "${GREEN}[  ok ]${RESET} $*"; }
warn()    { echo -e "${YELLOW}[ warn]${RESET} $*"; }
err()     { echo -e "${RED}[error]${RESET} $*" >&2; exit 1; }
header()  { echo -e "\n${BOLD}${CYAN}━━ $* ━━${RESET}"; }

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
echo ""

# ── Detect OS ─────────────────────────────────────────────────────────────────
OS="unknown"
ARCH=$(uname -m 2>/dev/null || echo "x86_64")
case "$(uname -s 2>/dev/null)" in
  Darwin*)  OS="macos" ;;
  Linux*)   OS="linux" ;;
  MINGW*|CYGWIN*|MSYS*) OS="windows" ;;
esac

[[ "$OS" == "unknown" ]] && err "Unsupported OS. Use macOS, Linux, or Windows (WSL)."
info "Detected: $OS / $ARCH"

# ── Check prerequisites ───────────────────────────────────────────────────────
header "Checking prerequisites"

check_cmd() {
  if command -v "$1" &>/dev/null; then
    ok "$1 found ($(command -v "$1"))"
    return 0
  else
    return 1
  fi
}

# Node.js 18+
if ! check_cmd node; then
  warn "Node.js not found. Installing..."
  if [[ "$OS" == "macos" ]] && command -v brew &>/dev/null; then
    brew install node &>/dev/null && ok "Node.js installed via brew"
  elif [[ "$OS" == "linux" ]]; then
    curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash - &>/dev/null
    sudo apt-get install -y nodejs &>/dev/null && ok "Node.js installed"
  else
    err "Install Node.js 18+ from https://nodejs.org then re-run this script."
  fi
fi

NODE_VER=$(node -e "process.exit(parseInt(process.versions.node)>=18?0:1)" 2>/dev/null && node --version || echo "old")
[[ "$NODE_VER" == "old" ]] && err "Node.js 18+ required. Found: $(node --version). Upgrade and retry."

# Rust + Cargo (required for Tauri)
if ! check_cmd cargo; then
  warn "Rust not found. Installing via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path &>/dev/null
  source "$HOME/.cargo/env" 2>/dev/null || true
  ok "Rust installed"
fi

# Tauri system deps
header "System dependencies"
if [[ "$OS" == "linux" ]]; then
  info "Installing Tauri Linux build dependencies..."
  sudo apt-get update -qq 2>/dev/null || true
  sudo apt-get install -y -qq \
    libwebkit2gtk-4.1-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    patchelf \
    2>/dev/null && ok "Linux deps installed"
elif [[ "$OS" == "macos" ]]; then
  if ! command -v xcode-select &>/dev/null || ! xcode-select -p &>/dev/null; then
    warn "Xcode Command Line Tools may be needed. Run: xcode-select --install"
  else
    ok "Xcode CLT present"
  fi
fi

# ── Get Torot source ──────────────────────────────────────────────────────────
header "Fetching Torot"

if [[ -d "$TOROT_DIR/.git" ]]; then
  info "Existing install found at $TOROT_DIR — updating..."
  cd "$TOROT_DIR"
  git pull origin main --quiet 2>/dev/null || warn "Git pull failed (offline?)"
  ok "Updated"
elif command -v git &>/dev/null; then
  info "Cloning Torot to $TOROT_DIR..."
  git clone --depth 1 "$TOROT_REPO" "$TOROT_DIR" --quiet 2>/dev/null || {
    warn "Git clone failed — using bundled source."
    mkdir -p "$TOROT_DIR"
    cp -r "$(dirname "$0")/." "$TOROT_DIR/" 2>/dev/null || true
  }
  ok "Source ready at $TOROT_DIR"
else
  info "Copying source to $TOROT_DIR..."
  mkdir -p "$TOROT_DIR"
  cp -r "$(dirname "$0")/." "$TOROT_DIR/"
  ok "Source copied"
fi

cd "$TOROT_DIR"

# ── Install JS dependencies ───────────────────────────────────────────────────
header "Installing dependencies"
info "Running npm install..."
npm install --silent 2>/dev/null && ok "npm dependencies installed"

# ── Build desktop app ─────────────────────────────────────────────────────────
header "Building Torot"
info "Building with Tauri (this takes a few minutes on first build)..."
npm run tauri build -- --no-bundle 2>&1 | tail -5

# Find the built binary
BUILT_BIN=""
if [[ "$OS" == "macos" ]]; then
  BUILT_BIN=$(find "$TOROT_DIR/src-tauri/target/release" -name "torot" -not -path "*/deps/*" 2>/dev/null | head -1)
elif [[ "$OS" == "linux" ]]; then
  BUILT_BIN=$(find "$TOROT_DIR/src-tauri/target/release" -name "torot" -not -path "*/deps/*" 2>/dev/null | head -1)
elif [[ "$OS" == "windows" ]]; then
  BUILT_BIN=$(find "$TOROT_DIR/src-tauri/target/release" -name "torot.exe" 2>/dev/null | head -1)
fi

if [[ -z "$BUILT_BIN" ]]; then
  warn "Binary not found — Tauri build may have failed. Trying dev mode launch..."
else
  ok "Build complete: $BUILT_BIN"

  # Install to user bin
  INSTALL_TARGET="$HOME/.local/bin"
  mkdir -p "$INSTALL_TARGET"
  cp "$BUILT_BIN" "$INSTALL_TARGET/torot"
  chmod +x "$INSTALL_TARGET/torot"

  # Add to PATH hint
  if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc" 2>/dev/null || true
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.zshrc"  2>/dev/null || true
    warn "Added $HOME/.local/bin to PATH. Restart your shell or run: export PATH=\"\$HOME/.local/bin:\$PATH\""
  fi
  ok "Installed to $INSTALL_TARGET/torot"
fi

# ── Install security tools ────────────────────────────────────────────────────
header "Security tools"
info "Installing available security tools..."

TOOLS_INSTALLED=()
TOOLS_SKIPPED=()

try_pip()   { local name="$1" pkg="$2" bin="$3"; command -v "$bin" &>/dev/null && { TOOLS_INSTALLED+=("$name"); return; }; pip3 install "$pkg" -q 2>/dev/null && TOOLS_INSTALLED+=("$name") || TOOLS_SKIPPED+=("$name"); }
try_npm()   { local name="$1" pkg="$2" bin="$3"; command -v "$bin" &>/dev/null && { TOOLS_INSTALLED+=("$name"); return; }; npm install -g "$pkg" -q 2>/dev/null && TOOLS_INSTALLED+=("$name") || TOOLS_SKIPPED+=("$name"); }
try_cargo() { local name="$1" pkg="$2" bin="$3"; command -v "$bin" &>/dev/null && { TOOLS_INSTALLED+=("$name"); return; }; command -v cargo &>/dev/null && cargo install "$pkg" -q 2>/dev/null && TOOLS_INSTALLED+=("$name") || TOOLS_SKIPPED+=("$name"); }
try_go()    { local name="$1" pkg="$2" bin="$3"; command -v "$bin" &>/dev/null && { TOOLS_INSTALLED+=("$name"); return; }; command -v go &>/dev/null && go install "$pkg" 2>/dev/null && TOOLS_INSTALLED+=("$name") || TOOLS_SKIPPED+=("$name"); }

try_pip   "slither"    "slither-analyzer"  "slither"
try_pip   "mythril"    "mythril"           "myth"
try_pip   "halmos"     "halmos"            "halmos"
try_pip   "semgrep"    "semgrep"           "semgrep"
try_pip   "sqlmap"     "sqlmap"            "sqlmap"
try_pip   "arjun"      "arjun"             "arjun"
try_pip   "binwalk"    "binwalk"           "binwalk"
try_pip   "checksec"   "checksec"          "checksec"
try_pip   "eth-wake"   "eth-wake"          "wake"
try_npm   "solhint"    "solhint"           "solhint"
try_cargo "aderyn"     "aderyn"            "aderyn"
try_cargo "cargo-audit" "cargo-audit"      "cargo-audit"
try_cargo "heimdall"   "heimdall-rs"       "heimdall"
try_go    "nuclei"     "github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest"  "nuclei"
try_go    "ffuf"       "github.com/ffuf/ffuf/v2@latest"                           "ffuf"
try_go    "gobuster"   "github.com/OJ/gobuster/v3@latest"                         "gobuster"
try_go    "dalfox"     "github.com/hahwul/dalfox/v2@latest"                       "dalfox"
try_go    "gitleaks"   "github.com/zricethezav/gitleaks/v8@latest"                "gitleaks"
try_go    "trufflehog" "github.com/trufflesecurity/trufflehog/v3@latest"          "trufflehog"

# System tools
if [[ "$OS" == "linux" ]] && command -v apt-get &>/dev/null; then
  for pkg in nikto radare2 ltrace strace; do
    command -v "$pkg" &>/dev/null && TOOLS_INSTALLED+=("$pkg") && continue
    sudo apt-get install -y -qq "$pkg" &>/dev/null && TOOLS_INSTALLED+=("$pkg") || TOOLS_SKIPPED+=("$pkg")
  done
elif [[ "$OS" == "macos" ]] && command -v brew &>/dev/null; then
  for pkg in nikto radare2; do
    command -v "$pkg" &>/dev/null && TOOLS_INSTALLED+=("$pkg") && continue
    brew install "$pkg" &>/dev/null 2>&1 && TOOLS_INSTALLED+=("$pkg") || TOOLS_SKIPPED+=("$pkg")
  done
fi

# ── Summary ───────────────────────────────────────────────────────────────────
header "Installation Complete"
echo ""
echo -e "  ${GREEN}Tools installed (${#TOOLS_INSTALLED[@]}):${RESET}"
echo "    ${TOOLS_INSTALLED[*]:-none}"
echo ""
[[ ${#TOOLS_SKIPPED[@]} -gt 0 ]] && echo -e "  ${YELLOW}Tools skipped (${#TOOLS_SKIPPED[@]}):${RESET}\n    ${TOOLS_SKIPPED[*]}"
echo ""
echo -e "  ${BOLD}To launch Torot:${RESET}"
if [[ -n "${BUILT_BIN:-}" ]]; then
  echo "    torot"
else
  echo "    cd $TOROT_DIR && npm run tauri:dev"
fi
echo ""
echo -e "  ${BOLD}To update:${RESET}"
echo "    cd $TOROT_DIR && ./update.sh"
echo ""
