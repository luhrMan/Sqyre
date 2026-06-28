package actions

import (
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

// Pause blocks macro execution until the user presses a configured continue key chord.
type Pause struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Message     string   `mapstructure:"message"`
	ContinueKey []string `mapstructure:"continuekey"`
	// PassThrough when true delivers the continue chord to the focused application;
	// when false the chord is suppressed as much as the platform allows.
	PassThrough bool `mapstructure:"passthrough"`
}

func NewPause(message string, continueKey []string, passThrough bool) *Pause {
	return &Pause{
		BaseAction:  newBaseAction("pause"),
		Message:     message,
		ContinueKey: append([]string(nil), continueKey...),
		PassThrough: passThrough,
	}
}

func (a *Pause) String() string {
	return stringifyParams(a.parameters())
}

func (a *Pause) Display() fyne.CanvasObject {
	return displayFromParams(a.parameters())
}

func (a *Pause) parameters() []actionParam {
	keyLabel := formatContinueKey(a.ContinueKey)
	if keyLabel == "" {
		keyLabel = "not set"
	}
	pass := "suppress"
	if a.PassThrough {
		pass = "pass through"
	}
	params := []actionParam{
		newParam("Type", a.GetType()),
		newParam("Continue", keyLabel),
		newParam("Key", pass),
	}
	if strings.TrimSpace(a.Message) != "" {
		params = append(params, newParam("Message", a.Message))
	}
	return params
}

func (a *Pause) Icon() fyne.Resource {
	return theme.MediaPauseIcon()
}

// FormatContinueKey returns a human-readable continue chord label.
func FormatContinueKey(keys []string) string {
	return formatContinueKey(keys)
}

func formatContinueKey(keys []string) string {
	if len(keys) == 0 {
		return ""
	}
	var b strings.Builder
	for i, k := range keys {
		if i > 0 {
			b.WriteString(" + ")
		}
		b.WriteString(k)
	}
	return b.String()
}
