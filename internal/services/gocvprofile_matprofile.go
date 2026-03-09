//go:build matprofile

package services

import (
	"bytes"
	"fmt"
	"log"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"

	"Sqyre/internal/config"

	_ "net/http/pprof"

	"gocv.io/x/gocv"
)

var matProfileLogFile *os.File

// LogMatProfile logs the current gocv Mat profile count and stack traces.
func LogMatProfile() {
	n := gocv.MatProfile.Count()
	log.Printf("gocv MatProfile count: %d", n)
	if n > 0 {
		var b bytes.Buffer
		gocv.MatProfile.WriteTo(&b, 1)
		log.Print(b.String())
	}
}

func init() {
	sqyreDir := config.GetSqyreDir()
	if err := os.MkdirAll(sqyreDir, 0755); err != nil {
		return
	}
	logPath := filepath.Join(sqyreDir, "sqyre.log")
	f, err := os.OpenFile(logPath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		return
	}
	matProfileLogFile = f
	log.SetOutput(&SyncWriter{F: f})
	log.SetFlags(log.LstdFlags)

	addr := strings.TrimSpace(os.Getenv("SQYRE_PPROF"))
	if addr == "0" || strings.EqualFold(addr, "off") {
		log.Printf("matprofile: pprof disabled by SQYRE_PPROF")
		return
	}
	if addr == "" {
		addr = "127.0.0.1:6060"
	}
	addr = strings.Replace(addr, "localhost", "127.0.0.1", 1)
	if !strings.Contains(addr, ":") {
		addr = "127.0.0.1:" + addr
	}
	pprofAddr := addr

	go func() {
		var ln net.Listener
		var err error
		if pprofAddr == "127.0.0.1:6060" {
			for port := 6060; port <= 6065; port++ {
				tryAddr := "127.0.0.1:" + fmt.Sprintf("%d", port)
				ln, err = net.Listen("tcp", tryAddr)
				if err == nil {
					pprofAddr = tryAddr
					break
				}
			}
		} else {
			ln, err = net.Listen("tcp", pprofAddr)
		}
		if err != nil {
			log.Printf("matprofile: pprof listen failed: %v", err)
			return
		}
		log.Printf("matprofile: pprof at http://%s/debug/pprof/ (gocv Mat profile: .../gocv.io/x/gocv.Mat)", ln.Addr().String())
		if matProfileLogFile != nil {
			_ = matProfileLogFile.Sync()
		}
		if err := http.Serve(ln, nil); err != nil {
			log.Printf("matprofile: pprof server error: %v", err)
		}
	}()

	go func() {
		time.Sleep(2 * time.Second)
		log.Printf("matprofile: MatProfile count %d", gocv.MatProfile.Count())
		if matProfileLogFile != nil {
			_ = matProfileLogFile.Sync()
		}
	}()
}
