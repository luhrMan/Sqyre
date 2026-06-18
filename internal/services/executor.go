package services

import (
	"Sqyre/internal/assets"
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"bytes"
	"fmt"
	"image"
	_ "image/png"
	"log"
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
	return executeWithContext(a, macroCtx)
}

// ExecuteMacroWithLogging runs a macro with log capture and shows the log popup.
// Use this instead of Execute when running a macro from the UI or hotkey.
func ExecuteMacroWithLogging(m *models.Macro) {
	if m == nil {
		return
	}
	defer func() {
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
	if m.Variables != nil {
		m.Variables.NormalizeKeys()
	}
	ApplyMonitorBuiltinVariables(m)
	SnapshotRuntimeVariables(m)
	if err := Execute(m.Root, m); err != nil {
		log.Printf("Macro %q: execution error: %v", m.Name, err)
	}
}

// showMacroLogPopupFunc is set by ui/macro to avoid import cycle
var showMacroLogPopupFunc func(macroName string)

// SetShowMacroLogPopupFunc sets the callback to show the macro log popup.
// Called from ui package during initialization.
func SetShowMacroLogPopupFunc(fn func(macroName string)) {
	showMacroLogPopupFunc = fn
}

// macroRunningCallback is invoked on the UI thread when a macro starts (true) or stops (false).
// Used to disable the macro play button while a macro is running.
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

func executeForEachRow(node *actions.ForEachRow, macro *models.Macro) error {
	if len(node.Sources) == 0 {
		return fmt.Errorf("for each row %q: at least one source is required", node.Name)
	}
	rowCount, err := node.Sources[0].LineCount()
	if err != nil {
		return fmt.Errorf("for each row %q: %w", node.Name, err)
	}
	for i := 0; i < rowCount; i++ {
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
		for _, action := range node.GetSubActions() {
			if err := executeWithContext(action, macro); err != nil {
				return err
			}
		}
	}
	return nil
}

func executeWithContext(a actions.ActionInterface, macro *models.Macro) error {
	switch node := a.(type) {
	case *actions.Wait:
		log.Println("Wait:", node.String())
		time := node.Time
		getAutomationBackend().MilliSleep(time)
		return nil
	case *actions.Move:
		log.Println("Move:", node.String())
		x, err := ResolveInt(node.Point.X, macro)
		if err != nil {
			log.Printf("Move: failed to resolve X %v: %v, using 0 (ensure variable is set by an earlier action, e.g. Image Search output)", node.Point.X, err)
			x = 0
		}
		y, err := ResolveInt(node.Point.Y, macro)
		if err != nil {
			log.Printf("Move: failed to resolve Y %v: %v, using 0 (ensure variable is set by an earlier action, e.g. Image Search output)", node.Point.Y, err)
			y = 0
		}
		getAutomationBackend().Move(x, y, node.Smooth)
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
			backend.TypeChar(string(r))
			if delayMs > 0 {
				backend.MilliSleep(delayMs)
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
			log.Printf("Loop: %s iteration %d", node.Name, i+1)
			for _, action := range node.GetSubActions() {
				if err := executeWithContext(action, macro); err != nil {
					fyne.DoAndWait(func() {
						MacroActiveIndicator().Stop()
						MacroActiveIndicator().Hide()
					})
					return err
				}
			}
			if node.Name == "root" {
				fyne.Do(func() {
					MacroActiveIndicator().Stop()
					MacroActiveIndicator().Hide()
				})
			}

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
		for _, action := range node.GetSubActions() {
			if err := executeWithContext(action, macro); err != nil {
				return err
			}
		}
		return nil
	case *actions.ImageSearch:
		log.Println("Image Search:", node.String())
		results, err := imageSearch(node, macro)
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
				results, err = imageSearch(node, macro)
				if err != nil {
					log.Printf("Image Search: %v (macro continues)", err)
					if results == nil {
						results = make(map[string][]robotgo.Point)
					}
				}
			}
		}
		searchLeftX, err := ResolveInt(node.SearchArea.LeftX, macro)
		if err != nil {
			log.Printf("Image Search: failed to resolve SearchArea.LeftX %v: %v, using 0", node.SearchArea.LeftX, err)
			searchLeftX = 0
		}
		searchTopY, err := ResolveInt(node.SearchArea.TopY, macro)
		if err != nil {
			log.Printf("Image Search: failed to resolve SearchArea.TopY %v: %v, using 0", node.SearchArea.TopY, err)
			searchTopY = 0
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
		var firstPoint *robotgo.Point
		for _, np := range sorted {
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
									ip := parts[0] + config.ProgramDelimiter + parts[1] + config.ProgramDelimiter + variants[0] + config.PNG
									if res := assets.GetFyneResource(ip); res != nil {
										if cfg, _, err := image.DecodeConfig(bytes.NewReader(res.Content())); err == nil {
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

			for _, a := range node.SubActions {
				if err := executeWithContext(a, macro); err != nil {
					return err
				}
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

		if strings.Contains(foundText, node.Target) {
			for _, action := range node.SubActions {
				if err := executeWithContext(action, macro); err != nil {
					return err
				}
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
		if targetMacro.Variables != nil {
			targetMacro.Variables.NormalizeKeys()
		}
		ApplyMonitorBuiltinVariables(targetMacro)
		return executeWithContext(targetMacro.Root, targetMacro)
	case *actions.FindPixel:
		log.Println("Find pixel:", node.String())
		sa := node.SearchArea
		leftX, topY, rightX, bottomY, err := ResolveSearchAreaCoords(sa.LeftX, sa.TopY, sa.RightX, sa.BottomY, macro)
		if err != nil {
			log.Printf("FindPixel: failed to resolve search area coords: %v, skipping", err)
			return nil
		}

		var foundX, foundY int
		scanOnce := func() bool {
			captureImg, capLeftX, capTopY, _, _, capErr := CaptureSearchArea(leftX, topY, rightX, bottomY)
			if capErr != nil || captureImg == nil {
				log.Printf("FindPixel: screen capture failed: %v", capErr)
				return false
			}
			bounds := captureImg.Bounds()
			for py := bounds.Min.Y; py < bounds.Max.Y; py++ {
				for px := bounds.Min.X; px < bounds.Max.X; px++ {
					r, g, b, _ := captureImg.At(px, py).RGBA()
					hex := fmt.Sprintf("%02x%02x%02x", uint8(r>>8), uint8(g>>8), uint8(b>>8))
					if node.MatchColor(hex) {
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
			for _, sub := range node.GetSubActions() {
				if err := executeWithContext(sub, macro); err != nil {
					return err
				}
			}
		} else {
			log.Println("FindPixel: pixel not found, continuing without children")
		}
	}
	return nil
}
