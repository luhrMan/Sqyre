package serialize

import (
	"fmt"
	"regexp"
	"sort"
	"strconv"
	"strings"
)

var yamlErrLineRE = regexp.MustCompile(`line (\d+)`)

// YAMLErrorWithContent returns err with numbered excerpts from content for each
// "line N" referenced in typical gopkg.in/yaml.v3 errors.
func YAMLErrorWithContent(content []byte, err error) error {
	if err == nil {
		return nil
	}
	msg := err.Error()
	lines := yamlErrorLineNumbers(msg)
	if len(lines) == 0 || len(content) == 0 {
		return err
	}
	snip := formatYAMLLineSnippets(content, lines, 2)
	if snip == "" {
		return err
	}
	return fmt.Errorf("%s\n--- relevant lines ---\n%s", msg, strings.TrimSuffix(snip, "\n"))
}

func yamlErrorLineNumbers(msg string) []int {
	found := yamlErrLineRE.FindAllStringSubmatch(msg, -1)
	uniq := make(map[int]struct{})
	for _, m := range found {
		if len(m) < 2 {
			continue
		}
		n, err := strconv.Atoi(m[1])
		if err != nil || n < 1 {
			continue
		}
		uniq[n] = struct{}{}
	}
	out := make([]int, 0, len(uniq))
	for n := range uniq {
		out = append(out, n)
	}
	sort.Ints(out)
	return out
}

func formatYAMLLineSnippets(content []byte, centerLines []int, context int) string {
	src := strings.ReplaceAll(string(content), "\r\n", "\n")
	lines := strings.Split(src, "\n")
	if len(lines) == 0 {
		return ""
	}
	var b strings.Builder
	printed := make(map[int]bool)
	for _, center := range centerLines {
		if center < 1 || center > len(lines) {
			continue
		}
		lo := center - context
		if lo < 1 {
			lo = 1
		}
		hi := center + context
		if hi > len(lines) {
			hi = len(lines)
		}
		for i := lo; i <= hi; i++ {
			if printed[i] {
				continue
			}
			printed[i] = true
			fmt.Fprintf(&b, "%5d| %s\n", i, lines[i-1])
		}
	}
	return b.String()
}
