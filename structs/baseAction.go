package structs

type BaseAction struct {
	UID           string                  `json:"uid"`
	Parent        AdvancedActionInterface `json:"-"`
	NewBaseAction func()                  `json:"-"`
}

func NewBaseAction() BaseAction {
	return BaseAction{
		UID: "temp uid",
	}
}

func (a *BaseAction) updateBaseAction(uid string, parent AdvancedActionInterface) {
	a.SetUID(uid)
	a.SetParent(parent)
}
