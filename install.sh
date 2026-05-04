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
