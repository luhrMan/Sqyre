package actions

import (
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
	return stringifyParams(a.parameters())
}

func (a *RunMacro) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *RunMacro) parameters() []actionParam {
	target := a.MacroName
	if target == "" {
		target = "not set"
	}
	return []actionParam{
		newParam("Type", a.GetType()),
		newParam("Macro", target),
	}
}

func (a *RunMacro) Icon() fyne.Resource {
	return theme.MediaPlayIcon()
}
