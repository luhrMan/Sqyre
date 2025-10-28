package repositories

import (
	"Squire/internal/models/serialize"
	"fmt"
	"log"
	"strings"
)

type repositoryInterface[T any] interface {
	// Init()
	Get(s string) *T
	GetAll() map[string]*T
	GetAllAsStringSlice() []string
	Set(s string, t T)
	SetAll(s map[string]T)
	EncodeAll(k string) error
	DecodeAll(k string, decode func(s string) (*T, error)) map[string]T
}

type repository[T any] struct {
	// model  T
	model  string
	models map[string]*T
}

func (r *repository[T]) Get(s string) *T {
	if m, ok := r.models[strings.ToLower(s)]; ok {
		return m
	}
	return nil
}

func (r *repository[T]) GetAll() map[string]*T {
	return r.models
}

func (r *repository[T]) GetAllAsStringSlice() []string {
	keys := make([]string, len(r.models))

	i := 0
	for s := range r.models {
		keys[i] = s
		i++

	}
	return keys
}

func (r *repository[T]) Set(s string, t *T) {
	r.models[s] = t
}
func (r *repository[T]) SetAll(s map[string]*T) {
	r.models = s
}

func (r *repository[T]) Delete(s string) {
	delete(r.models, strings.ToLower(s))
	r.EncodeAll()
}

func (r *repository[T]) EncodeAll() error {
	serialize.GetViper().Set(r.model, r.models)
	err := serialize.GetViper().WriteConfig()
	if err != nil {
		return fmt.Errorf("error encoding %v: %v", r.model, err)
	}
	log.Printf("Successfully encoded ", r.model)
	return nil
}

func (r *repository[T]) DecodeAll(k string, decode func(s string) (*T, error)) map[string]*T {
	var (
		ps = make(map[string]*T)
		ss = serialize.GetViper().GetStringMap(k)
	)
	for s := range ss {
		p, _ := decode(s)
		ps[s] = p
	}
	log.Printf("Successfully decoded all %v: %v", k, ps)
	return ps
}

// func EncodeAll(pm map[string]*Program) error {
// 	serialize.GetViper().Set("programs", pm)
// 	err := serialize.GetViper().WriteConfig()
// 	if err != nil {
// 		return fmt.Errorf("error encoding programs: %v", err)
// 	}
// 	log.Printf("Successfully encoded programs")
// 	return nil
// }
