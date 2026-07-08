package macro

import (
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
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
	assertTypePillText(t, row.Objects[0], actions.ActionTypeLabel("wait"))
}

func TestActionTooltipPanel_enterEditMode_selectsOwningRow(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)
	wait := actions.NewWait(100)
	root := actions.NewLoop(1, "root", nil)
	root.AddSubAction(wait)
	mt := &MacroTree{Macro: &models.Macro{Root: root}}
	mt.setTree()

	rowBody := newTreeRowBody(container.NewHScroll(widget.NewLabel("wait")))
	rowBody.tree = mt
	rowBody.uid = wait.GetUID()

	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	hover.bindRowBody(rowBody)
	panel := newActionDisplayTooltipPanel(hover)
	panel.enterEditMode()

	if mt.SelectedNode != wait.GetUID() {
		t.Fatalf("SelectedNode = %q, want %q", mt.SelectedNode, wait.GetUID())
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

func TestPointerInRowKeepAlive_excludesRemoveButton(t *testing.T) {
	t.Helper()
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	wait := actions.NewWait(100)
	hover := newActionDisplayTooltipHover(wait, nil, nil, wait.GetType(), nil, nil)
	row := canvas.NewRectangle(theme.Color(theme.ColorNameBackground))
	row.Resize(fyne.NewSize(200, 24))
	removeBtn := widget.NewButton("x", nil)
	border := container.NewBorder(nil, nil, nil, removeBtn, row)
	border.Resize(fyne.NewSize(200, 24))
	w.SetContent(border)
	w.Resize(fyne.NewSize(220, 40))

	hover.setTooltipKeepAliveArea(border)
	hover.setTooltipKeepAliveExclude(removeBtn)

	rowOrigin := fyne.CurrentApp().Driver().AbsolutePositionForObject(row)
	if !hover.pointerInRowKeepAlive(rowOrigin.Add(fyne.NewPos(10, 10))) {
		t.Fatal("pointer over action body should be inside keep-alive area")
	}

	removeOrigin := fyne.CurrentApp().Driver().AbsolutePositionForObject(removeBtn)
	removeSize := removeBtn.Size()
	if hover.pointerInRowKeepAlive(removeOrigin.Add(fyne.NewPos(removeSize.Width/2, removeSize.Height/2))) {
		t.Fatal("pointer over remove button should be outside keep-alive area")
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

func TestCapturePreview_forceRefreshBypassesCache(t *testing.T) {
	t.Helper()
	move := actions.NewMove(actions.NewCoordinateRef("prog", "home"), false)
	loader := func() (custom_widgets.PreviewTooltipResult, error) {
		return custom_widgets.PreviewTooltipResult{Caption: "fresh"}, nil
	}
	hover := newActionDisplayTooltipHover(move, nil, nil, move.GetType(), loader, nil)
	hover.tooltipPanel = newActionDisplayTooltipPanel(hover)
	hover.previewCacheReady = true
	hover.previewCache = custom_widgets.PreviewTooltipResult{Caption: "cached"}

	hover.capturePreview(true, false)
	if hover.previewCacheReady {
		t.Fatal("force refresh should clear hover preview cache")
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

	hover.capturePreview(false, true)
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

func TestActionTooltipPanel_editModeShowsCoordPickerForEmptyRef(t *testing.T) {
	t.Helper()
	t.Cleanup(ResetActionTooltipOwnershipForTesting)

	cases := []struct {
		name      string
		node      actions.ActionInterface
		wantLabel string
	}{
		{
			name:      "semanticsearch",
			node:      actions.NewSemanticSearch("s", nil, "button", actions.CoordinateRef("")),
			wantLabel: "Select search area…",
		},
		{
			name:      "move",
			node:      actions.NewMove(actions.CoordinateRef(""), false),
			wantLabel: "Select point…",
		},
		{
			name:      "imagesearch",
			node:      actions.NewImageSearch("s", nil, nil, actions.CoordinateRef(""), 1, 1, 0.95, 0),
			wantLabel: "Select search area…",
		},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			hover := newActionDisplayTooltipHover(tc.node, nil, nil, tc.node.GetType(), nil, nil)
			panel := newActionDisplayTooltipPanel(hover)
			if panel.withPreview {
				t.Fatal("expected no preview section before edit mode")
			}
			panel.enterEditMode()
			if !panel.withPreview {
				t.Fatal("expected preview section in edit mode for coordinate actions")
			}
			if panel.editForm == nil || panel.editForm.coordEditActions == nil {
				t.Fatal("expected coordinate picker controls")
			}
			if got := coordPickerButtonText(panel.editForm.coordEditActions); got != tc.wantLabel {
				t.Fatalf("picker label = %q, want %q", got, tc.wantLabel)
			}
			panel.exitEditMode()
			if panel.withPreview {
				t.Fatal("expected preview section hidden after edit when ref still empty")
			}
		})
	}
}

func TestBuildPreviewRefreshRow(t *testing.T) {
	t.Helper()
	move := actions.NewMove(actions.NewCoordinateRef("prog", "home"), false)
	loader := func() (custom_widgets.PreviewTooltipResult, error) {
		return custom_widgets.PreviewTooltipResult{}, nil
	}
	hover := newActionDisplayTooltipHover(move, nil, nil, move.GetType(), loader, nil)
	if buildPreviewRefreshRow(nil) != nil {
		t.Fatal("expected nil refresh row without owner")
	}
	hover.previewLoader = nil
	if buildPreviewRefreshRow(hover) != nil {
		t.Fatal("expected nil refresh row without loader")
	}
	hover.previewLoader = loader
	if buildPreviewRefreshRow(hover) == nil {
		t.Fatal("expected refresh row when loader is set")
	}
}

func coordPickerButtonText(obj fyne.CanvasObject) string {
	switch v := obj.(type) {
	case *widget.Button:
		return v.Text
	case *fyne.Container:
		for _, child := range v.Objects {
			if label := coordPickerButtonText(child); label != "" {
				return label
			}
		}
	}
	return ""
}
