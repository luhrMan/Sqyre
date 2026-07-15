# Sqyre build helpers. Default output: ./bin (workspace /workspace/bin in devcontainer).
.PHONY: all linux sqyre tessdata windows windows-matprofile appimage appimage-matprofile \
	rust rust-release rust-test rust-run help

ROOT := $(abspath .)
BIN := $(abspath bin)
BUILD_TAGS ?= gocv_specific_modules

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

all: linux

help:
	@echo "Targets:"
	@echo "  linux               - go build -> $(BIN)/sqyre (override BUILD_TAGS=...)"
	@echo "  tessdata            - scripts/download-tessdata.sh"
	@echo "  windows             - cross-compile exe -> $(BIN)/windows-amd64/ (Docker + fyne-cross)"
	@echo "  windows-matprofile  - same with matprofile tag"
	@echo "  appimage            - AppImage -> $(BIN)/ (AppDir still under scripts/linux/packaging/appimage/)"
	@echo "  appimage-matprofile - same with matprofile tag -> $(BIN)/Sqyre-*-matprofile-x86_64.AppImage"
	@echo "  rust                - cargo build (debug) -> $(BIN)/sqyre-rust"
	@echo "  rust-release        - cargo build --release -> $(BIN)/sqyre-rust"
	@echo "  rust-test           - cargo test (rust/ workspace)"
	@echo "  rust-run            - cargo run -p sqyre-app"
	@echo "                        (override with CARGO_FLAGS=...)"

$(BIN):
	mkdir -p $(BIN)

linux: $(BIN)
	go build -trimpath -tags "$(BUILD_TAGS)" -o $(BIN)/sqyre ./cmd/sqyre

tessdata:
	./scripts/download-tessdata.sh

windows: $(BIN)
	env BIN_DIR="$(BIN)" ./scripts/windows/build.sh

windows-matprofile: $(BIN)
	env BIN_DIR="$(BIN)" ./scripts/windows/build-matprofile.sh

appimage:
	./scripts/linux/packaging/appimage/build-appimage.sh

appimage-matprofile: $(BIN)
	./scripts/linux/packaging/appimage/build-appimage-matprofile.sh

rust: $(BIN)
	cd $(RUST_DIR) && $(CARGO) build -p sqyre-app $(CARGO_FLAGS)
	cp -f $(RUST_DIR)/target/debug/sqyre $(BIN)/sqyre-rust

rust-release: $(BIN)
	cd $(RUST_DIR) && $(CARGO) build -p sqyre-app --release $(CARGO_FLAGS)
	cp -f $(RUST_DIR)/target/release/sqyre $(BIN)/sqyre-rust

rust-test:
	cd $(RUST_DIR) && $(CARGO) test $(CARGO_FLAGS)

rust-run:
	cd $(RUST_DIR) && $(CARGO) run -p sqyre-app $(CARGO_FLAGS)
