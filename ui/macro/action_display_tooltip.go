package macro

import (
	"strings"

	"Sqyre/internal/models/actions"
	"Sqyre/ui/actiondisplay"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
)

type actionDisplayHandlers struct {
	onActionSaved func()
}

func actionDisplay(node actions.ActionInterface, handlers actionDisplayHandlers) fyne.CanvasObject {
	return actionDisplayFromParams(node, node.Params(), handlers)
}

func actionDisplayForTree(node actions.ActionInterface, handlers actionDisplayHandlers) fyne.CanvasObject {
	return actionDisplayFromParams(node, actionDisplayParamsForTree(node), handlers)
}

func actionDisplayParamsForTree(node actions.ActionInterface) []actions.Param {
	params := node.Params()
	if _, ok := node.(*actions.ImageSearch); !ok {
		return params
	}
	filtered := make([]actions.Param, 0, len(params))
	for _, p := range params {
		if strings.EqualFold(p.Label, "Items") {
			continue
		}
		filtered = append(filtered, p)
	}
	return filtered
}

func actionDisplayFromParams(node actions.ActionInterface, params []actions.Param, handlers actionDisplayHandlers) fyne.CanvasObject {
	known := macroKnownVariables()
	line, extra := buildActionSummaryLine(node, params, known)
	actionType := actions.ActionTypeFromParams(params)
	loader := actionPreviewLoader(node)
	return newActionDisplayTooltipHover(node, line, extra, actionType, loader, handlers.onActionSaved)
}

// buildActionSummaryLine renders the inline summary pills for a tree row.
// Output variables render as variable-name chips (matching the tooltip popup):
// in place when the name already appears as a summary param (e.g. Set),
// otherwise appended after the other pills (e.g. search coordinate outputs).
func buildActionSummaryLine(node actions.ActionInterface, params []actions.Param, known map[string]bool) (fyne.CanvasObject, []actions.Param) {
	summary, extra := actions.DisplayParams(params)
	actionType := actions.ActionTypeFromParams(params)

	outputLabels := map[string]string{}
	var bindings []actions.VariableBinding
	if producer, ok := node.(actions.VariableProducer); ok {
		for _, b := range producer.VariableBindings() {
			if b.Name == "" {
				continue
			}
			bindings = append(bindings, b)
			outputLabels[b.Name] = outputBindingLabel(b.Role)
		}
	}

	box := container.NewHBox()
	consumed := map[string]bool{}
	for _, p := range summary {
		text := actions.FormatParamMinimal(p)
		if text == "" {
			continue
		}
		if label, ok := outputLabels[text]; ok && !consumed[text] {
			consumed[text] = true
			box.Add(actiondisplay.NewDisplayVariablePill(label, text, actionType, known))
			continue
		}
		box.Add(actiondisplay.NewDisplayValuePill(text, actionType, known))
	}
	for _, b := range bindings {
		if consumed[b.Name] {
			continue
		}
		consumed[b.Name] = true
		box.Add(actiondisplay.NewDisplayVariablePill(outputLabels[b.Name], b.Name, actionType, known))
	}
	return box, extra
}

// outputBindingLabel maps a variable binding role to a compact pill label,
// mirroring the labels used in the action tooltip popup.
func outputBindingLabel(role string) string {
	switch role {
	case "output_x":
		return "X"
	case "output_y":
		return "Y"
	case "length":
		return "Length"
	case "value":
		return "Variable"
	default:
		return "Output"
	}
}
