package vision

import (
	"os"
	"path/filepath"
	"testing"

	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/models/serialize"
)

func initTestConfig(t *testing.T) {
	t.Helper()
	os.Setenv("SQYRE_TEST_MODE", "1")
	dir := t.TempDir()
	configPath := filepath.Join(dir, "db.yaml")
	if err := os.WriteFile(configPath, []byte("macros: {}\nprograms: {}\n"), 0644); err != nil {
		t.Fatalf("write temp config: %v", err)
	}
	repositories.ResetAllForTesting()
	yamlConfig := serialize.GetYAMLConfig()
	yamlConfig.SetConfigFile(configPath)
	if err := yamlConfig.ReadConfig(); err != nil {
		t.Fatalf("read temp config: %v", err)
	}
	viperCfg := serialize.GetViper()
	viperCfg.SetConfigFile(configPath)
	viperCfg.SetConfigType("yaml")
	if err := viperCfg.ReadInConfig(); err != nil {
		t.Fatalf("read viper config: %v", err)
	}
	_ = config.GetDbPath()
}

func TestMacroUsesOCR_Direct(t *testing.T) {
	m := models.NewMacro("main", 0, nil)
	m.Root.SubActions = []actions.ActionInterface{
		actions.NewOcr("read", "Submit", "area"),
	}
	if !MacroUsesOCR(m) {
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
	if !MacroUsesOCR(parent) {
		t.Fatal("expected parent macro calling OCR child to use OCR")
	}
}

func TestMacroUsesOCR_NoOCR(t *testing.T) {
	m := models.NewMacro("main", 0, nil)
	m.Root.SubActions = []actions.ActionInterface{
		actions.NewWait(100),
	}
	if MacroUsesOCR(m) {
		t.Fatal("expected macro without OCR action to not use OCR")
	}
}
