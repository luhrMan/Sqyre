//go:build android

package services

import (
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"Squire/internal/models/repositories"
	"Squire/internal/android"
	"fmt"
	"log"
	"path/filepath"
	"strings"
	"time"

	"fyne.io/fyne/v2"
)

// Point is used for image search results on Android (replaces robotgo.Point).
type point struct{ X, Y int }

// Execute runs the action with optional macro context (Android implementation).
func Execute(a actions.ActionInterface, macro ...*models.Macro) error {
	var macroCtx *models.Macro
	if len(macro) > 0 {
		macroCtx = macro[0]
	}
	return executeWithContext(a, macroCtx)
}

func resetDataListsInTree(a actions.ActionInterface) {
	if dl, ok := a.(*actions.DataList); ok {
		dl.Reset()
	}
	if adv, ok := a.(actions.AdvancedActionInterface); ok {
		for _, sub := range adv.GetSubActions() {
			resetDataListsInTree(sub)
		}
	}
}

func executeWithContext(a actions.ActionInterface, macro *models.Macro) error {
	switch node := a.(type) {
	case *actions.Wait:
		log.Println("Wait:", node.String())
		time.Sleep(time.Duration(node.Time) * time.Millisecond)
		return nil

	case *actions.Move:
		log.Println("Move:", node.String())
		x, err := ResolveInt(node.Point.X, macro)
		if err != nil {
			log.Printf("Move: failed to resolve X %v: %v, using 0", node.Point.X, err)
			x = 0
		}
		y, err := ResolveInt(node.Point.Y, macro)
		if err != nil {
			log.Printf("Move: failed to resolve Y %v: %v, using 0", node.Point.Y, err)
			y = 0
		}
		android.SetLastTapPosition(x, y)
		// On Android we don't have a cursor; tap at (x,y) to simulate move + click position
		if err := android.PerformTap(x, y); err != nil {
			return fmt.Errorf("move/tap: %w", err)
		}
		return nil

	case *actions.Click:
		log.Println("Click:", node.String())
		x, y := android.GetLastTapPosition()
		if err := android.PerformTap(x, y); err != nil {
			return fmt.Errorf("click: %w", err)
		}
		return nil

	case *actions.Key:
		log.Println("Key:", node.String())
		key := node.Key
		if macro != nil {
			if resolved, err := ResolveString(key, macro); err == nil {
				key = resolved
			}
		}
		if err := android.KeyEvent(key, node.State); err != nil {
			return fmt.Errorf("key: %w", err)
		}
		return nil

	case *actions.Type:
		log.Println("Type:", node.String())
		text := node.Text
		if macro != nil {
			if resolved, err := ResolveString(text, macro); err == nil {
				text = resolved
			}
		}
		delayMs := node.DelayMs
		if delayMs < 0 {
			delayMs = 0
		}
		if err := android.TypeText(text, delayMs); err != nil {
			return fmt.Errorf("type: %w", err)
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
		var progress, progressStep float64
		if node.Name == "root" {
			resetDataListsInTree(node)
			progressStep = (100.0 / float64(len(node.GetSubActions()))) / 100
			fyne.Do(func() {
				MacroActiveIndicator().Show()
				MacroActiveIndicator().Start()
			})
		}
		for i := range count {
			log.Printf("Loop: %s iteration %d", node.Name, i+1)
			for j, action := range node.GetSubActions() {
				if node.Name == "root" {
					progress = progressStep * float64(j+1)
					fyne.Do(func() {
						MacroProgressBar().SetValue(progress)
						MacroProgressBar().Refresh()
					})
				}
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

	case *actions.ImageSearch:
		log.Println("Image Search:", node.String())
		results, err := imageSearchAndroid(node, macro)
		if err != nil {
			return err
		}
		if node.WaitTilFound && node.WaitTilFoundSeconds > 0 {
			deadline := time.Now().Add(time.Duration(node.WaitTilFoundSeconds) * time.Second)
			for len(results) == 0 && time.Now().Before(deadline) {
				time.Sleep(100 * time.Millisecond)
				results, err = imageSearchAndroid(node, macro)
				if err != nil {
					return err
				}
			}
		}
		searchLeftX, _ := ResolveInt(node.SearchArea.LeftX, macro)
		searchTopY, _ := ResolveInt(node.SearchArea.TopY, macro)
		var firstPoint *point
		for i := range results {
			pt := &results[i]
			pt.X += searchLeftX
			pt.Y += searchTopY
			if firstPoint == nil {
				firstPoint = &point{X: pt.X, Y: pt.Y}
			}
			if macro != nil && macro.Variables != nil {
				if node.OutputXVariable != "" {
					macro.Variables.Set(node.OutputXVariable, pt.X)
				}
				if node.OutputYVariable != "" {
					macro.Variables.Set(node.OutputYVariable, pt.Y)
				}
			}
			for _, sub := range node.SubActions {
				if err := executeWithContext(sub, macro); err != nil {
					return err
				}
			}
		}
		if firstPoint != nil && macro != nil && macro.Variables != nil {
			if node.OutputXVariable != "" {
				macro.Variables.Set(node.OutputXVariable, firstPoint.X)
			}
			if node.OutputYVariable != "" {
				macro.Variables.Set(node.OutputYVariable, firstPoint.Y)
			}
		}
		log.Printf("Image Search: %d match(es)", len(results))
		return nil

	case *actions.Ocr:
		foundText, err := ocrAndroid(node, macro)
		if err != nil {
			log.Println(err)
			return err
		}
		if node.WaitTilFound && node.WaitTilFoundSeconds > 0 {
			deadline := time.Now().Add(time.Duration(node.WaitTilFoundSeconds) * time.Second)
			for !strings.Contains(foundText, node.Target) && time.Now().Before(deadline) {
				time.Sleep(500 * time.Millisecond)
				foundText, err = ocrAndroid(node, macro)
				if err != nil {
					return err
				}
			}
		}
		if macro != nil && macro.Variables != nil && node.OutputVariable != "" {
			macro.Variables.Set(node.OutputVariable, foundText)
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
		if macro != nil && macro.Variables != nil {
			macro.Variables.Set(node.VariableName, node.Value)
		}
		return nil

	case *actions.Calculate:
		log.Println("Calculate:", node.String())
		if macro != nil {
			if macro.Variables == nil {
				macro.Variables = models.NewVariableStore()
			}
			result, err := EvaluateExpression(node.Expression, macro)
			if err != nil {
				return fmt.Errorf("calculation failed: %w", err)
			}
			macro.Variables.Set(node.OutputVar, result)
		}
		return nil

	case *actions.DataList:
		log.Println("Data List:", node.String())
		if macro != nil && macro.Variables != nil {
			line, err := node.GetCurrentLine()
			if err != nil {
				return fmt.Errorf("data list error: %w", err)
			}
			macro.Variables.Set(node.OutputVar, line)
			if node.LengthVar != "" {
				lineCount, err := node.LineCount()
				if err != nil {
					return fmt.Errorf("data list length: %w", err)
				}
				macro.Variables.Set(node.LengthVar, lineCount)
			}
			node.NextLine()
		}
		return nil

	case *actions.SaveVariable:
		log.Println("Save Variable:", node.String())
		if macro != nil && macro.Variables != nil {
			val, ok := macro.Variables.Get(node.VariableName)
			if !ok {
				return fmt.Errorf("variable %s not found", node.VariableName)
			}
			valStr := fmt.Sprintf("%v", val)
			if node.Destination == "clipboard" {
				if err := android.SetClipboard(valStr); err != nil {
					return fmt.Errorf("clipboard: %w", err)
				}
			} else {
				filePath := filepath.Join(config.GetVariablesPath(), node.Destination)
				if err := node.SaveToFile(valStr, filePath); err != nil {
					return fmt.Errorf("save variable to file: %w", err)
				}
			}
		}
		return nil

	case *actions.FocusWindow:
		log.Println("Focus Window:", node.String())
		return runFocusWindowAndroid(node)

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
		return executeWithContext(targetMacro.Root, targetMacro)

	case *actions.WaitForPixel:
		log.Println("Wait for pixel:", node.String())
		x, err := ResolveInt(node.Point.X, macro)
		if err != nil {
			x = 0
		}
		y, err := ResolveInt(node.Point.Y, macro)
		if err != nil {
			y = 0
		}
		var deadline time.Time
		if node.TimeoutSeconds > 0 {
			deadline = time.Now().Add(time.Duration(node.TimeoutSeconds) * time.Second)
		}
		for {
			hex, err := android.GetPixelColor(x, y)
			if err != nil {
				return fmt.Errorf("waitForPixel: %w", err)
			}
			if node.MatchColor(hex) {
				for _, sub := range node.GetSubActions() {
					if err := executeWithContext(sub, macro); err != nil {
						return err
					}
				}
				return nil
			}
			if node.TimeoutSeconds > 0 && !time.Now().Before(deadline) {
				log.Println("WaitForPixel: timeout, continuing without children")
				return nil
			}
			time.Sleep(50 * time.Millisecond)
		}
	}
	return nil
}

func imageSearchAndroid(a *actions.ImageSearch, macro *models.Macro) ([]point, error) {
	leftX, topY, rightX, bottomY, err := ResolveSearchAreaCoords(a.SearchArea.LeftX, a.SearchArea.TopY, a.SearchArea.RightX, a.SearchArea.BottomY, macro)
	if err != nil {
		return nil, err
	}
	w := rightX - leftX
	h := bottomY - topY
	if w <= 0 || h <= 0 {
		return nil, fmt.Errorf("image search: invalid search area (width=%d height=%d)", w, h)
	}
	pts, err := android.ImageSearch(leftX, topY, w, h, a.Targets, a.Tolerance)
	if err != nil {
		return nil, err
	}
	out := make([]point, len(pts))
	for i := range pts {
		out[i] = point{X: pts[i].X, Y: pts[i].Y}
	}
	return out, nil
}

func ocrAndroid(a *actions.Ocr, macro *models.Macro) (string, error) {
	leftX, topY, rightX, bottomY, err := ResolveSearchAreaCoords(a.SearchArea.LeftX, a.SearchArea.TopY, a.SearchArea.RightX, a.SearchArea.BottomY, macro)
	if err != nil {
		return "", err
	}
	w := rightX - leftX
	h := bottomY - topY
	if w <= 0 || h <= 0 {
		return "", fmt.Errorf("OCR: invalid search area (width=%d height=%d)", w, h)
	}
	return android.OCR(leftX, topY, w, h)
}

func runFocusWindowAndroid(a *actions.FocusWindow) error {
	return android.FocusWindow(a.WindowTarget)
}

// ActiveWindowNames returns running app/window names (for Focus Window picker).
func ActiveWindowNames() ([]string, error) {
	return android.WindowNames()
}

// RunFocusWindow focuses the window matching the target name.
func RunFocusWindow(a *actions.FocusWindow) error {
	return runFocusWindowAndroid(a)
}
