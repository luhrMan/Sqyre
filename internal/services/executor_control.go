package services

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"fmt"
	"log"

	"fyne.io/fyne/v2"
)

func init() {
	registerActionRunner("loop", executeLoop)
	registerActionRunner("conditional", executeConditional)
	registerActionRunner("break", executeBreak)
	registerActionRunner("continue", executeContinue)
	registerActionRunner("pause", executePause)
}

func executeLoop(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.Loop)
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
}

func executeConditional(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.Conditional)
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
}

func executeBreak(actions.ActionInterface, *models.Macro) error {
	log.Println("Break")
	return actions.ErrBreak
}

func executeContinue(actions.ActionInterface, *models.Macro) error {
	log.Println("Continue")
	return actions.ErrContinue
}

func executePause(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.Pause)
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
}
