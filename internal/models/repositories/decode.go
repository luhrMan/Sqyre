package repositories

import (
	"Squire/internal/models"
	"Squire/internal/models/serialize"
	"fmt"
	"log"

	"github.com/spf13/viper"
)

// decodeMacro unmarshals a macro from Viper configuration by key.
// It uses the MacroDecodeHookFunc to handle complex action tree structures.
func decodeMacro(key string) (*models.Macro, error) {
	keyPath := "macros." + key
	macro := &models.Macro{}

	err := serialize.GetViper().UnmarshalKey(
		keyPath,
		macro,
		viper.DecodeHook(serialize.MacroDecodeHookFunc()),
	)
	if err != nil {
		return nil, fmt.Errorf("%w: macro '%s': %v", ErrDecodeFailed, key, err)
	}

	log.Printf("Successfully decoded macro: %s", macro.Name)
	return macro, nil
}

// decodeProgram unmarshals a program from Viper configuration by key.
// It handles nested unmarshalling for Items and Coordinates.
func decodeProgram(key string) (*models.Program, error) {
	keyPath := "programs." + key
	program := models.NewProgram()

	// Decode name
	if err := serialize.GetViper().UnmarshalKey(keyPath+".name", &program.Name); err != nil {
		return nil, fmt.Errorf("%w: program '%s' name: %v", ErrDecodeFailed, key, err)
	}

	// Decode items
	if err := serialize.GetViper().UnmarshalKey(keyPath+".items", &program.Items); err != nil {
		return nil, fmt.Errorf("%w: program '%s' items: %v", ErrDecodeFailed, key, err)
	}

	// Decode coordinates
	if err := serialize.GetViper().UnmarshalKey(keyPath+".coordinates", &program.Coordinates); err != nil {
		return nil, fmt.Errorf("%w: program '%s' coordinates: %v", ErrDecodeFailed, key, err)
	}

	log.Printf("Successfully decoded program: %s", program.Name)
	return program, nil
}
