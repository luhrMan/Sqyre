package actions

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
	return stringifyParams(a.Params())
}

func (a *Move) Params() []Param {
	params := []Param{
		newParam("Type", a.GetType()),
		newParam("Point", a.Point.DisplayLabel()),
	}
	if a.Smooth {
		params = append(params, newExtraParam("Smooth", true))
		params = append(params,
			newExtraParam("Smooth low", a.EffectiveSmoothLow()),
			newExtraParam("Smooth high", a.EffectiveSmoothHigh()),
			newExtraParam("Smooth delay (ms)", a.EffectiveSmoothDelayMs()),
		)
	}
	return params
}
