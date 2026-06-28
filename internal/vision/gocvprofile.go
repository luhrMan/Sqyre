//go:build !matprofile

package vision

// LogMatProfile is a no-op unless built with -tags matprofile.
// See README for building with matprofile (gocv Mat leak profiling).
func LogMatProfile() {}
