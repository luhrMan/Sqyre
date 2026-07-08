package actions

// SemanticSearch finds on-screen objects described by natural-language prompts
// using an open-vocabulary detector (YOLO-World ONNX). Sub-actions run once per match.
type SemanticSearch struct {
	Prompt              string        `mapstructure:"prompt"`
	SearchArea          CoordinateRef `mapstructure:"searcharea"`
	ConfidenceThreshold float32       `mapstructure:"confidencethreshold"`
	IoUThreshold        float32       `mapstructure:"iouthreshold"`
	MaxMatches          int           `mapstructure:"maxmatches"`
	OutputLabelVariable string        `mapstructure:"outputlabelvariable"`
	WaitTilFoundConfig  `yaml:",inline" mapstructure:",squash"`
	CoordinateOutputs   `yaml:",inline" mapstructure:",squash"`
	RunBranchOnNoFind   bool `mapstructure:"runbranchonnofind"`
	*AdvancedAction     `yaml:",inline" mapstructure:",squash"`
}

func NewSemanticSearch(name string, subActions []ActionInterface, prompt string, searchbox CoordinateRef) *SemanticSearch {
	return &SemanticSearch{
		AdvancedAction: newAdvancedAction(name, "semanticsearch", subActions),
		Prompt:         prompt,
		SearchArea:     searchbox,
		ConfidenceThreshold: 0.25,
		IoUThreshold:        0.45,
		CoordinateOutputs: CoordinateOutputs{
			OutputXVariable: "foundX",
			OutputYVariable: "foundY",
		},
	}
}

func (a *SemanticSearch) String() string {
	return stringifyParams(a.Params())
}

func (a *SemanticSearch) Params() []Param {
	mode := a.WaitTilFoundConfig.DisplayWaitMode("instant")
	params := []Param{
		newParam("Type", a.GetType()),
		newParam("Name", a.Name),
		newParam("Prompt", a.Prompt),
		newParam("Search Area", a.SearchArea.DisplayLabel()),
		newExtraParam("Wait", mode),
	}
	if a.ConfidenceThreshold != 0.25 {
		params = append(params, newExtraParam("Confidence", a.ConfidenceThreshold))
	}
	if a.MaxMatches > 0 {
		params = append(params, newExtraParam("Max matches", a.MaxMatches))
	}
	if a.RunBranchOnNoFind {
		params = append(params, newExtraParam("Run on no find", "yes"))
	}
	return params
}

func (a *SemanticSearch) VariableBindings() []VariableBinding {
	out := a.CoordinateOutputs.VariableBindings()
	if a.OutputLabelVariable != "" {
		out = append(out, VariableBinding{Name: a.OutputLabelVariable, Role: "output"})
	}
	return out
}
