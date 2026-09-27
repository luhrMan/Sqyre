// Minimal stub so historical Sqyre packages that only need *gocv.Mat
// (e.g. models.Program masks) can compile without a matching OpenCV.
// Vision/match benches require real gocv + compatible OpenCV (see docs).
package gocv

type Mat struct{}

func NewMat() Mat { return Mat{} }

func (m *Mat) Close() {}

func (m *Mat) Empty() bool { return true }
