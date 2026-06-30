package macro

import (
	"sync"
	"time"

	"Sqyre/internal/services"

	"fyne.io/fyne/v2"
)

// highlightPumpInterval is how often the UI thread drains pending highlight
// updates during macro execution. Coalescing still applies, but this caps lag
// when the Fyne queue is busy with log rendering or tree layout.
const highlightPumpInterval = 100 * time.Millisecond

// highlightPump coalesces cursor highlight updates on the UI thread so a backlog
// of fyne.Do callbacks cannot paint stale actions after execution has moved on.
type highlightPump struct {
	mui *MacroUi

	mu            sync.Mutex
	pendingCursor *services.HighlightEvent
	cursorQueued  bool
	pendingFills  map[string]services.HighlightEvent
	fillQueued    bool

	tickerStop chan struct{}
}

func newHighlightPump(mui *MacroUi) *highlightPump {
	return &highlightPump{mui: mui}
}

func (p *highlightPump) startTicker() {
	p.stopTicker()
	stop := make(chan struct{})
	p.tickerStop = stop
	go func() {
		t := time.NewTicker(highlightPumpInterval)
		defer t.Stop()
		for {
			select {
			case <-stop:
				fyne.Do(p.flushAll)
				return
			case <-t.C:
				fyne.Do(p.flushAll)
			}
		}
	}()
}

func (p *highlightPump) stopTicker() {
	if p.tickerStop != nil {
		close(p.tickerStop)
		p.tickerStop = nil
	}
}

func (p *highlightPump) handle(ev services.HighlightEvent) {
	if ev.Kind == services.HighlightSimple {
		evCopy := ev
		p.mu.Lock()
		p.pendingCursor = &evCopy
		if !p.cursorQueued {
			p.cursorQueued = true
			p.mu.Unlock()
			fyne.Do(p.flushCursor)
			return
		}
		p.mu.Unlock()
		return
	}

	if ev.Kind == services.HighlightFill {
		evCopy := ev
		key := ev.MacroName + "\x00" + ev.UID
		p.mu.Lock()
		if p.pendingFills == nil {
			p.pendingFills = map[string]services.HighlightEvent{}
		}
		p.pendingFills[key] = evCopy
		if !p.fillQueued {
			p.fillQueued = true
			p.mu.Unlock()
			fyne.Do(p.flushFills)
			return
		}
		p.mu.Unlock()
		return
	}

	if ev.Kind == services.HighlightNone && ev.UID == "" {
		p.mu.Lock()
		p.pendingCursor = nil
		p.pendingFills = nil
		p.mu.Unlock()
	}

	fyne.Do(func() {
		p.apply(ev)
	})
}

func (p *highlightPump) flushAll() {
	p.flushCursor()
	p.flushFills()
}

func (p *highlightPump) flushCursor() {
	for {
		p.mu.Lock()
		ev := p.pendingCursor
		if ev == nil {
			p.cursorQueued = false
			p.mu.Unlock()
			return
		}
		p.pendingCursor = nil
		p.mu.Unlock()

		p.apply(*ev)

		p.mu.Lock()
		if p.pendingCursor == nil {
			p.cursorQueued = false
			p.mu.Unlock()
			return
		}
		p.mu.Unlock()
	}
}

func (p *highlightPump) flushFills() {
	for {
		p.mu.Lock()
		if len(p.pendingFills) == 0 {
			p.fillQueued = false
			p.mu.Unlock()
			return
		}
		batch := p.pendingFills
		p.pendingFills = map[string]services.HighlightEvent{}
		p.mu.Unlock()

		for _, ev := range batch {
			p.apply(ev)
		}

		p.mu.Lock()
		if len(p.pendingFills) == 0 {
			p.fillQueued = false
			p.mu.Unlock()
			return
		}
		p.mu.Unlock()
	}
}

func (p *highlightPump) apply(ev services.HighlightEvent) {
	if ev.Kind == services.HighlightNone && ev.UID == "" {
		for _, tree := range p.mui.MTabs.AllTrees() {
			tree.ClearAllHighlights()
		}
		return
	}
	tree := p.mui.MTabs.TreeForMacro(ev.MacroName)
	if tree == nil {
		return
	}
	switch ev.Kind {
	case services.HighlightSimple:
		tree.SetCursor(ev.UID)
	case services.HighlightFill:
		tree.SetFill(ev.UID, ev.Fill)
	case services.HighlightNone:
		tree.ClearHighlight(ev.UID)
	}
}
