package actions

import (
	"Dark-And-Darker/structs"
	"strconv"
)

type Search struct {
	SearchBox structs.SearchBox
	Item      structs.Item
	Amount    int
}

func (Search) ActionType() string {
	return "Search"
}

func (s Search) PrintParams() string {
	str := s.ActionType() + " " + s.SearchBox.AreaName + " for " + strconv.FormatInt(int64(s.Amount), 10) + " " + s.Item.Name
	return str
}
