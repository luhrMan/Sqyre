package serialize

import (
	"errors"
	"strings"
	"testing"
)

func TestYAMLErrorWithContent_IncludesLineSnippet(t *testing.T) {
	yml := "a: 1\nb: [broken\nprograms: {}\n"
	err := errors.New("yaml: line 2: did not find expected ',' or ']'")
	wrapped := YAMLErrorWithContent([]byte(yml), err)
	msg := wrapped.Error()
	if !strings.Contains(msg, "line 2") {
		t.Fatalf("expected original message, got: %s", msg)
	}
	if !strings.Contains(msg, "relevant lines") {
		t.Fatalf("expected snippet header: %s", msg)
	}
	if !strings.Contains(msg, "1| a: 1") || !strings.Contains(msg, "2| b: [broken") {
		t.Fatalf("expected numbered source, got: %s", msg)
	}
}

func TestYAMLErrorWithContent_NoLineNumbers(t *testing.T) {
	err := errors.New("something unrelated")
	if YAMLErrorWithContent([]byte("x: y\n"), err) != err {
		t.Fatal("expected same error when no line refs")
	}
}
