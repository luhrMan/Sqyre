package services

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"testing"
)

func TestMacroUsesOCR_Direct(t *testing.T) {
	m := models.NewMacro("main", 0, nil)
	m.Root.SubActions = []actions.ActionInterface{
		actions.NewOcr("read", "Submit", "area"),
	}
	if !macroUsesOCR(m) {
		t.Fatal("expected macro with OCR action to use OCR")
	}
}

func TestMacroUsesOCR_NestedRunMacro(t *testing.T) {
	initTestConfig(t)
	child := models.NewMacro("child", 0, nil)
	child.Root.SubActions = []actions.ActionInterface{
		actions.NewOcr("read", "OK", "area"),
	}
	if err := repositories.MacroRepo().Set("child", child); err != nil {
		t.Fatalf("set child macro: %v", err)
	}

	parent := models.NewMacro("parent", 0, nil)
	parent.Root.SubActions = []actions.ActionInterface{
		actions.NewRunMacro("child"),
	}
	if !macroUsesOCR(parent) {
		t.Fatal("expected parent macro calling OCR child to use OCR")
	}
}

func TestMacroUsesOCR_NoOCR(t *testing.T) {
	m := models.NewMacro("main", 0, nil)
	m.Root.SubActions = []actions.ActionInterface{
		actions.NewWait(100),
	}
	if macroUsesOCR(m) {
		t.Fatal("expected macro without OCR action to not use OCR")
	}
}
