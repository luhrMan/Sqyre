package programs

import "fmt"

type Store struct {
	AllPrograms     map[string]*Program
	EnabledPrograms map[string]*Program
	filePath        string
}

func NewStore(filePath string) (*Store, error) {
	s := &Store{
		AllPrograms: make(map[string]*Program),
		filePath:    filePath,
	}
	if err := s.load(); err != nil {
		return nil, err
	}
	s.rebuildEnabled()
	return s, nil
}

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
