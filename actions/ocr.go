package actions

type OCR struct {
	Word string
}

func (OCR) ActionType() string {
	return "OCR"
}

func (s OCR) PrintParams() string {
	str := s.ActionType() +
		" " +
		s.Word
	return str
}
