#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
#  Torot — Universal Security Agent
#  UPDATER  (update.sh)
#
#  Usage:
#    chmod +x update.sh && ./update.sh
#    ./update.sh --tools-only    # only update security tools
#    ./update.sh --torot-only    # only update Torot itself
#    ./update.sh --check         # check what has updates, don't install
# ─────────────────────────────────────────────────────────────────

set -euo pipefail

TOROT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UPDATE_TOROT=true
UPDATE_TOOLS=true
CHECK_ONLY=false

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

info()   { echo -e "${CYAN}  [upd ]${RESET}  $*"; }
ok()     { echo -e "${GREEN}  [ ok ]${RESET}  $*"; }
warn()   { echo -e "${YELLOW}  [skip]${RESET}  $*"; }
check()  { echo -e "${CYAN}  [chk ]${RESET}  $*"; }
header() { echo -e "\n${BOLD}${CYAN}══ $* ══${RESET}"; }

for arg in "$@"; do
  case $arg in
    --tools-only)  UPDATE_TOROT=false ;;
    --torot-only)  UPDATE_TOOLS=false ;;
    --check)       CHECK_ONLY=true ;;
    --help|-h)
      echo "Usage: $0 [--tools-only] [--torot-only] [--check]"
      exit 0 ;;
  esac
done

echo -e "\n${BOLD}${CYAN}  Torot Updater${RESET}\n"

# ─────────────────────────────────────────────────────────────────
# 1. Update Torot itself
# ─────────────────────────────────────────────────────────────────
if [[ "$UPDATE_TOROT" == true ]]; then
  header "Torot Core"

  # Check if this is a git repo
  if [[ -d "$TOROT_DIR/.git" ]]; then
    info "Git repo detected — pulling latest changes..."
    if [[ "$CHECK_ONLY" == true ]]; then
      cd "$TOROT_DIR"
      BEHIND=$(git rev-list HEAD..origin/main --count 2>/dev/null || echo "?")
      if [[ "$BEHIND" == "0" ]]; then
        ok "Torot is up to date"
      else
        check "Torot: $BEHIND commit(s) behind origin/main"
      fi
    else
      cd "$TOROT_DIR"
      git fetch origin --quiet 2>/dev/null || warn "Could not reach remote (offline?)"
      git pull origin main --quiet 2>/dev/null || warn "Pull failed — local changes may conflict"
      pip3 install -e . --quiet
      ok "Torot updated from git"
    fi

  else
    # Not a git repo — reinstall from current directory
    if [[ "$CHECK_ONLY" == true ]]; then
      CURRENT=$(torot --version 2>/dev/null | grep -oP '\d+\.\d+\.\d+' || echo "unknown")
      check "Installed version: $CURRENT"
      check "To update: run ./update.sh (from the extracted zip)"
    else
      info "Reinstalling Torot from $TOROT_DIR ..."
      pip3 install "$TOROT_DIR" --quiet --force-reinstall
      ok "Torot reinstalled"
    fi
  fi
fi

# ─────────────────────────────────────────────────────────────────
# 2. Update all installed security tools
# ─────────────────────────────────────────────────────────────────
if [[ "$UPDATE_TOOLS" == true ]]; then
  header "Security Tools"

  UPDATED=()
  FAILED=()
  NOT_INSTALLED=()

  # Helper: run an update command, track result
  try_update() {
    local name="$1"
    local check_bin="$2"
    local cmd="$3"

    if ! command -v "$check_bin" &>/dev/null; then
      NOT_INSTALLED+=("$name")
      return
    fi

    if [[ "$CHECK_ONLY" == true ]]; then
      check "$name is installed"
      return
    fi

    info "Updating $name..."
    if eval "$cmd" &>/dev/null 2>&1; then
      ok "$name updated"
      UPDATED+=("$name")
    else
      warn "$name — update failed"
      FAILED+=("$name")
    fi
  }

  # ── Python tools ───────────────────────────────────────────────
  header "Python Tools"
  try_update "slither"      "slither"     "pip3 install --upgrade slither-analyzer --quiet"
  try_update "mythril"      "myth"        "pip3 install --upgrade mythril --quiet"
  try_update "manticore"    "manticore"   "pip3 install --upgrade 'manticore[native]' --quiet"
  try_update "halmos"       "halmos"      "pip3 install --upgrade halmos --quiet"
  try_update "semgrep"      "semgrep"     "pip3 install --upgrade semgrep --quiet"
  try_update "sqlmap"       "sqlmap"      "pip3 install --upgrade sqlmap --quiet"
  try_update "wfuzz"        "wfuzz"       "pip3 install --upgrade wfuzz --quiet"
  try_update "arjun"        "arjun"       "pip3 install --upgrade arjun --quiet"
  try_update "binwalk"      "binwalk"     "pip3 install --upgrade binwalk --quiet"
  try_update "checksec"     "checksec"    "pip3 install --upgrade checksec --quiet"
  try_update "eth-wake"     "wake"        "pip3 install --upgrade eth-wake --quiet"
  try_update "trufflehog"   "trufflehog"  "pip3 install --upgrade truffleHog --quiet"

  # ── Node tools ─────────────────────────────────────────────────
  if command -v npm &>/dev/null; then
    header "Node Tools"
    try_update "solhint"    "solhint"     "npm update -g solhint --quiet"
    try_update "smartcheck" "smartcheck"  "npm update -g smartcheck --quiet"
  fi

  # ── Rust tools ─────────────────────────────────────────────────
  if command -v cargo &>/dev/null; then
    header "Rust Tools"
    try_update "aderyn"      "aderyn"      "cargo install aderyn --quiet"
    try_update "cargo-audit" "cargo-audit" "cargo install cargo-audit --quiet"
    try_update "pyrometer"   "pyrometer"   "cargo install pyrometer --quiet"
    try_update "heimdall"    "heimdall"    "cargo install heimdall-rs --quiet"
    # Rustup self-update
    if command -v rustup &>/dev/null; then
      if [[ "$CHECK_ONLY" == false ]]; then
        info "Updating rustup toolchain..."
        rustup update stable --quiet 2>/dev/null && ok "rustup updated" || warn "rustup update failed"
      fi
    fi
  fi

  # ── Go tools ───────────────────────────────────────────────────
  if command -v go &>/dev/null; then
    header "Go Tools"
    try_update "nuclei"     "nuclei"   "go install github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest"
    try_update "ffuf"       "ffuf"     "go install github.com/ffuf/ffuf/v2@latest"
    try_update "gobuster"   "gobuster" "go install github.com/OJ/gobuster/v3@latest"
    try_update "dalfox"     "dalfox"   "go install github.com/hahwul/dalfox/v2@latest"
    try_update "gitleaks"   "gitleaks" "go install github.com/zricethezav/gitleaks/v8@latest"
    try_update "kiterunner" "kr"       "go install github.com/assetnote/kiterunner/cmd/kr@latest"
  fi

  # ── System tools ───────────────────────────────────────────────
  if command -v apt-get &>/dev/null; then
    header "System Tools (apt)"
    for pkg in nikto radare2 ltrace strace; do
      if command -v "$pkg" &>/dev/null; then
        if [[ "$CHECK_ONLY" == false ]]; then
          info "Updating $pkg..."
          sudo apt-get install -y --only-upgrade "$pkg" &>/dev/null 2>&1 && \
            ok "$pkg updated" && UPDATED+=("$pkg") || \
            warn "$pkg — apt upgrade failed" && FAILED+=("$pkg")
        else
          check "$pkg is installed"
        fi
      else
        NOT_INSTALLED+=("$pkg")
      fi
    done
  elif command -v brew &>/dev/null; then
    header "System Tools (brew)"
    for pkg in nikto radare2; do
      if command -v "$pkg" &>/dev/null; then
        if [[ "$CHECK_ONLY" == false ]]; then
          info "Updating $pkg..."
          brew upgrade "$pkg" &>/dev/null 2>&1 && \
            ok "$pkg updated" && UPDATED+=("$pkg") || \
            warn "$pkg — already latest or failed" && UPDATED+=("$pkg")
        else
          check "$pkg is installed"
        fi
      else
        NOT_INSTALLED+=("$pkg")
      fi
    done
  fi

  # ── Summary ────────────────────────────────────────────────────
  header "Update Summary"
  echo ""
  if [[ "$CHECK_ONLY" == true ]]; then
    echo -e "  Run ${BOLD}./update.sh${RESET} to apply updates."
  else
    [[ ${#UPDATED[@]}       -gt 0 ]] && echo -e "  ${GREEN}Updated  (${#UPDATED[@]}):${RESET}       ${UPDATED[*]}"
    [[ ${#FAILED[@]}        -gt 0 ]] && echo -e "  ${YELLOW}Failed   (${#FAILED[@]}):${RESET}        ${FAILED[*]}"
    [[ ${#NOT_INSTALLED[@]} -gt 0 ]] && echo -e "  ${CYAN}Not installed:${RESET}  ${NOT_INSTALLED[*]}"
    echo ""
    echo -e "  Run ${BOLD}torot --list-tools${RESET} to see full tool status."
  fi
  echo ""
fi
