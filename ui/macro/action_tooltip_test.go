package macro

import (
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/driver/desktop"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"
	"image/color"
)

func TestViewParamPills_AllActionTypes(t *testing.T) {
	t.Helper()
	cases := []struct {
		name   string
		node   actions.ActionInterface
	}{
		{"click", actions.NewClick(actions.ClickButtonLeft, false)},
		{"key", actions.NewKey("a", true)},
		{"wait", actions.NewWait(100)},
		{"move", actions.NewMove(actions.CoordinateRef(""), false)},
		{"loop", actions.NewLoop(3, "inner", nil)},
		{"conditional", actions.NewConditional(nil, actions.MatchAll, "c", nil)},
		{"setvariable", actions.NewSetVariable("x", 1)},
		{"calculate", actions.NewCalculate("1+1", "out")},
		{"runmacro", actions.NewRunMacro("other")},
		{"break", actions.NewBreak()},
		{"continue", actions.NewContinue()},
		{"type", actions.NewType("hello", 0)},
		{"pause", actions.NewPause("wait", nil, false)},
		{"savevariable", actions.NewSaveVariable("v", "dest", false, false)},
		{"focuswindow", actions.NewFocusWindow("title", "path")},
		{"foreachrow", actions.NewForEachRow("rows", nil, nil)},
		{"imagesearch", actions.NewImageSearch("s", nil, nil, actions.CoordinateRef(""), 1, 1, 0.95, 0)},
		{"findpixel", actions.NewFindPixel("f", actions.CoordinateRef(""), "ffffff", 0)},
		{"ocr", actions.NewOcr("o", "target", actions.CoordinateRef(""))},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if viewParamPills(tc.node, tc.node.GetType()) == nil {
				t.Fatalf("%s: expected view param pills", tc.name)
			}
		})
	}
}

func TestColorSwatchPill(t *testing.T) {
	t.Helper()
	if colorSwatchPill("ff8800", "findpixel") == nil {
		t.Fatal("expected swatch pill for valid hex")
	}
	if colorSwatchPill("aaff8800", "findpixel") == nil {
		t.Fatal("expected swatch pill for 8-char hex")
	}
	if colorSwatchPill("${color}", "findpixel") != nil {
		t.Fatal("variable reference should not render a swatch")
	}
	if colorSwatchPill("nothex", "findpixel") != nil {
		t.Fatal("malformed hex should not render a swatch")
	}
	c, ok := parseHexColor("#ff8800")
	if !ok || c.R != 0xff || c.G != 0x88 || c.B != 0x00 || c.A != 0xff {
		t.Fatalf("parseHexColor = %+v ok=%v, want ff8800 opaque", c, ok)
	}
}

func TestEditableColorSwatchPill_LiveUpdate(t *testing.T) {
	t.Helper()
	pill, update := editableColorSwatchPill("ff0000", "findpixel")
	if !pill.Visible() {
		t.Fatal("swatch should be visible for a valid initial hex")
	}
	update("${var}")
	if pill.Visible() {
		t.Fatal("swatch should hide when value becomes a variable reference")
	}
	update("00ff00")
	if !pill.Visible() {
		t.Fatal("swatch should reappear for a valid hex")
	}
}

func TestFindPixelEdit_SwatchTracksHexEntry(t *testing.T) {
	t.Helper()
	test.NewApp()
	fp := actions.NewFindPixel("f", actions.CoordinateRef(""), "ff0000", 0)
	pills, _ := buildParamEditPills(fp, fp.GetType(), nil, nil)
	if pills == nil {
		t.Fatal("expected FindPixel edit pills")
	}
	if !containsSwatchColor(pills, color.NRGBA{R: 0xff, A: 0xff}) {
		t.Fatal("expected initial red swatch in edit pills")
	}

	entry := findBorderlessEntryWithText(pills, "ff0000")
	if entry == nil {
		t.Fatal("expected a color entry in the FindPixel edit form")
	}
	entry.SetText("")
	test.Type(entry, "00ff00")

	if !containsSwatchColor(pills, color.NRGBA{G: 0xff, A: 0xff}) {
		t.Fatal("swatch should recolor to green when the hex entry changes")
	}
	if containsSwatchColor(pills, color.NRGBA{R: 0xff, A: 0xff}) {
		t.Fatal("stale red swatch should be gone after edit")
	}
}

func findBorderlessEntryWithText(obj fyne.CanvasObject, text string) *custom_widgets.BorderlessEntry {
	switch o := obj.(type) {
	case *custom_widgets.BorderlessEntry:
		if o.Text == text {
			return o
		}
	case *fyne.Container:
		for _, c := range o.Objects {
			if e := findBorderlessEntryWithText(c, text); e != nil {
				return e
			}
		}
	case fyne.Widget:
		for _, c := range test.WidgetRenderer(o).Objects() {
			if e := findBorderlessEntryWithText(c, text); e != nil {
				return e
			}
		}
	}
	return nil
}

func containsSwatchColor(obj fyne.CanvasObject, want color.NRGBA) bool {
	switch o := obj.(type) {
	case *canvas.Rectangle:
		c, ok := o.FillColor.(color.NRGBA)
		return ok && c == want
	case *fyne.Container:
		for _, child := range o.Objects {
			if containsSwatchColor(child, want) {
				return true
			}
		}
	case fyne.Widget:
		for _, child := range test.WidgetRenderer(o).Objects() {
			if containsSwatchColor(child, want) {
				return true
			}
		}
	}
	return false
}

func TestActionTooltipPanel_viewModeTypeHeaderCentered(t *testing.T) {
	t.Helper()
	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	panel := newActionDisplayTooltipPanel(hover)
	if len(panel.body.Objects) < 2 {
		t.Fatalf("expected header and param sections, got %d body rows", len(panel.body.Objects))
	}
	center, ok := panel.body.Objects[0].(*fyne.Container)
	if !ok || len(center.Objects) != 1 {
		t.Fatal("expected centered action type header at top of tooltip")
	}
	assertTypePillText(t, center.Objects[0], actions.ActionTypeLabel("wait"))
	layout.NewVBoxLayout().Layout(panel.body.Objects, fyne.NewSize(500, 400))
	if pillW := center.Objects[0].(*fyne.Container).Size().Width; pillW > 120 {
		t.Fatalf("type pill width = %v, want text-sized pill", pillW)
	}
	if isWrappedTooltipSection(panel.body.Objects[1]) {
		t.Fatal("view param pills should not be wrapped in an extra tooltip section")
	}
}

func TestActionTooltipPanel_editModeHeaderRow(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	panel := newActionDisplayTooltipPanel(hover)
	panel.enterEditMode()
	if len(panel.body.Objects) == 0 {
		t.Fatal("expected edit toolbar row")
	}
	row, ok := panel.body.Objects[0].(*fyne.Container)
	if !ok || len(row.Objects) < 4 {
		t.Fatal("expected edit header row with type pill and save/cancel buttons")
	}
	pill, ok := row.Objects[0].(*actiondisplay.HoverTipPill)
	if !ok {
		t.Fatal("expected edit type pill to be a HoverTipPill")
	}
	if got := pill.Label(); got != actions.ActionTypeLabel("wait") {
		t.Fatalf("edit type pill label = %q, want %q", got, actions.ActionTypeLabel("wait"))
	}
	if got := pill.ToolTip(); got != actions.ActionTypeDescription("wait") {
		t.Fatalf("edit type pill tooltip = %q, want %q", got, actions.ActionTypeDescription("wait"))
	}
}

func TestActionDisplayTooltipHover_showTooltipPanel_preservesEditMode(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	panel := newActionDisplayTooltipPanel(hover)
	hover.tooltipPanel = panel
	panel.enterEditMode()
	if !panel.editing {
		t.Fatal("expected edit mode")
	}

	hover.showTooltipPanel()

	if !panel.editing {
		t.Fatal("showTooltipPanel must not exit edit mode")
	}
}

func TestActionDisplayTooltipHover_TappedSecondary_opensEditMode(t *testing.T) {
	t.Helper()
	ResetActionTooltipOwnershipForTesting()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	is := actions.NewImageSearch("find", nil, []string{"Demo~Item"}, actions.NewCoordinateRef("Demo", "Main"), 1, 1, 0.9, 0)
	hover := newActionDisplayTooltipHover(is, canvas.NewRectangle(color.Transparent), nil, is.GetType(), nil, nil)
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(hover, w.Canvas()))

	hover.TappedSecondary(nil)
	if hover.tooltipPanel == nil || !hover.tooltipPanel.editing {
		t.Fatal("right-click should open tooltip in edit mode")
	}
}

func TestActionIconTooltipHover_forwardsHoverToTarget(t *testing.T) {
	t.Helper()
	is := actions.NewImageSearch("find", nil, []string{"Demo~Item"}, actions.CoordinateRef(""), 1, 1, 0.9, 0)
	target := newActionDisplayTooltipHover(is, nil, nil, is.GetType(), nil, nil)
	iconHover := newActionIconTooltipHover()
	iconHover.bindActionTooltip(target)

	iconHover.MouseIn(&desktop.MouseEvent{
		PointEvent: fyne.PointEvent{AbsolutePosition: fyne.NewPos(10, 20)},
	})
	if !target.iconHovering {
		t.Fatal("expected icon hover to mark target as icon-hovering")
	}

	iconHover.MouseOut()
	if target.iconHovering || target.displayHovering {
		t.Fatal("expected hover flags cleared after mouse out")
	}
}

func TestActionRowTooltipHover_forwardsHoverToTarget(t *testing.T) {
	t.Helper()
	wait := actions.NewWait(100)
	target := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	rowHover := newActionRowTooltipHover()
	rowHover.bindActionTooltip(target)

	rowHover.MouseIn(&desktop.MouseEvent{
		PointEvent: fyne.PointEvent{AbsolutePosition: fyne.NewPos(50, 20)},
	})
	if !target.rowHovering {
		t.Fatal("expected row hover to mark target as row-hovering")
	}

	rowHover.MouseOut()
	if target.rowHovering || target.displayHovering {
		t.Fatal("expected hover flags cleared after mouse out")
	}
}

// The row overlay sits on top of the whole row, so primary taps must be
// forwarded to the display; otherwise Fyne drops them and selection/double-click
// edit never fire.
func TestActionRowTooltipHover_primaryTapSelectsRow(t *testing.T) {
	t.Helper()
	wait := actions.NewWait(100)
	root := actions.NewLoop(1, "root", nil)
	root.AddSubAction(wait)
	mt := &MacroTree{Macro: &models.Macro{Root: root}}

	rowBody := newTreeRowBody(container.NewHScroll(widget.NewLabel("wait")))
	rowBody.tree = mt
	rowBody.uid = wait.GetUID()

	target := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	target.bindRowBody(rowBody)
	rowHover := newActionRowTooltipHover()
	rowHover.bindActionTooltip(target)

	rowHover.Tapped(nil)
	if mt.SelectedNode != wait.GetUID() {
		t.Fatalf("row overlay tap selected %q, want %q", mt.SelectedNode, wait.GetUID())
	}
}

func TestShouldFollowMouse_viewModeDoesNotFollowOverPanel(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)

	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	panel := newActionDisplayTooltipPanel(hover)
	hover.tooltipPanel = panel
	hover.refreshTooltipPanelGeometry(fyne.NewPos(0, 0), fyne.NewPos(20, 20), fyne.NewSize(120, 80))
	hover.absoluteMousePos = fyne.NewPos(40, 40)

	if hover.shouldFollowMouse() {
		t.Fatal("view tooltip should not follow while pointer is only inside panel bounds")
	}
}

func TestShouldKeepViewTooltip_dismissesWhenOnlyOverPanel(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	row := canvas.NewRectangle(color.White)
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(row, w.Canvas()))
	panel := newActionDisplayTooltipPanel(hover)
	hover.tooltipPanel = panel
	layer := custom_widgets.FindItemTooltipLayer(w.Canvas(), nil)
	if layer == nil {
		t.Fatal("expected item tooltip layer")
	}
	layer.Container.Objects = []fyne.CanvasObject{panel}
	hover.tooltipMounted = true
	hover.refreshTooltipPanelGeometry(fyne.NewPos(0, 0), fyne.NewPos(20, 20), fyne.NewSize(120, 80))
	hover.absoluteMousePos = fyne.NewPos(40, 40)

	if hover.shouldKeepViewTooltip() {
		t.Fatal("view tooltip should dismiss when pointer left the tree action row")
	}
}

func TestPointerInTooltipPanel_outsideTreeActionSpace(t *testing.T) {
	t.Helper()
	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	panel := newActionDisplayTooltipPanel(hover)
	hover.tooltipPanel = panel
	hover.refreshTooltipPanelGeometry(fyne.NewPos(0, 0), fyne.NewPos(20, 20), fyne.NewSize(120, 80))
	hover.absoluteMousePos = fyne.NewPos(40, 40)

	if hover.pointerInTreeActionSpace(hover.absoluteMousePos) {
		t.Fatal("pointer over panel alone should be outside tree action space")
	}
	if hover.shouldFollowMouse() {
		t.Fatal("view tooltip should not follow pointer that left the tree action row")
	}
}

func TestWithinActionLimits_dismissesOutsideRowAndPanel(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	row := canvas.NewRectangle(color.White)
	row.Resize(fyne.NewSize(200, 24))
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(row, w.Canvas()))
	panel := newActionDisplayTooltipPanel(hover)
	hover.tooltipPanel = panel
	layer := custom_widgets.FindItemTooltipLayer(w.Canvas(), nil)
	if layer == nil {
		t.Fatal("expected item tooltip layer")
	}
	layer.Container.Objects = []fyne.CanvasObject{panel}
	hover.tooltipMounted = true
	hover.absoluteMousePos = fyne.NewPos(9999, 9999)

	if hover.pointerInTreeActionSpace(hover.absoluteMousePos) {
		t.Fatal("pointer outside row should be outside tree action space")
	}
	if hover.shouldFollowMouse() {
		t.Fatal("view tooltip should not follow when pointer is outside tree action space")
	}
	if hover.shouldKeepViewTooltip() {
		t.Fatal("view tooltip should dismiss when pointer is outside tree action space")
	}
}

func TestTrackMouseForTooltip_dismissesOutsideActionLimit(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	row := canvas.NewRectangle(theme.Color(theme.ColorNameBackground))
	row.Resize(fyne.NewSize(200, 24))
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(row, w.Canvas()))
	w.Resize(fyne.NewSize(400, 300))
	hover.setTooltipKeepAliveArea(row)

	hover.rowHovering = true
	hover.openViewTooltip()
	if !hover.isTooltipMounted() {
		t.Fatal("expected view tooltip mounted")
	}

	rowOrigin := fyne.CurrentApp().Driver().AbsolutePositionForObject(row)
	hover.trackMouseForTooltip(&desktop.MouseEvent{
		PointEvent: fyne.PointEvent{AbsolutePosition: rowOrigin.Add(fyne.NewPos(10, 10))},
	})
	if !hover.isTooltipMounted() {
		t.Fatal("tooltip should stay open while pointer remains inside action limit")
	}

	hover.displayHovering = true
	hover.trackMouseForTooltip(&desktop.MouseEvent{
		PointEvent: fyne.PointEvent{AbsolutePosition: fyne.NewPos(9999, 9999)},
	})
	if hover.isTooltipMounted() {
		t.Fatal("tooltip should dismiss when pointer exits action limit even if hover flags linger")
	}
}

func TestShouldFollowMouse_viewModeFollowsWithinRow(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	row := canvas.NewRectangle(theme.Color(theme.ColorNameBackground))
	row.Resize(fyne.NewSize(200, 24))
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(row, w.Canvas()))
	w.Resize(fyne.NewSize(400, 300))
	hover.setTooltipKeepAliveArea(row)

	panel := newActionDisplayTooltipPanel(hover)
	hover.tooltipPanel = panel
	layer := custom_widgets.FindItemTooltipLayer(w.Canvas(), nil)
	if layer == nil {
		t.Fatal("expected item tooltip layer")
	}
	layer.Container.Objects = []fyne.CanvasObject{panel}
	panel.Resize(fyne.NewSize(80, 60))
	panel.Move(fyne.NewPos(20, 20))
	layer.Container.Refresh()

	rowOrigin := fyne.CurrentApp().Driver().AbsolutePositionForObject(row)
	hover.refreshKeepAliveGeometry()
	hover.absoluteMousePos = rowOrigin.Add(fyne.NewPos(10, 10))
	hover.rowHovering = true
	if !hover.shouldFollowMouse() {
		t.Fatal("view tooltip should follow cursor while pointer is in the action row")
	}

	hover.rowHovering = false
	hover.absoluteMousePos = fyne.NewPos(9999, 9999)
	if hover.shouldFollowMouse() {
		t.Fatal("view tooltip should not follow when pointer left the action row")
	}
}

func TestShouldKeepViewTooltip_ignoresPanelBounds(t *testing.T) {
	t.Helper()
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	row := canvas.NewRectangle(theme.Color(theme.ColorNameBackground))
	row.Resize(fyne.NewSize(200, 24))
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(row, w.Canvas()))
	hover.setTooltipKeepAliveArea(row)

	panel := newActionDisplayTooltipPanel(hover)
	hover.tooltipPanel = panel
	layer := custom_widgets.FindItemTooltipLayer(w.Canvas(), nil)
	if layer == nil {
		t.Fatal("expected item tooltip layer")
	}
	layer.Container.Objects = []fyne.CanvasObject{panel}
	hover.absoluteMousePos = fyne.NewPos(9999, 9999)

	if hover.shouldKeepViewTooltip() {
		t.Fatal("view tooltip should dismiss when pointer is outside the row")
	}
}

func assertTypePillText(t *testing.T, pill fyne.CanvasObject, want string) {
	t.Helper()
	stack, ok := pill.(*fyne.Container)
	if !ok || len(stack.Objects) < 2 {
		t.Fatal("expected action type pill")
	}
	padded, ok := stack.Objects[1].(*fyne.Container)
	if !ok || len(padded.Objects) == 0 {
		t.Fatal("expected padded pill content")
	}
	text, ok := padded.Objects[0].(*canvas.Text)
	if !ok || text.Text != want {
		t.Fatalf("tooltip header text = %q, want %q", text.Text, want)
	}
}

func isWrappedTooltipSection(obj fyne.CanvasObject) bool {
	stack, ok := obj.(*fyne.Container)
	if !ok || len(stack.Objects) < 2 {
		return false
	}
	border, ok := stack.Objects[0].(*canvas.Rectangle)
	if !ok {
		return false
	}
	return border.StrokeWidth > 0
}

func TestMeasureVBoxContentHeight_multiSectionViewPills(t *testing.T) {
	t.Helper()
	is := actions.NewImageSearch(
		"find",
		nil,
		[]string{"Demo~Item"},
		actions.NewCoordinateRef("Demo", "Main"),
		2, 2, 0.9, 1,
	)
	hover := newActionDisplayTooltipHover(is, nil, nil, is.GetType(), nil, nil)
	panel := newActionDisplayTooltipPanel(hover)

	width := float32(280)
	size := panel.contentSize(width)
	innerPad := panel.Theme().Size(theme.SizeNameInnerPadding)
	innerW := size.Width - innerPad*2
	innerH := size.Height - innerPad*2
	needed := measureVBoxContentHeight(panel.body.Objects, innerW)
	if needed > innerH+1 {
		t.Fatalf("content height %v too small for body %v", innerH, needed)
	}
}

func TestActionTooltipPanel_contentSizeFitsViewBody(t *testing.T) {
	t.Helper()
	cases := []struct {
		name string
		node actions.ActionInterface
	}{
		{"wait", actions.NewWait(100)},
		{"imagesearch", actions.NewImageSearch("s", nil, []string{"Demo~Item"}, actions.NewCoordinateRef("Demo", "Main"), 1, 1, 0.9, 0)},
		{"conditional", actions.NewConditional(nil, actions.MatchAll, "c", nil)},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			hover := newActionDisplayTooltipHover(tc.node, nil, nil, tc.node.GetType(), nil, nil)
			panel := newActionDisplayTooltipPanel(hover)
			width := float32(320)
			size := panel.contentSize(width)
			innerPad := panel.Theme().Size(theme.SizeNameInnerPadding)
			innerW := size.Width - innerPad*2
			innerH := size.Height - innerPad*2
			needed := measureVBoxContentHeight(panel.body.Objects, innerW)
			if needed > innerH+1 {
				t.Fatalf("content height %v too small for body %v", innerH, needed)
			}
		})
	}
}

func TestBuildParamEditPills_AllEditableActionTypes(t *testing.T) {
	t.Helper()
	cases := []struct {
		name string
		node actions.ActionInterface
	}{
		{"click", actions.NewClick(actions.ClickButtonLeft, false)},
		{"key", actions.NewKey("a", true)},
		{"wait", actions.NewWait(100)},
		{"move", actions.NewMove(actions.CoordinateRef(""), false)},
		{"loop", actions.NewLoop(3, "inner", nil)},
		{"conditional", actions.NewConditional(nil, actions.MatchAll, "c", nil)},
		{"setvariable", actions.NewSetVariable("x", 1)},
		{"calculate", actions.NewCalculate("1+1", "out")},
		{"runmacro", actions.NewRunMacro("other")},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			pills, _ := buildParamEditPills(tc.node, tc.node.GetType(), nil, nil)
			if pills == nil {
				t.Fatalf("%s: expected edit param pills", tc.name)
			}
		})
	}
}

func TestCapturePreview_usesCacheWithoutRecapture(t *testing.T) {
	t.Helper()
	move := actions.NewMove(actions.NewCoordinateRef("prog", "home"), false)
	calls := 0
	loader := func() (custom_widgets.PreviewTooltipResult, error) {
		calls++
		return custom_widgets.PreviewTooltipResult{Caption: "cached"}, nil
	}
	hover := newActionDisplayTooltipHover(move, nil, nil, move.GetType(), loader, nil)
	hover.tooltipPanel = newActionDisplayTooltipPanel(hover)
	hover.previewCacheReady = true
	hover.previewCache = custom_widgets.PreviewTooltipResult{Caption: "cached"}

	hover.capturePreview(false, false)
	if calls != 0 {
		t.Fatalf("expected cached preview, loader called %d times", calls)
	}
	if hover.tooltipPanel.caption == nil || hover.tooltipPanel.caption.Text != "cached" {
		t.Fatalf("expected cached caption on panel, got %q", hover.tooltipPanel.caption.Text)
	}
}

func TestViewTooltipRemountsWithoutEdit(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	row := canvas.NewRectangle(theme.Color(theme.ColorNameBackground))
	row.Resize(fyne.NewSize(200, 24))
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(row, w.Canvas()))
	hover.setTooltipKeepAliveArea(row)

	hover.rowHovering = true
	hover.openViewTooltip()
	if !hover.isTooltipMounted() {
		t.Fatal("expected view tooltip mounted on first hover")
	}

	hover.rowHovering = false
	hover.hideViewTooltip()
	if hover.isTooltipMounted() {
		t.Fatal("expected view tooltip unmounted after leave")
	}

	hover.rowHovering = true
	hover.noteHoverIn(&desktop.MouseEvent{
		PointEvent: fyne.PointEvent{AbsolutePosition: fyne.NewPos(10, 10)},
	})
	if !hover.isTooltipMounted() {
		t.Fatal("view tooltip must remount on re-hover without edit mode")
	}
}

func TestHideViewTooltip_preservesPreviewCache(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)

	move := actions.NewMove(actions.NewCoordinateRef("prog", "home"), false)
	calls := 0
	loader := func() (custom_widgets.PreviewTooltipResult, error) {
		calls++
		return custom_widgets.PreviewTooltipResult{Caption: "cached"}, nil
	}
	hover := newActionDisplayTooltipHover(move, nil, nil, move.GetType(), loader, nil)
	hover.tooltipPanel = newActionDisplayTooltipPanel(hover)
	hover.previewCacheReady = true
	hover.previewCache = custom_widgets.PreviewTooltipResult{Caption: "cached"}
	hover.applyPreviewCache(hover.tooltipPanel)

	hover.hideViewTooltip()
	if hover.isTooltipMounted() {
		t.Fatal("view hide should unmount")
	}
	if !hover.previewCacheReady {
		t.Fatal("view hide must preserve cache")
	}

	hover.rowHovering = true
	hover.beginPreviewCapture()
	if calls != 0 {
		t.Fatalf("re-hover should reuse cache, loader called %d times", calls)
	}
}

func TestHideTooltip_clearsPanelForReshow(t *testing.T) {
	t.Helper()
	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	hover.tooltipPanel = newActionDisplayTooltipPanel(hover)

	hover.hideTooltip()

	if hover.tooltipPanel != nil {
		t.Fatal("hideTooltip must nil tooltipPanel so a later hover can show again")
	}
}

func TestShouldKeepViewTooltip_keepsWhileHovering(t *testing.T) {
	t.Helper()
	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	hover.tooltipPanel = newActionDisplayTooltipPanel(hover)
	hover.displayHovering = true

	if !hover.shouldKeepViewTooltip() {
		t.Fatal("expected view tooltip to stay open while display is hovered")
	}
}

func TestViewTooltipOpen_requiresMountedPanel(t *testing.T) {
	t.Helper()
	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	hover.tooltipPanel = newActionDisplayTooltipPanel(hover)

	if hover.viewTooltipOpen() {
		t.Fatal("view tooltip is only open when the panel is mounted in the layer")
	}
}

func TestEditModeBlocksOtherRowTooltips(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)

	waitA := actions.NewWait(100)
	waitB := actions.NewWait(200)
	hoverA := newActionDisplayTooltipHover(waitA, nil, nil, waitA.GetType(), nil, nil)
	hoverB := newActionDisplayTooltipHover(waitB, nil, nil, waitB.GetType(), nil, nil)

	hoverA.tooltipPanel = newActionDisplayTooltipPanel(hoverA)
	hoverA.tooltipPanel.enterEditMode()
	if activeActionEditTooltip != hoverA {
		t.Fatal("expected edit ownership on row A")
	}

	hoverB.rowHovering = true
	hoverB.noteHoverIn(&desktop.MouseEvent{
		PointEvent: fyne.PointEvent{AbsolutePosition: fyne.NewPos(50, 20)},
	})

	if hoverB.tooltipPanel != nil {
		t.Fatal("row B must not show a tooltip while row A is in edit mode")
	}
}

func TestHideTooltipPreservesOtherRowLayerObject(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	waitA := actions.NewWait(100)
	waitB := actions.NewWait(200)
	hoverA := newActionDisplayTooltipHover(waitA, nil, nil, waitA.GetType(), nil, nil)
	hoverB := newActionDisplayTooltipHover(waitB, nil, nil, waitB.GetType(), nil, nil)

	bg := canvas.NewRectangle(theme.Color(theme.ColorNameBackground))
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(bg, w.Canvas()))
	layer := custom_widgets.FindItemTooltipLayer(w.Canvas(), nil)
	if layer == nil {
		t.Fatal("expected item tooltip layer")
	}

	panelB := newActionDisplayTooltipPanel(hoverB)
	layer.Container.Objects = []fyne.CanvasObject{panelB}
	hoverB.tooltipPanel = panelB
	activeActionViewTooltip = hoverB

	hoverA.tooltipPanel = newActionDisplayTooltipPanel(hoverA)
	hoverA.hideTooltip()

	if len(layer.Container.Objects) != 1 || layer.Container.Objects[0] != panelB {
		t.Fatalf("row A hide cleared layer; want B's panel, got %d objects", len(layer.Container.Objects))
	}
	if hoverB.tooltipPanel == nil {
		t.Fatal("row B tooltip panel reference must remain")
	}
}

func TestViewTooltipHandoffThroughOverlay(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	waitA := actions.NewWait(100)
	waitB := actions.NewWait(200)
	hoverA := newActionDisplayTooltipHover(waitA, nil, nil, waitA.GetType(), nil, nil)
	hoverB := newActionDisplayTooltipHover(waitB, nil, nil, waitB.GetType(), nil, nil)

	rowA := canvas.NewRectangle(theme.Color(theme.ColorNameBackground))
	rowA.Resize(fyne.NewSize(200, 24))
	rowB := canvas.NewRectangle(theme.Color(theme.ColorNameBackground))
	rowB.Resize(fyne.NewSize(200, 24))
	content := container.NewVBox(rowA, rowB)
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(content, w.Canvas()))
	hoverA.setTooltipKeepAliveArea(rowA)
	hoverB.setTooltipKeepAliveArea(rowB)

	hoverA.rowHovering = true
	hoverA.openViewTooltip()
	if hoverA.tooltipPanel == nil || !hoverA.isTooltipMounted() {
		t.Fatal("expected row A view tooltip mounted")
	}
	panelA := hoverA.tooltipPanel

	// Pointer sits where row B is, visually under row A's tooltip panel.
	rowBOrigin := fyne.CurrentApp().Driver().AbsolutePositionForObject(rowB)
	hoverA.rowHovering = false
	hoverA.noteHoverOut()
	hoverB.rowHovering = true
	hoverB.noteHoverIn(&desktop.MouseEvent{
		PointEvent: fyne.PointEvent{AbsolutePosition: rowBOrigin.Add(fyne.NewPos(10, 10))},
	})

	if hoverA.isTooltipMounted() {
		t.Fatal("row A tooltip should close when row B is hovered through the overlay")
	}
	if hoverB.tooltipPanel == nil || !hoverB.isTooltipMounted() {
		t.Fatal("expected row B view tooltip mounted after handoff")
	}
	if activeActionViewTooltip != hoverB {
		t.Fatal("view tooltip ownership should move to row B")
	}
	if hoverA.tooltipPanel != panelA {
		t.Fatal("row A should keep its panel for cache reuse")
	}
	if hoverB.tooltipPanel == panelA {
		t.Fatal("row B must mount its own panel")
	}
}

func TestViewTooltipHandoffBetweenRows(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	moveA := actions.NewMove(actions.NewCoordinateRef("prog", "home"), false)
	moveB := actions.NewMove(actions.NewCoordinateRef("prog", "away"), false)
	loader := func() (custom_widgets.PreviewTooltipResult, error) {
		return custom_widgets.PreviewTooltipResult{Caption: "preview"}, nil
	}
	rect := func() fyne.CanvasObject { return canvas.NewRectangle(color.Transparent) }
	hoverA := newActionDisplayTooltipHover(moveA, rect(), nil, moveA.GetType(), loader, nil)
	hoverB := newActionDisplayTooltipHover(moveB, rect(), nil, moveB.GetType(), loader, nil)

	bg := canvas.NewRectangle(theme.Color(theme.ColorNameBackground))
	content := container.NewHBox(hoverA, hoverB)
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(container.NewStack(bg, content), w.Canvas()))
	hoverA.setTooltipKeepAliveArea(bg)
	hoverB.setTooltipKeepAliveArea(bg)

	hoverA.beginPreviewCapture()
	if hoverA.tooltipPanel == nil {
		t.Fatal("expected row A to open a view tooltip")
	}
	panelA := hoverA.tooltipPanel
	hoverA.previewCacheReady = true
	hoverA.previewCache = custom_widgets.PreviewTooltipResult{Caption: "preview"}

	hoverA.rowHovering = false
	hoverA.hideViewTooltip()
	if hoverA.isTooltipMounted() {
		t.Fatal("view hide should unmount panel")
	}
	if !hoverA.previewCacheReady {
		t.Fatal("view hide must preserve preview cache")
	}

	hoverB.beginPreviewCapture()
	if hoverB.tooltipPanel == nil {
		t.Fatal("expected row B to open a view tooltip after fast switch")
	}
	if hoverA.tooltipPanel != panelA {
		t.Fatal("row A view hide should keep its panel for cache reuse")
	}
	if activeActionViewTooltip != hoverB {
		t.Fatal("view tooltip ownership should move to row B")
	}

	layer := custom_widgets.FindItemTooltipLayer(w.Canvas(), nil)
	if layer == nil || len(layer.Container.Objects) != 1 || layer.Container.Objects[0] != hoverB.tooltipPanel {
		t.Fatalf("layer should contain only row B's panel, got %d objects", len(layer.Container.Objects))
	}
	if hoverB.tooltipPanel == panelA {
		t.Fatal("row B must mount its own panel, not reuse row A's")
	}
}

func TestEnterEditMode_preservesTooltipVerticalPosition(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	test.NewApp()
	w := test.NewWindow(canvas.NewRectangle(color.White))
	w.Resize(fyne.NewSize(800, 600))
	t.Cleanup(w.Close)

	is := actions.NewImageSearch("find", nil, []string{"Demo~Item"}, actions.NewCoordinateRef("Demo", "Main"), 1, 1, 0.9, 0)
	hover := newActionDisplayTooltipHover(is, canvas.NewRectangle(color.Transparent), nil, is.GetType(), nil, nil)
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(hover, w.Canvas()))

	hover.absoluteMousePos = fyne.NewPos(200, 450)
	hover.showTooltipPanel()
	if hover.tooltipPanel == nil {
		t.Fatal("expected view tooltip panel")
	}
	initialY := hover.tooltipPanel.Position().Y
	initialH := hover.tooltipPanel.Size().Height

	hover.tooltipPanel.enterEditMode()
	newY := hover.tooltipPanel.Position().Y
	newH := hover.tooltipPanel.Size().Height
	if newH <= initialH {
		t.Fatalf("edit mode should grow tooltip: height %v -> %v", initialH, newH)
	}

	mouseBasedY := actionDisplayTooltipPosition(w.Canvas(), hover.absoluteMousePos, hover.tooltipPanel.Size()).Y
	if abs32(newY-mouseBasedY) < 1 {
		t.Fatalf("edit relayout re-anchored to mouse (y=%v); would jump from view y=%v", mouseBasedY, initialY)
	}
	if abs32(newY-initialY) > 80 {
		t.Fatalf("edit relayout moved tooltip too far vertically: %v -> %v", initialY, newY)
	}
}

func findBorderlessEntry(obj fyne.CanvasObject) *custom_widgets.BorderlessEntry {
	switch o := obj.(type) {
	case *custom_widgets.BorderlessEntry:
		return o
	case *fyne.Container:
		for _, c := range o.Objects {
			if e := findBorderlessEntry(c); e != nil {
				return e
			}
		}
	case fyne.Widget:
		for _, c := range test.WidgetRenderer(o).Objects() {
			if e := findBorderlessEntry(c); e != nil {
				return e
			}
		}
	}
	return nil
}

// TestRunMacroEdit_growsBackgroundOnSelection guards against tooltip background
// drift: selecting a macro widens the row (and can wrap it), so the picker must
// grow the tooltip to match instead of leaving a stale, undersized background.
func TestRunMacroEdit_growsBackgroundOnSelection(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	test.NewApp()
	w := test.NewWindow(canvas.NewRectangle(color.White))
	w.Resize(fyne.NewSize(800, 600))
	t.Cleanup(w.Close)

	run := actions.NewRunMacro("")
	hover := newActionDisplayTooltipHover(run, canvas.NewRectangle(color.Transparent), nil, run.GetType(), nil, nil)
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(container.NewStack(hover), w.Canvas()))

	hover.absoluteMousePos = fyne.NewPos(200, 200)
	hover.showTooltipPanel()
	hover.tooltipPanel.enterEditMode()

	before := hover.tooltipPanel.Size()

	entry := findBorderlessEntry(hover.tooltipPanel.body)
	if entry == nil {
		t.Fatal("expected a macro name entry in the run-macro edit form")
	}
	// Mirror what macroPickerButton's onSelect now does on selection.
	entry.SetText("A Very Long Macro Name That Forces The Row To Grow")
	hover.refreshTooltipLayout()

	after := hover.tooltipPanel.Size()
	if after.Width <= before.Width && after.Height <= before.Height {
		t.Fatalf("selecting a macro should grow the tooltip: %v -> %v", before, after)
	}

	// Background must track content, i.e. no drift between panel size and freshly
	// measured layout (invalidate first since measureLayoutSize is cached).
	hover.tooltipPanel.invalidateLayoutSize()
	want := hover.tooltipPanel.measureLayoutSize(w.Canvas())
	if !fyneSizesClose(after, want) {
		t.Fatalf("tooltip background drifted from content: panel=%v measured=%v", after, want)
	}
}

func findPillSelect(obj fyne.CanvasObject) *actiondisplay.PillSelect {
	switch o := obj.(type) {
	case *actiondisplay.PillSelect:
		return o
	case *fyne.Container:
		for _, c := range o.Objects {
			if s := findPillSelect(c); s != nil {
				return s
			}
		}
	case fyne.Widget:
		for _, c := range test.WidgetRenderer(o).Objects() {
			if s := findPillSelect(c); s != nil {
				return s
			}
		}
	}
	return nil
}

func tapPillSelectNext(t *testing.T, sel *actiondisplay.PillSelect) {
	t.Helper()
	for _, obj := range test.WidgetRenderer(sel).Objects() {
		btn, ok := obj.(fyne.Tappable)
		if !ok {
			continue
		}
		// Objects() lists the next button before the previous button.
		btn.Tapped(&fyne.PointEvent{})
		return
	}
	t.Fatal("no cycle button found on pill select")
}

// TestClickEdit_growsBackgroundOnButtonChange guards against tooltip background
// drift: cycling the click button select to a wider option ("left" -> "center")
// widens the pill, so the select must relayout the tooltip. Before the fix the
// panel kept its old size and the background no longer covered the content.
func TestClickEdit_growsBackgroundOnButtonChange(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	test.NewApp()
	w := test.NewWindow(canvas.NewRectangle(color.White))
	w.Resize(fyne.NewSize(800, 600))
	t.Cleanup(w.Close)

	click := actions.NewClick(actions.ClickButtonLeft, false)
	hover := newActionDisplayTooltipHover(click, canvas.NewRectangle(color.Transparent), nil, click.GetType(), nil, nil)
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(container.NewStack(hover), w.Canvas()))

	hover.absoluteMousePos = fyne.NewPos(200, 200)
	hover.showTooltipPanel()
	hover.tooltipPanel.enterEditMode()

	sel := findPillSelect(hover.tooltipPanel.body)
	if sel == nil {
		t.Fatal("expected a button select in the click edit form")
	}
	if sel.Value != actions.ClickButtonLeft {
		t.Fatalf("expected initial button %q, got %q", actions.ClickButtonLeft, sel.Value)
	}

	initialW := hover.tooltipPanel.Size().Width

	// Cycle to a wider label; the panel must grow to keep the background covering it.
	for sel.Value != actions.ClickButtonCenter {
		tapPillSelectNext(t, sel)
	}

	if grownW := hover.tooltipPanel.Size().Width; grownW <= initialW {
		t.Fatalf("wider button should grow tooltip: width %v -> %v", initialW, grownW)
	}
}

func TestActionDisplayTooltipPositionClamped_preservesTopWhenGrowing(t *testing.T) {
	t.Helper()
	test.NewApp()
	w := test.NewWindow(nil)
	w.Resize(fyne.NewSize(800, 600))
	t.Cleanup(w.Close)
	c := w.Canvas()

	anchor := fyne.NewPos(100, 100)
	large := fyne.NewSize(200, 350)

	clamped := actionDisplayTooltipPositionClamped(c, anchor, large)
	mouseBased := actionDisplayTooltipPosition(c, fyne.NewPos(150, 380), large)
	if abs32(clamped.Y-mouseBased.Y) < 1 {
		t.Fatalf("clamped position should differ from mouse re-anchor: clamped=%v mouse=%v", clamped.Y, mouseBased.Y)
	}
	if abs32(clamped.Y-anchor.Y) > 1 {
		t.Fatalf("clamped position should preserve anchor when size fits: %v -> %v", anchor.Y, clamped.Y)
	}
}

func TestActionTooltips_suppressedDuringDrag(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)

	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	rowBody := newTreeRowBody(container.NewHScroll(canvas.NewRectangle(color.White)))
	rowBody.tree = &MacroTree{dragActive: true}
	hover.bindRowBody(rowBody)

	hover.noteHoverIn(&desktop.MouseEvent{
		PointEvent: fyne.PointEvent{AbsolutePosition: fyne.NewPos(10, 10)},
	})
	if hover.tooltipPanel != nil {
		t.Fatal("tooltip should not open while drag-and-drop is active")
	}
}

func TestDismissActiveActionTooltips_clearsViewTooltip(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	row := canvas.NewRectangle(theme.Color(theme.ColorNameBackground))
	w.SetContent(custom_widgets.AddWindowItemTooltipLayer(row, w.Canvas()))

	panel := newActionDisplayTooltipPanel(hover)
	hover.tooltipPanel = panel
	claimActionViewTooltip(hover)
	layer := custom_widgets.FindItemTooltipLayer(w.Canvas(), nil)
	layer.Container.Objects = []fyne.CanvasObject{panel}

	dismissActiveActionTooltips()
	if activeActionViewTooltip != nil {
		t.Fatal("expected view tooltip ownership cleared")
	}
	if hover.isTooltipMounted() {
		t.Fatal("expected view tooltip unmounted")
	}
}
