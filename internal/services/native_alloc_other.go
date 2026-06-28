//go:build !linux || !cgo

package services

// ConfigureNativeAllocator is a no-op on platforms without glibc malloc tuning.
func ConfigureNativeAllocator() {}

// trimNativeHeap is a no-op on platforms without glibc malloc_trim.
func trimNativeHeap() {}
