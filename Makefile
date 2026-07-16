# Sqyre build helpers. Default output: ./bin
# Binary is Rust (sqyre-app). Linux AppImage packaging uses the same stack.
.PHONY: all sqyre rust rust-release rust-test rust-run run \
	tessdata appimage help

ROOT := $(abspath .)
BIN := $(abspath bin)

RUST_DIR := $(ROOT)/rust
CARGO ?= cargo
CARGO_FLAGS ?=

# Prefer env/devcontainer cargo; fall back to workspace-local toolchain on the host.
ifeq ($(origin CARGO_HOME),undefined)
  ifneq ($(wildcard $(ROOT)/.cargo-home/bin/cargo),)
    export CARGO_HOME := $(ROOT)/.cargo-home
  endif
endif
ifeq ($(origin RUSTUP_HOME),undefined)
  ifneq ($(wildcard $(ROOT)/.rustup-home),)
    export RUSTUP_HOME := $(ROOT)/.rustup-home
  endif
endif
ifneq ($(wildcard $(CARGO_HOME)/bin/cargo),)
  export PATH := $(CARGO_HOME)/bin:$(PATH)
endif

all: sqyre

help:
	@echo "Targets:"
	@echo "  all / sqyre / rust  - cargo build (debug) -> $(BIN)/sqyre  [default]"
	@echo "  rust-release        - cargo build --release -> $(BIN)/sqyre"
	@echo "  rust-test           - cargo test (rust/ workspace)"
	@echo "  run / rust-run      - cargo run -p sqyre-app"
	@echo "  tessdata            - scripts/download-tessdata.sh"
	@echo "  appimage            - Rust AppImage -> $(BIN)/ (Docker fallback if tools missing)"
	@echo "                        (RELEASE_VERSION=…; SQYRE_APPIMAGE_FORCE_NATIVE=1)"

$(BIN):
	mkdir -p $(BIN)

sqyre rust: $(BIN)
	cd $(RUST_DIR) && $(CARGO) build -p sqyre-app $(CARGO_FLAGS)
	cp -f $(RUST_DIR)/target/debug/sqyre $(BIN)/sqyre

rust-release: $(BIN)
	cd $(RUST_DIR) && $(CARGO) build -p sqyre-app --release $(CARGO_FLAGS)
	cp -f $(RUST_DIR)/target/release/sqyre $(BIN)/sqyre

rust-test:
	cd $(RUST_DIR) && $(CARGO) test $(CARGO_FLAGS)

run rust-run:
	cd $(RUST_DIR) && $(CARGO) run -p sqyre-app $(CARGO_FLAGS)

tessdata:
	./scripts/download-tessdata.sh

appimage:
	./scripts/linux/packaging/appimage/build-appimage.sh
