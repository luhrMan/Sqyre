# Sqyre build helpers. Default output: ./bin (workspace /workspace/bin in devcontainer).
.PHONY: all linux sqyre tessdata windows windows-matprofile appimage wasm wasm-serve help

BIN := $(abspath bin)
ICON := $(abspath internal/assets/icons/sqyre.png)
WASM_PORT ?= 8080
FYNE := fyne
BUILD_TAGS ?= gocv_specific_modules

all: linux

help:
	@echo "Targets:"
	@echo "  linux  - go build -> $(BIN)/sqyre (override BUILD_TAGS=...)"
	@echo "  tessdata       - scripts/download-tessdata.sh"
	@echo "  windows        - cross-compile exe -> $(BIN)/windows-amd64/ (Docker + fyne-cross)"
	@echo "  windows-matprofile - same with matprofile tag"
	@echo "  appimage       - AppImage -> $(BIN)/ (AppDir still under scripts/linux/packaging/appimage/)"
	@echo "  wasm           - browser demo -> $(BIN)/sqyre.wasm (needs CGO_ENABLED=0; set automatically)"
	@echo "  wasm-serve     - fyne serve web demo (CGO_ENABLED=0; port WASM_PORT=$(WASM_PORT))"

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

wasm: $(BIN)
	CGO_ENABLED=0 GOOS=js GOARCH=wasm go build -trimpath -buildvcs=false -o $(BIN)/sqyre.wasm ./cmd/sqyre-wasm

wasm-serve:
	CGO_ENABLED=0 $(FYNE) serve --src cmd/sqyre-wasm --icon $(ICON) --http-port $(WASM_PORT)
