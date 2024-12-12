package actions

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
