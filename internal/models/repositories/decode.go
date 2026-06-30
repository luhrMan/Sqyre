package repositories

import (
	"Sqyre/internal/models"
	"Sqyre/internal/models/serialize"
	"fmt"
	"log"

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

	macro, err := serialize.DecodeMacroFromMap(macroData)
	if err != nil {
		return nil, fmt.Errorf("%w: macro '%s': %v", ErrDecodeFailed, key, err)
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
		return nil, fmt.Errorf("%w: program '%s': %w", ErrDecodeFailed, key, serialize.YAMLErrorWithContent(yamlBytes, err))
	}

	log.Printf("Successfully decoded program: %s", program.Name)
	return program, nil
}
