package repositories

import (
	"Squire/internal/models"
	"sync"
)

// MacroRepository provides data access operations for Macro models
type MacroRepository struct {
	*BaseRepository[models.Macro]
}

var (
	macroRepo *MacroRepository
	macroOnce sync.Once
)

// MacroRepo returns the singleton MacroRepository instance.
// On first call, it initializes the repository and loads all macros from disk.
func MacroRepo() *MacroRepository {
	macroOnce.Do(func() {
		macroRepo = &MacroRepository{
			BaseRepository: NewBaseRepository(
				"macros",
				decodeMacro,
				func() *models.Macro {
					return models.NewMacro("", 0, []string{})
				},
			),
		}
		// Load all macros from disk on initialization
		if err := macroRepo.Reload(); err != nil {
			// Log error but don't panic - repository is still usable
			// Error is already logged in Reload()
		}
	})
	return macroRepo
}
