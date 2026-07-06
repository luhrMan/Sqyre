package editor

import (
	"testing"

	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"
)

func TestResolveCoordinateRefProgram(t *testing.T) {
	t.Helper()
	repositories.ResetAllForTesting()
	t.Cleanup(repositories.ResetAllForTesting)

	program := repositories.ProgramRepo().New()
	program.Name = "Demo"
	if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
		t.Fatalf("set program: %v", err)
	}
	pointRepo, err := program.PointRepo(config.MainMonitorSizeString)
	if err != nil {
		t.Fatalf("point repo: %v", err)
	}
	if err := pointRepo.Set("home", &models.Point{Name: "home", X: 1, Y: 2}); err != nil {
		t.Fatalf("set point: %v", err)
	}

	getKeys := func(p *models.Program) []string {
		repo, err := p.PointRepo(config.MainMonitorSizeString)
		if err != nil {
			return nil
		}
		return repo.GetAllKeys()
	}

	ref := actions.NewCoordinateRef("Demo", "home")
	gotProgram, gotKey, err := resolveCoordinateRefProgram(getKeys, ref)
	if err != nil {
		t.Fatalf("resolve: %v", err)
	}
	if gotProgram.Name != "Demo" || gotKey != "home" {
		t.Fatalf("got program %q key %q, want Demo home", gotProgram.Name, gotKey)
	}

	legacy := actions.CoordinateRef("home")
	gotProgram, gotKey, err = resolveCoordinateRefProgram(getKeys, legacy)
	if err != nil {
		t.Fatalf("resolve legacy: %v", err)
	}
	if gotProgram.Name != "Demo" || gotKey != "home" {
		t.Fatalf("legacy: got program %q key %q, want Demo home", gotProgram.Name, gotKey)
	}
}
