package actions

type Goto struct {
	//ActionType  string
	Place       string
	Coordinates [2]int
}

func (Goto) ActionType() string {
	return "Go To"
}

func (gt Goto) PrintParams() string {
	str := gt.ActionType() + " " + gt.Place
	return str
}
