package actions

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

func (a *RunMacro) String() string           { return stringifyParams(a.parameters()) }
func (a *RunMacro) Parameters() []ActionParam { return a.parameters() }

func (a *RunMacro) parameters() []ActionParam {
	target := a.MacroName
	if target == "" {
		target = "not set"
	}
	return []ActionParam{
		newParam("Type", a.GetType()),
		newParam("Macro", target),
	}
}
