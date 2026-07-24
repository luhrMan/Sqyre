# Sqyre build helpers. Default output: ./bin
# Binary is Rust (sqyre-app). Linux AppImage packaging uses the same stack.
# Windows: Docker MinGW cross from Linux (scripts/windows/), or native on Windows.
.PHONY: all sqyre release windows macos test coverage check check-fmt fmt clippy deny machete \
	release-gate run tessdata appimage docs-media wasm help

ROOT := $(abspath .)
BIN := $(abspath bin)

CARGO ?= cargo
CARGO_FLAGS ?=
# Honor CARGO_TARGET_DIR when set (CI / sandbox); otherwise ./target
TARGET_DIR := $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),$(ROOT)/target)

# Host OS for native binary targets (Windows_NT / Darwin / Linux / MinGW / MSYS).
ifeq ($(OS),Windows_NT)
  HOST_OS := windows
  BIN_EXT := .exe
else
  UNAME_S := $(shell uname -s 2>/dev/null)
  ifeq ($(UNAME_S),Darwin)
    HOST_OS := macos
    BIN_EXT :=
  else ifeq ($(UNAME_S),Linux)
    HOST_OS := linux
    BIN_EXT :=
  else ifneq (,$(findstring MINGW,$(UNAME_S)))
    HOST_OS := windows
    BIN_EXT := .exe
  else ifneq (,$(findstring MSYS,$(UNAME_S)))
    HOST_OS := windows
    BIN_EXT := .exe
  else ifneq (,$(findstring CYGWIN,$(UNAME_S)))
    HOST_OS := windows
    BIN_EXT := .exe
  else
    HOST_OS := unknown
    BIN_EXT :=
  endif
endif

# Prefer env/devcontainer cargo; fall back to workspace-local, then ~/.cargo.
# Docker/CI use .cache/cargo (or inherit CARGO_HOME when Make exports .cargo-home).
ifeq ($(origin CARGO_HOME),undefined)
  ifneq ($(wildcard $(ROOT)/.cargo-home/bin/cargo),)
    export CARGO_HOME := $(ROOT)/.cargo-home
  else ifneq ($(wildcard $(HOME)/.cargo/bin/cargo),)
    export CARGO_HOME := $(HOME)/.cargo
  endif
endif
ifeq ($(origin RUSTUP_HOME),undefined)
  ifneq ($(wildcard $(ROOT)/.rustup-home),)
    export RUSTUP_HOME := $(ROOT)/.rustup-home
  else ifneq ($(wildcard $(HOME)/.rustup),)
    export RUSTUP_HOME := $(HOME)/.rustup
  endif
endif
ifneq ($(wildcard $(CARGO_HOME)/bin/cargo),)
  export PATH := $(CARGO_HOME)/bin:$(PATH)
  CARGO := $(CARGO_HOME)/bin/cargo
endif

all: sqyre

help:
	@echo "Targets:"
	@echo "  all / sqyre  - cargo build (debug) -> $(BIN)/sqyre$(BIN_EXT)  [default]"
	@echo "  release      - fmt + check, then cargo build --release -> $(BIN)/sqyre$(BIN_EXT)"
	@echo "  windows      - fmt + check, then Windows release -> $(BIN)/sqyre.exe"
	@echo "                 (Docker MinGW cross on Linux; native on Windows)"
	@echo "  macos        - fmt + check, then native macOS release -> $(BIN)/sqyre  (macOS host)"
	@echo "  test         - cargo nextest (fallback: cargo test)"
	@echo "  check-fmt    - cargo fmt --check"
	@echo "  fmt          - cargo fmt --all (write)"
	@echo "  clippy       - cargo clippy --workspace --all-targets (-D warnings)"
	@echo "  deny         - cargo deny check (licenses / advisories / bans / sources)"
	@echo "  machete      - cargo machete (unused path/crate deps)"
	@echo "  check        - check-fmt + clippy + deny (CI quality gates)"
	@echo "  release-gate - fmt then check (used by release/packaging targets)"
	@echo "  coverage     - cargo llvm-cov HTML + lcov (install: cargo install cargo-llvm-cov)"
	@echo "  run          - cargo run -p sqyre-app"
	@echo "  tessdata     - scripts/download-tessdata.sh"
	@echo "  docs-media   - regenerate docs/images screenshots"
	@echo "  appimage     - fmt + check, then AppImage -> $(BIN)/ (Docker fallback if tools missing)"
	@echo "                 (RELEASE_VERSION=…; SQYRE_APPIMAGE_FORCE_NATIVE=1)"
	@echo "  wasm         - fmt + check, then GUI-only WASM editor -> $(BIN)/wasm/ (requires Trunk)"

$(BIN):
	mkdir -p $(BIN)

sqyre: $(BIN)
	$(CARGO) build -p sqyre-app $(CARGO_FLAGS)
	cp -f $(TARGET_DIR)/debug/sqyre$(BIN_EXT) $(BIN)/sqyre$(BIN_EXT)

# Sequential fmt → check so release/packaging stays gated under make -j.
release-gate:
	$(MAKE) fmt
	$(MAKE) check

release: release-gate $(BIN)
	$(CARGO) build -p sqyre-app --release $(CARGO_FLAGS)
	cp -f $(TARGET_DIR)/release/sqyre$(BIN_EXT) $(BIN)/sqyre$(BIN_EXT)

# Windows release binary (no MSI). Docker MinGW cross from Linux; native on Windows.
windows: release-gate $(BIN)
	./scripts/windows/build.sh

macos: release-gate $(BIN)
	@if [ "$(HOST_OS)" != "macos" ]; then \
		echo "make macos requires a macOS host (got $(HOST_OS))"; \
		exit 1; \
	fi
	$(CARGO) build -p sqyre-app --release $(CARGO_FLAGS)
	cp -f $(TARGET_DIR)/release/sqyre $(BIN)/sqyre

test:
	@if $(CARGO) nextest --version >/dev/null 2>&1; then \
		$(CARGO) nextest run --workspace $(CARGO_FLAGS); \
	else \
		echo "cargo-nextest not found; falling back to cargo test"; \
		echo "  Install: cargo install cargo-nextest --locked"; \
		$(CARGO) test $(CARGO_FLAGS); \
	fi

check-fmt:
	$(CARGO) fmt --all -- --check

fmt:
	$(CARGO) fmt --all

clippy:
	$(CARGO) clippy --workspace --all-targets $(CARGO_FLAGS) -- -D warnings

deny:
	@if ! $(CARGO) deny --version >/dev/null 2>&1; then \
		echo "cargo-deny not found. Install with:"; \
		echo "  cargo install cargo-deny --locked"; \
		exit 1; \
	fi
	$(CARGO) deny check

machete:
	@if ! $(CARGO) machete --version >/dev/null 2>&1; then \
		echo "cargo-machete not found. Install with:"; \
		echo "  cargo install cargo-machete --locked"; \
		exit 1; \
	fi
	$(CARGO) machete

check: check-fmt clippy deny

# Report-only coverage (no % gate). Requires cargo-llvm-cov + llvm-tools-preview.
# One instrumented run; emit both HTML and LCOV from the same profile data.
coverage:
	@if ! $(CARGO) llvm-cov --version >/dev/null 2>&1; then \
		echo "cargo-llvm-cov not found. Install with:"; \
		echo "  rustup component add llvm-tools-preview"; \
		echo "  cargo install cargo-llvm-cov --locked"; \
		exit 1; \
	fi
	@mkdir -p $(TARGET_DIR)/coverage
	$(CARGO) llvm-cov clean --workspace
	$(CARGO) llvm-cov --workspace --no-report $(CARGO_FLAGS)
	$(CARGO) llvm-cov report --html --output-dir $(TARGET_DIR)/coverage
	$(CARGO) llvm-cov report --lcov --output-path $(TARGET_DIR)/coverage/lcov.info
	@echo "HTML report: $(TARGET_DIR)/coverage/html/index.html"
	@echo "LCOV:        $(TARGET_DIR)/coverage/lcov.info"

run:
	$(CARGO) run -p sqyre-app $(CARGO_FLAGS)

tessdata:
	./scripts/download-tessdata.sh

docs-media:
	./scripts/generate-docs-media.sh

appimage: release-gate
	./scripts/linux/packaging/appimage/build-appimage.sh

# Browser GUI editor (no Run / capture / OCR). Requires: rustup target wasm32-unknown-unknown, trunk.
wasm: release-gate
	@command -v trunk >/dev/null 2>&1 || { \
		echo "trunk not found. Install with:"; \
		echo "  cargo install --locked trunk"; \
		echo "Also: rustup target add wasm32-unknown-unknown"; \
		exit 1; \
	}
	rustup target add wasm32-unknown-unknown
	# Trunk's clap rejects NO_COLOR=1 (expects true/false).
	cd crates/sqyre-app && env -u NO_COLOR trunk build --release
	@echo "WASM editor: $(BIN)/wasm/index.html  (serve with: cd crates/sqyre-app && env -u NO_COLOR trunk serve)"
