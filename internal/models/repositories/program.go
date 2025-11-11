package repositories

import (
	"Squire/internal/models"
	"sync"
)

// ProgramRepository manages Program model persistence using the generic BaseRepository
type ProgramRepository struct {
	*BaseRepository[models.Program]
}

var (
	programRepo *ProgramRepository
	programOnce sync.Once
)

// ProgramRepo returns the singleton ProgramRepository instance.
// It initializes the repository on first call and loads existing programs from disk.
func ProgramRepo() *ProgramRepository {
	programOnce.Do(func() {
		programRepo = &ProgramRepository{
			BaseRepository: NewBaseRepository(
				"programs",
				decodeProgram,
				models.NewProgram,
			),
		}
		// Load existing programs from disk
		if err := programRepo.Reload(); err != nil {
			// Log error but don't fail - repository will be empty
			// This allows the application to start even if config is missing/corrupt
		}
	})
	return programRepo
}
