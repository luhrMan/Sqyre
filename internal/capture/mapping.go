package capture

import (
	"Sqyre/internal/screen"
	"fmt"
	"image"

	"github.com/vcaesar/screenshot"
)

func resolveScreenshotMapping(specs []monitorSpec) (map[int]int, error) {
	mapping := make(map[int]int, len(specs))
	used := make(map[int]bool, screenshot.NumActiveDisplays())
	for _, spec := range specs {
		ssIndex, ok := screenshotIndexForDesktop(spec.bounds)
		if !ok {
			return nil, fmt.Errorf("no screenshot monitor matches desktop bounds %v", spec.bounds)
		}
		if used[ssIndex] {
			return nil, fmt.Errorf("screenshot monitor %d matched multiple desktop monitors", ssIndex)
		}
		used[ssIndex] = true
		mapping[spec.displayIndex] = ssIndex
	}
	return mapping, nil
}

func screenshotIndexForDesktop(desktop image.Rectangle) (int, bool) {
	for i := 0; i < screenshot.NumActiveDisplays(); i++ {
		if screen.ScreenshotDisplayBoundsAbs(i) == desktop {
			return i, true
		}
	}
	return 0, false
}

func applyScreenshotMapping(plan *SessionPlan, mapping map[int]int) {
	for i := range plan.Monitors {
		mon := &plan.Monitors[i]
		ssIndex, ok := mapping[mon.DisplayIndex]
		if !ok {
			continue
		}
		mon.BackendDisplayIndex = ssIndex
		mon.BackendBounds = screenshot.GetDisplayBounds(ssIndex)
	}
}

func validateScreenshotAlignment(specs []monitorSpec, mapping map[int]int, boundsFor func(int) image.Rectangle) error {
	for _, spec := range specs {
		ssIndex, ok := mapping[spec.displayIndex]
		if !ok {
			return fmt.Errorf("desktop display %d missing screenshot mapping", spec.displayIndex)
		}
		ssAbs := boundsFor(ssIndex)
		if ssAbs != spec.bounds {
			return fmt.Errorf(
				"desktop display %d bounds=%v screenshot index %d abs=%v",
				spec.displayIndex, spec.bounds, ssIndex, ssAbs,
			)
		}
	}
	return nil
}
