package actions

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

// RunMacro executes another macro by name. The user selects from existing macros.
type RunMacro struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	MacroName   string `mapstructure:"macroname"`
}

func NewRunMacro(macroName string) *RunMacro {
	return &RunMacro{
		BaseAction: newBaseAction("runmacro"),
		MacroName:  macroName,
	}
}

func (a *RunMacro) String() string {
	if a.MacroName == "" {
		return "Run macro (not set)"
	}
	return fmt.Sprintf("Run: %s", a.MacroName)
}

func (a *RunMacro) Icon() fyne.Resource {
	return theme.MediaPlayIcon()
}
