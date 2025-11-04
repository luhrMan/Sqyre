package models

import (
	"Squire/internal/config"
	"Squire/internal/models/coordinates"
	"Squire/internal/models/serialize"
	"log"
	"strconv"

	"gocv.io/x/gocv"
)

type Program struct {
	Name        string
	Items       map[string]*Item
	Coordinates map[string]*coordinates.Coordinates
	masks       map[string]func(f ...any) *gocv.Mat
}

func NewProgram() *Program {
	return &Program{
		Items: make(map[string]*Item),
		Coordinates: map[string]*coordinates.Coordinates{
			strconv.Itoa(config.MonitorWidth) + "x" + strconv.Itoa(config.MonitorHeight): { //"2560x1440": {
				Points:      make(map[string]*coordinates.Point),
				SearchAreas: make(map[string]*coordinates.SearchArea),
			},
		},
		masks: make(map[string]func(f ...any) *gocv.Mat),
	}
}

func (p *Program) GetMasks() map[string]func(f ...any) *gocv.Mat {
	return p.masks
}

type Item struct {
	Name     string   `json:"name"`
	GridSize [2]int   `json:"gridSize"`
	Tags     []string `json:"tags"`
	StackMax int      `json:"stackMax"`
	Merchant string   `json:"merchant"`
}

func (p *Program) Decode(s string) (*Program, error) {
	var (
		keyStr = "programs" + "." + s + "."
		err    error
		errStr = "problem here lol"
	)

	p = NewProgram()
	err = serialize.GetViper().UnmarshalKey(keyStr+"name", &p.Name)
	if err != nil {
		log.Fatalf(errStr, err)
	}

	err = serialize.GetViper().UnmarshalKey(keyStr+"items", &p.Items)
	if err != nil {
		log.Fatalf(errStr, err)
	}

	err = serialize.GetViper().UnmarshalKey(keyStr+"coordinates", &p.Coordinates)
	if err != nil {
		log.Fatalf(errStr, err)
	}
	log.Println("Successfully decoded program:", p.Name)
	return p, nil
}

// func DecodeAll() map[string]*Program {
// 	var (
// 		ps = make(map[string]*Program)
// 		ss = serialize.GetViper().GetStringMap("programs")
// 	)
// 	for s := range ss {
// 		p := Decode(s)
// 		ps[s] = p
// 	}
// 	log.Println("Successfully decoded all programs", ps)
// 	return ps
// }

// func Encode(p *Program) error {
// 	serialize.GetViper().Set("programs."+p.Name, p)
// 	err := serialize.GetViper().WriteConfig()
// 	if err != nil {
// 		return fmt.Errorf("error encoding program: %v", err)
// 	}
// 	log.Println("Successfully encoded program:", p.Name)
// 	return nil
// }

// func EncodeAll(pm map[string]*Program) error {
// 	serialize.GetViper().Set("programs", pm)
// 	err := serialize.GetViper().WriteConfig()
// 	if err != nil {
// 		return fmt.Errorf("error encoding programs: %v", err)
// 	}
// 	log.Printf("Successfully encoded programs")
// 	return nil
// }
