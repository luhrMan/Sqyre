package detector

import (
	"strings"
)

// ParsePrompt splits a user query into one or more class prompts for the detector.
// Examples:
//   - "All Healing potions" → ["healing potion"]
//   - "Metal Armor, Boots" → ["metal armor", "boots"]
//   - "healing potion and mana potion" → ["healing potion", "mana potion"]
func ParsePrompt(query string) []string {
	q := strings.TrimSpace(query)
	if q == "" {
		return nil
	}
	q = stripLeadingQuantifier(q)

	var parts []string
	if strings.Contains(q, ",") {
		for _, p := range strings.Split(q, ",") {
			if s := normalizePromptPart(p); s != "" {
				parts = append(parts, s)
			}
		}
	} else if strings.Contains(strings.ToLower(q), " and ") {
		for _, p := range strings.Split(q, " and ") {
			if s := normalizePromptPart(p); s != "" {
				parts = append(parts, s)
			}
		}
	} else {
		if s := normalizePromptPart(q); s != "" {
			parts = append(parts, s)
		}
	}
	return dedupePrompts(parts)
}

func stripLeadingQuantifier(s string) string {
	lower := strings.ToLower(s)
	for _, prefix := range []string{"all ", "any ", "every "} {
		if strings.HasPrefix(lower, prefix) {
			return strings.TrimSpace(s[len(prefix):])
		}
	}
	return s
}

func normalizePromptPart(s string) string {
	return strings.ToLower(strings.TrimSpace(s))
}

func dedupePrompts(parts []string) []string {
	seen := make(map[string]bool, len(parts))
	out := make([]string, 0, len(parts))
	for _, p := range parts {
		if p == "" || seen[p] {
			continue
		}
		seen[p] = true
		out = append(out, p)
	}
	return out
}
