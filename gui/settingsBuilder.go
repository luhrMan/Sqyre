package gui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

var sequenceName = widget.NewEntry()
var sequenceLoops = widget.NewSlider(1, 10)

var sequenceBuilderForm = widget.Form{
	Items: []*widget.FormItem{
		{Text: "", Widget: widget.NewLabelWithStyle("Sequence Settings", fyne.TextAlignCenter, fyne.TextStyle{Bold: true})},
		{Text: "Name", Widget: sequenceName},
		{Text: "Loops", Widget: sequenceLoops},
	},

	OnSubmit: func() {
		sequence.Name = sequenceName.Text
		sequence.Loops = int(sequenceLoops.Value)
		macro.Sequences = append(macro.Sequences, sequence)
		sequenceList.Refresh()
	},
}

var actionBuilder = container.NewVBox(
	widget.NewLabelWithStyle("Action Settings", fyne.TextAlignCenter, fyne.TextStyle{Bold: true}),
	actionSelector,
	&mouseMoveSettingsForm,
	&clickSettingsForm,

	//&searchSettingsForm,
	//&ocrSettingsForm,
	//&repeaterSettingsForm,

)
