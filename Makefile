# Sqyre build helpers. Default output: ./bin (workspace /workspace/bin in devcontainer).
# Default binary is Rust (sqyre-app). Go/Fyne remains under `make go` until full deletion.
.PHONY: all sqyre rust rust-release rust-test rust-run run \
	go fyne tessdata windows windows-matprofile appimage appimage-matprofile help

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

all: sqyre

help:
	@echo "Targets:"
	@echo "  all / sqyre / rust  - cargo build (debug) -> $(BIN)/sqyre  [default]"
	@echo "  rust-release        - cargo build --release -> $(BIN)/sqyre"
	@echo "  rust-test           - cargo test (rust/ workspace)"
	@echo "  run / rust-run      - cargo run -p sqyre-app"
	@echo "  go / fyne           - go build -> $(BIN)/sqyre-go (legacy Fyne app)"
	@echo "  tessdata            - scripts/download-tessdata.sh"
	@echo "  windows             - cross-compile Go exe -> $(BIN)/windows-amd64/ (Docker + fyne-cross)"
	@echo "  windows-matprofile  - same with matprofile tag"
	@echo "  appimage            - Go AppImage -> $(BIN)/ (AppDir under scripts/linux/packaging/appimage/)"
	@echo "  appimage-matprofile - same with matprofile tag -> $(BIN)/Sqyre-*-matprofile-x86_64.AppImage"
	@echo "                        (override Rust with CARGO_FLAGS=...; Go with BUILD_TAGS=...)"

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

go fyne: $(BIN)
	go build -trimpath -tags "$(BUILD_TAGS)" -o $(BIN)/sqyre-go ./cmd/sqyre

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
