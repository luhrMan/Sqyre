package actions

import "fmt"

// WaitTilFoundConfig configures retry-until-found behavior for search actions.
type WaitTilFoundConfig struct {
	WaitTilFound           bool `mapstructure:"waittilfound"`
	WaitTilFoundSeconds    int  `mapstructure:"waittilfoundseconds"`
	WaitTilFoundIntervalMs int  `mapstructure:"waittilfoundintervalms"`
}

// Active reports whether wait-until-found retry should run.
func (w WaitTilFoundConfig) Active() bool {
	return w.WaitTilFound && w.WaitTilFoundSeconds > 0
}

// EffectiveIntervalMs returns the configured interval or defaultMs when unset.
func (w WaitTilFoundConfig) EffectiveIntervalMs(defaultMs int) int {
	if w.WaitTilFoundIntervalMs > 0 {
		return w.WaitTilFoundIntervalMs
	}
	return defaultMs
}

// DisplayWaitMode returns a human-readable wait mode for action display strings.
func (w WaitTilFoundConfig) DisplayWaitMode(instantLabel string) string {
	if !w.WaitTilFound {
		return instantLabel
	}
	if w.WaitTilFoundSeconds > 0 {
		return fmt.Sprintf("%d seconds or until found", w.WaitTilFoundSeconds)
	}
	return fmt.Sprintf("wait %ds", w.WaitTilFoundSeconds)
}

// CoordinateOutputs names macro variables that receive match coordinates.
type CoordinateOutputs struct {
	OutputXVariable string `mapstructure:"outputxvariable"`
	OutputYVariable string `mapstructure:"outputyvariable"`
}

// VariableBindings returns output_x / output_y bindings for macro variable analysis.
func (c CoordinateOutputs) VariableBindings() []VariableBinding {
	var out []VariableBinding
	if c.OutputXVariable != "" {
		out = append(out, VariableBinding{Name: c.OutputXVariable, Role: "output_x"})
	}
	if c.OutputYVariable != "" {
		out = append(out, VariableBinding{Name: c.OutputYVariable, Role: "output_y"})
	}
	return out
}
