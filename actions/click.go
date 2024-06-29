package actions

import "strconv"

type Click struct {
	Amount       int
	Spot         [2]int
	KeysHeldDown []string
}

func (Click) ActionType() string {
	return "Click"
}

func (s Click) PrintParams() string {
	str := s.ActionType() +
		" at X:" + strconv.FormatInt(int64(s.Spot[0]), 10) +
		" Y:" + strconv.FormatInt(int64(s.Spot[1]), 10) +
		" " +
		strconv.FormatInt(int64(s.Amount), 10) +
		" time(s) with " //+ s.KeysHeldDown[]
	return str
}
