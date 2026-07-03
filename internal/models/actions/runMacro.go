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

func (a *RunMacro) String() string {
	return stringifyParams(a.Params())
}

func (a *RunMacro) Params() []Param {
	target := a.MacroName
	if target == "" {
		target = "not set"
	}
	return []Param{
		newParam("Type", a.GetType()),
		newParam("Macro", target),
	}
}

