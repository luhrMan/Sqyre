package structs

type baseAction struct {
	UID           string                  `json:"uid"`
	Parent        AdvancedActionInterface `json:"-"`
	newBaseAction func()
}

func newBaseAction() baseAction {
	return baseAction{
		UID: "temp uid",
	}
}

func (a *baseAction) UpdateBaseAction(uid string, parent AdvancedActionInterface) {
	a.SetUID(uid)
	a.SetParent(parent)
}
