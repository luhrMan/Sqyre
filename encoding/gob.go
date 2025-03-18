package encoding

import (
	"encoding/gob"
	"fmt"
	"log"
	"os"
)

type sGob struct {
	serializer
}

func (s *sGob) Encode(data any, filename string) error {
	filename += ".gob"
	file, err := os.Create(filename)
	if err != nil {
		fmt.Errorf("Error creating file: ", err)
		return err
	}
	defer file.Close()

	encoder := gob.NewEncoder(file)
	if err := encoder.Encode(data); err != nil {
		fmt.Errorf("Error encoding data:", err)
		return err
	}
	log.Println("Data encoded and saved to ", filename)
	return nil
}

func (s *sGob) Decode(filename string) (any, error) {
	file, err := os.Open(filename + ".gob")
	if err != nil {
		return nil, fmt.Errorf("Error opening file:", err)
	}
	defer file.Close()

	var data any
	decoder := gob.NewDecoder(file)
	if err := decoder.Decode(&data); err != nil {
		return nil, fmt.Errorf("Error decoding data: ", err)
	}

	return data, nil
}
