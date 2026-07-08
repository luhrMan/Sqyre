# Sqyre build helpers. Default output: ./bin (workspace /workspace/bin in devcontainer).
.PHONY: all linux sqyre sqyre-vision sqyre-vision-embed linux-vision \
	tessdata clip-tokenizer vision-models \
	windows windows-matprofile appimage appimage-matprofile appimage-vision help

BIN := $(abspath bin)
BUILD_TAGS ?= gocv_specific_modules

all: linux

help:
	@echo "Targets:"
	@echo "  linux              - go build -> $(BIN)/sqyre (override BUILD_TAGS=...)"
	@echo "  sqyre-vision       - lean vision worker -> $(BIN)/sqyre-vision (uses ~/.sqyre/models)"
	@echo "  sqyre-vision-embed - embedded vision worker -> $(BIN)/sqyre-vision (models inside binary)"
	@echo "  linux-vision       - $(BIN)/sqyre + embedded $(BIN)/sqyre-vision"
	@echo "  tessdata           - scripts/download-tessdata.sh"
	@echo "  clip-tokenizer     - scripts/vision/download-clip-tokenizer.sh"
	@echo "  vision-models      - scripts/vision/download-models.sh (tokenizer + ONNX)"
	@echo "  windows            - cross-compile exe -> $(BIN)/windows-amd64/ (Docker + fyne-cross)"
	@echo "  windows-matprofile - same with matprofile tag"
	@echo "  appimage           - AppImage -> $(BIN)/ (lean Sqyre only)"
	@echo "  appimage-matprofile - same with matprofile tag"
	@echo "  appimage-vision    - AppImage with sqyre + embedded sqyre-vision"

$(BIN):
	mkdir -p $(BIN)

linux: $(BIN)
	go build -trimpath -tags "$(BUILD_TAGS)" -o $(BIN)/sqyre ./cmd/sqyre

sqyre-vision: $(BIN)
	./scripts/vision/build-worker-external.sh $(BIN)/sqyre-vision

sqyre-vision-embed: $(BIN)
	./scripts/vision/build-worker.sh $(BIN)/sqyre-vision

linux-vision: linux sqyre-vision-embed

tessdata:
	./scripts/download-tessdata.sh

clip-tokenizer:
	./scripts/vision/download-clip-tokenizer.sh

vision-models:
	./scripts/vision/download-models.sh

windows: $(BIN)
	env BIN_DIR="$(BIN)" ./scripts/windows/build.sh

windows-matprofile: $(BIN)
	env BIN_DIR="$(BIN)" ./scripts/windows/build-matprofile.sh

appimage:
	./scripts/linux/packaging/appimage/build-appimage.sh

appimage-matprofile: $(BIN)
	./scripts/linux/packaging/appimage/build-appimage-matprofile.sh

appimage-vision:
	./scripts/linux/packaging/appimage/build-appimage-vision.sh
