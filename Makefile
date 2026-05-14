.PHONY: all build release install uninstall clean test lint help

PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin

all: build

build:
	cargo build

release:
	cargo build --release

install: release
	install -d $(DESTDIR)$(BINDIR)
	install -m 755 target/release/torot $(DESTDIR)$(BINDIR)/torot
	@echo "Installed torot to $(DESTDIR)$(BINDIR)/torot"

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/torot

clean:
	cargo clean

test:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

check: lint test build

release-check: lint test release

help:
	@echo "Targets:"
	@echo "  build       - Build in debug mode"
	@echo "  release     - Build in release mode"
	@echo "  install     - Install release binary"
	@echo "  uninstall   - Remove installed binary"
	@echo "  clean       - Clean build artifacts"
	@echo "  test        - Run tests"
	@echo "  lint        - Run clippy"
	@echo "  fmt         - Format code"
	@echo "  check       - Lint + test + build"
