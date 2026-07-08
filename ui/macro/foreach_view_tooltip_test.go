package macro

import (
	"strings"
	"testing"

	"Sqyre/internal/models/actions"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/widget"
)

func collectText(obj fyne.CanvasObject, out *[]string) {
	switch o := obj.(type) {
	case *canvas.Text:
		*out = append(*out, o.Text)
	case *widget.Label:
		*out = append(*out, o.Text)
	case *fyne.Container:
		for _, c := range o.Objects {
			collectText(c, out)
		}
	case fyne.Widget:
		for _, c := range test.WidgetRenderer(o).Objects() {
			collectText(c, out)
		}
	}
}

func viewTextContains(t *testing.T, view fyne.CanvasObject, want string) bool {
	t.Helper()
	var texts []string
	collectText(view, &texts)
	return strings.Contains(strings.Join(texts, "\x00"), want)
}

func TestForEachRowViewTooltip_showsSourceOutputVariable(t *testing.T) {
	test.NewApp()
	fer := actions.NewForEachRow("rows", []actions.ListColumn{
		{Source: "list.txt", OutputVar: "col", IsFile: true},
	}, nil)

	view := viewParamPills(fer, fer.GetType())
	if view == nil {
		t.Fatal("expected a view tooltip for for-each-row")
	}
	if !viewTextContains(t, view, "col") {
		t.Fatal("for-each view tooltip should display the source output variable name")
	}
	if !viewTextContains(t, view, "rows") {
		t.Fatal("for-each view tooltip should display the name")
	}
}
