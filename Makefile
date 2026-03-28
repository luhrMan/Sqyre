# Sqyre build helpers. Default output: ./bin (workspace /workspace/bin in devcontainer).
.PHONY: all linux sqyre tessdata windows windows-matprofile appimage help

BIN := $(abspath bin)
BUILD_TAGS ?= gocv_specific_modules

all: linux

help:
	@echo "Targets:"
	@echo "  linux  - go build -> $(BIN)/sqyre (override BUILD_TAGS=...)"
	@echo "  tessdata       - scripts/download-tessdata.sh"
	@echo "  windows        - cross-compile exe -> $(BIN)/windows-amd64/ (Docker + fyne-cross)"
	@echo "  windows-matprofile - same with matprofile tag"
	@echo "  appimage       - AppImage -> $(BIN)/ (AppDir still under scripts/linux/packaging/appimage/)"

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
