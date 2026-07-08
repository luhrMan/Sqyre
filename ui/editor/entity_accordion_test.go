package editor

import (
	"testing"

	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/repositories"
	"Sqyre/ui/custom_widgets"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/widget"
)

func TestSyncProgramEntityAccordionReusesListWidgets(t *testing.T) {
	test.NewApp()
	w := test.NewWindow(nil)
	t.Cleanup(w.Close)

	repositories.ResetAllForTesting()
	t.Cleanup(repositories.ResetAllForTesting)

	program := repositories.ProgramRepo().New()
	program.Name = "Demo"
	if err := repositories.ProgramRepo().Set(program.Name, program); err != nil {
		t.Fatalf("set program: %v", err)
	}
	pointRepo, err := program.PointRepo(config.MainMonitorSizeString)
	if err != nil {
		t.Fatalf("point repo: %v", err)
	}
	for _, name := range []string{"alpha", "beta", "home"} {
		if err := pointRepo.Set(name, &models.Point{Name: name}); err != nil {
			t.Fatalf("set point %s: %v", name, err)
		}
	}

	tab := &EditorTab{
		Widgets: map[string]fyne.CanvasObject{
			"searchbar": widget.NewEntry(),
		},
		TabItem: container.NewTabItem("Points", widget.NewLabel("")),
	}
	acc := custom_widgets.NewAccordionWithHeaderWidgets()
	acc.Resize(fyne.NewSize(400, 400))
	w.SetContent(acc)
	cfg := pointsAccordionConfigForTab(tab)

	syncProgramEntityAccordion(acc, cfg)
	if tab.entityAccordionState == nil {
		t.Fatal("entity accordion state not initialized")
	}
	row, ok := tab.entityAccordionState.rows["Demo"]
	if !ok {
		t.Fatal("Demo row missing")
	}
	listBefore := row.list
	itemBefore := row.item
	countBefore := len(acc.Items)

	tab.Widgets["searchbar"].(*widget.Entry).SetText("ho")
	syncProgramEntityAccordion(acc, cfg)

	listAfter := tab.entityAccordionState.rows["Demo"].list
	itemAfter := tab.entityAccordionState.rows["Demo"].item
	if listBefore != listAfter {
		t.Fatal("entity list widget was recreated on filter change")
	}
	if itemBefore != itemAfter {
		t.Fatal("accordion item was recreated on filter change")
	}
	if len(acc.Items) != countBefore {
		t.Fatalf("accordion row count changed %d -> %d on filter tweak", countBefore, len(acc.Items))
	}
	if row.item.Title != "Demo (1)" {
		t.Fatalf("filtered title = %q, want Demo (1)", row.item.Title)
	}
}

// pointsAccordionConfigForTab builds a points accordion config without shell().
func pointsAccordionConfigForTab(tab *EditorTab) entityAccordionConfig {
	return entityAccordionConfig{
		tab: tab,
		getKeys: func(p *models.Program) []string {
			return ProgramPointRepo(p, config.MainMonitorSizeString).GetAllKeys()
		},
		sortKeys: sortPointKeysByDisplayName,
		getEntity: func(p *models.Program, key string) (string, error) {
			pt, err := ProgramPointRepo(p, config.MainMonitorSizeString).Get(key)
			if err != nil {
				return "", err
			}
			return pt.Name, nil
		},
		onSelected: func(*models.Program, string) {},
	}
}
