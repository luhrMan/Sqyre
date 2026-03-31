// Package main runs the Sqyre UI shell in WebAssembly (no desktop automation).
//
// WebAssembly requires CGO off. If the environment sets CGO_ENABLED=1 (e.g. for native
// OpenCV builds), force it off or std/os/user fails to compile for js/wasm.
//
// Build (from repo root):
//
//	CGO_ENABLED=0 GOOS=js GOARCH=wasm go build -trimpath -buildvcs=false -o bin/sqyre-wasm.wasm ./cmd/sqyre-wasm
//
// Local preview with the Fyne CLI (icon path must resolve from repo root):
//
//	CGO_ENABLED=0 fyne serve --src cmd/sqyre-wasm --icon internal/assets/icons/sqyre.png
//
// See https://docs.fyne.io/started/webapp/
package main

import (
	"Sqyre/internal/appdata"
	_ "Sqyre/internal/models/repositories" // registers Item/Mask/Point repository factories (same as desktop main)
	"Sqyre/ui"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
)

func main() {
	mem := appdata.NewMemory()
	mem.SeedDemo()
	appdata.Register(appdata.MemoryPrograms{M: mem}, appdata.MemoryMacros{M: mem})

	a := app.NewWithID("com.sqyre.app.webdemo")
	w := a.NewWindow("Sqyre (web demo)")
	w.Resize(fyne.NewSize(1100, 720))

	u := ui.InitializeUi(w)
	u.ConstructUi()
	w.ShowAndRun()
}
