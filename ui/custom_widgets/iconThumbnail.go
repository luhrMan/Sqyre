package custom_widgets

import (
	"Squire/internal/assets"
	"Squire/internal/config"
	"image/color"
	"path/filepath"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

// IconThumbnail is a custom widget that displays an icon variant with preview, label, and delete button
type IconThumbnail struct {
	widget.BaseWidget

	iconPath    string
	variantName string
	onDelete    func()

	// UI components
	image     *canvas.Image
	label     *widget.Label
	deleteBtn *widget.Button
	container *fyne.Container
}

// NewIconThumbnail creates a new icon thumbnail widget
func NewIconThumbnail(iconPath, variantName string, onDelete func()) *IconThumbnail {
	thumbnail := &IconThumbnail{
		iconPath:    iconPath,
		variantName: variantName,
		onDelete:    onDelete,
	}

	thumbnail.ExtendBaseWidget(thumbnail)
	thumbnail.createUI()

	return thumbnail
}

// createUI initializes the UI components
func (t *IconThumbnail) createUI() {
	// Load the icon image
	t.image = t.loadIcon()

	// Create variant name label
	t.label = widget.NewLabel(t.variantName)
	t.label.Alignment = fyne.TextAlignCenter
	t.label.Truncation = fyne.TextTruncateEllipsis

	// Create delete button with danger styling
	t.deleteBtn = widget.NewButtonWithIcon("", theme.CancelIcon(), func() {
		if t.onDelete != nil {
			t.onDelete()
		}
	})
	t.deleteBtn.Importance = widget.LowImportance

	// Layout: button top right corner, image in middle, label on bottom
	t.container = container.NewVBox(
		container.NewBorder(
			container.NewHBox(layout.NewSpacer(), t.deleteBtn),
			nil,
			nil,
			nil,
			container.NewCenter(t.image),
		),
		t.label,
	)
}

// loadIcon loads the PNG image from cache or returns a placeholder if missing/corrupted
func (t *IconThumbnail) loadIcon() *canvas.Image {
	// Construct cache key from icon path
	key := t.constructIconKey()
	if key == "" {
		return t.createPlaceholder(true)
	}

	// Get cached canvas.Image (includes decoded pixel data)
	// This prevents memory bloat from repeatedly decoding the same PNG
	img := assets.GetCanvasImage(
		key,
		fyne.NewSize(config.IconThumbnailSize, config.IconThumbnailSize),
		canvas.ImageFillContain,
	)

	if img != nil {
		return img
	}

	// Resource not found in cache, return placeholder
	return t.createPlaceholder(true)
}

// constructIconKey constructs the cache key from the icon file path
// Key format: "programName|filename.png"
// Example: "/home/user/.sqyre/images/icons/dark and darker/Health Potion.png"
//
//	-> "dark and darker|Health Potion.png"
func (t *IconThumbnail) constructIconKey() string {
	if t.iconPath == "" {
		return ""
	}

	// Get the filename (e.g., "Health Potion.png" or "Health Potion|Variant1.png")
	filename := filepath.Base(t.iconPath)

	// Get the parent directory name (program name, e.g., "dark and darker")
	parentDir := filepath.Base(filepath.Dir(t.iconPath))

	// Construct key: programName|filename
	key := parentDir + config.ProgramDelimiter + filename

	//log.Printf("DEBUG: IconThumbnail constructIconKey - iconPath: %s, key: %s, variant: %s", t.iconPath, key, t.variantName)
	return key
}

// createPlaceholder creates a placeholder image with error indicator
func (t *IconThumbnail) createPlaceholder(showError bool) *canvas.Image {
	// Create a rectangle as placeholder
	rect := canvas.NewRectangle(color.RGBA{R: 200, G: 200, B: 200, A: 255})
	rect.SetMinSize(fyne.NewSize(config.IconThumbnailSize, config.IconThumbnailSize))

	// For now, return a simple gray placeholder
	// In a real implementation, you might want to create an actual placeholder image
	img := canvas.NewImageFromResource(theme.BrokenImageIcon())
	img.FillMode = canvas.ImageFillContain
	img.SetMinSize(fyne.NewSize(config.IconThumbnailSize, config.IconThumbnailSize))

	return img
}

// CreateRenderer creates the widget renderer
func (t *IconThumbnail) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(t.container)
}

// SetIconPath updates the icon path and reloads the image
func (t *IconThumbnail) SetIconPath(path string) {
	t.iconPath = path

	// Invalidate cache for the new path to ensure fresh load from disk
	key := t.constructIconKey()
	if key != "" {
		assets.InvalidateFyneResourceCache(key)
	}

	// Reload the image and update the existing UI
	newImage := t.loadIcon()
	if t.image != nil && t.container != nil {
		// Update the existing image in the container instead of recreating everything
		t.image.Resource = newImage.Resource
		t.image.Refresh()
	} else {
		// Fallback to full recreation if components don't exist yet
		t.image = newImage
		t.createUI()
	}

	t.Refresh()
}

// SetVariantName updates the variant name label
func (t *IconThumbnail) SetVariantName(name string) {
	t.variantName = name
	if t.label != nil {
		t.label.SetText(name)
	}
}

// SetOnDelete updates the delete callback
func (t *IconThumbnail) SetOnDelete(callback func()) {
	t.onDelete = callback
	if t.deleteBtn != nil {
		t.deleteBtn.OnTapped = callback
	}
}
