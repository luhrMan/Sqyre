package services

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"fmt"
	"log"
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
		ReleaseAllMacroInputs()
		ClearHighlights()
		NotifyMacroPause(false, "", "")
		onUIThread(func() {
			stopMacroIndicator()
			hideMacroIndicator()
			if macroRunningCallback != nil {
				macroRunningCallback(false)
			}
		})
		if r := recover(); r != nil {
			LogPanicToFile(r, fmt.Sprintf("Macro %q", m.Name))
		}
		scheduleMemoryReclaim(m.Name)
	}()
	if showMacroLogPopupFunc != nil {
		onUIThreadAndWait(func() {
			showMacroLogPopupFunc(m.Name)
			if macroRunningCallback != nil {
				macroRunningCallback(true)
			}
		})
		defer StopMacroLogCapture()
	} else {
		onUIThreadAndWait(func() {
			if macroRunningCallback != nil {
				macroRunningCallback(true)
			}
		})
	}
	ClearRuntimeVariables()
	m.InitRuntimeVariables()
	resetMacroHeldKeys()
	ApplyMonitorBuiltinVariables(m)
	SnapshotRuntimeVariables(m)
	if macroUsesOCR(m) {
		WarmUpOCR()
	}
	if macroUsesSemantic(m) {
		WarmUpDetector()
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

var showMacroLogPopupFunc func(macroName string)

func SetShowMacroLogPopupFunc(fn func(macroName string)) {
	showMacroLogPopupFunc = fn
}

var macroRunningCallback func(running bool)

func SetMacroRunningCallback(fn func(running bool)) {
	macroRunningCallback = fn
}

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
	highlightCursor(target.Name, "")
	return nil
}

func executeWithContext(a actions.ActionInterface, macro *models.Macro) error {
	err := executeAction(a, macro)
	if err == nil || actions.IsFlowControl(err) {
		if delayErr := applyActionDelay(macro, a); delayErr != nil {
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
