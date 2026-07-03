package actions

import (
	"fmt"
	"strings"

	"Sqyre/internal/config"
)

type Param struct {
	Label string
	Value any
}

func newParam(label string, value any) Param {
	return Param{Label: label, Value: value}
}

// FormatParamValue renders a param value for display (e.g. floats trimmed to 2 decimals).
func FormatParamValue(value any) string {
	switch v := value.(type) {
	case float32:
		return formatFloatUpTo2Decimals(float64(v))
	case float64:
		return formatFloatUpTo2Decimals(v)
	default:
		return fmt.Sprintf("%v", value)
	}
}

func formatFloatUpTo2Decimals(f float64) string {
	s := fmt.Sprintf("%.2f", f)
	s = strings.TrimRight(s, "0")
	s = strings.TrimRight(s, ".")
	return s
}

// FormatParamEntry returns "Label: value" for one param, or "" when the value is empty.
func FormatParamEntry(p Param) string {
	value := strings.TrimSpace(FormatParamValue(p.Value))
	if value == "" {
		return ""
	}
	return fmt.Sprintf("%s: %s", p.Label, value)
}

func stringifyParams(params []Param) string {
	parts := make([]string, 0, len(params))
	for _, p := range params {
		if entry := FormatParamEntry(p); entry != "" {
			parts = append(parts, entry)
		}
	}
	return strings.Join(parts, " "+config.DescriptionDelimiter+" ")
}

// ActionTypeFromParams reads the action type from a Type param, if present.
func ActionTypeFromParams(params []Param) string {
	for _, p := range params {
		if strings.EqualFold(p.Label, "Type") {
			return strings.TrimSpace(fmt.Sprintf("%v", p.Value))
		}
	}
	return ""
}
