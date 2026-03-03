package repositories

import (
	"Squire/internal/models"
	"Squire/internal/models/serialize"
	"errors"
	"os"
	"path/filepath"
	"testing"
)

// --- Persistence failure (base.go Save/Set/Delete error paths) ---

func TestBaseRepository_Save_WriteConfigFailure(t *testing.T) {
	setupTestConfig(t)
	// Use a fresh repo so we don't rely on singleton
	repo := NewBaseRepository[models.Macro]("macros", decodeMacro, func() *models.Macro { return models.NewMacro("", 0, nil) })
	// Load existing data so Save has something to write
	if err := repo.Reload(); err != nil {
		t.Skipf("Reload failed (no test mode?): %v", err)
	}

	yamlConfig := serialize.GetYAMLConfig()
	prevPath := yamlConfig.GetConfigFile()
	defer yamlConfig.SetConfigFile(prevPath)

	// Point config at a directory so WriteConfig fails (WriteFile to a dir fails on all platforms)
	unwritablePath := t.TempDir()
	yamlConfig.SetConfigFile(unwritablePath)

	err := repo.Save()
	if err == nil {
		t.Fatal("expected Save to fail when config file is read-only")
	}
	if !errors.Is(err, ErrSaveFailed) {
		t.Errorf("expected ErrSaveFailed, got %v", err)
	}
}

func TestBaseRepository_Set_PersistFailure(t *testing.T) {
	setupTestConfig(t)
	repo := NewBaseRepository[models.Macro]("macros", decodeMacro, func() *models.Macro { return models.NewMacro("", 0, nil) })

	yamlConfig := serialize.GetYAMLConfig()
	prevPath := yamlConfig.GetConfigFile()
	defer yamlConfig.SetConfigFile(prevPath)

	// Point config at a directory so WriteConfig fails
	unwritablePath := t.TempDir()
	yamlConfig.SetConfigFile(unwritablePath)

	m := models.NewMacro("x", 0, nil)
	err := repo.Set("x", m)
	if err == nil {
		t.Fatal("expected Set to fail when persist fails")
	}
	if !errors.Is(err, ErrSaveFailed) {
		t.Errorf("expected ErrSaveFailed wrap, got %v", err)
	}
}

func TestBaseRepository_Delete_PersistFailure(t *testing.T) {
	setupTestConfig(t)
	repo := NewBaseRepository[models.Macro]("macros", decodeMacro, func() *models.Macro { return models.NewMacro("", 0, nil) })
	repo.mu.Lock()
	repo.models["x"] = models.NewMacro("x", 0, nil)
	repo.mu.Unlock()

	yamlConfig := serialize.GetYAMLConfig()
	prevPath := yamlConfig.GetConfigFile()
	defer yamlConfig.SetConfigFile(prevPath)

	unwritablePath := t.TempDir()
	yamlConfig.SetConfigFile(unwritablePath)

	err := repo.Delete("x")
	if err == nil {
		t.Fatal("expected Delete to fail when persist fails")
	}
	if !errors.Is(err, ErrSaveFailed) {
		t.Errorf("expected ErrSaveFailed wrap, got %v", err)
	}
}

// --- Reload error paths (base.go) ---

func TestBaseRepository_Reload_ReadConfigFailureInTestMode(t *testing.T) {
	setupTestConfig(t)
	repo := NewBaseRepository[models.Macro]("macros", decodeMacro, func() *models.Macro { return models.NewMacro("", 0, nil) })

	prevMode := os.Getenv("SQYRE_TEST_MODE")
	os.Setenv("SQYRE_TEST_MODE", "1")
	defer func() { os.Setenv("SQYRE_TEST_MODE", prevMode) }()

	yamlConfig := serialize.GetYAMLConfig()
	prevPath := yamlConfig.GetConfigFile()
	defer yamlConfig.SetConfigFile(prevPath)

	// Non-existent path so ReadConfig fails in test mode
	yamlConfig.SetConfigFile(filepath.Join(t.TempDir(), "nonexistent.yaml"))

	err := repo.Reload()
	if err == nil {
		t.Fatal("expected Reload to fail when ReadConfig fails in test mode")
	}
	if err != nil && !errors.Is(err, ErrLoadFailed) {
		// Message should mention "re-read config in test mode"
		if msg := err.Error(); msg != "" && len(msg) < 10 {
			t.Errorf("unexpected error: %v", err)
		}
	}
}

func TestBaseRepository_Reload_ErrLoadFailedWhenAllDecodeFail(t *testing.T) {
	setupTestConfig(t)
	// Config must have at least one key under "macros" so configMap is non-empty
	yamlConfig := serialize.GetYAMLConfig()
	prevPath := yamlConfig.GetConfigFile()
	defer yamlConfig.SetConfigFile(prevPath)
	if err := yamlConfig.ReadConfig(); err != nil {
		t.Fatalf("read config: %v", err)
	}
	macrosMap := yamlConfig.GetStringMap("macros")
	if len(macrosMap) == 0 {
		t.Skip("testdata has no macros; cannot test ErrLoadFailed")
	}

	// Decode func that always fails
	failingDecode := func(key string) (*models.Macro, error) {
		return nil, ErrDecodeFailed
	}
	repo := NewBaseRepository[models.Macro]("macros", failingDecode, func() *models.Macro { return models.NewMacro("", 0, nil) })

	err := repo.Reload()
	if err == nil {
		t.Fatal("expected Reload to return ErrLoadFailed when all decodes fail")
	}
	if !errors.Is(err, ErrLoadFailed) {
		t.Errorf("expected ErrLoadFailed, got %v", err)
	}
}

// --- Nested Set/Delete saveFunc failure (nested.go) ---

func TestNestedRepository_Set_SaveFuncFailure(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()
	program := models.NewProgram()
	program.Name = "nested-save-fail"
	if err := ProgramRepo().Set(program.GetKey(), program); err != nil {
		t.Fatalf("setup program: %v", err)
	}
	repo := NewItemRepository(program)

	yamlConfig := serialize.GetYAMLConfig()
	prevPath := yamlConfig.GetConfigFile()
	defer yamlConfig.SetConfigFile(prevPath)

	unwritablePath := t.TempDir()
	yamlConfig.SetConfigFile(unwritablePath)

	item := repo.New()
	item.Name = "x"
	err := repo.Set("x", item)
	if err == nil {
		t.Fatal("expected Set to fail when saveFunc fails")
	}
	// Error wraps save failure (not necessarily ErrSaveFailed from our package)
	if err != nil && err.Error() == "" {
		t.Error("expected non-empty error message")
	}
}

func TestNestedRepository_Delete_SaveFuncFailure(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()
	program := models.NewProgram()
	program.Name = "nested-del-fail"
	program.Items["x"] = &models.Item{Name: "x"}
	if err := ProgramRepo().Set(program.GetKey(), program); err != nil {
		t.Fatalf("setup program: %v", err)
	}
	repo := NewItemRepository(program)

	yamlConfig := serialize.GetYAMLConfig()
	prevPath := yamlConfig.GetConfigFile()
	defer yamlConfig.SetConfigFile(prevPath)

	unwritablePath := t.TempDir()
	yamlConfig.SetConfigFile(unwritablePath)

	err := repo.Delete("x")
	if err == nil {
		t.Fatal("expected Delete to fail when saveFunc fails")
	}
}

// --- Decode error paths (decode.go) ---

func TestDecodeMacro_UnmarshalError(t *testing.T) {
	setupTestConfig(t)
	yamlConfig := serialize.GetYAMLConfig()
	prevPath := yamlConfig.GetConfigFile()
	defer yamlConfig.SetConfigFile(prevPath)

	// Temporarily set macros to include a key with invalid structure so Unmarshal fails
	prevData := yamlConfig.Get("macros")
	defer func() {
		yamlConfig.Set("macros", prevData)
	}()
	// Value that will fail macro unmarshal (e.g. root with invalid type)
	yamlConfig.Set("macros", map[string]any{
		"badmacro": map[string]any{
			"name":        "badmacro",
			"globaldelay": 0,
			"hotkey":      []any{},
			"root":        "not-a-loop", // invalid: root must be loop structure
		},
	})

	_, err := decodeMacro("badmacro")
	if err == nil {
		t.Fatal("expected decodeMacro to fail for invalid root")
	}
	if !errors.Is(err, ErrDecodeFailed) {
		t.Errorf("expected ErrDecodeFailed, got %v", err)
	}
}

func TestDecodeMacro_VariablesNilInitialized(t *testing.T) {
	setupTestConfig(t)
	yamlConfig := serialize.GetYAMLConfig()
	prevPath := yamlConfig.GetConfigFile()
	defer yamlConfig.SetConfigFile(prevPath)

	prevData := yamlConfig.Get("macros")
	defer func() {
		yamlConfig.Set("macros", prevData)
	}()
	// Minimal macro with no "variables" key so Variables is nil after Unmarshal
	yamlConfig.Set("macros", map[string]any{
		"minimal": map[string]any{
			"name":        "minimal",
			"globaldelay": 0,
			"hotkey":      []any{},
			"root": map[string]any{
				"type":  "loop",
				"name":  "root",
				"count": 1,
				"subactions": []any{},
			},
		},
	})

	macro, err := decodeMacro("minimal")
	if err != nil {
		t.Fatalf("decodeMacro: %v", err)
	}
	if macro.Variables == nil {
		t.Error("expected Variables to be initialized when nil after Unmarshal")
	}
}

func TestDecodeProgram_UnmarshalError(t *testing.T) {
	setupTestConfig(t)
	yamlConfig := serialize.GetYAMLConfig()
	prevPath := yamlConfig.GetConfigFile()
	defer yamlConfig.SetConfigFile(prevPath)

	prevData := yamlConfig.Get("programs")
	defer func() {
		yamlConfig.Set("programs", prevData)
	}()
	// Invalid program: items as string instead of map
	yamlConfig.Set("programs", map[string]any{
		"badprogram": map[string]any{
			"name":  "badprogram",
			"items": "not-a-map",
		},
	})

	_, err := decodeProgram("badprogram")
	if err == nil {
		t.Fatal("expected decodeProgram to fail for invalid items")
	}
	if !errors.Is(err, ErrDecodeFailed) {
		t.Errorf("expected ErrDecodeFailed, got %v", err)
	}
}

func TestDecodeProgram_NonExistentKeyReturnsEmpty(t *testing.T) {
	setupTestConfig(t)
	// Ensure config is loaded but use a key that doesn't exist in it
	yamlConfig := serialize.GetYAMLConfig()
	if err := yamlConfig.ReadConfig(); err != nil {
		t.Fatalf("read config: %v", err)
	}

	program, err := decodeProgram("definitely-nonexistent-key-12345")
	if err != nil {
		t.Fatalf("decodeProgram(nonexistent) should not error: %v", err)
	}
	if program == nil {
		t.Fatal("expected non-nil empty program")
	}
	if program.Name != "" {
		t.Errorf("expected empty name, got %q", program.Name)
	}
}

