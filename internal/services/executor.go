package services

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"fmt"
	"image"
	_ "image/png"
	"log"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"time"

	"fyne.io/fyne/v2"
	"github.com/go-vgo/robotgo"
)

// Execute executes an action with optional macro context for variable resolution
func Execute(a actions.ActionInterface, macro ...*models.Macro) error {
	var macroCtx *models.Macro
	if len(macro) > 0 {
		macroCtx = macro[0]
	}
	err := executeWithContext(a, macroCtx)
	if actions.IsFlowControl(err) {
		log.Printf("Flow control %v outside loop (ignored)", err)
		return nil
	}
	return err
}

// ExecuteMacroWithLogging runs a macro with log capture and shows the log popup.
// Use this instead of Execute when running a macro from the UI or hotkey.
// Only one macro may run at a time; concurrent requests are ignored.
func ExecuteMacroWithLogging(m *models.Macro) {
	if m == nil {
		return
	}
	if !tryStartMacroRun(m.Name) {
		log.Printf("Macro %q: not started — %q is already running", m.Name, RunningMacroName())
		return
	}
	defer endMacroRun()
	defer func() {
		ClearHighlights()
		NotifyMacroPause(false, "", "")
		fyne.Do(func() {
			MacroActiveIndicator().Stop()
			MacroActiveIndicator().Hide()
			if macroRunningCallback != nil {
				macroRunningCallback(false)
			}
		})
		if r := recover(); r != nil {
			LogPanicToFile(r, fmt.Sprintf("Macro %q", m.Name))
		}
		// Reclaim after the queued UI teardown (highlight clear, tree collapse,
		// log pump stop) has run, so its allocation spike is actually scavenged
		// instead of left resident until the background scavenger catches up.
		scheduleMemoryReclaim(m.Name)
	}()
	if showMacroLogPopupFunc != nil {
		fyne.DoAndWait(func() {
			showMacroLogPopupFunc(m.Name)
			if macroRunningCallback != nil {
				macroRunningCallback(true)
			}
		})
		defer StopMacroLogCapture()
	} else {
		fyne.DoAndWait(func() {
			if macroRunningCallback != nil {
				macroRunningCallback(true)
			}
		})
	}
	ClearRuntimeVariables()
	m.InitRuntimeVariables()
	ApplyMonitorBuiltinVariables(m)
	SnapshotRuntimeVariables(m)
	if macroUsesOCR(m) {
		WarmUpOCR()
	}
	if err := Execute(m.Root, m); err != nil {
		if actions.IsStopped(err) {
			log.Printf("Macro %q: stopped by user", m.Name)
		} else {
			log.Printf("Macro %q: execution error: %v", m.Name, err)
		}
	}
	ReleaseMacroLogCapture()
}

// showMacroLogPopupFunc is set by ui/macro to avoid import cycle
var showMacroLogPopupFunc func(macroName string)

// SetShowMacroLogPopupFunc sets the callback to show the macro log popup.
// Called from ui package during initialization.
func SetShowMacroLogPopupFunc(fn func(macroName string)) {
	showMacroLogPopupFunc = fn
}

// macroRunningCallback is invoked on the UI thread when a macro starts (true) or stops (false).
// Used to swap the macro play button for a stop button while a macro is running.
var macroRunningCallback func(running bool)

// SetMacroRunningCallback sets the callback invoked when macro execution starts or stops.
// The callback is always run on the Fyne UI thread.
func SetMacroRunningCallback(fn func(running bool)) {
	macroRunningCallback = fn
}

// resetListSourcesInTree resets line cursors for every for-each row in the tree.
func resetListSourcesInTree(a actions.ActionInterface) {
	if fer, ok := a.(*actions.ForEachRow); ok {
		fer.Reset()
	}
	if adv, ok := a.(actions.AdvancedActionInterface); ok {
		for _, sub := range adv.GetSubActions() {
			resetListSourcesInTree(sub)
		}
	}
}

// resolveRowBound resolves a For Each Row StartRow/EndRow value (int literal or
// "${variable}") to a 1-based row number. An unset value (nil or blank string)
// yields def.
func resolveRowBound(v any, def int, macro *models.Macro) (int, error) {
	if !actions.RowBoundIsSet(v) {
		return def, nil
	}
	return ResolveInt(v, macro)
}

func executeForEachRow(node *actions.ForEachRow, macro *models.Macro) error {
	if len(node.Sources) == 0 {
		return fmt.Errorf("for each row %q: at least one source is required", node.Name)
	}
	rowCount, err := node.Sources[0].LineCount()
	if err != nil {
		return fmt.Errorf("for each row %q: %w", node.Name, err)
	}
	start, err := resolveRowBound(node.StartRow, 1, macro)
	if err != nil {
		return fmt.Errorf("for each row %q start row: %w", node.Name, err)
	}
	end, err := resolveRowBound(node.EndRow, rowCount, macro)
	if err != nil {
		return fmt.Errorf("for each row %q end row: %w", node.Name, err)
	}
	if start < 1 {
		start = 1
	}
	if end > rowCount {
		end = rowCount
	}
	for i := start - 1; i < end; i++ {
		if err := checkMacroStop(); err != nil {
			return err
		}
		if macro != nil && rowCount > 0 {
			highlightFill(macro.Name, node.GetUID(), float64(i)/float64(rowCount))
		}
		for j := range node.Sources {
			col := &node.Sources[j]
			col.SetLineIndex(i)
			line, err := col.GetCurrentLine()
			if err != nil {
				return fmt.Errorf("for each row %q source %d (%s): %w", node.Name, j+1, col.OutputVar, err)
			}
			if macro != nil && col.OutputVar != "" {
				setMacroVariable(macro, col.OutputVar, line)
			}
		}
		if macro != nil {
			setMacroVariable(macro, actions.ForEachRowBuiltinRow, i+1)
			setMacroVariable(macro, actions.ForEachRowBuiltinRowCount, rowCount)
		}
		log.Printf("For each row: %s row %d/%d", node.Name, i+1, rowCount)
		brk, cont, err := handleLoopFlow(executeSubActions(node.GetSubActions(), macro))
		if err != nil {
			return err
		}
		if cont {
			continue
		}
		if brk {
			break
		}
	}
	if macro != nil {
		highlightClear(macro.Name, node.GetUID())
	}
	return nil
}

// executeRunMacroTree runs the target macro's top-level actions while driving a
// progress fill highlight on the calling Run Macro action. The fill advances as
// each top-level action of the target completes, so the Run Macro node stays
// highlighted until the designated macro has finished executing.
func executeRunMacroTree(rm *actions.RunMacro, target *models.Macro, caller *models.Macro) error {
	root := target.Root
	resetListSourcesInTree(root)
	subs := root.GetSubActions()
	total := len(subs)
	for i, action := range subs {
		if err := checkMacroStop(); err != nil {
			return err
		}
		brk, cont, err := handleLoopFlow(executeWithContext(action, target))
		if err != nil {
			return err
		}
		if cont {
			continue
		}
		if brk {
			break
		}
		if caller != nil && total > 0 {
			highlightFill(caller.Name, rm.GetUID(), float64(i+1)/float64(total))
		}
	}
	// Clear the target tree's moving cursor so it doesn't linger after return.
	highlightCursor(target.Name, "")
	return nil
}

func executeWithContext(a actions.ActionInterface, macro *models.Macro) error {
	err := executeAction(a, macro)
	if err == nil || actions.IsFlowControl(err) {
		if delayErr := applyGlobalDelay(macro); delayErr != nil {
			return delayErr
		}
	}
	return err
}

func moveOpts(node *actions.Move) MoveOptions {
	if !node.Smooth {
		return MoveOptions{Smooth: false}
	}
	return MoveOptions{
		Smooth:  true,
		Low:     node.EffectiveSmoothLow(),
		High:    node.EffectiveSmoothHigh(),
		DelayMs: node.EffectiveSmoothDelayMs(),
	}
}

func executeAction(a actions.ActionInterface, macro *models.Macro) error {
	if macro != nil && a != nil {
		// Skip the synthetic root loop: it is not a visible tree node.
		if macro.Root == nil || a.GetUID() != macro.Root.GetUID() {
			highlightCursor(macro.Name, a.GetUID())
		}
	}
	switch node := a.(type) {
	case *actions.Wait:
		log.Println("Wait:", node.String())
		time, err := ResolveInt(node.Time, macro)
		if err != nil {
			return fmt.Errorf("wait time: %w", err)
		}
		return interruptibleSleep(time)
	case *actions.Move:
		log.Println("Move:", node.String())
		pt, err := LookupPoint(node.Point, DefaultResolutionKey())
		if err != nil {
			log.Printf("Move: failed to lookup point %q: %v, using (0,0)", node.Point, err)
			getAutomationBackend().Move(0, 0, moveOpts(node))
			return nil
		}
		x, err := ResolveInt(pt.X, macro)
		if err != nil {
			log.Printf("Move: failed to resolve X %v: %v, using 0 (ensure variable is set by an earlier action, e.g. Image Search output)", pt.X, err)
			x = 0
		}
		y, err := ResolveInt(pt.Y, macro)
		if err != nil {
			log.Printf("Move: failed to resolve Y %v: %v, using 0 (ensure variable is set by an earlier action, e.g. Image Search output)", pt.Y, err)
			y = 0
		}
		getAutomationBackend().Move(x, y, moveOpts(node))
		return nil
	case *actions.Click:
		log.Println("Click:", node.String())
		btn := actions.LeftOrRight(node.Button)
		if node.State {
			return getAutomationBackend().Click(btn, true)
		}
		return getAutomationBackend().Click(btn, false)
	case *actions.Key:
		log.Println("Key:", node.String())
		key := node.Key
		// Resolve key if it contains variable references
		if macro != nil {
			resolved, err := ResolveString(key, macro)
			if err == nil {
				key = resolved
			}
		}
		if node.State {
			return getAutomationBackend().KeyDown(key)
		}
		return getAutomationBackend().KeyUp(key)
	case *actions.Type:
		log.Println("Type:", node.String())
		text := node.Text
		if macro != nil {
			resolved, err := ResolveString(text, macro)
			if err == nil {
				text = resolved
			}
		}
		delayMs := node.DelayMs
		if delayMs < 0 {
			delayMs = 0
		}
		backend := getAutomationBackend()
		for _, r := range text {
			if err := checkMacroStop(); err != nil {
				return err
			}
			backend.TypeChar(string(r))
			if delayMs > 0 {
				if err := interruptibleSleep(delayMs); err != nil {
					return err
				}
			}
		}
		return nil

	case *actions.Loop:
		log.Println("Loop:", node.String())
		count, err := ResolveInt(node.Count, macro)
		if err != nil {
			return fmt.Errorf("loop count: %w", err)
		}
		if count < 1 {
			return fmt.Errorf("loop count must be at least 1, got %d", count)
		}
		if node.Name == "root" {
			resetListSourcesInTree(node)
			fyne.Do(func() {
				MacroActiveIndicator().Show()
				MacroActiveIndicator().Start()
			})
		}

		for i := range count {
			if err := checkMacroStop(); err != nil {
				if node.Name == "root" {
					fyne.Do(func() {
						MacroActiveIndicator().Stop()
						MacroActiveIndicator().Hide()
					})
				}
				return err
			}
			log.Printf("Loop: %s iteration %d", node.Name, i+1)
			brk, cont, err := handleLoopFlow(executeSubActions(node.GetSubActions(), macro))
			if err != nil {
				if node.Name == "root" {
					fyne.DoAndWait(func() {
						MacroActiveIndicator().Stop()
						MacroActiveIndicator().Hide()
					})
				}
				return err
			}
			if cont {
				continue
			}
			if brk {
				break
			}
		}
		if node.Name == "root" {
			fyne.Do(func() {
				MacroActiveIndicator().Stop()
				MacroActiveIndicator().Hide()
			})
		}
		return nil
	case *actions.Conditional:
		log.Println("Conditional:", node.String())
		result, err := EvaluateCondition(node, macro)
		if err != nil {
			log.Printf("Conditional: %v; treating as false (skipping branch)", err)
			return nil
		}
		if !result {
			log.Printf("Conditional %q: false, skipping branch", node.Name)
			return nil
		}
		log.Printf("Conditional %q: true, running branch", node.Name)
		return executeSubActions(node.GetSubActions(), macro)
	case *actions.ImageSearch:
		log.Println("Image Search:", node.String())
		if macro != nil {
			highlightFill(macro.Name, node.GetUID(), 0)
			defer highlightClear(macro.Name, node.GetUID())
		}
		results, searchLeftX, searchTopY, err := imageSearch(node, macro)
		if err != nil {
			log.Printf("Image Search: %v (macro continues)", err)
			if results == nil {
				results = make(map[string][]robotgo.Point)
			}
		}
		if node.WaitTilFound && node.WaitTilFoundSeconds > 0 {
			deadline := time.Now().Add(time.Duration(node.WaitTilFoundSeconds) * time.Second)
			intervalMs := node.WaitTilFoundIntervalMs
			if intervalMs <= 0 {
				intervalMs = 100
			}
			for len(SortListOfPoints(results)) == 0 && time.Now().Before(deadline) {
				time.Sleep(time.Duration(intervalMs) * time.Millisecond)
				if err := checkMacroStop(); err != nil {
					return err
				}
				results, searchLeftX, searchTopY, err = imageSearch(node, macro)
				if err != nil {
					log.Printf("Image Search: %v (macro continues)", err)
					if results == nil {
						results = make(map[string][]robotgo.Point)
					}
				}
			}
		}
		sorted := SortListOfPoints(results)
		var foundNames, notFoundNames []string
		for name, points := range results {
			if len(points) > 0 {
				foundNames = append(foundNames, name)
			} else {
				notFoundNames = append(notFoundNames, name)
			}
		}
		slices.Sort(foundNames)
		slices.Sort(notFoundNames)
		count := 0
		totalMatches := len(sorted)
		var firstPoint *robotgo.Point
		for _, np := range sorted {
			if macro != nil && totalMatches > 0 {
				highlightFill(macro.Name, node.GetUID(), float64(count)/float64(totalMatches))
			}
			point := np.Point
			count++
			point.X += searchLeftX
			point.Y += searchTopY
			if firstPoint == nil {
				firstPoint = &robotgo.Point{X: point.X, Y: point.Y}
			}

			// Store current match and item internal variables so sub-actions can use ${StackMax}, ${Cols}, ${Rows}, ${ItemName}, ${ImagePixelWidth}, ${ImagePixelHeight}
			if macro != nil {
				if node.OutputXVariable != "" {
					setMacroVariable(macro, node.OutputXVariable, point.X)
				}
				if node.OutputYVariable != "" {
					setMacroVariable(macro, node.OutputYVariable, point.Y)
				}
				// Set item parameters from current match target (programName~itemName)
				if np.Name != "" {
					parts := strings.SplitN(np.Name, config.ProgramDelimiter, 2)
					if len(parts) == 2 {
						program, _ := repositories.ProgramRepo().Get(parts[0])
						if program != nil {
							item, _ := program.ItemRepo().Get(parts[1])
							if item != nil {
								setMacroVariable(macro, "StackMax", item.StackMax)
								setMacroVariable(macro, "Cols", item.GridSize[0])
								setMacroVariable(macro, "Rows", item.GridSize[1])
								setMacroVariable(macro, "ItemName", item.Name)

								vs := IconVariantServiceInstance()
								variants, vErr := vs.GetVariants(parts[0], parts[1])
								if vErr == nil && len(variants) > 0 {
									iconPath := vs.GetVariantPath(parts[0], parts[1], variants[0])
									if f, openErr := os.Open(iconPath); openErr == nil {
										cfg, _, decErr := image.DecodeConfig(f)
										_ = f.Close()
										if decErr == nil {
											setMacroVariable(macro, "ImagePixelWidth", cfg.Width)
											setMacroVariable(macro, "ImagePixelHeight", cfg.Height)
										}
									}
								}
							}
						}
					}
				}
			}

			brk, cont, err := handleLoopFlow(executeSubActions(node.SubActions, macro))
			if err != nil {
				return err
			}
			if cont {
				continue
			}
			if brk {
				break
			}
		}
		if len(sorted) == 0 && node.RunBranchOnNoFind {
			if _, _, err := handleLoopFlow(executeSubActions(node.SubActions, macro)); err != nil {
				return err
			}
		}
		// After the loop, set output variables to the first match so sibling actions (e.g. Move after Image Search) use first match, not last
		if firstPoint != nil && macro != nil {
			if node.OutputXVariable != "" {
				setMacroVariable(macro, node.OutputXVariable, firstPoint.X)
			}
			if node.OutputYVariable != "" {
				setMacroVariable(macro, node.OutputYVariable, firstPoint.Y)
			}
		}
		log.Printf("Total # found: %v (found: %v; not found: %v)\n", count, foundNames, notFoundNames)

		return nil
	case *actions.Ocr:
		foundText, centerX, centerY, err := OCR(node, macro)
		if err != nil {
			log.Printf("OCR: %v (macro continues)", err)
			return nil
		}
		if node.WaitTilFound && node.WaitTilFoundSeconds > 0 {
			deadline := time.Now().Add(time.Duration(node.WaitTilFoundSeconds) * time.Second)
			intervalMs := node.WaitTilFoundIntervalMs
			if intervalMs <= 0 {
				intervalMs = 500
			}
			for !strings.Contains(foundText, node.Target) && time.Now().Before(deadline) {
				time.Sleep(time.Duration(intervalMs) * time.Millisecond)
				if err := checkMacroStop(); err != nil {
					return err
				}
				foundText, centerX, centerY, err = OCR(node, macro)
				if err != nil {
					log.Printf("OCR: %v (macro continues)", err)
					break
				}
			}
		}

		// Store found text in variable if configured
		if macro != nil && node.OutputVariable != "" {
			setMacroVariable(macro, node.OutputVariable, foundText)
		}
		if macro != nil {
			if node.OutputXVariable != "" {
				setMacroVariable(macro, node.OutputXVariable, centerX)
			}
			if node.OutputYVariable != "" {
				setMacroVariable(macro, node.OutputYVariable, centerY)
			}
		}
		return nil
	case *actions.SetVariable:
		log.Println("Set Variable:", node.String())
		if macro != nil {
			val, err := ResolveSetVariableValue(node.Value, macro)
			if err != nil {
				return fmt.Errorf("set variable %s: %w", node.VariableName, err)
			}
			setMacroVariable(macro, node.VariableName, val)
		}
		return nil
	case *actions.Calculate:
		log.Println("Calculate:", node.String())
		if macro != nil {
			log.Println("evaluating expression", node.Expression)
			result, err := EvaluateExpression(node.Expression, macro)
			if err != nil {
				return fmt.Errorf("calculation failed: %w", err)
			}
			setMacroVariable(macro, node.OutputVar, result)
			log.Println("successfully set variable", node.OutputVar, result)
		}
		log.Println("successfully calculated")
		return nil
	case *actions.ForEachRow:
		log.Println("For each row:", node.String())
		return executeForEachRow(node, macro)
	case *actions.SaveVariable:
		log.Println("Save Variable:", node.String())
		if macro != nil && macro.Variables != nil {
			val, ok := macro.Variables.Get(node.VariableName)
			if !ok {
				return fmt.Errorf("variable %s not found", node.VariableName)
			}
			valStr := fmt.Sprintf("%v", val)

			if node.Destination == "clipboard" {
				if err := getAutomationBackend().WriteClipboard(valStr); err != nil {
					return fmt.Errorf("failed to write clipboard: %w", err)
				}
			} else {
				filePath := filepath.Join(config.GetVariablesPath(), node.Destination)
				if err := node.SaveToFile(valStr, filePath); err != nil {
					return fmt.Errorf("failed to save variable to file: %w", err)
				}
			}
		}
		return nil
	// case *actions.Calibration:
	// 	log.Println("Calibration:", node.String())
	// 	return RunCalibration(node, macro)
	case *actions.FocusWindow:
		log.Println("Focus Window:", node.String())
		return RunFocusWindow(node)
	case *actions.RunMacro:
		log.Println("Run Macro:", node.String())
		if node.MacroName == "" {
			return fmt.Errorf("run macro: macro name not set")
		}
		targetMacro, err := repositories.MacroRepo().Get(node.MacroName)
		if err != nil {
			return fmt.Errorf("run macro: %w", err)
		}
		if targetMacro.Root == nil {
			return fmt.Errorf("run macro: macro %q has no root", node.MacroName)
		}
		targetMacro.InitRuntimeVariables()
		ApplyMonitorBuiltinVariables(targetMacro)
		if macro != nil {
			highlightFill(macro.Name, node.GetUID(), 0)
			defer highlightClear(macro.Name, node.GetUID())
		}
		return executeRunMacroTree(node, targetMacro, macro)
	case *actions.Break:
		log.Println("Break")
		return actions.ErrBreak
	case *actions.Continue:
		log.Println("Continue")
		return actions.ErrContinue
	case *actions.Pause:
		log.Println("Pause:", node.String())
		msg := node.Message
		if macro != nil {
			if resolved, err := ResolveString(msg, macro); err == nil {
				msg = resolved
			}
		}
		keyLabel := actions.FormatContinueKey(node.ContinueKey)
		if msg != "" {
			log.Printf("Pause: waiting for %s — %q", keyLabel, msg)
		} else {
			log.Printf("Pause: waiting for %s", keyLabel)
		}
		NotifyMacroPause(true, msg, keyLabel)
		defer NotifyMacroPause(false, "", "")
		keys := append([]string(nil), node.ContinueKey...)
		passThrough := node.PassThrough
		err := WaitForContinueKey(ContinueWaitOptions{
			Keys:        keys,
			PassThrough: passThrough,
			OnMatch: func() {
				if !passThrough {
					SuppressContinueChord(keys)
				}
			},
		})
		if err != nil {
			return err
		}
		log.Printf("Pause: continued (%s)", keyLabel)
		return nil
	case *actions.FindPixel:
		log.Println("Find pixel:", node.String())
		leftX, topY, rightX, bottomY, err := ResolveSearchAreaCoordsFromRef(node.SearchArea, macro, DefaultResolutionKey())
		if err != nil {
			log.Printf("FindPixel: failed to resolve search area %q: %v, skipping", node.SearchArea, err)
			return nil
		}

		var foundX, foundY int
		matchColor := node.ColorMatcher()
		scanOnce := func() bool {
			captureImg, capLeftX, capTopY, _, _, capErr := CaptureSearchArea(leftX, topY, rightX, bottomY)
			if capErr != nil || captureImg == nil {
				log.Printf("FindPixel: screen capture failed: %v", capErr)
				return false
			}
			// Scan the raw RGBA buffer directly: avoids an interface call,
			// hex formatting, and hex parsing for every pixel.
			rgba := captureToRGBA(captureImg)
			bounds := rgba.Bounds()
			for py := bounds.Min.Y; py < bounds.Max.Y; py++ {
				for px := bounds.Min.X; px < bounds.Max.X; px++ {
					o := rgba.PixOffset(px, py)
					if matchColor(rgba.Pix[o], rgba.Pix[o+1], rgba.Pix[o+2]) {
						foundX = capLeftX + px - bounds.Min.X
						foundY = capTopY + py - bounds.Min.Y
						return true
					}
				}
			}
			return false
		}

		found := scanOnce()
		if !found && node.WaitTilFound && node.WaitTilFoundSeconds > 0 {
			deadline := time.Now().Add(time.Duration(node.WaitTilFoundSeconds) * time.Second)
			intervalMs := node.WaitTilFoundIntervalMs
			if intervalMs <= 0 {
				intervalMs = 100
			}
			for !found && time.Now().Before(deadline) {
				time.Sleep(time.Duration(intervalMs) * time.Millisecond)
				if err := checkMacroStop(); err != nil {
					return err
				}
				found = scanOnce()
			}
		}

		if found {
			log.Printf("FindPixel: found matching pixel at screen (%d, %d)", foundX, foundY)
			if macro != nil {
				if node.OutputXVariable != "" {
					setMacroVariable(macro, node.OutputXVariable, foundX)
				}
				if node.OutputYVariable != "" {
					setMacroVariable(macro, node.OutputYVariable, foundY)
				}
			}
		} else {
			log.Println("FindPixel: pixel not found")
		}
		return nil
	}
	return nil
}
