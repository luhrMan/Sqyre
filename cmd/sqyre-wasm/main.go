// Package main is a browser-only demo: minimal Fyne UI with no desktop automation deps.
//
// WebAssembly requires CGO off. If the environment sets CGO_ENABLED=1 (e.g. for native
// OpenCV builds), force it off or std/os/user fails to compile for js/wasm.
//
// Build (from repo root):
//
//	CGO_ENABLED=0 GOOS=js GOARCH=wasm go build -trimpath -buildvcs=false -o bin/sqyre.wasm ./cmd/sqyre-wasm
//
// Local preview with the Fyne CLI (icon path must resolve from repo root):
//
//	CGO_ENABLED=0 fyne serve --src cmd/sqyre-wasm --icon internal/assets/icons/sqyre.png
//
// See https://docs.fyne.io/started/webapp/
package main

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

func main() {
	a := app.NewWithID("com.sqyre.app.webdemo")
	w := a.NewWindow("Sqyre (web demo)")
	w.Resize(fyne.NewSize(480, 320))

	intro := widget.NewRichTextFromMarkdown(
		"**Sqyre** — browser preview.\n\n" +
			"The full app runs on the desktop; this build is UI-only in WebAssembly.",
	)
	intro.Wrapping = fyne.TextWrapWord

	w.SetContent(container.NewVBox(
		widget.NewLabel("Sqyre"),
		intro,
		widget.NewSeparator(),
		widget.NewButton("Close", func() { w.Close() }),
	))
	w.ShowAndRun()
}
