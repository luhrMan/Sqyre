# Code Coverage Analysis: `internal/models/repositories`

**Overall: 98.5% of statements** (as of last `go test -coverprofile=... -covermode=atomic`)

Coverage gap tests live in **coverage_test.go** and cover persistence failures, Reload error paths, nested saveFunc failure, and decode error paths.

---

## Summary by file

| File | Notes |
|------|--------|
| **base.go** | 100% on all functions (Set, Delete, Save, Reload error paths covered by coverage_test.go) |
| **nested.go** | 100% on all functions (Set/Delete saveFunc failure covered) |
| **decode.go** | decodeMacro 90.9%, decodeProgram 93.8% — remaining lines: marshal/ReadConfig error paths (hard to trigger without un-marshallable config data) |
| **item.go** | 100% |
| **coordinates.go** | 100% |
| **macro.go** | 100% |
| **program.go** | 100% |
| **errors.go** | N/A (no executable statements) |

---

## Coverage by function (after coverage_test.go)

### base.go — 100% on all
NewBaseRepository, Get, GetAll, GetAllKeys, Set, Delete, Save, Reload, Count, New.

### nested.go — 100% on all
NewNestedRepository, Get, GetAll, GetAllKeys, Set, Delete, Count, Save.

### decode.go
| Function | Coverage | Notes |
|----------|----------|--------|
| **decodeMacro** | **90.9%** | Unmarshal error and Variables nil init covered. Remaining: `yaml.Marshal` error, `tempViper.ReadConfig` error (require injectable bad data). |
| **decodeProgram** | **93.8%** | Unmarshal error and non-existent key path covered. Remaining: `yaml.Marshal` error. |

### item.go, coordinates.go, macro.go, program.go
All reported functions: **100%**.

---

## Tests in coverage_test.go

- **TestBaseRepository_Save_WriteConfigFailure** — Save() when config file is read-only → ErrSaveFailed.
- **TestBaseRepository_Set_PersistFailure** — Set() when persist fails.
- **TestBaseRepository_Delete_PersistFailure** — Delete() when persist fails.
- **TestBaseRepository_Reload_ReadConfigFailureInTestMode** — Reload() when ReadConfig fails (SQYRE_TEST_MODE=1).
- **TestBaseRepository_Reload_ErrLoadFailedWhenAllDecodeFail** — Reload() returns ErrLoadFailed when decodeFunc always fails.
- **TestNestedRepository_Set_SaveFuncFailure** — Nested Set() when saveFunc (ProgramRepo().Set) fails.
- **TestNestedRepository_Delete_SaveFuncFailure** — Nested Delete() when saveFunc fails.
- **TestDecodeMacro_UnmarshalError** — decodeMacro with invalid root structure.
- **TestDecodeMacro_VariablesNilInitialized** — decodeMacro when Variables is nil after Unmarshal.
- **TestDecodeProgram_UnmarshalError** — decodeProgram with invalid items.
- **TestDecodeProgram_NonExistentKeyReturnsEmpty** — decodeProgram for non-existent key returns empty program.

---

## How to reproduce

```bash
cd /workspace
go test ./internal/models/repositories -coverprofile=repos_cover.out -covermode=atomic
go tool cover -func=repos_cover.out
go tool cover -html=repos_cover.out -o cover.html   # open cover.html in browser for line-by-line
```

---

## Remaining gaps (decode.go)

The only remaining uncovered branches are in **decode.go**:

- **decodeMacro**: `yaml.Marshal(macroData)` error — would require config to contain a value that cannot be marshaled (e.g. a channel or func), which normal YAML config never has.
- **decodeMacro**: `tempViper.ReadConfig(bytes.NewReader(yamlBytes))` error — bytes come from Marshal, so they are valid; this path is effectively unreachable in production.
- **decodeProgram**: `yaml.Marshal(programData)` error — same as above.

These can be covered only with a test-only injector or by exporting a helper that accepts raw data; the current design uses the global config, so coverage_test.go does not add those cases.
