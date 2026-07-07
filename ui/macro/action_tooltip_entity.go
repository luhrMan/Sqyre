package macro

import (
	"fmt"
	"strconv"
	"strings"
)

func formatCoordValue(v any) string {
	if v == nil {
		return ""
	}
	return fmt.Sprintf("%v", v)
}

func parseCoordValue(text string) any {
	text = strings.TrimSpace(text)
	if text == "" {
		return 0
	}
	if strings.HasPrefix(text, "${") {
		return text
	}
	if i, err := strconv.Atoi(text); err == nil {
		return i
	}
	if f, err := strconv.ParseFloat(text, 64); err == nil {
		return f
	}
	return text
}
