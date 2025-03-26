package programs

import (
	"Squire/internal/config"
	"Squire/internal/programs/macro"
	"log"
	"strconv"
)

var programs = &Programs{}
var currentProgram *Program

type Programs map[string]*Program

func SetCurrentProgram(p *Program) {
	currentProgram = p
}
func CurrentProgram() *Program {
	return currentProgram
}

func GetPrograms() *Programs                        { return programs }
func (p *Programs) GetProgram(name string) *Program { return (*p)[name] }

func (p *Programs) InitPrograms() {
	keystr := "programs" + "." + config.DarkAndDarker + "."
	(*p)[config.DarkAndDarker] = NewProgram()
	SetCurrentProgram(p.GetProgram(config.DarkAndDarker))
	macros := config.ViperConfig.GetStringSlice(keystr + "macros")
	for i := range macros {
		p.GetProgram(config.DarkAndDarker).Macros = append(p.GetProgram(config.DarkAndDarker).Macros, macro.NewMacro("New Macro "+strconv.Itoa(i), 30, ""))
		err := p.GetProgram(config.DarkAndDarker).GetMacroAtIndex(i).UnmarshalMacro(i)
		if err != nil {
			log.Println(err)
		}
	}
	log.Println("viper key coordinates: ", config.ViperConfig.Get(keystr+"coordinates"))
	config.ViperConfig.UnmarshalKey(keystr+"coordinates", &p.GetProgram(config.DarkAndDarker).Coordinates)
	log.Println("program struct coordinates: ", p.GetProgram(config.DarkAndDarker).Coordinates)
}
