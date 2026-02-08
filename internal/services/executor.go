package services

import (
	"Squire/internal/config"
	"Squire/internal/models"
	"Squire/internal/models/actions"
	"fmt"
	"log"
	"path/filepath"
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

// resetDataListsInTree resets every DataList in the action tree so each macro run starts from line 0.
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
		time := node.Time
		robotgo.MilliSleep(time)
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
		robotgo.Move(x+config.XOffset, y+config.YOffset)
		return nil
	case *actions.Click:
		log.Println("Click:", node.String())
		robotgo.Click(actions.LeftOrRight(node.Button))
		return nil
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
			err := robotgo.KeyDown(key)
			if err != nil {
				return err
			}
		} else {
			err := robotgo.KeyUp(key)
			if err != nil {
				return err
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
			fmt.Printf("Loop: %s iteration %d\n", node.Name, i+1)
			for j, action := range node.GetSubActions() {
				if node.Name == "root" {
					progress = progressStep * float64(j+1)
					log.Println(progress)
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
		results, err := imageSearch(node, macro)
		if err != nil {
			return err
		}
		if node.WaitTilFound && node.WaitTilFoundSeconds > 0 {
			deadline := time.Now().Add(time.Duration(node.WaitTilFoundSeconds) * time.Second)
			for len(SortListOfPoints(results)) == 0 && time.Now().Before(deadline) {
				time.Sleep(100 * time.Millisecond)
				results, err = imageSearch(node, macro)
				if err != nil {
					return err
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
		count := 0
		var firstPoint *robotgo.Point
		for _, point := range sorted {
			count++
			point.X += searchLeftX
			point.Y += searchTopY
			if firstPoint == nil {
				firstPoint = &robotgo.Point{X: point.X, Y: point.Y}
			}

			// Store current match in variables so sub-actions see this match
			if macro != nil && macro.Variables != nil {
				if node.OutputXVariable != "" {
					macro.Variables.Set(node.OutputXVariable, point.X)
				}
				if node.OutputYVariable != "" {
					macro.Variables.Set(node.OutputYVariable, point.Y)
				}
			}

			for _, a := range node.SubActions {
				if err := executeWithContext(a, macro); err != nil {
					return err
				}
			}
		}
		// After the loop, set output variables to the first match so sibling actions (e.g. Move after Image Search) use first match, not last
		if firstPoint != nil && macro != nil && macro.Variables != nil {
			if node.OutputXVariable != "" {
				macro.Variables.Set(node.OutputXVariable, firstPoint.X)
			}
			if node.OutputYVariable != "" {
				macro.Variables.Set(node.OutputYVariable, firstPoint.Y)
			}
		}
		log.Printf("Total # found: %v\n", count)

		return nil
	case *actions.Ocr:
		foundText, err := OCR(node, macro)
		if err != nil {
			log.Println(err)
			return err
		}
		if node.WaitTilFound && node.WaitTilFoundSeconds > 0 {
			deadline := time.Now().Add(time.Duration(node.WaitTilFoundSeconds) * time.Second)
			for !strings.Contains(foundText, node.Target) && time.Now().Before(deadline) {
				time.Sleep(500 * time.Millisecond)
				foundText, err = OCR(node, macro)
				if err != nil {
					log.Println(err)
					return err
				}
			}
		}

		// Store found text in variable if configured
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
			log.Println("evaluating expression", node.Expression)
			result, err := EvaluateExpression(node.Expression, macro)
			if err != nil {
				return fmt.Errorf("calculation failed: %w", err)
			}
			macro.Variables.Set(node.OutputVar, result)
			log.Println("successfully set variable", node.OutputVar, result)
		}
		log.Println("successfully calculated")
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
			node.NextLine() // Advance to next line for next cycle
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
				robotgo.WriteAll(valStr)
			} else {
				filePath := filepath.Join(config.GetVariablesPath(), node.Destination)
				if err := node.SaveToFile(valStr, filePath); err != nil {
					return fmt.Errorf("failed to save variable to file: %w", err)
				}
			}
		}
		return nil
	case *actions.Calibration:
		log.Println("Calibration:", node.String())
		return RunCalibration(node, macro)
	case *actions.WaitForPixel:
		log.Println("Wait for pixel:", node.String())
		x, err := ResolveInt(node.Point.X, macro)
		if err != nil {
			log.Printf("WaitForPixel: failed to resolve X %v: %v, using 0", node.Point.X, err)
			x = 0
		}
		y, err := ResolveInt(node.Point.Y, macro)
		if err != nil {
			log.Printf("WaitForPixel: failed to resolve Y %v: %v, using 0", node.Point.Y, err)
			y = 0
		}
		screenX := x + config.XOffset
		screenY := y + config.YOffset
		var deadline time.Time
		if node.TimeoutSeconds > 0 {
			deadline = time.Now().Add(time.Duration(node.TimeoutSeconds) * time.Second)
		}
		for {
			hex := robotgo.GetPixelColor(screenX, screenY)
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
