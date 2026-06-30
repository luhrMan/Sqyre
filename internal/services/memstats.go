package services

import (
	"fmt"
	"log"
	"os"
	"runtime"
	"runtime/debug"
	"strconv"
	"strings"
	"time"

	"Sqyre/internal/vision"
)

// ProcessRSSBytes returns the resident set size (RSS) of this process in bytes,
// i.e. how much physical RAM the OS reports the process as using. Returns 0 when
// unavailable (non-Linux platforms or a read failure).
func ProcessRSSBytes() uint64 {
	if runtime.GOOS != "linux" {
		return 0
	}
	data, err := os.ReadFile("/proc/self/status")
	if err != nil {
		return 0
	}
	for _, line := range strings.Split(string(data), "\n") {
		if !strings.HasPrefix(line, "VmRSS:") {
			continue
		}
		fields := strings.Fields(line)
		if len(fields) >= 2 {
			if kb, err := strconv.ParseUint(fields[1], 10, 64); err == nil {
				return kb * 1024
			}
		}
	}
	return 0
}

func memMB(b uint64) string {
	return fmt.Sprintf("%.1f MB", float64(b)/(1024*1024))
}

// LogMemoryUsage logs Go runtime memory stats alongside the process RSS so the
// gap between Go-managed memory (heap) and total resident memory (native/CGO/GPU
// allocations the Go GC cannot reclaim) is visible in sqyre.log.
//
// Reading the result:
//   - If sys is close to rss, almost all memory is Go-runtime-managed; heapIdle
//     minus heapReleased shows pages the scavenger is still holding.
//   - If sys is far below rss, the difference is native memory (OpenCV/gocv,
//     OpenGL textures, X11) that GC and FreeOSMemory cannot return.
func LogMemoryUsage(tag string) {
	var m runtime.MemStats
	runtime.ReadMemStats(&m)
	rss := ProcessRSSBytes()
	rssStr := memMB(rss)
	if rss == 0 {
		rssStr = "n/a"
	}
	log.Printf("MEM[%s] rss=%s | go: heapInuse=%s heapIdle=%s heapReleased=%s heapSys=%s stackSys=%s otherSys=%s sys=%s | numGC=%d",
		tag, rssStr,
		memMB(m.HeapInuse), memMB(m.HeapIdle), memMB(m.HeapReleased), memMB(m.HeapSys),
		memMB(m.StackSys), memMB(m.OtherSys), memMB(m.Sys), m.NumGC,
	)
}

// scheduleMemoryReclaim returns memory to the OS after a macro run. It runs off
// the UI thread and waits briefly so the queued Fyne teardown (highlight clear,
// tree collapse, log pump stop) finishes allocating/freeing first; reclaiming
// before that work runs leaves its allocation spike resident until the next GC.
//
// Two GC + FreeOSMemory passes are used: the first lets finalizers (e.g. Fyne GL
// resources) run, the second reclaims what those finalizers freed.
func scheduleMemoryReclaim(tag string) {
	go func() {
		time.Sleep(1500 * time.Millisecond)
		LogMemoryUsage("post-macro/before-reclaim:" + tag)
		runtime.GC()
		debug.FreeOSMemory()
		time.Sleep(100 * time.Millisecond)
		runtime.GC()
		debug.FreeOSMemory()
		// Return native (glibc) free memory to the OS. The Go GC above only
		// reclaims the Go heap; OpenCV/OCR buffers live in the C heap, which
		// glibc retains until malloc_trim forces a release.
		vision.TrimNativeHeap()
		LogMemoryUsage("post-macro/after-reclaim:" + tag)
	}()
}
