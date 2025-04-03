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
	GetType() string
}

type AdvancedActionInterface interface {
	ActionInterface

	GetName() string
	SetName(string)

	GetAction(string) ActionInterface
	GetSubActions() []ActionInterface
	SetSubActions([]ActionInterface)
	AddSubAction(ActionInterface)
	RemoveSubAction(ActionInterface)
}

func (a *BaseAction) GetType() string                          { return a.Type }
func (a *BaseAction) GetUID() string                           { return a.uid }
func (a *BaseAction) SetUID(uid string)                        { a.uid = uid }
func (a *BaseAction) GetParent() AdvancedActionInterface       { return a.Parent }
func (a *BaseAction) SetParent(action AdvancedActionInterface) { a.Parent = action }
func (a *BaseAction) Execute(ctx any) error                    { return nil }
func (a *BaseAction) String() string                           { return "This is a baseAction" }
func (a *AdvancedAction) GetName() string                      { return a.Name }
func (a *AdvancedAction) SetName(name string)                  { a.Name = name }
func (a *AdvancedAction) GetSubActions() []ActionInterface     { return a.SubActions }
func (a *AdvancedAction) SetSubActions(sa []ActionInterface)   { a.SubActions = sa }

func (a *AdvancedAction) GetAction(uid string) ActionInterface {
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

func (a *AdvancedAction) AddSubAction(action ActionInterface) {
	action.SetParent(a)
	a.SubActions = append(a.SubActions, action)
	log.Printf("Added new action: %s to parent: %v", action.String(), a.Name)
}

func (a *AdvancedAction) RemoveSubAction(action ActionInterface) {
	for i, c := range a.SubActions {
		if c == action {
			a.SubActions = append(a.SubActions[:i], a.SubActions[i+1:]...)
			log.Printf("Removing %s", action.GetUID())
			return
		}
	}
}

func (a *AdvancedAction) Execute(ctx any) error {
	log.Printf("Executing %s", a.Name)
	for _, c := range a.SubActions {
		c.Execute(ctx)
	}
	return nil
}
func (a *AdvancedAction) String() string { return fmt.Sprintf("Advanced Action: %v", a.Type) }

func (a *AdvancedAction) GetType() string { return a.Type }
