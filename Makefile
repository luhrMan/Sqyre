# Sqyre build helpers. Default output: ./bin
# Binary is Rust (sqyre-app). Linux AppImage packaging uses the same stack.
.PHONY: all sqyre release test run tessdata appimage docs-media help

ROOT := $(abspath .)
BIN := $(abspath bin)

CARGO ?= cargo
CARGO_FLAGS ?=
# Honor CARGO_TARGET_DIR when set (CI / sandbox); otherwise ./target
TARGET_DIR := $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),$(ROOT)/target)

# Prefer env/devcontainer cargo; fall back to workspace-local toolchain on the host.
# Docker/CI use .cache/cargo (or inherit CARGO_HOME when Make exports .cargo-home).
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
	@echo "  all / sqyre  - cargo build (debug) -> $(BIN)/sqyre  [default]"
	@echo "  release      - cargo build --release -> $(BIN)/sqyre"
	@echo "  test         - cargo test (workspace)"
	@echo "  run          - cargo run -p sqyre-app"
	@echo "  tessdata     - scripts/download-tessdata.sh"
	@echo "  docs-media   - regenerate docs/images screenshots"
	@echo "  appimage     - AppImage -> $(BIN)/ (Docker fallback if tools missing)"
	@echo "                 (RELEASE_VERSION=…; SQYRE_APPIMAGE_FORCE_NATIVE=1)"

$(BIN):
	mkdir -p $(BIN)

sqyre: $(BIN)
	$(CARGO) build -p sqyre-app $(CARGO_FLAGS)
	cp -f $(TARGET_DIR)/debug/sqyre $(BIN)/sqyre

release: $(BIN)
	$(CARGO) build -p sqyre-app --release $(CARGO_FLAGS)
	cp -f $(TARGET_DIR)/release/sqyre $(BIN)/sqyre

test:
	$(CARGO) test $(CARGO_FLAGS)

run:
	$(CARGO) run -p sqyre-app $(CARGO_FLAGS)

tessdata:
	./scripts/download-tessdata.sh

docs-media:
	./scripts/generate-docs-media.sh

appimage:
	./scripts/linux/packaging/appimage/build-appimage.sh
