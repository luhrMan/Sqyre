package services


// RunOnMainThread executes a function on the GUI main thread.
// Set to fyne.Do by the UI layer at startup; defaults to synchronous execution for tests.
var RunOnMainThread = func(fn func()) { fn() }

// RunOnMainThreadAndWait executes a function on the GUI main thread and blocks until complete.
// Set to fyne.DoAndWait by the UI layer at startup; defaults to synchronous execution for tests.
var RunOnMainThreadAndWait = func(fn func()) { fn() }

// ProgressReporter abstracts a progress bar widget.
type ProgressReporter interface {
	SetValue(float64)
	Show()
	Hide()
}

// ActivityReporter abstracts an activity indicator widget.
type ActivityReporter interface {
	Show()
	Hide()
	Start()
	Stop()
}

type noopProgress struct{}

func (noopProgress) SetValue(float64) {}
func (noopProgress) Show()            {}
func (noopProgress) Hide()            {}

type noopActivity struct{}

func (noopActivity) Show()  {}
func (noopActivity) Hide()  {}
func (noopActivity) Start() {}
func (noopActivity) Stop()  {}

var (
	progressReporter ProgressReporter = noopProgress{}
	activityReporter ActivityReporter = noopActivity{}
)

func SetProgressReporter(p ProgressReporter)  { progressReporter = p }
func SetActivityReporter(a ActivityReporter)   { activityReporter = a }
func GetProgressReporter() ProgressReporter    { return progressReporter }
func GetActivityReporter() ActivityReporter    { return activityReporter }

