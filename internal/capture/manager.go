package capture

import "sync"

var (
	planOnce sync.Once
	planMu   sync.Mutex
	plan     SessionPlan
	planErr  error
)

func SessionPlanForOverlay() (SessionPlan, error) {
	planOnce.Do(func() {
		plan, planErr = probeSessionPlan(defaultProbeOptions())
	})
	planMu.Lock()
	defer planMu.Unlock()
	return plan, planErr
}
