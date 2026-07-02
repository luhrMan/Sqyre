package capture

import "image"

type BackendKind string

const (
	BackendRobotgoMonitorRect BackendKind = "robotgo-monitor-rect"
	BackendRobotgoVirtual     BackendKind = "robotgo-virtual"
	BackendScreenshotDisplay  BackendKind = "screenshot-display"
	BackendScreenshotRect     BackendKind = "screenshot-rect"
)

type MonitorPlan struct {
	DisplayIndex        int
	BackendDisplayIndex int
	DesktopBounds       image.Rectangle
	BackendBounds       image.Rectangle
}

func monitorByIndex(monitors []MonitorPlan, displayIndex int) (MonitorPlan, bool) {
	for _, m := range monitors {
		if m.DisplayIndex == displayIndex {
			return m, true
		}
	}
	return MonitorPlan{}, false
}

type ProbeReport struct {
	Mode                string
	Enabled             bool
	BackendResults      []BackendProbeResult
	SelectedBackend     BackendKind
	SelectedBackendNote string
}

type BackendProbeResult struct {
	Backend  BackendKind
	Passed   bool
	Reason   string
	Monitors []MonitorProbeResult
}

type MonitorProbeResult struct {
	DisplayIndex int
	Expected     image.Rectangle
	Captured     image.Rectangle
	Passed       bool
	Reason       string
	Score        float64
}

type SessionPlan struct {
	Backend        BackendKind
	VirtualDesktop image.Rectangle
	Monitors       []MonitorPlan
}
