package actions

import "strconv"

type Click struct {
	Amount       int
	Button       string
	KeysHeldDown []string
}

func (Click) ActionType() string {
	return "Click"
}

func (s Click) PrintParams() string {
	str := s.Button +
		" " +
		s.ActionType() +
		" " +
		strconv.FormatInt(int64(s.Amount), 10) +
		" time(s) with KEYSHELDDOWN HERE" //+ s.KeysHeldDown[]
	return str
}
