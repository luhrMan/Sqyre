package repositories

import (
	"Squire/internal/models"
	"Squire/internal/models/serialize"
	"bytes"
	"fmt"
	"log"

	"github.com/spf13/viper"
	"gopkg.in/yaml.v3"
)

// decodeMacro unmarshals a macro from YAML configuration by key (case-sensitive).
// It uses the MacroDecodeHookFunc to handle complex action tree structures.
// For non-existent keys, it returns an empty macro (matching Viper behavior).
func decodeMacro(key string) (*models.Macro, error) {
	yamlConfig := serialize.GetYAMLConfig()
	macrosMap := yamlConfig.GetStringMap("macros")

	macroData, ok := macrosMap[key]
	if !ok {
		// Return empty macro for non-existent keys (matches Viper behavior)
		macro := models.NewMacro("", 0, []string{})
		log.Printf("Successfully decoded macro: %s", macro.Name)
		return macro, nil
	}

	// Convert to YAML bytes and back to handle the decode properly
	yamlBytes, err := yaml.Marshal(macroData)
	if err != nil {
		return nil, fmt.Errorf("%w: macro '%s': failed to marshal: %v", ErrDecodeFailed, key, err)
	}

	macro := &models.Macro{}

	// For now, use Viper's decode hook functionality by creating a temporary viper instance
	// This maintains compatibility with existing macro decode logic
	tempViper := viper.New()
	tempViper.SetConfigType("yaml")
	if err := tempViper.ReadConfig(bytes.NewReader(yamlBytes)); err != nil {
		return nil, fmt.Errorf("%w: macro '%s': failed to read: %v", ErrDecodeFailed, key, err)
	}

	err = tempViper.Unmarshal(macro, viper.DecodeHook(serialize.MacroDecodeHookFunc()))
	if err != nil {
		return nil, fmt.Errorf("%w: macro '%s': %v", ErrDecodeFailed, key, err)
	}

	// Ensure Variables is initialized so variable resolution works (e.g. Move with ${var})
	if macro.Variables == nil {
		macro.Variables = models.NewVariableStore()
	}

	log.Printf("Successfully decoded macro: %s", macro.Name)
	return macro, nil
}

// decodeProgram unmarshals a program from YAML configuration by key (case-sensitive).
// It handles nested unmarshalling for Items and Coordinates.
// For non-existent keys, it returns an empty program (matching Viper behavior).
func decodeProgram(key string) (*models.Program, error) {
	yamlConfig := serialize.GetYAMLConfig()
	programsMap := yamlConfig.GetStringMap("programs")

	programData, ok := programsMap[key]
	if !ok {
		// Return empty program for non-existent keys (matches Viper behavior)
		program := models.NewProgram()
		program.Name = ""
		log.Printf("Successfully decoded program: %s", program.Name)
		return program, nil
	}

	// Convert to YAML bytes and unmarshal
	yamlBytes, err := yaml.Marshal(programData)
	if err != nil {
		return nil, fmt.Errorf("%w: program '%s': failed to marshal: %v", ErrDecodeFailed, key, err)
	}

	program := models.NewProgram()
	if err := yaml.Unmarshal(yamlBytes, program); err != nil {
		return nil, fmt.Errorf("%w: program '%s': %v", ErrDecodeFailed, key, err)
	}

	log.Printf("Successfully decoded program: %s", program.Name)
	return program, nil
}
