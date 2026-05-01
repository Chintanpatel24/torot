#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
#  Torot — Universal Security Agent
#  INSTALLER  (install.sh)
#
#  Usage:
#    chmod +x install.sh && ./install.sh
#    ./install.sh --no-tools      # skip security tool install
#    ./install.sh --dev           # install in editable mode
# ─────────────────────────────────────────────────────────────────

set -euo pipefail

TOROT_VERSION="2.0.0"
TOROT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_TOOLS=true
DEV_MODE=false

# ── Colours ──────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

info()    { echo -e "${CYAN}  [info]${RESET}  $*"; }
ok()      { echo -e "${GREEN}  [ ok ]${RESET}  $*"; }
warn()    { echo -e "${YELLOW}  [warn]${RESET}  $*"; }
error()   { echo -e "${RED}  [err ]${RESET}  $*"; exit 1; }
header()  { echo -e "\n${BOLD}${CYAN}══ $* ══${RESET}"; }

# ── Argument parsing ─────────────────────────────────────────────
for arg in "$@"; do
  case $arg in
    --no-tools) INSTALL_TOOLS=false ;;
    --dev)      DEV_MODE=true ;;
    --help|-h)
      echo "Usage: $0 [--no-tools] [--dev]"
      echo "  --no-tools   Skip security tool installation"
      echo "  --dev        Install Torot in editable (development) mode"
      exit 0 ;;
  esac
done

# ── Banner ───────────────────────────────────────────────────────
echo -e "${CYAN}"
cat << 'EOF'
 ████████╗ ██████╗ ██████╗  ██████╗ ████████╗
    ██╔══╝██╔═══██╗██╔══██╗██╔═══██╗╚══██╔══╝
    ██║   ██║   ██║██████╔╝██║   ██║   ██║   
    ██║   ██║   ██║██╔══██╗██║   ██║   ██║   
    ██║   ╚██████╔╝██║  ██║╚██████╔╝   ██║   
    ╚═╝    ╚═════╝ ╚═╝  ╚═╝ ╚═════╝    ╚═╝   
EOF
echo -e "${RESET}"
echo -e "  ${BOLD}Universal Security Agent  |  v${TOROT_VERSION}  |  Installer${RESET}"
echo ""

# ── OS detection ─────────────────────────────────────────────────
OS="unknown"
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
  OS="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
  OS="macos"
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
  OS="windows"
fi
info "Detected OS: $OS"

# ── Require Python 3.10+ ─────────────────────────────────────────
header "Python"
if ! command -v python3 &>/dev/null; then
  error "Python 3 not found. Install from https://python.org"
fi

PYTHON_VERSION=$(python3 -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
PYTHON_MAJOR=$(echo $PYTHON_VERSION | cut -d. -f1)
PYTHON_MINOR=$(echo $PYTHON_VERSION | cut -d. -f2)

if [[ $PYTHON_MAJOR -lt 3 || ($PYTHON_MAJOR -eq 3 && $PYTHON_MINOR -lt 10) ]]; then
  error "Python 3.10+ required (found $PYTHON_VERSION). Upgrade Python first."
fi
ok "Python $PYTHON_VERSION"

# ── Install Torot Python package ─────────────────────────────────
header "Installing Torot"
cd "$TOROT_DIR"

if [[ "$DEV_MODE" == true ]]; then
  info "Installing in editable (dev) mode..."
  pip3 install -e . --quiet
else
  info "Installing Torot..."
  pip3 install . --quiet
fi
ok "Torot v${TOROT_VERSION} installed"

# Verify the command is available
if command -v torot &>/dev/null; then
  ok "torot command available at: $(which torot)"
else
  warn "torot not found in PATH. You may need to add pip's bin dir:"
  warn "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# ── Optional: install security tools ────────────────────────────
if [[ "$INSTALL_TOOLS" == false ]]; then
  echo ""
  info "Skipping security tool installation (--no-tools)"
  echo ""
  echo -e "  Run ${BOLD}torot --list-tools${RESET} to see what to install later."
  echo ""
  exit 0
fi

header "Security Tools"
echo "  Installing available security tools."
echo "  Tools that fail or require extra setup will be skipped."
echo ""

INSTALLED=()
SKIPPED=()

try_install() {
  local name="$1"
  local cmd="$2"
  local check="$3"
  if command -v "$check" &>/dev/null; then
    ok "$name (already installed)"
    INSTALLED+=("$name")
    return
  fi
  info "Installing $name..."
  if eval "$cmd" &>/dev/null 2>&1; then
    ok "$name"
    INSTALLED+=("$name")
  else
    warn "$name — install failed (skipped)"
    SKIPPED+=("$name")
  fi
}

# ── Python-based tools ───────────────────────────────────────────
header "Python Tools (pip)"

try_install "slither"    "pip3 install slither-analyzer --quiet"   "slither"
try_install "mythril"    "pip3 install mythril --quiet"            "myth"
try_install "manticore"  "pip3 install 'manticore[native]' --quiet" "manticore"
try_install "solhint"    "npm install -g solhint --quiet 2>/dev/null || true" "solhint"
try_install "halmos"     "pip3 install halmos --quiet"             "halmos"
try_install "semgrep"    "pip3 install semgrep --quiet"            "semgrep"
try_install "sqlmap"     "pip3 install sqlmap --quiet"             "sqlmap"
try_install "wfuzz"      "pip3 install wfuzz --quiet"              "wfuzz"
try_install "arjun"      "pip3 install arjun --quiet"              "arjun"
try_install "binwalk"    "pip3 install binwalk --quiet"            "binwalk"
try_install "checksec"   "pip3 install checksec --quiet"           "checksec"
try_install "solc-select" "pip3 install solc-select --quiet && solc-select install latest --quiet && solc-select use latest --quiet" "solc"
try_install "eth-wake"   "pip3 install eth-wake --quiet"           "wake"
try_install "trufflehog" "pip3 install truffleHog --quiet"         "trufflehog"

# ── Node-based tools ─────────────────────────────────────────────
if command -v npm &>/dev/null; then
  header "Node Tools (npm)"
  try_install "solhint"     "npm install -g solhint --quiet"     "solhint"
  try_install "smartcheck"  "npm install -g smartcheck --quiet"  "smartcheck"
else
  warn "npm not found — skipping Node.js tools (solhint, smartcheck)"
  SKIPPED+=("solhint" "smartcheck")
fi

# ── Rust-based tools ─────────────────────────────────────────────
if command -v cargo &>/dev/null; then
  header "Rust Tools (cargo)"
  try_install "aderyn"      "cargo install aderyn --quiet"       "aderyn"
  try_install "cargo-audit" "cargo install cargo-audit --quiet"  "cargo-audit"
  try_install "clippy"      "rustup component add clippy"        "cargo"
  try_install "pyrometer"   "cargo install pyrometer --quiet"    "pyrometer"
  try_install "heimdall"    "cargo install heimdall-rs --quiet"  "heimdall"
else
  warn "cargo not found — skipping Rust tools (aderyn, cargo-audit, heimdall)"
  warn "Install Rust from: https://rustup.rs"
  SKIPPED+=("aderyn" "cargo-audit" "clippy" "pyrometer" "heimdall")
fi

# ── Go-based tools ───────────────────────────────────────────────
if command -v go &>/dev/null; then
  header "Go Tools"
  try_install "nuclei"    "go install github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest" "nuclei"
  try_install "ffuf"      "go install github.com/ffuf/ffuf/v2@latest" "ffuf"
  try_install "gobuster"  "go install github.com/OJ/gobuster/v3@latest" "gobuster"
  try_install "dalfox"    "go install github.com/hahwul/dalfox/v2@latest" "dalfox"
  try_install "gitleaks"  "go install github.com/zricethezav/gitleaks/v8@latest" "gitleaks"
  try_install "kiterunner" "go install github.com/assetnote/kiterunner/cmd/kr@latest" "kr"
else
  warn "go not found — skipping Go tools (nuclei, ffuf, gobuster, dalfox)"
  warn "Install Go from: https://go.dev/dl"
  SKIPPED+=("nuclei" "ffuf" "gobuster" "dalfox" "gitleaks" "kiterunner")
fi

# ── System tools (apt / brew) ────────────────────────────────────
header "System Tools"
if [[ "$OS" == "linux" ]] && command -v apt-get &>/dev/null; then
  for pkg in nikto radare2 ltrace strace; do
    if command -v "$pkg" &>/dev/null; then
      ok "$pkg (already installed)"
      INSTALLED+=("$pkg")
    else
      info "Installing $pkg via apt..."
      if sudo apt-get install -y "$pkg" &>/dev/null 2>&1; then
        ok "$pkg"
        INSTALLED+=("$pkg")
      else
        warn "$pkg — apt install failed"
        SKIPPED+=("$pkg")
      fi
    fi
  done
elif [[ "$OS" == "macos" ]] && command -v brew &>/dev/null; then
  for pkg in nikto radare2; do
    if command -v "$pkg" &>/dev/null; then
      ok "$pkg (already installed)"
      INSTALLED+=("$pkg")
    else
      info "Installing $pkg via brew..."
      if brew install "$pkg" &>/dev/null 2>&1; then
        ok "$pkg"
        INSTALLED+=("$pkg")
      else
        warn "$pkg — brew install failed"
        SKIPPED+=("$pkg")
      fi
    fi
  done
else
  warn "No apt or brew found — skipping system tools (nikto, radare2)"
  SKIPPED+=("nikto" "radare2" "ltrace" "strace")
fi

# ── Summary ──────────────────────────────────────────────────────
header "Summary"
echo ""
echo -e "  ${GREEN}${BOLD}Installed (${#INSTALLED[@]}):${RESET}  ${INSTALLED[*]:-none}"
echo ""
if [[ ${#SKIPPED[@]} -gt 0 ]]; then
  echo -e "  ${YELLOW}Skipped  (${#SKIPPED[@]}):${RESET}  ${SKIPPED[*]}"
  echo ""
fi
echo -e "  Run ${BOLD}torot --list-tools${RESET} for a full status table."
echo ""
echo -e "  ${BOLD}Get started:${RESET}"
echo "    torot                    # interactive wizard"
echo "    torot ./contracts/       # scan a folder"
echo "    torot --help             # all options"
echo ""
