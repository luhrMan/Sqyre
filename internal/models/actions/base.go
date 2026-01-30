package actions

import "github.com/google/uuid"

type BaseAction struct {
	Type   string
	uid    string
	Parent AdvancedActionInterface `yaml:"-" gob:"-" mapstructure:"-"`
}

func newBaseAction(t string) *BaseAction {
	return &BaseAction{
		Type: t,
		uid:  uuid.NewString(),
	}
}

// Point represents a screen coordinate for move actions.
// X and Y may be int (literal) or string (variable reference e.g. "${resultX}").
type Point struct {
	Name string
	X    interface{}
	Y    interface{}
}
