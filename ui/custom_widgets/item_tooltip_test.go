package custom_widgets

import (
	"testing"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/theme"
)

func TestRemoveLayerObject_keepsOtherObjects(t *testing.T) {
	t.Helper()
	test.NewApp()
	keep := canvas.NewRectangle(theme.Color(theme.ColorNamePrimary))
	remove := canvas.NewRectangle(theme.Color(theme.ColorNameBackground))

	layer := &ItemTooltipLayer{}
	layer.Container.Objects = []fyne.CanvasObject{keep, remove}

	remaining := removeLayerObject(layer, remove)
	if len(remaining) != 1 || remaining[0] != keep {
		t.Fatalf("expected one remaining object, got %d", len(remaining))
	}
	if len(layer.Container.Objects) != 1 || layer.Container.Objects[0] != keep {
		t.Fatalf("layer should keep unrelated object")
	}
}

func TestItemTooltipLabel_hideTooltip_keepsOtherLayerObjects(t *testing.T) {
	t.Helper()
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	label := NewItemTooltipLabel()
	keep := canvas.NewRectangle(theme.Color(theme.ColorNamePrimary))
	itemPanel := newItemTooltipPanel("demo", nil)
	label.tooltipPanel = itemPanel

	content := container.NewStack(label)
	w.SetContent(AddWindowItemTooltipLayer(content, w.Canvas()))
	layer := findItemTooltipLayer(w.Canvas(), nil)
	if layer == nil {
		t.Fatal("expected item tooltip layer")
	}
	layer.Container.Objects = []fyne.CanvasObject{keep, itemPanel}

	label.hideTooltip()

	if len(layer.Container.Objects) != 1 || layer.Container.Objects[0] != keep {
		t.Fatalf("expected layer to keep unrelated object, got %d objects", len(layer.Container.Objects))
	}
	if label.tooltipPanel != nil {
		t.Fatal("expected item tooltip panel reference cleared")
	}
}

func TestItemTooltipLabel_showTooltip_skipsWhenLayerBusy(t *testing.T) {
	t.Helper()
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	label := NewItemTooltipLabel()
	label.SetItem("busy", nil)

	content := container.NewStack(label)
	w.SetContent(AddWindowItemTooltipLayer(content, w.Canvas()))
	layer := findItemTooltipLayer(w.Canvas(), nil)
	if layer == nil {
		t.Fatal("expected item tooltip layer")
	}
	layer.Container.Objects = []fyne.CanvasObject{canvas.NewRectangle(theme.Color(theme.ColorNamePrimary))}

	label.showTooltip()

	if len(layer.Container.Objects) != 1 {
		t.Fatalf("expected showTooltip to leave busy layer alone, got %d objects", len(layer.Container.Objects))
	}
	if label.tooltipPanel != nil {
		t.Fatal("expected no item tooltip panel when layer is busy")
	}
}
