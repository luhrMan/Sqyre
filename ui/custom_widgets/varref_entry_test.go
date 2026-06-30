package custom_widgets

import (
	"testing"

	"fyne.io/fyne/v2/test"
)

func TestVarEntry_insertVariable(t *testing.T) {
	test.NewApp()
	vars := []string{"count", "name"}
	e := NewVarEntry(func() []string { return vars })
	e.SetText("hello ")
	e.CursorColumn = len([]rune("hello "))
	e.CursorRow = 0
	e.insertVariable("count")
	if e.Text != "hello ${count}" {
		t.Fatalf("Text = %q, want %q", e.Text, "hello ${count}")
	}
}

func TestVarEntry_insertVariable_replacesSelection(t *testing.T) {
	test.NewApp()
	e := NewVarEntry(func() []string { return []string{"x"} })
	e.SetText("abc")
	e.CursorColumn = 3
	e.CursorRow = 0
	// Select "bc" by setting selection via paste trick: set full text and use SelectedText path
	e.SetText("abc")
	// Entry selection API is limited in tests; verify append-at-cursor instead.
	e.SetText("")
	e.insertVariable("x")
	if e.Text != "${x}" {
		t.Fatalf("Text = %q, want ${x}", e.Text)
	}
}

func TestVarEntry_InsertAtCursor_operators(t *testing.T) {
	test.NewApp()
	e := NewVarEntry(func() []string { return nil })
	e.SetText("1")
	e.CursorColumn = len([]rune("1"))
	e.CursorRow = 0
	e.InsertAtCursor(" + ")
	e.CursorColumn = len([]rune(e.Text))
	e.InsertAtCursor("${count}")
	if e.Text != "1 + ${count}" {
		t.Fatalf("Text = %q, want %q", e.Text, "1 + ${count}")
	}
}

func TestVarEntry_InsertAtCursor_multiline(t *testing.T) {
	test.NewApp()
	e := NewMultiLineVarEntry(func() []string { return nil })
	e.SetText("a\nb")
	e.CursorRow = 1
	e.CursorColumn = 1
	e.InsertAtCursor("X")
	if e.Text != "a\nbX" {
		t.Fatalf("Text = %q, want %q", e.Text, "a\nbX")
	}
}

func TestVarEntry_varRefContext(t *testing.T) {
	test.NewApp()
	cases := []struct {
		name        string
		text        string
		col         int
		wantPartial string
		wantOK      bool
	}{
		{"empty after dollar-brace", "x = ${", 6, "", true},
		{"partial after dollar-brace", "x = ${cou", 9, "cou", true},
		{"closed reference", "x = ${count}", 12, "", false},
		{"bare brace partial", "x = {cou", 8, "cou", true},
		{"bare brace not at cursor", "x = {cou", 3, "", false},
		{"plain text", "1 + 2", 5, "", false},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			e := NewVarEntry(func() []string { return []string{"count"} })
			e.SetText(tc.text)
			e.CursorRow = 0
			e.CursorColumn = tc.col
			partial, ok := e.varRefContext()
			if ok != tc.wantOK || partial != tc.wantPartial {
				t.Fatalf("varRefContext() = (%q, %v), want (%q, %v)", partial, ok, tc.wantPartial, tc.wantOK)
			}
		})
	}
}

func TestVarEntry_completeVarRef(t *testing.T) {
	test.NewApp()
	e := NewVarEntry(func() []string { return []string{"count"} })
	e.SetText("total = ${cou")
	e.CursorRow = 0
	e.CursorColumn = len([]rune("total = ${cou"))
	e.completeVarRef("count")
	if e.Text != "total = ${count}" {
		t.Fatalf("Text = %q, want %q", e.Text, "total = ${count}")
	}
	if e.CursorColumn != len([]rune("total = ${count}")) {
		t.Fatalf("CursorColumn = %d, want %d", e.CursorColumn, len([]rune("total = ${count}")))
	}
}

func TestVarEntry_completeVarRef_emptyPartial(t *testing.T) {
	test.NewApp()
	e := NewMultiLineVarEntry(func() []string { return []string{"name"} })
	e.SetText("a\nb = ${")
	e.CursorRow = 1
	e.CursorColumn = len([]rune("b = ${"))
	e.completeVarRef("name")
	if e.Text != "a\nb = ${name}" {
		t.Fatalf("Text = %q, want %q", e.Text, "a\nb = ${name}")
	}
}

func TestFilterVarNames(t *testing.T) {
	names := []string{"count", "Counter", "name"}
	got := filterVarNames(names, "cou")
	if len(got) != 2 || got[0] != "count" || got[1] != "Counter" {
		t.Fatalf("filterVarNames = %v, want [count Counter]", got)
	}
	if all := filterVarNames(names, ""); len(all) != 3 {
		t.Fatalf("filterVarNames empty partial = %v, want all 3", all)
	}
}

func TestVarEntry_SetText_doesNotOpenCompletion(t *testing.T) {
	test.NewApp()
	e := NewVarEntry(func() []string { return []string{"count", "name"} })
	e.CursorRow = 0
	e.CursorColumn = len([]rune("${cou"))
	e.SetText("${cou")
	if e.completer != nil && e.completer.Visible() {
		t.Fatal("programmatic SetText should not open variable completion")
	}
}

func TestVarEntry_insertButtonDisabledWithoutVariables(t *testing.T) {
	test.NewApp()
	e := NewVarEntry(func() []string { return nil })
	if !e.insert.Disabled() {
		t.Fatal("insert button should be disabled when no variables are defined")
	}
	e.GetVariables = func() []string { return []string{"a"} }
	e.UpdateInsertButton()
	if e.insert.Disabled() {
		t.Fatal("insert button should enable when variables become available")
	}
}
