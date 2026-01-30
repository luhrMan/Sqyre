package actions

import (
	"fmt"
	"os"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/theme"
)

type DataList struct {
	*BaseAction `yaml:",inline" mapstructure:",squash"`
	Source      string // File path or manual text
	OutputVar   string // Variable to store current line
	IsFile      bool   // True if Source is a file path, false if manual text
	currentLine int    // Current line index (not serialized)
	lines       []string // Cached lines (not serialized)
}

func NewDataList(source string, outputVar string, isFile bool) *DataList {
	return &DataList{
		BaseAction: newBaseAction("datalist"),
		Source:     source,
		OutputVar:  outputVar,
		IsFile:     isFile,
		currentLine: 0,
		lines:       []string{},
	}
}

// GetCurrentLine returns the current line from the data list
func (a *DataList) GetCurrentLine() (string, error) {
	if len(a.lines) == 0 {
		if err := a.loadLines(); err != nil {
			return "", err
		}
	}

	if a.currentLine >= len(a.lines) {
		return "", fmt.Errorf("line index %d out of range (total: %d)", a.currentLine, len(a.lines))
	}

	return a.lines[a.currentLine], nil
}

// NextLine advances to the next line
func (a *DataList) NextLine() {
	a.currentLine++
	if a.currentLine >= len(a.lines) {
		a.currentLine = 0 // Wrap around
	}
}

// Reset resets to the first line
func (a *DataList) Reset() {
	a.currentLine = 0
}

// loadLines loads lines from source (file or manual text)
func (a *DataList) loadLines() error {
	if a.IsFile {
		data, err := os.ReadFile(a.Source)
		if err != nil {
			return fmt.Errorf("failed to read file %s: %w", a.Source, err)
		}
		a.lines = strings.Split(string(data), "\n")
		// Remove empty lines at the end
		for len(a.lines) > 0 && strings.TrimSpace(a.lines[len(a.lines)-1]) == "" {
			a.lines = a.lines[:len(a.lines)-1]
		}
	} else {
		a.lines = strings.Split(a.Source, "\n")
		// Remove empty lines at the end
		for len(a.lines) > 0 && strings.TrimSpace(a.lines[len(a.lines)-1]) == "" {
			a.lines = a.lines[:len(a.lines)-1]
		}
	}
	return nil
}

func (a *DataList) String() string {
	if a.IsFile {
		return fmt.Sprintf("DataList from file: %s -> %s", a.Source, a.OutputVar)
	}
	return fmt.Sprintf("DataList: %d lines -> %s", len(a.lines), a.OutputVar)
}

func (a *DataList) Icon() fyne.Resource {
	return theme.FileTextIcon()
}
