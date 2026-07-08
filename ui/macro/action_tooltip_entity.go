package macro

import "fmt"

func formatAnyValue(v any) string {
	if v == nil {
		return ""
	}
	return fmt.Sprintf("%v", v)
}
