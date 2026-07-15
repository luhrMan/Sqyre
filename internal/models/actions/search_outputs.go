package actions

import "fmt"

const (
	RepeatOnce           = "once"
	RepeatWaitUntilFound = "waituntilfound"
	RepeatWhileFound     = "repeatwhilefound"
)

// RepeatModes are the valid search repeat-mode values.
var RepeatModes = []string{RepeatOnce, RepeatWaitUntilFound, RepeatWhileFound}

// WaitTilFoundConfig configures retry / repeat behavior for search actions.
type WaitTilFoundConfig struct {
	RepeatMode             string `mapstructure:"repeatmode"`
	WaitTilFoundSeconds    int    `mapstructure:"waittilfoundseconds"`
	WaitTilFoundIntervalMs int    `mapstructure:"waittilfoundintervalms"`
	MaxIterations          int    `mapstructure:"maxiterations"`
}

// EffectiveRepeatMode returns a known mode, defaulting to once.
func (w WaitTilFoundConfig) EffectiveRepeatMode() string {
	switch w.RepeatMode {
	case RepeatOnce, RepeatWaitUntilFound, RepeatWhileFound:
		return w.RepeatMode
	default:
		return RepeatOnce
	}
}

// Active reports whether wait-until-found retry should run.
func (w WaitTilFoundConfig) Active() bool {
	return w.EffectiveRepeatMode() == RepeatWaitUntilFound && w.WaitTilFoundSeconds > 0
}

// IsRepeatWhileFound reports whether the search should loop while matches exist.
func (w WaitTilFoundConfig) IsRepeatWhileFound() bool {
	return w.EffectiveRepeatMode() == RepeatWhileFound
}

// EffectiveIntervalMs returns the configured interval or defaultMs when unset.
func (w WaitTilFoundConfig) EffectiveIntervalMs(defaultMs int) int {
	if w.WaitTilFoundIntervalMs > 0 {
		return w.WaitTilFoundIntervalMs
	}
	return defaultMs
}

// EffectiveMaxIterations returns the configured cap or 100 when unset.
func (w WaitTilFoundConfig) EffectiveMaxIterations() int {
	if w.MaxIterations > 0 {
		return w.MaxIterations
	}
	return 100
}

// RepeatModeLabel returns a short UI label for a repeat mode value.
func RepeatModeLabel(mode string) string {
	switch mode {
	case RepeatWaitUntilFound:
		return "Wait until found"
	case RepeatWhileFound:
		return "Repeat while found"
	default:
		return "Once"
	}
}

// DisplayWaitMode returns a human-readable wait mode for action display strings.
func (w WaitTilFoundConfig) DisplayWaitMode(instantLabel string) string {
	switch w.EffectiveRepeatMode() {
	case RepeatWaitUntilFound:
		if w.WaitTilFoundSeconds > 0 {
			return fmt.Sprintf("%d seconds or until found", w.WaitTilFoundSeconds)
		}
		return fmt.Sprintf("wait %ds", w.WaitTilFoundSeconds)
	case RepeatWhileFound:
		if w.WaitTilFoundSeconds > 0 {
			return fmt.Sprintf("repeat while found (%ds)", w.WaitTilFoundSeconds)
		}
		return "repeat while found"
	default:
		return instantLabel
	}
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
