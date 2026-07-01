package actions

import (
	"Sqyre/internal/assets"

	"fyne.io/fyne/v2"
)

const (
	DefaultSmoothLow     = 0.05
	DefaultSmoothHigh    = 0.20
	DefaultSmoothDelayMs = 1
)

type Move struct {
	*BaseAction   `yaml:",inline" mapstructure:",squash"`
	Point         CoordinateRef `mapstructure:"point"`
	Smooth        bool          `yaml:"smooth" mapstructure:"smooth"`
	SmoothLow     float64       `yaml:"smoothlow,omitempty" mapstructure:"smoothlow"`
	SmoothHigh    float64       `yaml:"smoothhigh,omitempty" mapstructure:"smoothhigh"`
	SmoothDelayMs int           `yaml:"smoothdelayms,omitempty" mapstructure:"smoothdelayms"`
}

func NewMove(p CoordinateRef, smooth bool) *Move {
	m := &Move{
		BaseAction: newBaseAction("move"),
		Point:      p,
		Smooth:     smooth,
	}
	if smooth {
		m.SmoothLow = DefaultSmoothLow
		m.SmoothHigh = DefaultSmoothHigh
		m.SmoothDelayMs = DefaultSmoothDelayMs
	}
	return m
}

func (a *Move) EffectiveSmoothLow() float64 {
	if a.SmoothLow > 0 {
		return a.SmoothLow
	}
	return DefaultSmoothLow
}

func (a *Move) EffectiveSmoothHigh() float64 {
	if a.SmoothHigh > 0 {
		return a.SmoothHigh
	}
	return DefaultSmoothHigh
}

func (a *Move) EffectiveSmoothDelayMs() int {
	if a.SmoothDelayMs > 0 {
		return a.SmoothDelayMs
	}
	return DefaultSmoothDelayMs
}

func (a *Move) String() string {
	return stringifyParams(a.parameters())
}

func (a *Move) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *Move) parameters() []actionParam {
	params := []actionParam{
		newParam("Type", a.GetType()),
		newParam("Point", a.Point.DisplayLabel()),
	}
	if a.Smooth {
		params = append(params, newParam("Smooth", true))
		params = append(params,
			newParam("Smooth low", a.EffectiveSmoothLow()),
			newParam("Smooth high", a.EffectiveSmoothHigh()),
			newParam("Smooth delay (ms)", a.EffectiveSmoothDelayMs()),
		)
	}
	return params
}

func (a *Move) Icon() fyne.Resource {
	return assets.MouseIcon
}
