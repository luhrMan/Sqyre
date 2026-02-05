package actions

import (
	"fmt"
	"os"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

type SaveVariable struct {
	*BaseAction   `yaml:",inline" mapstructure:",squash"`
	VariableName  string
	Destination   string // File path or "clipboard"
	Append        bool   // Append to file if true, overwrite if false
	AppendNewline bool   // When Append is true, write a newline before each value
}

func NewSaveVariable(varName string, destination string, append bool, appendNewline bool) *SaveVariable {
	return &SaveVariable{
		BaseAction:    newBaseAction("savevariable"),
		VariableName:  varName,
		Destination:   destination,
		Append:        append,
		AppendNewline: appendNewline,
	}
}

func (a *SaveVariable) String() string {
	if a.Destination == "clipboard" {
		return fmt.Sprintf("Save %s to clipboard", a.VariableName)
	}
	if a.Append {
		return fmt.Sprintf("Append %s to %s", a.VariableName, a.Destination)
	}
	return fmt.Sprintf("Save %s to %s", a.VariableName, a.Destination)
}

func (a *SaveVariable) Icon() fyne.Resource {
	return theme.DocumentSaveIcon()
}

// SaveToFile saves the variable value to a file
func (a *SaveVariable) SaveToFile(value string, filePath string) error {
	var file *os.File
	var err error

	if a.Append {
		file, err = os.OpenFile(filePath, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	} else {
		file, err = os.Create(filePath)
	}

	if err != nil {
		return fmt.Errorf("failed to open file %s: %w", filePath, err)
	}
	defer file.Close()

	if a.Append && a.AppendNewline {
		value = "\n" + value
	}
	_, err = file.WriteString(value)
	if err != nil {
		return fmt.Errorf("failed to write to file %s: %w", filePath, err)
	}

	return nil
}
