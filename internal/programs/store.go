package programs

import (
	"Squire/internal/config"
	"Squire/internal/programs/coordinates"
	"Squire/internal/programs/macro"
	"fmt"
	"sync"
)

type Store struct {
	AllPrograms     map[string]*Program
	EnabledPrograms map[string]*Program
	//filePath        string
}

var lock = &sync.Mutex{}
var s *Store

func GetStore() *Store {
	if s == nil {
		lock.Lock()
		defer lock.Unlock()
		if s == nil {
			fmt.Println("Creating store")
			s = &Store{
				AllPrograms: make(map[string]*Program),
			}
			s.rebuildEnabled()
		} else {
			fmt.Println("store already created.")
		}
	} else {
		fmt.Println("store already created.")
	}
	return s
}

func (s *Store) GetEnabledProgramsPoints() map[string]map[string]coordinates.Point {
	points := make(map[string]map[string]coordinates.Point)
	for _, pro := range s.EnabledPrograms {
		for _, poi := range pro.Coordinates[config.MainMonitorSizeString].Points {
			points[pro.Name][config.MainMonitorSizeString] = poi
		}
	}
	return points
}

func (s *Store) GetEnabledProgramsMacros() map[string][]*macro.Macro {
	macros := make(map[string][]*macro.Macro)
	for _, pro := range s.EnabledPrograms {
		macros[pro.Name] = pro.Macros
	}
	return macros
}

// func (s *Store) GetPrograms()  map[string]*Program { return s.AllPrograms }
// func (s *Store) GetEnabledPrograms()  map[string]*Program { return s.EnabledPrograms }

// load reads the YAML file into AllPrograms.
// func (s *Store) load() error {
//     data, err := ioutil.ReadFile(s.filePath)
//     if err != nil {
//         return err
//     }
//     return yaml.Unmarshal(data, s)
// }

// // Save writes AllPrograms back to the YAML file.
// func (s *Store) Save() error {
//     out, err := yaml.Marshal(s)
//     if err != nil {
//         return err
//     }
//     return ioutil.WriteFile(s.filePath, out, 0644)
// }

func (s *Store) rebuildEnabled() {
	s.EnabledPrograms = make(map[string]*Program)
	for k, p := range s.AllPrograms {
		if p.Enabled {
			s.EnabledPrograms[k] = p
		}
	}
}

// SetEnabled toggles a program’s Enabled flag and updates the derived map.
func (s *Store) SetEnabled(name string, enabled bool) error {
	prog, ok := s.AllPrograms[name]
	if !ok {
		return fmt.Errorf("program %s not found", name)
	}
	prog.Enabled = enabled
	if enabled {
		s.EnabledPrograms[name] = prog
	} else {
		delete(s.EnabledPrograms, name)
	}
	return nil
}
