package programs

import (
	"Squire/internal/config"
	"Squire/internal/programs/coordinates"
	"Squire/internal/programs/items"
	"Squire/internal/programs/macro"
	"log"
)

type Program struct {
	Name        string
	Macros      []*macro.Macro
	Items       map[string]items.Item
	Coordinates map[string]*coordinates.Coordinates
	Enabled     bool
}

var (
	programs             = make(map[string]*Program)
	enabledPrograms      = make(map[string]*Program)
	coords               = make(map[string]coordinates.Coordinates)
	points               = make(map[string]coordinates.Point)
	allPointsStringSlice = []string{}
)

func ReadPrograms() map[string]*Program { return programs }
func CreateProgram(name string) {
	p := &Program{
		Name:   name,
		Macros: []*macro.Macro{},
		Items:  make(map[string]items.Item),
		Coordinates: map[string]*coordinates.Coordinates{
			config.MainMonitorSizeString: { //"2560x1440": {
				Points:      make(map[string]coordinates.Point),
				SearchAreas: make(map[string]coordinates.SearchArea),
			},
		},
		Enabled: true,
	}
	programs[name] = p
}
func ReadProgram(name string) *Program {
	return programs[name]
}

func UpdateProgram(name string) {
	_, ok := programs[name]
	if ok {

	}
}

func DeleteProgram(name string) {
	val, ok := programs[name]
	if ok {
		programs[val.Name] = nil
		if programs[val.Name].Enabled {
			enabledPrograms[val.Name] = nil
		}
	}
}

func ReadAllMacros() []*macro.Macro {
	ms := []*macro.Macro{}
	for _, p := range enabledPrograms {
		ms = append(ms, p.Macros...)
	}
	return ms
}
func (p *Program) ReadMacroAtIndex(i int) *macro.Macro { return p.Macros[i] }
func (p *Program) ReadMacroByName(s string) *macro.Macro {
	for _, m := range p.Macros {
		if m.Name == s {
			return m
		}
	}
	return nil
}

func (p *Program) CreateMacro(s string, d int) {
	if s == "" {
		return
	}
	p.Macros = append(p.Macros, macro.CreateMacro(s, d, []string{}))
}

func ReadCoordinates() map[string]coordinates.Coordinates {
	return coords
}

func UpdateCoordinates() {
	clear(coords)
	for _, pro := range enabledPrograms {
		for _, cs := range pro.Coordinates {
			coords[pro.Name+" "+config.MainMonitorSizeString] = *cs
		}
	}
}

// func ReadEnabledPrograms() map[string]*Program {
// 	return enabledPrograms
// }

//	func CalculateEnabledPoints() int {
//		ReadEnabledPrograms()
//		totalPoints := 0
//		for _, pro := range programs {
//			if pro.Enabled {
//				totalPoints += len(pro.Coordinates)
//			}
//		}
//		return totalPoints
//	}

func UpdatePoints() {
	clear(points)
	for s, cs := range coords {
		for _, poi := range cs.Points {
			points[s+" "+poi.Name] = poi
		}
	}
}

func ReadPoints() map[string]coordinates.Point {
	return points
}

func ReadPointsAsStringSlice() []string {
	for _, poi := range ReadPoints() {
		allPointsStringSlice = append(allPointsStringSlice, poi.Name)
	}
	return allPointsStringSlice
}

func ReadPoint(name string) coordinates.Point {
	val, ok := points[name]
	if ok {
		return val
	}
	return val
}

func InitPrograms() {
	keystr := "programs"
	programsList := config.ViperConfig.Get(keystr)
	for s := range programsList.(map[string]any) {
		log.Println(s)
		CreateProgram(s)
	}
	// keystr = "programs" + "." + config.DarkAndDarker + "."

	// SetCurrentProgram(ps.GetProgram(config.DarkAndDarker), config.DarkAndDarker)
	// macros := config.ViperConfig.GetStringSlice(keystr + "macros")
	// for i := range macros {
	// 	p.GetProgram(config.DarkAndDarker).Macros = append(p.GetProgram(config.DarkAndDarker).Macros, macro.NewMacro("New Macro "+strconv.Itoa(i), 30, []string{}))
	// 	err := p.GetProgram(config.DarkAndDarker).GetMacroAtIndex(i).UnmarshalMacro(i)
	// 	if err != nil {
	// 		log.Println(err)
	// 	}
	// }
	// config.ViperConfig.UnmarshalKey(keystr+"coordinates", &p.GetProgram(config.DarkAndDarker).Coordinates)
	// config.ViperConfig.UnmarshalKey(keystr+"items", &p.GetProgram(config.DarkAndDarker).Items)
	// items.SetItemsMap(currentProgram.Items)
}

// func (p *Program) GetItem(i string) items.Item {
// 	if item, ok := p.Items[i]; ok {
// 		return item
// 	}
// 	return items.Item{}
// }

// func (p *Program) AddProgramPoint(ss string, point config.Point) {
// 	c := p.Coordinates
// 	points := c[ss].Points
// 	points[point.Name] = point
// }
// func (p *Program) SerializeJsonPointsToProgram(ss string) {
// 	jpm := config.JsonPointMap()
// 	for _, point := range jpm {
// 		p.AddProgramPoint(ss, point)
// 	}
// }

// func (p *Program) AddProgramSearchArea(ss string, sa config.SearchArea) {
// 	c := p.Coordinates
// 	sas := c[ss].SearchAreas
// 	sas[sa.Name] = sa
// }
