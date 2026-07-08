package testsupport

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
	"Sqyre/internal/models/serialize"
	"os"
	"path/filepath"
	"testing"
)

// InitDocUIEnv loads a minimal in-memory config and binds macro UI for screenshot tests.
func InitDocUIEnv(t *testing.T) {
	t.Helper()
	os.Setenv("SQYRE_TEST_MODE", "1")
	os.Setenv("SQYRE_UI_TEST", "1")
	os.Setenv("SQYRE_NO_HOOK", "1")

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

	macroRepo := repositories.MacroRepo()
	programRepo := repositories.ProgramRepo()

	demoMacro := models.NewMacro("Demo Macro", 0, nil)
	buildDemoMacroActions(demoMacro)
	if err := macroRepo.Set("Demo Macro", demoMacro); err != nil {
		t.Fatalf("set demo macro: %v", err)
	}

	demoProgram := programRepo.New()
	demoProgram.Name = "Demo Program"
	demoProgram.Items["demo-item"] = &models.Item{
		Name:     "demo-item",
		GridSize: [2]int{1, 1},
		StackMax: 1,
	}
	if coords := demoProgram.Coordinates[config.MainMonitorSizeString]; coords != nil {
		coords.Points["center"] = &models.Point{Name: "center", X: 500, Y: 300}
		coords.SearchAreas["Main area"] = &models.SearchArea{
			Name:    "Main area",
			LeftX:   100,
			TopY:    100,
			RightX:  900,
			BottomY: 600,
		}
	}
	if err := programRepo.Set("Demo Program", demoProgram); err != nil {
		t.Fatalf("set demo program: %v", err)
	}
}

// buildDemoMacroActions fills the demo macro with a small but representative
// automation (window focus, mouse, detection with a nested step, typing, a
// timed pause, and a loop) so docs screenshots show a realistic action tree
// instead of a single lonely step.
func buildDemoMacroActions(m *models.Macro) {
	const (
		programName = "Demo Program"
		pointName   = "center"
		itemName    = "demo-item"
	)
	itemTarget := programName + config.ProgramDelimiter + itemName

	root := m.Root
	root.AddSubAction(actions.NewFocusWindow("", "Notepad"))
	root.AddSubAction(actions.NewMove(actions.NewCoordinateRef(programName, pointName), true))

	imageSearch := actions.NewImageSearch(
		"Find button",
		[]actions.ActionInterface{actions.NewClick(actions.ClickButtonLeft, true)},
		[]string{itemTarget},
		actions.NewCoordinateRef(programName, "Main area"),
		1, 1, 0.95, 5,
	)
	root.AddSubAction(imageSearch)

	root.AddSubAction(actions.NewType("Hello, Sqyre!", 40))
	root.AddSubAction(actions.NewWait(500))
	root.AddSubAction(actions.NewLoop(3, "", []actions.ActionInterface{actions.NewKey("enter", true)}))
}
