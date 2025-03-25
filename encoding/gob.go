package encoding

import (
	"Squire/internal/config"
	"encoding/gob"
	"fmt"
	"log"
	"os"
)

type sGob struct {
	serializer
}

func (s *sGob) Encode(filename string, d any) error {
	filename += config.GOB
	file, err := os.Create(filename)
	if err != nil {
		return fmt.Errorf("error creating file: %w", err)
	}
	defer file.Close()

	encoder := gob.NewEncoder(file)
	if err := encoder.Encode(&d); err != nil {
		return fmt.Errorf("error encoding config: %w", err)
	}
	log.Println("Data encoded and saved to ", filename)
	return nil
}

func (s *sGob) Decode(filename string, d any) error {
	file, err := os.Open(filename + config.GOB)
	if err != nil {
		return fmt.Errorf("error opening file: %w", err)
	}
	defer file.Close()
	log.Println("file: ", &file)
	decoder := gob.NewDecoder(file)
	log.Printf("config type: %T", d)

	if err := decoder.Decode(&d); err != nil {
		return fmt.Errorf("error decoding config: %w", err)
	}
	log.Println("Successfully decoded config: ", d)
	return nil
}
