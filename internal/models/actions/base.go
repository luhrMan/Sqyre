package actions

import "github.com/google/uuid"

type BaseAction struct {
	Type   string
	uid    string
	Parent AdvancedActionInterface `yaml:"-" gob:"-" mapstructure:"-"`
}

func (a *BaseAction) parameters() []actionParam {
	return []actionParam{
		newParam("Type", a.Type),
	}
}

// SetUID replaces the action UID when restoring from a snapshot (undo/redo).
func (a *BaseAction) SetUID(uid string) {
	if uid != "" {
		a.uid = uid
	}
}

// RestoreUID sets uid on a when non-empty (snapshot restore).
func RestoreUID(a ActionInterface, uid string) {
	if uid == "" || a == nil {
		return
	}
	type uidSetter interface {
		SetUID(string)
	}
	if s, ok := a.(uidSetter); ok {
		s.SetUID(uid)
	}
}

func newBaseAction(t string) *BaseAction {
	return &BaseAction{
		Type: t,
		uid:  uuid.NewString(),
	}
}
