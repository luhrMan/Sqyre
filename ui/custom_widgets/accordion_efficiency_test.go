package custom_widgets

import (
	"fmt"
	"testing"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/test"
	"fyne.io/fyne/v2/widget"
)

func accordionWithNRows(t *testing.T, n int) *AccordionWithHeaderWidgets {
	t.Helper()
	acc := NewAccordionWithHeaderWidgets()
	for i := 0; i < n; i++ {
		acc.Append(widget.NewAccordionItem(fmt.Sprintf("Program %d (1)", i), widget.NewLabel("detail")))
	}
	acc.Resize(fyne.NewSize(400, 800))
	return acc
}

func mountAccordion(t *testing.T, acc *AccordionWithHeaderWidgets) {
	t.Helper()
	test.NewApp()
	w := test.NewWindow(acc)
	t.Cleanup(w.Close)
}

// TestAccordionToggleUsesIncrementalRenderer verifies expand/collapse does not run
// updateObjects (full header rebuild) after the initial population.
func TestAccordionToggleUsesIncrementalRenderer(t *testing.T) {
	const rows = 25
	acc := accordionWithNRows(t, rows)
	mountAccordion(t, acc)

	full, inc := acc.RenderStats()
	if full != rows {
		t.Fatalf("initial full syncs = %d, want %d (one per Append)", full, rows)
	}
	acc.ResetRenderStats()

	acc.Open(0)
	full, inc = acc.RenderStats()
	if full != 0 {
		t.Fatalf("Open: full syncs = %d, want 0 (incremental path)", full)
	}
	if inc != 1 {
		t.Fatalf("Open: incremental toggles = %d, want 1", inc)
	}

	acc.ResetRenderStats()
	acc.Close(0)
	full, inc = acc.RenderStats()
	if full != 0 {
		t.Fatalf("Close: full syncs = %d, want 0", full)
	}
	if inc != 1 {
		t.Fatalf("Close: incremental toggles = %d, want 1", inc)
	}
}

// TestAccordionToggleReusesHeaderButtons verifies header widgets are not recreated on toggle.
func TestAccordionToggleReusesHeaderButtons(t *testing.T) {
	acc := accordionWithNRows(t, 10)
	mountAccordion(t, acc)

	r := acc.CreateRenderer().(*accordionWithHeaderRenderer)
	headersBefore := append([]*widget.Button(nil), r.headers...)

	acc.Open(3)
	acc.Close(3)

	if len(r.headers) != len(headersBefore) {
		t.Fatalf("header count changed: %d -> %d", len(headersBefore), len(r.headers))
	}
	for i, h := range headersBefore {
		if r.headers[i] != h {
			t.Fatalf("header %d recreated on toggle", i)
		}
	}
}

// TestAccordionToggleCostScalesWeaklyWithRowCount checks that toggle time does not grow
// linearly with row count. A full updateObjects-per-toggle implementation would make
// the 100-row case ~10x slower than the 10-row case; incremental sync should be much less.
func TestAccordionToggleCostScalesWeaklyWithRowCount(t *testing.T) {
	mountAccordion(t, accordionWithNRows(t, 1)) // init app once

	ratio := benchmarkToggleCostRatio(10, 100)
	// Full O(n) header rebuild at n=100 vs n=10 typically yields ratio > 5.
	// Incremental path stays well below that.
	const maxRatio = 4.0
	if ratio > maxRatio {
		t.Fatalf("toggle cost ratio 100/10 rows = %.2fx, want <= %.1fx (suggests O(n) full rebuild per toggle)", ratio, maxRatio)
	}
	t.Logf("toggle cost ratio 100/10 rows = %.2fx (incremental path)", ratio)
}

func benchmarkToggleCostRatio(small, large int) float64 {
	smallNS := benchmarkToggleNanoseconds(small)
	largeNS := benchmarkToggleNanoseconds(large)
	if smallNS == 0 {
		return 0
	}
	return float64(largeNS) / float64(smallNS)
}

func benchmarkToggleNanoseconds(rows int) int64 {
	acc := NewAccordionWithHeaderWidgets()
	for i := 0; i < rows; i++ {
		acc.Append(widget.NewAccordionItem(fmt.Sprintf("P%d (1)", i), widget.NewLabel("d")))
	}
	acc.Resize(fyne.NewSize(400, 800))
	acc.ResetRenderStats()

	const iterations = 30
	var total int64
	for i := 0; i < iterations; i++ {
		start := time.Now()
		acc.Open(i % rows)
		acc.Close(i % rows)
		total += time.Since(start).Nanoseconds()
	}
	return total / iterations
}

func BenchmarkAccordionToggle_10Rows(b *testing.B) {
	benchmarkAccordionToggle(b, 10)
}

func BenchmarkAccordionToggle_100Rows(b *testing.B) {
	benchmarkAccordionToggle(b, 100)
}

func benchmarkAccordionToggle(b *testing.B, rows int) {
	test.NewApp()
	acc := NewAccordionWithHeaderWidgets()
	for i := 0; i < rows; i++ {
		acc.Append(widget.NewAccordionItem(fmt.Sprintf("P%d (1)", i), widget.NewLabel("d")))
	}
	acc.Resize(fyne.NewSize(400, 800))
	acc.ResetRenderStats()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		acc.Open(i % rows)
		acc.Close(i % rows)
	}
}
