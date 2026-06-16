package repositories

import (
	"testing"

	"Sqyre/internal/testdb"
)

// setupTestConfig installs the shared test-db.yaml into a per-test temp directory and
// points YAMLConfig at it so tests are isolated and do not mutate the repo fixture.
func setupTestConfig(t *testing.T) string {
	t.Helper()
	p := testdb.SetupYAMLConfig(t)
	resetMacroRepo()
	resetProgramRepo()
	return p
}
