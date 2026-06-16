package testdb

import (
	_ "embed"
	"os"
	"path/filepath"
	"testing"

	"Sqyre/internal/models/serialize"
)

//go:embed test-db.yaml
var fixture []byte

// Fixture returns the canonical test-db.yaml bytes (read-only template).
func Fixture() []byte {
	return fixture
}

// SetupYAMLConfig writes a copy of test-db.yaml into t.TempDir(), points
// serialize.GetYAMLConfig at it, and loads the file. Use this from any test
// package that needs the same DB shape as repository integration tests.
// Returns the absolute path to the temp db.yaml file.
func SetupYAMLConfig(t *testing.T) string {
	t.Helper()

	dir := t.TempDir()
	dbPath := filepath.Join(dir, "db.yaml")
	if err := os.WriteFile(dbPath, fixture, 0644); err != nil {
		t.Fatalf("testdb: write db: %v", err)
	}

	yamlConfig := serialize.GetYAMLConfig()
	yamlConfig.SetConfigFile(dbPath)
	if err := yamlConfig.ReadConfig(); err != nil {
		t.Fatalf("testdb: read config: %v", err)
	}

	return dbPath
}
