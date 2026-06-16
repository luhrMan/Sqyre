//go:build ignore

// Regenerate test-db.yaml:
//
//	go run internal/testdb/gengolden.go

package main

import (
	"fmt"
	"os"
	"path/filepath"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"

	"gopkg.in/yaml.v3"
)

func main() {
	wd, err := os.Getwd()
	if err != nil {
		panic(err)
	}
	outPath := filepath.Join(wd, "internal/testdb/test-db.yaml")

	integrationMacro := models.NewMacro("Integration Test Macro", 100, []string{"ctrl", "shift", "a"})
	integrationMacro.Root.SetSubActions([]actions.ActionInterface{
		actions.NewWait(100),
		actions.NewClick(true, true),
		actions.NewMove(actions.Point{Name: "p1", X: 10, Y: 20}, false),
	})

	macros := map[string]interface{}{
		"Integration Test Macro": integrationMacro,
		"Macro B":                models.NewMacro("Macro B", 0, nil),
	}
	programs := map[string]interface{}{}
	p0 := models.NewProgram()
	p0.Name = "integration test program"
	programs["integration test program"] = p0
	for i := 1; i <= 8; i++ {
		name := fmt.Sprintf("Program %d", i)
		p := models.NewProgram()
		p.Name = name
		programs[name] = p
	}
	root := map[string]interface{}{
		"macros":   macros,
		"programs": programs,
	}
	b, err := yaml.Marshal(root)
	if err != nil {
		panic(err)
	}
	if err := os.WriteFile(outPath, b, 0644); err != nil {
		panic(err)
	}
}
