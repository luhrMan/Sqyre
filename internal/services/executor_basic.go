package services

import (
	macropkg "Sqyre/internal/macro"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"fmt"
	"log"
)

func init() {
	registerActionRunner("wait", executeWait)
	registerActionRunner("move", executeMove)
	registerActionRunner("click", executeClick)
	registerActionRunner("key", executeKey)
	registerActionRunner("type", executeType)
}

func executeWait(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.Wait)
	log.Println("Wait:", node.String())
	time, err := macropkg.ResolveInt(node.Time, macro)
	if err != nil {
		return fmt.Errorf("wait time: %w", err)
	}
	return interruptibleSleep(time)
}

func executeMove(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.Move)
	log.Println("Move:", node.String())
	pt, err := macropkg.LookupPoint(node.Point, macropkg.DefaultResolutionKey())
	if err != nil {
		log.Printf("Move: failed to lookup point %q: %v, using (0,0)", node.Point, err)
		getAutomationBackend().Move(0, 0, moveOpts(node))
		return nil
	}
	x, err := macropkg.ResolveInt(pt.X, macro)
	if err != nil {
		log.Printf("Move: failed to resolve X %v: %v, using 0 (ensure variable is set by an earlier action, e.g. Image Search output)", pt.X, err)
		x = 0
	}
	y, err := macropkg.ResolveInt(pt.Y, macro)
	if err != nil {
		log.Printf("Move: failed to resolve Y %v: %v, using 0 (ensure variable is set by an earlier action, e.g. Image Search output)", pt.Y, err)
		y = 0
	}
	getAutomationBackend().Move(x, y, moveOpts(node))
	return nil
}

func executeClick(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.Click)
	log.Println("Click:", node.String())
	backend := getAutomationBackend()
	if node.Button == actions.ClickButtonScroll {
		return backend.Scroll(!node.State)
	}
	btn := node.Button
	if btn != actions.ClickButtonLeft && btn != actions.ClickButtonRight && btn != actions.ClickButtonCenter {
		btn = actions.ClickButtonLeft
	}
	if node.State {
		return backend.Click(btn, true)
	}
	return backend.Click(btn, false)
}

func executeKey(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.Key)
	log.Println("Key:", node.String())
	suppressEsc := IsEscapeKey(node.Key)
	if suppressEsc {
		BeginMacroKeyActionEscape()
		defer EndMacroKeyActionEscape()
	}
	var err error
	if node.State {
		err = getAutomationBackend().KeyDown(node.Key)
		if err == nil {
			noteMacroKeyDown(node.Key)
		}
	} else {
		err = getAutomationBackend().KeyUp(node.Key)
		if err == nil {
			noteMacroKeyUp(node.Key)
		}
	}
	if suppressEsc && err == nil {
		extendMacroEscapeSuppress(macroEscapeSuppressGrace)
	}
	return err
}

func executeType(a actions.ActionInterface, macro *models.Macro) error {
	node := a.(*actions.Type)
	log.Println("Type:", node.String())
	text := node.Text
	if macro != nil {
		if resolved, err := macropkg.ResolveString(text, macro); err == nil {
			text = resolved
		}
	}
	delayMs := max(node.DelayMs, 0)
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
}
