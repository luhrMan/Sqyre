package repositories

import (
	"Squire/internal/models"
	"errors"
	"sort"
	"testing"
)

// CRUD contract (single source of truth for all repositories):
//
//	Get: non-existent key → ErrNotFound; empty key → ErrInvalidKey; existing key → model, GetKey() == key (exact case); wrong case → ErrNotFound
//	GetAll: empty repo → non-nil empty map; non-empty → returns copy (mutating map doesn't change Count())
//	GetAllKeys: empty repo → non-nil empty slice; non-empty → sorted order
//	Set: new key → stored, Get(key) returns it, model.GetKey() == key; existing key → updated; empty key → ErrInvalidKey; nil model → error
//	Delete: existing key → removed, Get → ErrNotFound, Count decreased; non-existent → idempotent (no error); empty key → ErrInvalidKey
//	Count: empty → 0; after Set/Delete matches stored count
//	Save: after Set/Delete, Save() does not error (where applicable)
//
// Tests that are obsolete with this suite (same scenarios, removed to avoid duplication):
//   - item_test.go: TestItemRepository_Get, GetAll, GetAllKeys, Set, Delete, Count
//   - coordinates_test.go: TestPointRepository_Get/GetAll/GetAllKeys/Set/Delete/Count, TestSearchAreaRepository_Get/GetAll/GetAllKeys/Set/Delete/Count
//   - macro_test.go: TestMacroRepo_CRUD
// base_test.go is kept: it validates BaseRepository in isolation with testModel (no config). Other tests keep repo-specific behavior (ThreadSafety, Reload, Integration, GetAllWithProgramPrefix, etc.).

// CRUDHarness abstracts any repository so the same contract tests can run against all repo types.
type CRUDHarness struct {
	Get        func(key string) (interface{}, error)
	GetAll     func() map[string]interface{}
	GetAllKeys func() []string
	Set        func(key string, model interface{}) error
	Delete     func(key string) error
	Count      func() int
	Save       func() error
	NewModel   func(key string) interface{} // returns a model with GetKey() == key after Set
}

func testGetContract(t *testing.T, h CRUDHarness) {
	t.Helper()
	t.Run("Get_non_existent_returns_ErrNotFound", func(t *testing.T) {
		_, err := h.Get("nonexistent")
		if !errors.Is(err, ErrNotFound) {
			t.Errorf("expected ErrNotFound, got %v", err)
		}
	})
	t.Run("Get_empty_key_returns_ErrInvalidKey", func(t *testing.T) {
		_, err := h.Get("")
		if !errors.Is(err, ErrInvalidKey) {
			t.Errorf("expected ErrInvalidKey, got %v", err)
		}
	})
	t.Run("Get_existing_key_returns_model_with_matching_GetKey", func(t *testing.T) {
		key := "contract-get-test"
		model := h.NewModel(key)
		if err := h.Set(key, model); err != nil {
			t.Fatalf("setup Set: %v", err)
		}
		retrieved, err := h.Get(key)
		if err != nil {
			t.Fatalf("Get: %v", err)
		}
		bm := retrieved.(models.BaseModel)
		if bm.GetKey() != key {
			t.Errorf("GetKey() = %q, want %q", bm.GetKey(), key)
		}
	})
	t.Run("Get_wrong_case_returns_ErrNotFound", func(t *testing.T) {
		key := "ExactCaseKey"
		model := h.NewModel(key)
		if err := h.Set(key, model); err != nil {
			t.Fatalf("setup Set: %v", err)
		}
		for _, wrongKey := range []string{"exactcasekey", "EXACTCASEKEY", "ExactCaseKey "} {
			if wrongKey == key {
				continue
			}
			_, err := h.Get(wrongKey)
			if !errors.Is(err, ErrNotFound) {
				t.Errorf("Get(%q) expected ErrNotFound, got %v", wrongKey, err)
			}
		}
	})
}

func testGetAllContract(t *testing.T, h CRUDHarness) {
	t.Helper()
	t.Run("GetAll_empty_returns_non_nil_empty_map", func(t *testing.T) {
		all := h.GetAll()
		if all == nil {
			t.Error("GetAll() returned nil")
		}
		if len(all) != 0 {
			t.Errorf("expected empty map, got %d entries", len(all))
		}
	})
	t.Run("GetAll_returns_copy", func(t *testing.T) {
		key := "copy-test"
		if err := h.Set(key, h.NewModel(key)); err != nil {
			t.Fatalf("setup Set: %v", err)
		}
		all := h.GetAll()
		if all == nil {
			t.Fatal("GetAll() returned nil")
		}
		before := h.Count()
		all["mutated-key"] = nil
		after := h.Count()
		if after != before {
			t.Error("GetAll should return a copy; mutating it changed Count()")
		}
	})
}

func testGetAllKeysContract(t *testing.T, h CRUDHarness) {
	t.Helper()
	t.Run("GetAllKeys_empty_returns_non_nil_empty_slice", func(t *testing.T) {
		keys := h.GetAllKeys()
		if keys == nil {
			t.Error("GetAllKeys() returned nil")
		}
		if len(keys) != 0 {
			t.Errorf("expected empty slice, got %d keys", len(keys))
		}
	})
	t.Run("GetAllKeys_returns_sorted_order", func(t *testing.T) {
		names := []string{"zebra", "alpha", "mango"}
		for _, k := range names {
			if err := h.Set(k, h.NewModel(k)); err != nil {
				t.Fatalf("setup Set(%q): %v", k, err)
			}
		}
		keys := h.GetAllKeys()
		if !sort.SliceIsSorted(keys, func(i, j int) bool { return keys[i] < keys[j] }) {
			t.Errorf("GetAllKeys() not sorted: %v", keys)
		}
		exp := []string{"alpha", "mango", "zebra"}
		if len(keys) != len(exp) {
			t.Errorf("len(keys) = %d, want %d", len(keys), len(exp))
		}
		for i, k := range exp {
			if i < len(keys) && keys[i] != k {
				t.Errorf("keys[%d] = %q, want %q", i, keys[i], k)
			}
		}
	})
}

func testSetContract(t *testing.T, h CRUDHarness) {
	t.Helper()
	t.Run("Set_new_key_stores_and_GetKey_equals_key", func(t *testing.T) {
		key := "new-key"
		model := h.NewModel("other")
		if err := h.Set(key, model); err != nil {
			t.Fatalf("Set: %v", err)
		}
		retrieved, err := h.Get(key)
		if err != nil {
			t.Fatalf("Get: %v", err)
		}
		if gk := retrieved.(models.BaseModel).GetKey(); gk != key {
			t.Errorf("GetKey() = %q, want %q", gk, key)
		}
	})
	t.Run("Set_existing_key_updates", func(t *testing.T) {
		key := "update-key"
		if err := h.Set(key, h.NewModel(key)); err != nil {
			t.Fatalf("first Set: %v", err)
		}
		updated := h.NewModel(key)
		if err := h.Set(key, updated); err != nil {
			t.Fatalf("second Set: %v", err)
		}
		retrieved, err := h.Get(key)
		if err != nil {
			t.Fatalf("Get: %v", err)
		}
		if retrieved.(models.BaseModel).GetKey() != key {
			t.Errorf("GetKey() after update = %q, want %q", retrieved.(models.BaseModel).GetKey(), key)
		}
	})
	t.Run("Set_empty_key_returns_ErrInvalidKey", func(t *testing.T) {
		err := h.Set("", h.NewModel("x"))
		if !errors.Is(err, ErrInvalidKey) {
			t.Errorf("expected ErrInvalidKey, got %v", err)
		}
	})
	t.Run("Set_nil_model_returns_error", func(t *testing.T) {
		err := h.Set("nil-key", nil)
		if err == nil {
			t.Error("expected error when setting nil model")
		}
	})
}

// harnessFactory returns a fresh CRUDHarness for each contract group so "empty" assertions see a clean repo.
type harnessFactory func() CRUDHarness

func testDeleteContract(t *testing.T, h CRUDHarness) {
	t.Helper()
	t.Run("Delete_existing_removes_and_Get_returns_ErrNotFound", func(t *testing.T) {
		key := "delete-me"
		if err := h.Set(key, h.NewModel(key)); err != nil {
			t.Fatalf("setup Set: %v", err)
		}
		before := h.Count()
		if err := h.Delete(key); err != nil {
			t.Fatalf("Delete: %v", err)
		}
		if h.Count() != before-1 {
			t.Errorf("Count after Delete = %d, want %d", h.Count(), before-1)
		}
		_, err := h.Get(key)
		if !errors.Is(err, ErrNotFound) {
			t.Errorf("Get after Delete expected ErrNotFound, got %v", err)
		}
	})
	t.Run("Delete_non_existent_idempotent", func(t *testing.T) {
		if err := h.Delete("nonexistent"); err != nil {
			t.Errorf("Delete nonexistent should be idempotent, got %v", err)
		}
	})
	t.Run("Delete_empty_key_returns_ErrInvalidKey", func(t *testing.T) {
		err := h.Delete("")
		if !errors.Is(err, ErrInvalidKey) {
			t.Errorf("expected ErrInvalidKey, got %v", err)
		}
	})
}

func testCountContract(t *testing.T, h CRUDHarness) {
	t.Helper()
	t.Run("Count_empty_is_zero", func(t *testing.T) {
		if c := h.Count(); c != 0 {
			t.Errorf("Count() = %d, want 0", c)
		}
	})
	t.Run("Count_after_Set_and_Delete_matches_stored", func(t *testing.T) {
		for i, key := range []string{"c1", "c2", "c3"} {
			if err := h.Set(key, h.NewModel(key)); err != nil {
				t.Fatalf("Set: %v", err)
			}
			if h.Count() != i+1 {
				t.Errorf("after Set %q Count = %d, want %d", key, h.Count(), i+1)
			}
		}
		h.Delete("c2")
		if h.Count() != 2 {
			t.Errorf("after Delete Count = %d, want 2", h.Count())
		}
	})
}

func testSaveContract(t *testing.T, h CRUDHarness) {
	t.Helper()
	t.Run("Save_after_Set_does_not_error", func(t *testing.T) {
		if err := h.Set("save-test", h.NewModel("save-test")); err != nil {
			t.Fatalf("Set: %v", err)
		}
		if err := h.Save(); err != nil {
			t.Errorf("Save() after Set: %v", err)
		}
	})
}

func runCRUDContract(t *testing.T, name string, newHarness harnessFactory) {
	t.Helper()
	// Each group gets a fresh harness so "empty" contract assertions see a clean repo
	t.Run("GetAll", func(t *testing.T) { testGetAllContract(t, newHarness()) })
	t.Run("GetAllKeys", func(t *testing.T) { testGetAllKeysContract(t, newHarness()) })
	t.Run("Count", func(t *testing.T) { testCountContract(t, newHarness()) })
	t.Run("Get", func(t *testing.T) { testGetContract(t, newHarness()) })
	t.Run("Set", func(t *testing.T) { testSetContract(t, newHarness()) })
	t.Run("Delete", func(t *testing.T) { testDeleteContract(t, newHarness()) })
	t.Run("Save", func(t *testing.T) { testSaveContract(t, newHarness()) })
}

// --- Macro

func TestMacroRepository_CRUDContract(t *testing.T) {
	setupTestConfig(t)
	newHarness := func() CRUDHarness {
		repo := &MacroRepository{
			BaseRepository: NewBaseRepository(
				"macros",
				decodeMacro,
				func() *models.Macro { return models.NewMacro("", 0, nil) },
			),
		}
		return CRUDHarness{
			Get: func(key string) (interface{}, error) {
				m, err := repo.Get(key)
				if err != nil {
					return nil, err
				}
				return m, nil
			},
			GetAll: func() map[string]interface{} {
				all := repo.GetAll()
				out := make(map[string]interface{}, len(all))
				for k, v := range all {
					out[k] = v
				}
				return out
			},
			GetAllKeys: repo.GetAllKeys,
			Set: func(key string, model interface{}) error {
				var m *models.Macro
				if model != nil {
					m = model.(*models.Macro)
				}
				return repo.Set(key, m)
			},
			Delete: repo.Delete,
			Count:  repo.Count,
			Save:   repo.Save,
			NewModel: func(key string) interface{} {
				m := models.NewMacro(key, 0, nil)
				m.SetKey(key)
				return m
			},
		}
	}
	runCRUDContract(t, "Macro", newHarness)
}

// --- Program

func TestProgramRepository_CRUDContract(t *testing.T) {
	setupTestConfig(t)
	newHarness := func() CRUDHarness {
		repo := &ProgramRepository{
			BaseRepository: NewBaseRepository(
				"programs",
				decodeProgram,
				func() *models.Program { return models.NewProgram() },
			),
		}
		return CRUDHarness{
			Get: func(key string) (interface{}, error) {
				p, err := repo.Get(key)
				if err != nil {
					return nil, err
				}
				return p, nil
			},
			GetAll: func() map[string]interface{} {
				all := repo.GetAll()
				out := make(map[string]interface{}, len(all))
				for k, v := range all {
					out[k] = v
				}
				return out
			},
			GetAllKeys: repo.GetAllKeys,
			Set: func(key string, model interface{}) error {
				var p *models.Program
				if model != nil {
					p = model.(*models.Program)
				}
				return repo.Set(key, p)
			},
			Delete: repo.Delete,
			Count:  repo.Count,
			Save:   repo.Save,
			NewModel: func(key string) interface{} {
				p := models.NewProgram()
				p.SetKey(key)
				return p
			},
		}
	}
	runCRUDContract(t, "Program", newHarness)
}

// --- Item

func TestItemRepository_CRUDContract(t *testing.T) {
	setupTestConfig(t)
	newHarness := func() CRUDHarness {
		resetProgramRepo()
		program := models.NewProgram()
		program.Name = "crud-contract-program"
		if err := ProgramRepo().Set(program.GetKey(), program); err != nil {
			t.Fatalf("setup program: %v", err)
		}
		repo := NewItemRepository(program)
		return CRUDHarness{
			Get: func(key string) (interface{}, error) {
				item, err := repo.Get(key)
				if err != nil {
					return nil, err
				}
				return item, nil
			},
			GetAll: func() map[string]interface{} {
				all := repo.GetAll()
				out := make(map[string]interface{}, len(all))
				for k, v := range all {
					out[k] = v
				}
				return out
			},
			GetAllKeys: repo.GetAllKeys,
			Set: func(key string, model interface{}) error {
				if model == nil {
					return repo.Set(key, nil)
				}
				return repo.Set(key, model.(*models.Item))
			},
			Delete: repo.Delete,
			Count:  repo.Count,
			Save:   repo.Save,
			NewModel: func(key string) interface{} {
				item := repo.New()
				item.SetKey(key)
				return item
			},
		}
	}
	runCRUDContract(t, "Item", newHarness)
}

// --- Point

func TestPointRepository_CRUDContract(t *testing.T) {
	setupTestConfig(t)
	newHarness := func() CRUDHarness {
		resetProgramRepo()
		program := models.NewProgram()
		program.Name = "crud-contract-program"
		resKey := "2560x1440"
		program.Coordinates[resKey] = &models.Coordinates{
			Points:      make(map[string]*models.Point),
			SearchAreas: make(map[string]*models.SearchArea),
		}
		if err := ProgramRepo().Set(program.GetKey(), program); err != nil {
			t.Fatalf("setup program: %v", err)
		}
		repo := NewPointRepository(program, resKey)
		return CRUDHarness{
			Get: func(key string) (interface{}, error) {
				p, err := repo.Get(key)
				if err != nil {
					return nil, err
				}
				return p, nil
			},
			GetAll: func() map[string]interface{} {
				all := repo.GetAll()
				out := make(map[string]interface{}, len(all))
				for k, v := range all {
					out[k] = v
				}
				return out
			},
			GetAllKeys: repo.GetAllKeys,
			Set: func(key string, model interface{}) error {
				if model == nil {
					return repo.Set(key, nil)
				}
				return repo.Set(key, model.(*models.Point))
			},
			Delete: repo.Delete,
			Count:  repo.Count,
			Save:   repo.Save,
			NewModel: func(key string) interface{} {
				pt := repo.New()
				pt.SetKey(key)
				return pt
			},
		}
	}
	runCRUDContract(t, "Point", newHarness)
}

// --- SearchArea

func TestSearchAreaRepository_CRUDContract(t *testing.T) {
	setupTestConfig(t)
	newHarness := func() CRUDHarness {
		resetProgramRepo()
		program := models.NewProgram()
		program.Name = "crud-contract-program"
		resKey := "2560x1440"
		program.Coordinates[resKey] = &models.Coordinates{
			Points:      make(map[string]*models.Point),
			SearchAreas: make(map[string]*models.SearchArea),
		}
		if err := ProgramRepo().Set(program.GetKey(), program); err != nil {
			t.Fatalf("setup program: %v", err)
		}
		repo := NewSearchAreaRepository(program, resKey)
		return CRUDHarness{
			Get: func(key string) (interface{}, error) {
				sa, err := repo.Get(key)
				if err != nil {
					return nil, err
				}
				return sa, nil
			},
			GetAll: func() map[string]interface{} {
				all := repo.GetAll()
				out := make(map[string]interface{}, len(all))
				for k, v := range all {
					out[k] = v
				}
				return out
			},
			GetAllKeys: repo.GetAllKeys,
			Set: func(key string, model interface{}) error {
				if model == nil {
					return repo.Set(key, nil)
				}
				return repo.Set(key, model.(*models.SearchArea))
			},
			Delete: repo.Delete,
			Count:  repo.Count,
			Save:   repo.Save,
			NewModel: func(key string) interface{} {
				sa := repo.New()
				sa.SetKey(key)
				return sa
			},
		}
	}
	runCRUDContract(t, "SearchArea", newHarness)
}
