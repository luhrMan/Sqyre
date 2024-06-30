package actions

import "strconv"

type Repeater struct {
	Amount  int
	Starter bool
}

func (Repeater) ActionType() string {
	return "Repeater"
}

func (r Repeater) PrintParams() string {
	var str string
	if r.Starter {
		str = r.ActionType() + " " + strconv.FormatInt(int64(r.Amount), 10) + " times" //+ " at " + strconv.FormatInt(int64(gt.Coordinates.X), 10) + ", " + strconv.FormatInt(int64(gt.Coordinates.Y), 10)
	} else {
		str = r.ActionType() + " end"
	}
	return str
}
