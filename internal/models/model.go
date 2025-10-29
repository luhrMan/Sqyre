package models

type ModelInterface[T Program | Macro] interface {
	Decode(s string) (T, error)
	// DecodeAll()
}

// type Model[T Program | Macro] struct {
// 	// Model T
// }

// func (m *Model[T]) Decode(s string) (T, error) {
// 	return m, nil
// }
