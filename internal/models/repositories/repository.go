package repositories

type repository interface {
	Init()
	Get(s string) *any
	GetAll() map[string]any
	Set(s string)
	SetAll() error
}
