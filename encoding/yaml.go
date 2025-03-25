package encoding

import (
	"Squire/internal/config"
	"fmt"
	"log"
	"os"

	"gopkg.in/yaml.v3"
)

type sYaml struct {
	serializer
}

func (s *sYaml) Encode(filename string, d any) error {
	filename += config.YAML
	if filename == "" {
		return fmt.Errorf("cannot save empty filename")
	}

	yamlData, err := yaml.Marshal(d)
	if err != nil {
		return fmt.Errorf("error marshalling tree: %v", err)
	}

	err = os.WriteFile(filename, yamlData, 0644)
	if err != nil {
		return fmt.Errorf("error writing to file: %v", err)
	}

	log.Println("Successfully encoded:", filename)
	return nil
}

func (s *sYaml) Decode(filename string, d any) error {
	filename += config.YAML
	log.Printf(config.YAML+" decoding: attempting to read file %v", filename)
	yamlData, err := os.ReadFile(filename)
	if err != nil {
		return fmt.Errorf("error reading file: %v", err)
	}
	log.Printf("config type: %T", d)

	err = yaml.Unmarshal(yamlData, &d)
	if err != nil {
		return fmt.Errorf("error unmarhsalling yaml file: %v", err)
	}

	log.Printf("File successfuly decoded: %T", d)
	log.Println("File successfuly decoded:", d)

	return nil
}
