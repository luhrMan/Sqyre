package custom_widgets

import (
	"image/color"
	"os"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
)

const IconThumbnailSize = 64

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
	t.deleteBtn = widget.NewButton("Delete", func() {
		if t.onDelete != nil {
			t.onDelete()
		}
	})
	t.deleteBtn.Importance = widget.DangerImportance

	// Layout: image on top, label in middle, delete button at bottom
	t.container = container.NewVBox(
		container.NewCenter(t.image),
		t.label,
		t.deleteBtn,
	)
}

// loadIcon loads the PNG image or returns a placeholder if missing/corrupted
func (t *IconThumbnail) loadIcon() *canvas.Image {
	// Check if file exists and is readable
	if _, err := os.Stat(t.iconPath); err != nil {
		return t.createPlaceholder(true)
	}

	// Try to load the image
	img := canvas.NewImageFromFile(t.iconPath)
	if img == nil {
		return t.createPlaceholder(true)
	}

	// Validate it's a PNG by checking the file
	if !t.validatePNG(t.iconPath) {
		return t.createPlaceholder(true)
	}

	// Configure image display
	img.FillMode = canvas.ImageFillContain
	img.SetMinSize(fyne.NewSize(IconThumbnailSize, IconThumbnailSize))

	return img
}

// validatePNG checks if the file has a valid PNG header
func (t *IconThumbnail) validatePNG(path string) bool {
	file, err := os.Open(path)
	if err != nil {
		return false
	}
	defer file.Close()

	// Read first 8 bytes to check PNG signature
	header := make([]byte, 8)
	n, err := file.Read(header)
	if err != nil || n != 8 {
		return false
	}

	// PNG signature: \x89PNG\r\n\x1a\n
	pngSignature := []byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}
	for i := 0; i < 8; i++ {
		if header[i] != pngSignature[i] {
			return false
		}
	}

	return true
}

// createPlaceholder creates a placeholder image with error indicator
func (t *IconThumbnail) createPlaceholder(showError bool) *canvas.Image {
	// Create a rectangle as placeholder
	rect := canvas.NewRectangle(color.RGBA{R: 200, G: 200, B: 200, A: 255})
	rect.SetMinSize(fyne.NewSize(IconThumbnailSize, IconThumbnailSize))

	// For now, return a simple gray placeholder
	// In a real implementation, you might want to create an actual placeholder image
	img := canvas.NewImageFromResource(theme.BrokenImageIcon())
	img.FillMode = canvas.ImageFillContain
	img.SetMinSize(fyne.NewSize(IconThumbnailSize, IconThumbnailSize))

	return img
}

// CreateRenderer creates the widget renderer
func (t *IconThumbnail) CreateRenderer() fyne.WidgetRenderer {
	return widget.NewSimpleRenderer(t.container)
}

// SetIconPath updates the icon path and reloads the image
func (t *IconThumbnail) SetIconPath(path string) {
	t.iconPath = path
	t.image = t.loadIcon()
	t.createUI()
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
