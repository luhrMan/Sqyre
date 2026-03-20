package repositories

import (
	"cmp"
	"Sqyre/internal/models"
	"slices"
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

// GetAllSortedByName returns all programs sorted by Name ascending (stable UI order for accordions and lists).
func (r *ProgramRepository) GetAllSortedByName() []*models.Program {
	all := r.GetAll()
	out := make([]*models.Program, 0, len(all))
	for _, p := range all {
		out = append(out, p)
	}
	slices.SortFunc(out, func(a, b *models.Program) int {
		return cmp.Compare(a.Name, b.Name)
	})
	return out
}
