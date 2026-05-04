#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
#  Torot v3 — Updater
#  Updates Torot app + all installed security tools
#
#  Usage:
#    ./update.sh                  full update
#    ./update.sh --torot-only     only update Torot itself
#    ./update.sh --tools-only     only update security tools
#    ./update.sh --check          check versions, no changes
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

TOROT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UPDATE_TOROT=true
UPDATE_TOOLS=true
CHECK_ONLY=false

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

info()   { echo -e "${CYAN}[torot]${RESET} $*"; }
ok()     { echo -e "${GREEN}[  ok ]${RESET} $*"; }
warn()   { echo -e "${YELLOW}[ skip]${RESET} $*"; }
header() { echo -e "\n${BOLD}${CYAN}━━ $* ━━${RESET}"; }

for arg in "$@"; do
  case $arg in
    --torot-only) UPDATE_TOOLS=false ;;
    --tools-only) UPDATE_TOROT=false ;;
    --check)      CHECK_ONLY=true ;;
    --help|-h)
      echo "Usage: $0 [--torot-only] [--tools-only] [--check]"
      exit 0 ;;
  esac
done

echo -e "\n${BOLD}${CYAN}  Torot Updater  v3.0${RESET}\n"

# ─────────────────────────────────────────────────────────────────────────────
# 1. Update Torot itself
# ─────────────────────────────────────────────────────────────────────────────
if [[ "$UPDATE_TOROT" == true ]]; then
  header "Torot Application"
  cd "$TOROT_DIR"

  if [[ -d ".git" ]]; then
    info "Pulling latest changes from git..."
    if [[ "$CHECK_ONLY" == true ]]; then
      BEHIND=$(git rev-list HEAD..origin/main --count 2>/dev/null || echo "?")
      [[ "$BEHIND" == "0" ]] && ok "Already up to date" || warn "$BEHIND commit(s) behind main"
    else
      git fetch origin --quiet 2>/dev/null || warn "Cannot reach remote"
      git pull origin main --quiet 2>/dev/null || warn "Pull failed — local changes may conflict"
      npm install --silent 2>/dev/null
      info "Rebuilding app..."
      npm run tauri build -- --no-bundle 2>&1 | tail -3

      # Reinstall binary
      BUILT=$(find "$TOROT_DIR/src-tauri/target/release" -name "torot" -not -path "*/deps/*" 2>/dev/null | head -1)
      if [[ -n "$BUILT" ]]; then
        cp "$BUILT" "$HOME/.local/bin/torot" 2>/dev/null || true
        ok "Binary updated"
      fi
      ok "Torot updated"
    fi
  else
    if [[ "$CHECK_ONLY" == true ]]; then
      info "Not a git repo — run ./install.sh to get latest source"
    else
      npm install --silent 2>/dev/null && ok "npm deps refreshed"
    fi
  fi
fi

# ─────────────────────────────────────────────────────────────────────────────
# 2. Update security tools
# ─────────────────────────────────────────────────────────────────────────────
if [[ "$UPDATE_TOOLS" == true ]]; then
  header "Security Tools"

  UPDATED=()
  FAILED=()
  NOT_INSTALLED=()

  upd() {
    local name="$1" bin="$2" cmd="$3"
    if ! command -v "$bin" &>/dev/null; then
      NOT_INSTALLED+=("$name"); return
    fi
    if [[ "$CHECK_ONLY" == true ]]; then
      ok "$name installed ($(command -v "$bin"))"; return
    fi
    info "Updating $name..."
    if eval "$cmd" &>/dev/null 2>&1; then
      ok "$name"; UPDATED+=("$name")
    else
      warn "$name — failed"; FAILED+=("$name")
    fi
  }

  # pip
  upd "slither"    "slither"    "pip3 install --upgrade slither-analyzer -q"
  upd "mythril"    "myth"       "pip3 install --upgrade mythril -q"
  upd "halmos"     "halmos"     "pip3 install --upgrade halmos -q"
  upd "semgrep"    "semgrep"    "pip3 install --upgrade semgrep -q"
  upd "sqlmap"     "sqlmap"     "pip3 install --upgrade sqlmap -q"
  upd "arjun"      "arjun"      "pip3 install --upgrade arjun -q"
  upd "binwalk"    "binwalk"    "pip3 install --upgrade binwalk -q"
  upd "checksec"   "checksec"   "pip3 install --upgrade checksec -q"
  upd "eth-wake"   "wake"       "pip3 install --upgrade eth-wake -q"
  upd "manticore"  "manticore"  "pip3 install --upgrade manticore -q"

  # npm
  upd "solhint"    "solhint"    "npm update -g solhint -q"
  upd "smartcheck" "smartcheck" "npm update -g smartcheck -q"

  # cargo
  upd "aderyn"      "aderyn"      "cargo install aderyn -q"
  upd "cargo-audit" "cargo-audit" "cargo install cargo-audit -q"
  upd "heimdall"    "heimdall"    "cargo install heimdall-rs -q"

  # go
  upd "nuclei"    "nuclei"    "go install github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest"
  upd "ffuf"      "ffuf"      "go install github.com/ffuf/ffuf/v2@latest"
  upd "gobuster"  "gobuster"  "go install github.com/OJ/gobuster/v3@latest"
  upd "dalfox"    "dalfox"    "go install github.com/hahwul/dalfox/v2@latest"
  upd "gitleaks"  "gitleaks"  "go install github.com/zricethezav/gitleaks/v8@latest"
  upd "trufflehog" "trufflehog" "go install github.com/trufflesecurity/trufflehog/v3@latest"

  # rustup
  if command -v rustup &>/dev/null && [[ "$CHECK_ONLY" == false ]]; then
    info "Updating Rust toolchain..."
    rustup update stable -q 2>/dev/null && ok "Rust toolchain updated"
  fi

  # apt/brew system tools
  if [[ "$CHECK_ONLY" == false ]]; then
    if command -v apt-get &>/dev/null; then
      for pkg in nikto radare2; do
        command -v "$pkg" &>/dev/null || continue
        sudo apt-get install -y --only-upgrade "$pkg" -qq 2>/dev/null && \
          ok "$pkg" && UPDATED+=("$pkg") || warn "$pkg"
      done
    elif command -v brew &>/dev/null; then
      for pkg in nikto radare2; do
        command -v "$pkg" &>/dev/null || continue
        brew upgrade "$pkg" 2>/dev/null && UPDATED+=("$pkg") || true
      done
    fi
  fi

  header "Summary"
  [[ ${#UPDATED[@]}       -gt 0 ]] && echo -e "  ${GREEN}Updated:${RESET}       ${UPDATED[*]}"
  [[ ${#FAILED[@]}        -gt 0 ]] && echo -e "  ${YELLOW}Failed:${RESET}        ${FAILED[*]}"
  [[ ${#NOT_INSTALLED[@]} -gt 0 ]] && echo -e "  ${CYAN}Not installed:${RESET} ${NOT_INSTALLED[*]}"
  echo ""
fi
