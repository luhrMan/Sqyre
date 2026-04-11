package actions

import (
	"fmt"
	"strings"

	"Sqyre/internal/config"
)

type ActionParam struct {
	Label string
	Value any
}

func newParam(label string, value any) ActionParam {
	return ActionParam{Label: label, Value: value}
}

func stringifyParams(params []ActionParam) string {
	parts := make([]string, 0, len(params))
	for _, p := range params {
		value := strings.TrimSpace(fmt.Sprintf("%v", p.Value))
		if value == "" {
			continue
		}
		parts = append(parts, fmt.Sprintf("%s: %s", p.Label, value))
	}
	return strings.Join(parts, " "+config.DescriptionDelimiter+" ")
}

func FormatSearchAreaLabel(area SearchArea) string {
	name := strings.TrimSpace(area.Name)
	coordinates := fmt.Sprintf(
		"TopY: %v, LeftX: %v, BottomY: %v, RightX: %v",
		area.TopY,
		area.LeftX,
		area.BottomY,
		area.RightX,
	)

	if name == "" {
		return coordinates
	}

	return fmt.Sprintf("%s (%s)", name, coordinates)
}
