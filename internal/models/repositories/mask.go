package repositories

import (
	"Squire/internal/models"
)

func init() {
	models.MaskRepositoryFactory = func(p *models.Program) models.MaskRepositoryInterface {
		return NewMaskRepository(p)
	}
}

// MaskRepository manages Mask persistence within a Program context.
// It embeds NestedRepository to leverage generic CRUD operations while maintaining
// program context for specialized behavior.
type MaskRepository struct {
	*NestedRepository[models.Mask]
	program *models.Program
}

// NewMaskRepository creates a MaskRepository for a Program.
// The repository operates on a reference to the Program's Masks map,
// not a copy, ensuring single source of truth.
func NewMaskRepository(program *models.Program) *MaskRepository {
	if program.Masks == nil {
		program.Masks = make(map[string]*models.Mask)
	}

	return &MaskRepository{
		NestedRepository: NewNestedRepository[models.Mask](
			program.Masks,
			program.GetKey(),
			func() error {
				return ProgramRepo().Set(program.GetKey(), program)
			},
		),
		program: program,
	}
}

// New creates a new Mask instance with default values.
func (r *MaskRepository) New() *models.Mask {
	return &models.Mask{
		Name:    "",
		Shape:   "rectangle",
		CenterX: "50",
		CenterY: "50",
		Base:    "",
		Height:  "",
		Radius:  "",
	}
}
