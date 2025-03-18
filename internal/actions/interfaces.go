package actions

import (
	"fmt"
	"log"
)

type ActionInterface interface {
	Execute(ctx any) error

	GetUID() string
	SetUID(string)

	GetParent() AdvancedActionInterface
	SetParent(AdvancedActionInterface)

	String() string

	UpdateBaseAction(uid string, parent AdvancedActionInterface)
}
type AdvancedActionInterface interface {
	ActionInterface

	GetName() string
	SetName(string)

	GetAction(string) ActionInterface
	GetSubActions() []ActionInterface
	AddSubAction(ActionInterface)
	RemoveSubAction(ActionInterface)
	RenameActions()
}

func (a *baseAction) UpdateBaseAction(uid string, parent AdvancedActionInterface) {
	a.SetUID(uid)
	a.SetParent(parent)
}
func (a *baseAction) GetUID() string                           { return a.UID }
func (a *baseAction) SetUID(uid string)                        { a.UID = uid }
func (a *baseAction) GetParent() AdvancedActionInterface       { return a.Parent }
func (a *baseAction) SetParent(action AdvancedActionInterface) { a.Parent = action }
func (a *baseAction) Execute(ctx any) error                    { return nil }
func (a *baseAction) String() string                           { return "This is a baseAction" }
func (a *advancedAction) GetName() string                      { return a.Name }
func (a *advancedAction) SetName(name string)                  { a.Name = name }
func (a *advancedAction) GetSubActions() []ActionInterface     { return a.SubActions }

func (a *advancedAction) GetAction(uid string) ActionInterface {
	if a.GetUID() == uid {
		return a
	}
	for _, c := range a.SubActions {
		if c.GetUID() == uid {
			return c
		}
		if aa, ok := c.(AdvancedActionInterface); ok {
			if action := aa.GetAction(uid); action != nil && action.GetUID() == uid {
				return action
			}
		}
	}

	return nil
}

func (a *advancedAction) AddSubAction(action ActionInterface) {
	actionNum := len(a.GetSubActions()) + 1
	uid := fmt.Sprintf("%s.%d", a.GetUID(), actionNum)
	action.UpdateBaseAction(uid, a)

	a.SubActions = append(a.SubActions, action)
	log.Printf("Added new action: %v %s", uid, action.String())
}

func (a *advancedAction) RemoveSubAction(action ActionInterface) {
	for i, c := range a.SubActions {
		if c == action {
			a.SubActions = append(a.SubActions[:i], a.SubActions[i+1:]...)
			log.Printf("Removing %s", action.GetUID())
			a.RenameActions()
		}
	}
}

func (a *advancedAction) RenameActions() {
	for i, child := range a.SubActions {
		if n, ok := child.(AdvancedActionInterface); ok {
			n.RenameActions()
		}
		child.SetUID(fmt.Sprintf("%s.%d", a.UID, i+1))
	}
}

func (a *advancedAction) Execute(ctx any) error {
	log.Printf("Executing %s", a.Name)

	for _, c := range a.SubActions {
		c.Execute(ctx)
	}
	return nil
}
func (a *advancedAction) String() string { return "This is an Advanced Action" }
