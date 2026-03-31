package appdata

import (
	"cmp"
	"errors"
	"fmt"
	"slices"
	"sync"

	"Sqyre/internal/config"
	"Sqyre/internal/models"
	"Sqyre/internal/models/actions"
)

// Memory holds in-memory programs and macros for WASM demos and tests.
type Memory struct {
	mu       sync.RWMutex
	programs map[string]*models.Program
	macros   map[string]*models.Macro
}

// NewMemory returns empty in-memory stores.
func NewMemory() *Memory {
	return &Memory{
		programs: make(map[string]*models.Program),
		macros:   make(map[string]*models.Macro),
	}
}

// demoProgramTheme names WASM sample programs and their nested entities like real games/apps.
type demoProgramTheme struct {
	name        string
	itemTag     string
	items       []string
	points      []string
	searchAreas []string
	masks       []string
}

// demoProgramThemes has one theme per seeded program (items / points / regions / masks align by index).
var demoProgramThemes = []demoProgramTheme{
	{
		name:    "Eldoria Online",
		itemTag: "mmorpg",
		items: []string{
			"Health potion stack", "Iron ore bundle", "Quest log tab", "Raid frame slot", "Minimap expand", "Action bar 7",
		},
		points: []string{
			"NPC merchant interact", "Dungeon entrance ping", "Spellbook toggle", "Flight master icon", "Guild roster tab", "Accept resurrection",
		},
		searchAreas: []string{
			"Bag inventory grid", "Party chat window", "Minimap frame", "Quest tracker panel", "Buff bar row", "Encounter journal",
		},
		masks: []string{
			"Target frame portrait", "Cast bar fill", "Combo point pips", "Proc glow flash", "Raid ready icon", "Tooltip anchor",
		},
	},
	{
		name:    "Hex Dominion",
		itemTag: "strategy",
		items: []string{
			"Worker chip", "Siege workshop slot", "Tech tier badge", "Radar blip", "Alliance banner", "Resource ticker",
		},
		points: []string{
			"Command center", "Build grid cell", "Menu — Multiplayer", "Pause overlay resume", "Voice chat push-to-talk", "End turn button",
		},
		searchAreas: []string{
			"Build palette dock", "Unit selection ring", "Mission briefing panel", "Top resource bar", "Minimap corner", "Combat log strip",
		},
		masks: []string{
			"Selection halo", "Ping marker", "Health bar strip", "Upgrade chevron", "Rally flag", "Fog of war edge",
		},
	},
	{
		name:    "PixelSmith Studio",
		itemTag: "creative",
		items: []string{
			"Adjustment layer", "Brush preset small", "Swatch group cool", "Smart object thumb", "Export preset WebP", "History snapshot",
		},
		points: []string{
			"Eyedropper sample", "Transform handle SE", "Layer opacity slider", "New artboard", "Filter gallery OK", "Ruler origin",
		},
		searchAreas: []string{
			"Layers panel stack", "Tool options bar", "Histogram panel", "Navigator preview", "Save for Web dialog", "Color spectrum strip",
		},
		masks: []string{
			"Marquee feather edge", "Vignette ellipse", "Watermark corner", "Lens flare core", "Gradient overlay bar", "Clipping mask thumb",
		},
	},
	{
		name:    "Nimbus Mail",
		itemTag: "productivity",
		items: []string{
			"Unread thread row", "Calendar invite chip", "Signature block", "Promotions label", "PDF attachment icon", "Snooze menu item",
		},
		points: []string{
			"Compose floating button", "Search field clear", "Archive toolbar", "Star thread toggle", "Reply all", "Sidebar collapse",
		},
		searchAreas: []string{
			"Inbox message list", "Reading pane body", "Folder sidebar", "Meeting scheduler grid", "Quick settings drawer", "People picker popover",
		},
		masks: []string{
			"Unread dot badge", "Priority flag strip", "Avatar circle crop", "Thread count bubble", "Inline image thumb", "Meeting join pill",
		},
	},
	{
		name:    "Gem Stack",
		itemTag: "puzzle",
		items: []string{
			"Move counter gem", "Booster hammer charge", "Daily streak flame", "Leaderboard badge", "Reward video chest", "Level star row",
		},
		points: []string{
			"Play next level", "Shop cart FAB", "Lives hearts row", "Pause settings gear", "Continue after ad", "Home map node",
		},
		searchAreas: []string{
			"Puzzle board grid", "Booster tray", "Score header bar", "Victory banner", "Settings scroll panel", "Daily challenge strip",
		},
		masks: []string{
			"Gem match template", "Explosion burst core", "Progress bar fill", "Coin fly-out trail", "Streak multiplier text", "Ad close button",
		},
	},
}

func keyTap(key string) []actions.ActionInterface {
	return []actions.ActionInterface{
		actions.NewKey(key, true),
		actions.NewKey(key, false),
	}
}

// keyChord presses modifiers in order, taps key, releases modifiers in reverse order (e.g. Ctrl+Shift+E).
func keyChord(modifiers []string, key string) []actions.ActionInterface {
	var out []actions.ActionInterface
	for _, m := range modifiers {
		out = append(out, actions.NewKey(m, true))
	}
	out = append(out, actions.NewKey(key, true), actions.NewKey(key, false))
	for i := len(modifiers) - 1; i >= 0; i-- {
		out = append(out, actions.NewKey(modifiers[i], false))
	}
	return out
}

// SeedDemo adds sample data for browser exploration.
func (m *Memory) SeedDemo() {
	m.mu.Lock()
	defer m.mu.Unlock()

	const (
		macroCount = 7
		perNested  = 6 // items, points, search areas, masks per program (each repo 3–10)
	)

	for i := range demoProgramThemes {
		theme := demoProgramThemes[i]
		p := models.NewProgram()
		p.Name = theme.name
		seedProgramFromTheme(p, i+1, perNested, theme)
		m.programs[theme.name] = p
	}

	macroDefs := []struct {
		name    string
		delay   int
		hotkey  []string
		actions []actions.ActionInterface
	}{
		{name: "Eldoria — open bags", delay: 0, hotkey: []string{"ctrl", "shift", "d"}, actions: demoMacroEldoriaOpenBags()},
		{name: "Hex — repeat last build", delay: 25, hotkey: []string{"f5"}, actions: demoMacroHexRepeatBuild()},
		{name: "PixelSmith — export slice", delay: 50, hotkey: []string{"ctrl", "1"}, actions: demoMacroPixelSmithExport()},
		{name: "Nimbus — focus inbox", delay: 100, hotkey: []string{}, actions: demoMacroNimbusInbox()},
		{name: "Gem — claim daily", delay: 150, hotkey: []string{"alt", "q"}, actions: demoMacroGemDaily()},
		{name: "Studio — undo chain", delay: 200, hotkey: []string{}, actions: demoMacroStudioUndo()},
		{name: "Mail — quick archive", delay: 75, hotkey: []string{"ctrl", "space"}, actions: demoMacroMailArchive()},
	}
	for i := 0; i < macroCount && i < len(macroDefs); i++ {
		d := macroDefs[i]
		macro := models.NewMacro(d.name, d.delay, d.hotkey)
		macro.Root.SetSubActions(d.actions)
		m.macros[d.name] = macro
	}
}

// seedProgramFromTheme fills Items, default-resolution Points/SearchAreas, and Masks.
// Maps are updated in place so SeedDemo never calls repository Set (would deadlock with Memory.mu).
func seedProgramFromTheme(p *models.Program, programIdx, n int, theme demoProgramTheme) {
	resKey := config.MainMonitorSizeString
	coords := p.Coordinates[resKey]
	if coords == nil {
		return
	}
	for i := 0; i < n; i++ {
		if i >= len(theme.items) {
			break
		}
		itemName := theme.items[i]
		p.Items[itemName] = &models.Item{
			Name:     itemName,
			GridSize: [2]int{1 + (i+programIdx)%3, 1 + i%2},
			Tags:     []string{theme.itemTag, fmt.Sprintf("group-%d", i%3+1)},
			StackMax: 1 + i%5,
			Mask:     "",
		}
	}
	for i := 0; i < n; i++ {
		if i >= len(theme.points) {
			break
		}
		ptName := theme.points[i]
		coords.Points[ptName] = &models.Point{
			Name: ptName,
			X:    100 + (i+1)*80 + programIdx*10,
			Y:    200 + (i+1)*60,
		}
	}
	for i := 0; i < n; i++ {
		if i >= len(theme.searchAreas) {
			break
		}
		saName := theme.searchAreas[i]
		j := i + 1
		coords.SearchAreas[saName] = &models.SearchArea{
			Name:    saName,
			LeftX:   10 + j*5,
			TopY:    20 + j*5,
			RightX:  800 - j*10,
			BottomY: 600 - j*10,
		}
	}
	for i := 0; i < n; i++ {
		if i >= len(theme.masks) {
			break
		}
		maskName := theme.masks[i]
		j := i + 1
		shape := "rectangle"
		if j%2 == 0 {
			shape = "circle"
		}
		p.Masks[maskName] = &models.Mask{
			Name:    maskName,
			Shape:   shape,
			CenterX: "50%",
			CenterY: "50%",
			Base:    fmt.Sprintf("%d", 20+j*5),
			Height:  fmt.Sprintf("%d", 20+j*5),
			Radius:  fmt.Sprintf("%d", 15+j*3),
			Inverse: j%3 == 0,
		}
	}
}

func errNotFoundProgram(name string) error {
	return fmt.Errorf("program not found: %s", name)
}

func errNotFoundMacro(name string) error {
	return fmt.Errorf("macro not found: %s", name)
}

// MemoryPrograms adapts Memory to ProgramStore.
type MemoryPrograms struct{ M *Memory }

func (s MemoryPrograms) Get(name string) (*models.Program, error) {
	s.M.mu.RLock()
	defer s.M.mu.RUnlock()
	p, ok := s.M.programs[name]
	if !ok {
		return nil, errNotFoundProgram(name)
	}
	return p, nil
}

func (s MemoryPrograms) Set(name string, p *models.Program) error {
	if p == nil {
		return errors.New("program cannot be nil")
	}
	s.M.mu.Lock()
	defer s.M.mu.Unlock()
	if bm, ok := any(p).(models.BaseModel); ok {
		bm.SetKey(name)
	}
	s.M.programs[p.GetKey()] = p
	return nil
}

func (s MemoryPrograms) Delete(name string) error {
	s.M.mu.Lock()
	defer s.M.mu.Unlock()
	delete(s.M.programs, name)
	return nil
}

func (s MemoryPrograms) GetAll() map[string]*models.Program {
	s.M.mu.RLock()
	defer s.M.mu.RUnlock()
	out := make(map[string]*models.Program, len(s.M.programs))
	for k, v := range s.M.programs {
		out[k] = v
	}
	return out
}

func (s MemoryPrograms) GetAllKeys() []string {
	s.M.mu.RLock()
	defer s.M.mu.RUnlock()
	keys := make([]string, 0, len(s.M.programs))
	for k := range s.M.programs {
		keys = append(keys, k)
	}
	slices.Sort(keys)
	return keys
}

func (s MemoryPrograms) New() *models.Program {
	return models.NewProgram()
}

func (s MemoryPrograms) GetAllSortedByName() []*models.Program {
	all := s.GetAll()
	out := make([]*models.Program, 0, len(all))
	for _, p := range all {
		out = append(out, p)
	}
	slices.SortFunc(out, func(a, b *models.Program) int {
		return cmp.Compare(a.Name, b.Name)
	})
	return out
}

// MemoryMacros adapts Memory to MacroStore.
type MemoryMacros struct{ M *Memory }

func (s MemoryMacros) Get(name string) (*models.Macro, error) {
	s.M.mu.RLock()
	defer s.M.mu.RUnlock()
	x, ok := s.M.macros[name]
	if !ok {
		return nil, errNotFoundMacro(name)
	}
	return x, nil
}

func (s MemoryMacros) Set(name string, m *models.Macro) error {
	if m == nil {
		return errors.New("macro cannot be nil")
	}
	s.M.mu.Lock()
	defer s.M.mu.Unlock()
	if bm, ok := any(m).(models.BaseModel); ok {
		bm.SetKey(name)
	}
	s.M.macros[m.GetKey()] = m
	return nil
}

func (s MemoryMacros) Delete(name string) error {
	s.M.mu.Lock()
	defer s.M.mu.Unlock()
	delete(s.M.macros, name)
	return nil
}

func (s MemoryMacros) GetAll() map[string]*models.Macro {
	s.M.mu.RLock()
	defer s.M.mu.RUnlock()
	out := make(map[string]*models.Macro, len(s.M.macros))
	for k, v := range s.M.macros {
		out[k] = v
	}
	return out
}

func (s MemoryMacros) GetAllKeys() []string {
	s.M.mu.RLock()
	defer s.M.mu.RUnlock()
	keys := make([]string, 0, len(s.M.macros))
	for k := range s.M.macros {
		keys = append(keys, k)
	}
	slices.Sort(keys)
	return keys
}

func (s MemoryMacros) New() *models.Macro {
	return models.NewMacro("", 0, []string{})
}
