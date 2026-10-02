package main

import (
	"fmt"
	"os"
	"path/filepath"

	"Sqyre/internal/models/serialize"
)

func benchPersistLoad(iterations uint64, fixture string) sectionResult {
	path := absFixture(fixture)
	if err := ensureFile(path); err != nil {
		return skipped("persist_yaml_load", err.Error())
	}
	return timed("persist_yaml_load", iterations, "YAMLConfig.ReadConfig on fixture", func() error {
		cfg := serialize.GetYAMLConfig()
		cfg.SetDebounceWrites(false)
		cfg.SetConfigFile(path)
		return cfg.ReadConfig()
	})
}

func benchPersistSave(iterations uint64, fixture string) sectionResult {
	path := absFixture(fixture)
	if err := ensureFile(path); err != nil {
		return skipped("persist_yaml_save", err.Error())
	}
	cfg := serialize.GetYAMLConfig()
	cfg.SetDebounceWrites(false)
	cfg.SetConfigFile(path)
	if err := cfg.ReadConfig(); err != nil {
		return failed("persist_yaml_save", err.Error())
	}
	out := filepath.Join(os.TempDir(), fmt.Sprintf("sqyre-go-bench-persist-%d.yaml", os.Getpid()))
	cfg.SetConfigFile(out)
	res := timed("persist_yaml_save", iterations, "YAMLConfig.WriteConfig of fixture", func() error {
		return cfg.WriteConfig()
	})
	_ = os.Remove(out)
	return res
}
