#!/bin/bash
docker build -f .devcontainer/builds/Dockerfile.windows-amd64 -t fyne-cross-windows:local .
fyne-cross windows -image fyne-cross-windows:local ./cmd/sqyre