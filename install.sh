#!/usr/bin/env bash
set -euo pipefail

TOROT_VERSION="4.0.0"
BINARY_URL="https://github.com/torot/torot/releases/download/v${TOROT_VERSION}/torot-$(uname -s)-$(uname -m).tar.gz"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

info()  { printf "\033[32m->\033[0m %s\n" "$*"; }
warn()  { printf "\033[33m!\033[0m %s\n" "$*"; }
error() { printf "\033[31mX\033[0m %s\n" "$*"; exit 1; }

if [ -f "target/release/torot" ]; then
    info "Found local release build, installing..."
    install -m 755 target/release/torot "${INSTALL_DIR}/torot"
    info "Installed torot to ${INSTALL_DIR}/torot"
    exit 0
fi

if command -v cargo &>/dev/null; then
    info "Building from source..."
    cargo build --release
    install -m 755 target/release/torot "${INSTALL_DIR}/torot"
    info "Installed torot to ${INSTALL_DIR}/torot"
    exit 0
fi

info "Downloading torot v${TOROT_VERSION}..."
TMPDIR=$(mktemp -d)
cd "${TMPDIR}"

if command -v curl &>/dev/null; then
    curl -sL "$BINARY_URL" -o torot.tar.gz
elif command -v wget &>/dev/null; then
    wget -q "$BINARY_URL" -O torot.tar.gz
else
    error "Need curl or wget to download. Install Rust with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi

tar xzf torot.tar.gz
install -m 755 torot "${INSTALL_DIR}/torot"
rm -rf "${TMPDIR}"

info "Installed torot to ${INSTALL_DIR}/torot"
info "Run 'torot tools' to verify installation."
