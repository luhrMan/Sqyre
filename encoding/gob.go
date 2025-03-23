package encoding

import (
	"Squire/internal/data"
	"encoding/gob"
	"fmt"
	"log"
	"os"
)

type sGob struct {
	serializer
}

func (s *sGob) Encode(filename string, d any) error {
	filename += data.GOB
	file, err := os.Create(filename)
	if err != nil {
		return fmt.Errorf("error creating file: %w", err)
	}
	defer file.Close()

	encoder := gob.NewEncoder(file)
	if err := encoder.Encode(&d); err != nil {
		return fmt.Errorf("error encoding data: %w", err)
	}
	log.Println("Data encoded and saved to ", filename)
	return nil
}

func (s *sGob) Decode(filename string, d any) error {
	file, err := os.Open(filename + data.GOB)
	if err != nil {
		return fmt.Errorf("error opening file: %w", err)
	}
	defer file.Close()
	log.Println("file: ", &file)
	decoder := gob.NewDecoder(file)
	log.Printf("data type: %T", d)

	if err := decoder.Decode(&d); err != nil {
		return fmt.Errorf("error decoding data: %w", err)
	}
	log.Println("Successfully decoded data: ", d)
	return nil
}
