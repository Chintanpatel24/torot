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
