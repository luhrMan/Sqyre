//go:build !linux || !cgo

package vision

// ConfigureNativeAllocator is a no-op on platforms without glibc malloc tuning.
func ConfigureNativeAllocator() {}

func TrimNativeHeap() {}
