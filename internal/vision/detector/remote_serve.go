package detector

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os/exec"
	"strings"
	"sync"
)

type remoteWorker struct {
	path string

	mu     sync.Mutex
	cmd    *exec.Cmd
	stdin  io.WriteCloser
	stdout *bufio.Reader
	stderr bytes.Buffer
}

var (
	remoteWorkersMu sync.Mutex
	remoteWorkers   = map[string]*remoteWorker{}
)

func getRemoteWorker(path string) *remoteWorker {
	remoteWorkersMu.Lock()
	defer remoteWorkersMu.Unlock()
	if w, ok := remoteWorkers[path]; ok {
		return w
	}
	w := &remoteWorker{path: path}
	remoteWorkers[path] = w
	return w
}

func resetRemoteWorkers() {
	resetCachedRemoteDetector()
	warmUpMu.Lock()
	warmUpPath = ""
	warmUpMu.Unlock()
	remoteWorkersMu.Lock()
	workers := remoteWorkers
	remoteWorkers = map[string]*remoteWorker{}
	remoteWorkersMu.Unlock()
	for _, w := range workers {
		w.close()
	}
}

// StartWorker launches (or revives) a persistent sqyre-vision serve process and preloads models.
func StartWorker(workerPath string) error {
	if workerPath == "" {
		return fmt.Errorf("vision worker not configured")
	}
	return getRemoteWorker(workerPath).ensureRunning(nil)
}

func (w *remoteWorker) running() bool {
	w.mu.Lock()
	defer w.mu.Unlock()
	return w.cmd != nil && w.cmd.Process != nil && w.cmd.ProcessState == nil
}

func (w *remoteWorker) ensureRunning(ctx context.Context) error {
	w.mu.Lock()
	defer w.mu.Unlock()
	if w.cmd != nil && w.cmd.Process != nil && w.cmd.ProcessState == nil {
		return nil
	}
	return w.startLocked(ctx)
}

func (w *remoteWorker) startLocked(ctx context.Context) error {
	w.closeLocked()

	cmd := workerCommand(ctx, w.path, "serve")
	cmd.Env = workerEnv()
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return fmt.Errorf("worker stdin: %w", err)
	}
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		stdin.Close()
		return fmt.Errorf("worker stdout: %w", err)
	}
	w.stderr.Reset()
	cmd.Stderr = &w.stderr
	if err := cmd.Start(); err != nil {
		stdin.Close()
		stdout.Close()
		return fmt.Errorf("start sqyre-vision serve: %w", err)
	}

	readyReader := bufio.NewReader(stdout)
	var ready WorkerReady
	if err := json.NewDecoder(readyReader).Decode(&ready); err != nil {
		_ = cmd.Process.Kill()
		stdin.Close()
		stdout.Close()
		msg := w.stderr.String()
		if msg != "" {
			return fmt.Errorf("worker ready: %w: %s", err, strings.TrimSpace(msg))
		}
		return fmt.Errorf("worker ready: %w", err)
	}
	if !ready.Ready {
		_ = cmd.Process.Kill()
		stdin.Close()
		stdout.Close()
		return fmt.Errorf("sqyre-vision serve did not become ready")
	}

	w.cmd = cmd
	w.stdin = stdin
	w.stdout = readyReader
	return nil
}

func (w *remoteWorker) detect(ctx context.Context, req WorkerRequest) (WorkerResponse, error) {
	if err := w.ensureRunning(ctx); err != nil {
		return WorkerResponse{}, err
	}

	w.mu.Lock()
	defer w.mu.Unlock()

	if w.stdin == nil || w.stdout == nil {
		return WorkerResponse{}, fmt.Errorf("vision worker not running")
	}

	enc := json.NewEncoder(w.stdin)
	if err := enc.Encode(req); err != nil {
		w.closeLocked()
		return WorkerResponse{}, fmt.Errorf("write worker request: %w", err)
	}

	var resp WorkerResponse
	if err := json.NewDecoder(w.stdout).Decode(&resp); err != nil {
		w.closeLocked()
		msg := w.stderr.String()
		if msg != "" {
			return WorkerResponse{}, fmt.Errorf("read worker response: %w: %s", err, strings.TrimSpace(msg))
		}
		return WorkerResponse{}, fmt.Errorf("read worker response: %w", err)
	}
	if resp.Error != "" {
		return resp, fmt.Errorf("%s", resp.Error)
	}
	return resp, nil
}

func (w *remoteWorker) close() {
	w.mu.Lock()
	defer w.mu.Unlock()
	w.closeLocked()
}

func (w *remoteWorker) closeLocked() {
	if w.stdin != nil {
		_ = w.stdin.Close()
		w.stdin = nil
	}
	if w.cmd != nil && w.cmd.Process != nil {
		_ = w.cmd.Process.Kill()
	}
	w.cmd = nil
	w.stdout = nil
}
