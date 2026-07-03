package actions

import (
	"fmt"
	"strings"

	"Sqyre/internal/config"
)

type Param struct {
	Label string
	Value any
	// Extra marks detail params shown only in a hover tooltip, not inline.
	Extra bool
}

func newParam(label string, value any) Param {
	return Param{Label: label, Value: value}
}

func newExtraParam(label string, value any) Param {
	return Param{Label: label, Value: value, Extra: true}
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

// FormatParamMinimal returns the value alone for compact inline pills.
func FormatParamMinimal(p Param) string {
	return strings.TrimSpace(FormatParamValue(p.Value))
}

// FormatParamEntry returns "Label: value" for one param, or "" when the value is empty.
func FormatParamEntry(p Param) string {
	value := FormatParamMinimal(p)
	if value == "" {
		return ""
	}
	return fmt.Sprintf("%s: %s", p.Label, value)
}

// DisplayParams splits params into inline summary pills and tooltip-only extras.
func DisplayParams(params []Param) (summary, extra []Param) {
	for _, p := range params {
		if strings.EqualFold(p.Label, "Type") {
			continue
		}
		if FormatParamMinimal(p) == "" {
			continue
		}
		if p.Extra {
			extra = append(extra, p)
		} else {
			summary = append(summary, p)
		}
	}
	return summary, extra
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
