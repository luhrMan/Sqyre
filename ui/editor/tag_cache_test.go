package editor

import (
	"testing"

	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
)

func TestProgramTagsCacheInvalidation(t *testing.T) {
	ResetProgramTagsCacheForTesting()
	t.Cleanup(ResetProgramTagsCacheForTesting)

	repositories.ResetAllForTesting()
	t.Cleanup(repositories.ResetAllForTesting)

	program := repositories.ProgramRepo().New()
	program.Name = "Demo"
	if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
		t.Fatalf("set program: %v", err)
	}
	repo := ProgramItemRepo(program)
	if err := repo.Set("icon", &models.Item{Name: "icon", Tags: []string{"alpha"}}); err != nil {
		t.Fatalf("set item: %v", err)
	}

	first := getProgramTags("Demo")
	if len(first) != 1 || first[0] != "alpha" {
		t.Fatalf("first tags = %v, want [alpha]", first)
	}
	if err := repo.Set("icon", &models.Item{Name: "icon", Tags: []string{"alpha", "beta"}}); err != nil {
		t.Fatalf("update item: %v", err)
	}
	second := getProgramTags("Demo")
	if len(second) != 1 {
		t.Fatalf("cached tags = %v, want stale [alpha] before invalidation", second)
	}
	InvalidateProgramTagsCache("Demo")
	third := getProgramTags("Demo")
	if len(third) != 2 {
		t.Fatalf("after invalidation tags = %v, want 2 tags", third)
	}
}
