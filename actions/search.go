package actions

import "strconv"

type Search struct {
	Area   string
	Item   string
	Amount int
}

func (Search) ActionType() string {
	return "Search"
}

func (s Search) PrintParams() string {
	str := s.ActionType() + " " + s.Area + " for " + strconv.FormatInt(int64(s.Amount), 10) + " " + s.Item
	return str
}
