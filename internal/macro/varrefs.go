package macro

import "regexp"

// VarRefSegment is one plain-text or variable-reference segment in entry display text.
type VarRefSegment struct {
	Text  string
	IsRef bool
	Name  string
}

var varRefDollarPattern = regexp.MustCompile(`\$\{([^}]+)\}`)

// TextContainsVarRef reports whether text contains ${name} or {name} references.
func TextContainsVarRef(text string) bool {
	if text == "" {
		return false
	}
	if varRefDollarPattern.MatchString(text) {
		return true
	}
	return findBraceRefs(text) != nil
}

// ParseVarRefSegments splits text into plain and reference segments in document order.
// References use ${name} or {name} (not the inner brace of ${...}).
func ParseVarRefSegments(text string) []VarRefSegment {
	if text == "" {
		return nil
	}
	var matches []varRefMatch
	for _, loc := range varRefDollarPattern.FindAllStringSubmatchIndex(text, -1) {
		matches = append(matches, varRefMatch{loc[0], loc[1], text[loc[2]:loc[3]]})
	}
	for _, m := range findBraceRefs(text) {
		matches = append(matches, m)
	}
	if len(matches) == 0 {
		return []VarRefSegment{{Text: text}}
	}
	sortMatches(matches)
	filtered := dropOverlapping(matches)
	segs := make([]VarRefSegment, 0, len(filtered)*2+1)
	last := 0
	for _, m := range filtered {
		if m.start > last {
			segs = append(segs, VarRefSegment{Text: text[last:m.start]})
		}
		segs = append(segs, VarRefSegment{
			Text:  text[m.start:m.end],
			IsRef: true,
			Name:  m.name,
		})
		last = m.end
	}
	if last < len(text) {
		segs = append(segs, VarRefSegment{Text: text[last:]})
	}
	return segs
}

func findBraceRefs(text string) []varRefMatch {
	var out []varRefMatch
	for i := 0; i < len(text); i++ {
		if text[i] != '{' {
			continue
		}
		if i > 0 && text[i-1] == '$' {
			continue
		}
		end := -1
		for j := i + 1; j < len(text); j++ {
			if text[j] == '}' {
				end = j
				break
			}
		}
		if end < 0 {
			continue
		}
		name := text[i+1 : end]
		if name == "" {
			continue
		}
		out = append(out, varRefMatch{i, end + 1, name})
		i = end
	}
	return out
}

type varRefMatch struct {
	start, end int
	name       string
}

func sortMatches(matches []varRefMatch) {
	for i := range matches {
		for j := i + 1; j < len(matches); j++ {
			if matches[j].start < matches[i].start {
				matches[i], matches[j] = matches[j], matches[i]
			}
		}
	}
}

func dropOverlapping(matches []varRefMatch) []varRefMatch {
	filtered := matches[:0]
	for _, m := range matches {
		overlap := false
		for _, prev := range filtered {
			if m.start >= prev.start && m.end <= prev.end {
				overlap = true
				break
			}
		}
		if !overlap {
			filtered = append(filtered, m)
		}
	}
	return filtered
}
