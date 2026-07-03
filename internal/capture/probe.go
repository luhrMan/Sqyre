package capture

import (
	"Sqyre/internal/macro"
	"Sqyre/internal/screen"
	"fmt"
	"image"
	"image/draw"
	"log"

	"github.com/vcaesar/screenshot"
)

type probeOptions struct {
	DiagnosticsMode     string
	Frames              int
	SimilarityThreshold float64
}

type monitorSpec struct {
	displayIndex int
	bounds       image.Rectangle
}

func defaultProbeOptions() probeOptions {
	return probeOptions{
		DiagnosticsMode:     diagnosticsMode(),
		Frames:              3,
		SimilarityThreshold: 0.80,
	}
}

func probeSessionPlan(opts probeOptions) (SessionPlan, error) {
	if opts.Frames <= 0 {
		opts.Frames = 3
	}
	if opts.SimilarityThreshold <= 0 {
		opts.SimilarityThreshold = 0.80
	}
	if opts.DiagnosticsMode == "" {
		opts.DiagnosticsMode = diagnosticsMode()
	}

	specs := enabledMonitorSpecs()
	virtual := screen.VirtualBounds()
	report := ProbeReport{
		Mode:    opts.DiagnosticsMode,
		Enabled: diagnosticsEnabled(opts.DiagnosticsMode),
	}

	backends := []BackendKind{
		BackendScreenshotDisplay,
		BackendRobotgoMonitorRect,
		BackendScreenshotRect,
		BackendRobotgoVirtual,
	}

	var chosenPlan SessionPlan
	chosen := false

	for _, backend := range backends {
		result, plan, ok := probeBackend(backend, specs, opts)
		report.BackendResults = append(report.BackendResults, result)
		if !ok {
			continue
		}
		chosenPlan = plan
		chosen = true
		break
	}

	if !chosen {
		plan := SessionPlan{
			Backend:        BackendRobotgoMonitorRect,
			VirtualDesktop: virtual,
			Monitors:       buildMonitorPlans(specs),
		}
		applyScreenshotMappingToPlan(&plan, specs)
		if report.Enabled {
			logProbeDiagnostics(report)
		}
		return plan, nil
	}

	chosenPlan.VirtualDesktop = virtual
	applyScreenshotMappingToPlan(&chosenPlan, specs)
	report.SelectedBackend = chosenPlan.Backend
	report.SelectedBackendNote = "selected by deterministic probe order"
	if report.Enabled {
		logProbeDiagnostics(report)
	} else {
		log.Printf("overlay capture probe: backend=%s monitors=%d", chosenPlan.Backend, len(chosenPlan.Monitors))
	}
	return chosenPlan, nil
}

func probeBackend(kind BackendKind, specs []monitorSpec, opts probeOptions) (BackendProbeResult, SessionPlan, bool) {
	result := BackendProbeResult{Backend: kind}
	plan := SessionPlan{Backend: kind, Monitors: buildMonitorPlans(specs)}
	if len(specs) == 0 {
		result.Passed = false
		result.Reason = "no monitors"
		return result, plan, false
	}

	var screenshotMapping map[int]int
	mapping, err := resolveScreenshotMapping(specs)
	if err != nil {
		result.Passed = false
		result.Reason = err.Error()
		return result, plan, false
	}
	screenshotMapping = mapping

	if kind == BackendScreenshotDisplay || kind == BackendScreenshotRect {
		if err := validateScreenshotAlignment(specs, screenshotMapping); err != nil {
			result.Passed = false
			result.Reason = err.Error()
			return result, plan, false
		}
		applyScreenshotMapping(&plan, screenshotMapping)
	}
	if diagnosticsEnabled(opts.DiagnosticsMode) && screenshotMapping != nil {
		for _, spec := range specs {
			log.Printf(
				"overlay capture probe: desktop=%d screenshot=%d desktop_bounds=%v",
				spec.displayIndex, screenshotMapping[spec.displayIndex], spec.bounds,
			)
		}
	}

	var passingMonitors int
	for _, spec := range specs {
		mp := MonitorProbeResult{
			DisplayIndex: spec.displayIndex,
			Expected:     spec.bounds,
		}
		passFrames := 0
		var scoreSum float64
		var capturedBounds image.Rectangle
		geomMismatch := false
		for i := 0; i < opts.Frames; i++ {
			captured, reference, source, ok := captureAndReference(kind, spec, screenshotMapping)
			if !ok {
				mp.Passed = false
				mp.Reason = "capture failed"
				continue
			}
			capturedBounds = captured.Bounds()
			if capturedBounds.Dx() != spec.bounds.Dx() || capturedBounds.Dy() != spec.bounds.Dy() {
				geomMismatch = true
				mp.Reason = "geometry mismatch"
				break
			}
			if err := validateMonitorContentAlignment(captured, spec, specs, screenshotMapping); err != nil {
				geomMismatch = true
				mp.Reason = err.Error()
				break
			}
			score := imageSimilarity(captured, reference)
			scoreSum += score
			if score >= opts.SimilarityThreshold {
				passFrames++
			}
			if i == 0 {
				mp.Reason = string(source)
			}
		}
		mp.Captured = capturedBounds
		mp.Score = scoreSum / float64(maxInt(1, opts.Frames))
		if geomMismatch {
			mp.Passed = false
		} else {
			requiredPasses := min(opts.Frames, 2)
			mp.Passed = passFrames >= requiredPasses
			if !mp.Passed && mp.Reason == "" {
				mp.Reason = "similarity below threshold"
			}
		}
		if mp.Passed {
			passingMonitors++
		}
		result.Monitors = append(result.Monitors, mp)
	}

	allGeometryOk := true
	for _, m := range result.Monitors {
		if m.Captured.Empty() {
			allGeometryOk = false
			break
		}
		if m.Captured.Dx() != m.Expected.Dx() || m.Captured.Dy() != m.Expected.Dy() {
			allGeometryOk = false
			break
		}
	}
	if !allGeometryOk {
		result.Passed = false
		result.Reason = "geometry gate failed"
		return result, plan, false
	}
	if passingMonitors < len(specs) {
		result.Passed = false
		result.Reason = "not all monitors passed validation"
		return result, plan, false
	}
	result.Passed = true
	result.Reason = "validated"
	return result, plan, true
}

func captureAndReference(kind BackendKind, spec monitorSpec, screenshotMapping map[int]int) (captured image.Image, reference image.Image, source BackendKind, ok bool) {
	captured, ok = captureMonitorWithBackend(kind, spec, screenshotMapping)
	if !ok {
		return nil, nil, kind, false
	}
	reference, ok = crossReference(kind, spec, screenshotMapping)
	if !ok {
		reference, ok = independentReference(spec, screenshotMapping)
	}
	if !ok || reference == nil {
		return nil, nil, kind, false
	}
	return captured, reference, kind, true
}

func crossReference(kind BackendKind, spec monitorSpec, screenshotMapping map[int]int) (image.Image, bool) {
	switch kind {
	case BackendScreenshotDisplay, BackendScreenshotRect:
		img, err := macro.CaptureRect(spec.bounds.Min.X, spec.bounds.Min.Y, spec.bounds.Dx(), spec.bounds.Dy())
		if err != nil || img == nil {
			return nil, false
		}
		return macro.CaptureToRGBA(img), true
	case BackendRobotgoMonitorRect, BackendRobotgoVirtual:
		return captureIndependentReference(spec, screenshotMapping, nil)
	default:
		return nil, false
	}
}

func independentReference(spec monitorSpec, screenshotMapping map[int]int) (image.Image, bool) {
	if img, ok := captureIndependentReference(spec, screenshotMapping, nil); ok && img != nil {
		return img, true
	}
	full, vb, err := macro.CaptureVirtualDesktop()
	if err != nil || full == nil {
		return nil, false
	}
	ref := cropVirtualCapture(full, vb, spec.bounds)
	return ref, ref != nil
}

func captureMonitorWithBackend(kind BackendKind, spec monitorSpec, screenshotMapping map[int]int) (image.Image, bool) {
	switch kind {
	case BackendRobotgoMonitorRect:
		img, err := macro.CaptureRect(spec.bounds.Min.X, spec.bounds.Min.Y, spec.bounds.Dx(), spec.bounds.Dy())
		if err != nil || img == nil {
			return nil, false
		}
		return macro.CaptureToRGBA(img), true
	case BackendRobotgoVirtual:
		full, vb, err := macro.CaptureVirtualDesktop()
		if err != nil || full == nil {
			return nil, false
		}
		img := cropVirtualCapture(full, vb, spec.bounds)
		return img, img != nil
	case BackendScreenshotDisplay:
		ssIndex := spec.displayIndex
		if screenshotMapping != nil {
			ssIndex = screenshotMapping[spec.displayIndex]
		}
		img, err := screenshot.CaptureDisplay(ssIndex)
		if err != nil || img == nil {
			return nil, false
		}
		return macro.CaptureToRGBA(img), true
	case BackendScreenshotRect:
		ssIndex := spec.displayIndex
		if screenshotMapping != nil {
			ssIndex = screenshotMapping[spec.displayIndex]
		}
		rect := screenshot.GetDisplayBounds(ssIndex)
		img, err := screenshot.CaptureRect(rect)
		if err != nil || img == nil {
			return nil, false
		}
		return macro.CaptureToRGBA(img), true
	default:
		return nil, false
	}
}

func validateMonitorContentAlignment(captured image.Image, spec monitorSpec, specs []monitorSpec, screenshotMapping map[int]int) error {
	if screenshotMapping == nil {
		return nil
	}
	bestIdx := -1
	bestScore := -1.0
	secondScore := -1.0
	for _, other := range specs {
		ref, ok := captureIndependentReference(other, screenshotMapping, nil)
		if !ok || ref == nil {
			continue
		}
		score := imageSimilarity(captured, ref)
		if score > bestScore {
			secondScore = bestScore
			bestScore = score
			bestIdx = other.displayIndex
			continue
		}
		if score > secondScore {
			secondScore = score
		}
	}
	if bestIdx < 0 {
		return nil
	}
	if bestIdx != spec.displayIndex {
		return fmt.Errorf("content matches desktop %d not %d", bestIdx, spec.displayIndex)
	}
	if len(specs) > 1 && secondScore >= 0 && (bestScore-secondScore) < 0.05 {
		return fmt.Errorf("ambiguous content alignment score=%.3f second=%.3f", bestScore, secondScore)
	}
	return nil
}

func captureIndependentReference(spec monitorSpec, screenshotMapping map[int]int, fallback image.Image) (image.Image, bool) {
	ssIndex := spec.displayIndex
	if screenshotMapping != nil {
		ssIndex = screenshotMapping[spec.displayIndex]
	}
	if img, err := screenshot.CaptureDisplay(ssIndex); err == nil && img != nil {
		return macro.CaptureToRGBA(img), true
	}
	if rect := screenshot.GetDisplayBounds(ssIndex); !rect.Empty() {
		if img, err := screenshot.CaptureRect(rect); err == nil && img != nil {
			return macro.CaptureToRGBA(img), true
		}
	}
	if fallback != nil {
		return fallback, false
	}
	return nil, false
}

func buildMonitorPlans(specs []monitorSpec) []MonitorPlan {
	out := make([]MonitorPlan, 0, len(specs))
	for _, spec := range specs {
		out = append(out, MonitorPlan{
			DisplayIndex:        spec.displayIndex,
			BackendDisplayIndex: spec.displayIndex,
			DesktopBounds:       spec.bounds,
			BackendBounds:       spec.bounds,
		})
	}
	return out
}

func enabledMonitorSpecs() []monitorSpec {
	n := screen.NumDisplays()
	out := make([]monitorSpec, 0, n)
	for i := range n {
		b := screen.DisplayBoundsAbs(i)
		if b.Empty() || b.Dx() <= 0 || b.Dy() <= 0 {
			continue
		}
		out = append(out, monitorSpec{displayIndex: i, bounds: b})
	}
	if len(out) > 0 {
		return out
	}
	vb := screen.VirtualBounds()
	if vb.Empty() || vb.Dx() <= 0 || vb.Dy() <= 0 {
		return nil
	}
	return []monitorSpec{{displayIndex: 0, bounds: vb}}
}

func cropVirtualCapture(full image.Image, vb, target image.Rectangle) image.Image {
	region := target.Intersect(vb)
	if region.Empty() {
		return nil
	}
	local := image.Rect(region.Min.X-vb.Min.X, region.Min.Y-vb.Min.Y, region.Max.X-vb.Min.X, region.Max.Y-vb.Min.Y)
	out := image.NewRGBA(image.Rect(0, 0, local.Dx(), local.Dy()))
	draw.Draw(out, out.Bounds(), full, local.Min, draw.Src)
	return out
}

func imageSimilarity(a, b image.Image) float64 {
	if a == nil || b == nil {
		return 0
	}
	ab := a.Bounds()
	bb := b.Bounds()
	if ab.Empty() || bb.Empty() {
		return 0
	}
	ar := macro.CaptureToRGBA(a)
	br := macro.CaptureToRGBA(b)
	const sx = 24
	const sy = 24
	var total float64
	var n int
	for yi := range sy {
		ay := ab.Min.Y + (yi*(ab.Dy()-1))/maxInt(1, sy-1)
		by := bb.Min.Y + (yi*(bb.Dy()-1))/maxInt(1, sy-1)
		for xi := range sx {
			ax := ab.Min.X + (xi*(ab.Dx()-1))/maxInt(1, sx-1)
			bx := bb.Min.X + (xi*(bb.Dx()-1))/maxInt(1, sx-1)
			ac := ar.RGBAAt(ax-ab.Min.X, ay-ab.Min.Y)
			bc := br.RGBAAt(bx-bb.Min.X, by-bb.Min.Y)
			al := 0.2126*float64(ac.R) + 0.7152*float64(ac.G) + 0.0722*float64(ac.B)
			bl := 0.2126*float64(bc.R) + 0.7152*float64(bc.G) + 0.0722*float64(bc.B)
			d := al - bl
			if d < 0 {
				d = -d
			}
			total += d
			n++
		}
	}
	if n == 0 {
		return 0
	}
	avgDiff := total / float64(n)
	sim := 1.0 - (avgDiff / 255.0)
	if sim < 0 {
		return 0
	}
	if sim > 1 {
		return 1
	}
	return sim
}

func maxInt(a, b int) int {
	if a > b {
		return a
	}
	return b
}

func logProbeDiagnostics(report ProbeReport) {
	log.Printf("overlay capture probe: mode=%s", report.Mode)
	for _, r := range report.BackendResults {
		log.Printf("overlay capture probe: backend=%s passed=%v reason=%s", r.Backend, r.Passed, r.Reason)
		for _, m := range r.Monitors {
			log.Printf(
				"overlay capture probe: backend=%s display=%d expected=%v captured_size=%dx%d passed=%v score=%.3f reason=%s",
				r.Backend, m.DisplayIndex, m.Expected, m.Captured.Dx(), m.Captured.Dy(), m.Passed, m.Score, m.Reason,
			)
		}
	}
	log.Printf("overlay capture probe: selected=%s note=%s", report.SelectedBackend, report.SelectedBackendNote)
}
