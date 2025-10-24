package repositories

import "Squire/internal/models/macro"

var macros map[string]*macro.Macro

func InitMacros() {
	macros = macro.GetMacros()
}
func GetMacro(s string) *macro.Macro {
	return macros[s]
}

func GetMacros() map[string]*macro.Macro {
	return macros
}
