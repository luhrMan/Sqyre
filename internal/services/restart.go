package services

import "sync/atomic"

var restartRequested atomic.Bool

// RequestRestart marks that Sqyre should relaunch itself once it has shut down.
// The actual relaunch happens in the app entry point after the single-instance
// lock is released (see RelaunchExecutable).
func RequestRestart() { restartRequested.Store(true) }

// RestartRequested reports whether a relaunch was requested this session.
func RestartRequested() bool { return restartRequested.Load() }

// RelaunchExecutable restarts Sqyre. On Unix it re-execs in place (same PID) so
// an active AppImage's FUSE mount stays valid and no orphan runtime process is
// left behind; on Windows it spawns a fresh detached process. Callers must
// release the single-instance lock first so the new process can acquire it.
func RelaunchExecutable() error { return relaunchExec() }
