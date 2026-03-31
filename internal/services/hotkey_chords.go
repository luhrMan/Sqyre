package services

import "strings"

// ParseMacroHotkey splits a display string like "ctrl + shift + a" into key tokens.
func ParseMacroHotkey(hk string) []string {
	if hk == "" {
		return []string{}
	}
	parts := strings.Split(hk, "+")
	for i, part := range parts {
		parts[i] = strings.TrimSpace(part)
	}
	return parts
}

// ReverseParseMacroHotkey joins hotkey tokens for display.
func ReverseParseMacroHotkey(hk []string) string {
	var str string
	for i, k := range hk {
		if i == 0 {
			str = k
			continue
		}
		str = str + " + " + k
	}
	return str
}
