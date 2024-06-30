package actions

import (
	"Dark-And-Darker/structs"
)

type MouseMove struct {
	Coordinates structs.Spot
}

func (MouseMove) ActionType() string {
	return "Mouse Move"
}

func (gt MouseMove) PrintParams() string {
	str := gt.ActionType() + " to " + gt.Coordinates.SpotName //+ " at " + strconv.FormatInt(int64(gt.Coordinates.X), 10) + ", " + strconv.FormatInt(int64(gt.Coordinates.Y), 10)
	return str
}
