package custom_widgets

import (
	"Sqyre/internal/services"
	"testing"

	"fyne.io/fyne/v2/test"
)

func TestVarEntryField_expressionErrorBlocksSubmit(t *testing.T) {
	test.NewApp()
	field := NewVarEntryField(nil, func(text string) services.EntryValidation {
		if text == "bad" {
			return services.EntryValidation{Error: "invalid expression"}
		}
		return services.EntryValidation{}
	})
	field.Entry.SetText("bad")
	field.Revalidate()

	if field.Valid() {
		t.Fatal("expected invalid")
	}
	if field.ValidationError() != "invalid expression" {
		t.Fatalf("got error %q", field.ValidationError())
	}

	field.Entry.SetText("ok")
	field.Revalidate()
	if !field.Valid() {
		t.Fatal("expected valid")
	}
}

func TestVarEntryField_unknownVariableWarnsOnly(t *testing.T) {
	test.NewApp()
	field := NewVarEntryField(nil, func(text string) services.EntryValidation {
		return services.EntryValidation{Warning: "unknown variable \"missing\""}
	})
	field.Entry.SetText("${missing}")
	field.Revalidate()

	if !field.Valid() {
		t.Fatal("unknown variable should not block submit")
	}
	if field.ValidationWarning() == "" {
		t.Fatal("expected warning message")
	}
}

func TestVarEntryField_OnChanged(t *testing.T) {
	test.NewApp()
	field := NewVarEntryField(nil, nil)
	called := false
	field.OnChanged = func(string) { called = true }
	field.Entry.InsertAtCursor("x")
	if !called {
		t.Fatal("OnChanged not called")
	}
}
