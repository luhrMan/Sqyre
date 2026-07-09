package repositories

import (
	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"path/filepath"
	"strings"
	"testing"
)

func TestCollectionRepository_CRUD(t *testing.T) {
	setupTestConfig(t)
	resetProgramRepo()

	prog := models.NewProgram()
	prog.Name = "collectiongame"
	if err := ProgramRepo().Set("collectiongame", prog); err != nil {
		t.Fatal(err)
	}
	prog, err := ProgramRepo().Get("collectiongame")
	if err != nil {
		t.Fatal(err)
	}
	repo := NewCollectionRepository(prog)
	c := repo.New()
	c.Name = "inventory"
	c.SearchArea = "bag"
	c.Rows = 5
	c.Cols = 12
	if err := repo.Set(c.Name, c); err != nil {
		t.Fatal(err)
	}

	got, err := repo.Get("inventory")
	if err != nil {
		t.Fatal(err)
	}
	if got.SearchArea != "bag" || got.Rows != 5 || got.Cols != 12 {
		t.Fatalf("got %+v", got)
	}

	// Verify nested on the program aggregate after save
	prog2, err := ProgramRepo().Get("collectiongame")
	if err != nil {
		t.Fatal(err)
	}
	if prog2.Collections == nil || prog2.Collections["inventory"] == nil {
		t.Fatalf("collections not on program: %#v", prog2.Collections)
	}
	if prog2.Collections["inventory"].Cols != 12 {
		t.Fatalf("cols = %d", prog2.Collections["inventory"].Cols)
	}
}

func TestCollectionImagePath(t *testing.T) {
	dir := t.TempDir()
	config.SetSqyreDirOverride(dir)
	t.Cleanup(func() { config.SetSqyreDirOverride("") })
	path := config.CollectionImagePath("Prog", "Grid")
	want := filepath.Join(dir, "images", "Collections", "Prog", "Grid.png")
	if path != want {
		t.Fatalf("path = %q want %q", path, want)
	}
	if !strings.Contains(config.GetCollectionsPath(), "Collections") {
		t.Fatalf("GetCollectionsPath = %q", config.GetCollectionsPath())
	}
}
