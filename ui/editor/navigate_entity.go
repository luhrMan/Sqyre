package editor

import (
	"fmt"
	"slices"

	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
	"Sqyre/internal/models/repositories"

	"fyne.io/fyne/v2/widget"
)

// NavigateToCoordinateEntity selects the Points or Search Areas tab and focuses ref.
func NavigateToCoordinateEntity(ref actions.CoordinateRef, isPoint bool) error {
	if ref.IsEmpty() {
		return fmt.Errorf("coordinate reference is empty")
	}
	if !IsBuilt() {
		return fmt.Errorf("data editor is not built")
	}

	var cfg entityAccordionConfig
	et := shell().EditorTabs
	if isPoint {
		cfg = pointsAccordionConfig()
		et.Select(et.PointsTab.TabItem)
	} else {
		cfg = searchAreasAccordionConfig()
		et.Select(et.SearchAreasTab.TabItem)
	}
	shell().RefreshEditorActionBar()

	program, entityKey, err := resolveCoordinateRefProgram(cfg.getKeys, ref)
	if err != nil {
		return err
	}

	acc, ok := cfg.tab.Widgets["Accordion"].(*widget.Accordion)
	if !ok {
		return fmt.Errorf("entity accordion not found")
	}
	if sb, ok := cfg.tab.Widgets["searchbar"].(*widget.Entry); ok && sb.Text != "" {
		sb.SetText("")
		populateProgramEntityAccordion(acc, cfg)
	}

	rowIdx := entityAccordionRowIndex(acc, program.Name)
	if rowIdx < 0 {
		return fmt.Errorf("program %q not found in data editor", program.Name)
	}
	acc.Open(rowIdx)

	listKey := program.Name + "-list"
	list, ok := cfg.tab.Widgets[listKey].(*widget.List)
	if !ok {
		return fmt.Errorf("entity list for program %q not found", program.Name)
	}

	filtered := filterKeysByFuzzy("", cfg.getKeys(program))
	cfg.sortKeys(program, filtered)
	listIdx := slices.Index(filtered, entityKey)
	if listIdx < 0 {
		return fmt.Errorf("entity %q not found in program %q", entityKey, program.Name)
	}

	if cfg.tab.ProgramSelector != nil {
		cfg.tab.ProgramSelector.SetSelected(program.Name)
	}
	list.Select(widget.ListItemID(listIdx))
	return nil
}

func resolveCoordinateRefProgram(getKeys func(*models.Program) []string, ref actions.CoordinateRef) (*models.Program, string, error) {
	name := ref.Name()
	if progName := ref.Program(); progName != "" {
		program, err := repositories.ProgramRepo().Get(progName)
		if err != nil {
			return nil, "", fmt.Errorf("load program %q: %w", progName, err)
		}
		if !slices.Contains(getKeys(program), name) {
			return nil, "", fmt.Errorf("entity %q not found in program %q", name, progName)
		}
		return program, name, nil
	}
	for _, program := range repositories.ProgramRepo().GetAllSortedByName() {
		if slices.Contains(getKeys(program), name) {
			return program, name, nil
		}
	}
	return nil, "", fmt.Errorf("entity %q not found", name)
}
