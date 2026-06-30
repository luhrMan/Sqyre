//go:build linux && cgo

package vision

/*
#include <malloc.h>
*/
import "C"

// ConfigureNativeAllocator constrains glibc's malloc so heavy multithreaded
// native allocation (OpenCV template matching, OCR, screen capture) does not
// permanently inflate RSS.
//
// Closing a gocv.Mat or freeing OCR buffers returns the memory to glibc, not to
// the OS. By default glibc creates up to 8*NCPU per-thread arenas, each of which
// retains freed memory independently and is effectively never returned. Capping
// the arena count keeps that fragmentation bounded.
//
// Must be called once at startup, before any native worker threads are created.
func ConfigureNativeAllocator() {
	// Limit the number of malloc arenas to bound per-thread retention.
	C.mallopt(C.M_ARENA_MAX, 2)
	// Pin the trim threshold instead of letting glibc grow it dynamically; the
	// dynamic growth is what makes RSS ratchet up and stay high after big frees.
	C.mallopt(C.M_TRIM_THRESHOLD, 128*1024)
}

// trimNativeHeap asks glibc to return free heap memory to the OS. glibc retains
// freed buffers by default; malloc_trim forces the release that makes RSS drop
// after a macro frees its OpenCV/OCR buffers.
func trimNativeHeap() {
	C.malloc_trim(0)
}

func TrimNativeHeap() { trimNativeHeap() }
