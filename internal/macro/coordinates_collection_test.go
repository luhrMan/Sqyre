package macro

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

func initCollectionTestConfig(t *testing.T) {
	t.Helper()
	os.Setenv("SQYRE_TEST_MODE", "1")
	dir := t.TempDir()
	config.SetSqyreDirOverride(dir)
	t.Cleanup(func() { config.SetSqyreDirOverride("") })
	configPath := filepath.Join(dir, "db.yaml")
	if err := os.WriteFile(configPath, []byte("macros: {}\nprograms: {}\n"), 0644); err != nil {
		t.Fatalf("write temp config: %v", err)
	}
	repositories.ResetAllForTesting()
	yamlConfig := serialize.GetYAMLConfig()
	yamlConfig.SetConfigFile(configPath)
	yamlConfig.SetDebounceWrites(false)
	if err := yamlConfig.ReadConfig(); err != nil {
		t.Fatalf("read temp config: %v", err)
	}
}

func TestResolveCollectionRef_rectAndCenter(t *testing.T) {
	initCollectionTestConfig(t)

	res := config.MainMonitorSizeString
	prog := models.NewProgram()
	prog.Name = "Demo"
	saRepo, err := prog.SearchAreaRepo(res)
	if err != nil {
		t.Fatal(err)
	}
	sa := saRepo.New()
	sa.Name = "inv"
	sa.LeftX, sa.TopY, sa.RightX, sa.BottomY = 0, 0, 100, 100
	if err := saRepo.Set(sa.Name, sa); err != nil {
		t.Fatal(err)
	}
	colRepo, err := prog.CollectionRepo()
	if err != nil {
		t.Fatal(err)
	}
	col := colRepo.New()
	col.Name = "grid"
	col.SearchArea = "inv"
	col.Rows, col.Cols = 2, 2
	if err := colRepo.Set(col.Name, col); err != nil {
		t.Fatal(err)
	}
	if err := repositories.ProgramRepo().Set(prog.Name, prog); err != nil {
		t.Fatal(err)
	}

	ref := actions.NewCollectionRef("Demo", "grid", 1, 1, 1, 1)
	lx, ty, rx, by, err := ResolveSearchAreaCoordsFromRef(ref, nil, res)
	if err != nil {
		t.Fatal(err)
	}
	if lx != 0 || ty != 0 || rx != 50 || by != 50 {
		t.Fatalf("rect = %d,%d-%d,%d want 0,0-50,50", lx, ty, rx, by)
	}

	x, y, err := ResolvePointCoordsFromRef(ref, nil, res)
	if err != nil {
		t.Fatal(err)
	}
	if x != 25 || y != 25 {
		t.Fatalf("center = %d,%d want 25,25", x, y)
	}
}
