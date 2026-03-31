package appdata

import "Sqyre/internal/models"

// ensureDefaultProgram inserts the built-in "default" program if the store does not already have it.
func ensureDefaultProgram(s ProgramStore) {
	if s == nil {
		return
	}
	if _, err := s.Get(models.DefaultProgramName); err == nil {
		return
	}
	_ = s.Set(models.DefaultProgramName, models.NewDefaultProgram())
}
