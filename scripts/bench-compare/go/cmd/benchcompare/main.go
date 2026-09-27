// Comparative harness for the historical Go Sqyre tree.
// Overlayed into .cache/go-sqyre/cmd/benchcompare by run-go.sh.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
)

const schemaVersion = 1

type hostInfo struct {
	OS   string `json:"os"`
	Arch string `json:"arch"`
	CPUs int    `json:"cpus"`
}

type sectionResult struct {
	Name          string  `json:"name"`
	Status        string  `json:"status"`
	SkipReason    *string `json:"skip_reason,omitempty"`
	Iterations    uint64  `json:"iterations"`
	WallNsTotal   uint64  `json:"wall_ns_total"`
	WallNsPerIter uint64  `json:"wall_ns_per_iter"`
	CPUUserNs     uint64  `json:"cpu_user_ns"`
	CPUSysNs      uint64  `json:"cpu_sys_ns"`
	PeakRSSKb     uint64  `json:"peak_rss_kb"`
	RSSKb         uint64  `json:"rss_kb"`
	IOReadBytes   uint64  `json:"io_read_bytes"`
	IOWriteBytes  uint64  `json:"io_write_bytes"`
	Notes         *string `json:"notes,omitempty"`
}

type report struct {
	SchemaVersion     int             `json:"schema_version"`
	ImplName          string          `json:"impl_name"`
	GitRev            string          `json:"git_rev"`
	GitDescribe       string          `json:"git_describe"`
	Host              hostInfo        `json:"host"`
	IterationsDefault uint64          `json:"iterations_default"`
	FixtureDB         string          `json:"fixture_db"`
	Sections          []sectionResult `json:"sections"`
}

var allSections = []string{
	"match_direct",
	"match_fft",
	"match_multi_variant",
	"search_prep",
	"find_pixels",
	"ocr_preprocess",
	"persist_yaml_load",
	"persist_yaml_save",
	"macro_codec_encode",
	"macro_codec_decode",
	"zpixmap_swizzle",
}

func main() {
	log.SetOutput(io.Discard)
	jsonOut := flag.Bool("json", false, "emit JSON report")
	list := flag.Bool("list", false, "list section names")
	iterations := flag.Uint64("iterations", 40, "loops per section")
	fixture := flag.String("fixture-db", "", "db.yaml for persist sections")
	sectionFlags := multiFlag{}
	flag.Var(&sectionFlags, "section", "section name (repeatable)")
	flag.Parse()

	if *list {
		for _, s := range allSections {
			fmt.Println(s)
		}
		return
	}

	sections := sectionFlags
	if len(sections) == 0 {
		sections = append([]string{}, allSections...)
	}
	if *fixture == "" {
		fmt.Fprintln(os.Stderr, "benchcompare: --fixture-db is required")
		os.Exit(2)
	}

	results := make([]sectionResult, 0, len(sections))
	for _, name := range sections {
		results = append(results, runSection(name, *iterations, *fixture))
	}

	rep := report{
		SchemaVersion:     schemaVersion,
		ImplName:          "go",
		GitRev:            gitOutput("rev-parse", "HEAD"),
		GitDescribe:       gitOutput("describe", "--always", "--dirty"),
		Host:              hostInfo{OS: runtime.GOOS, Arch: runtime.GOARCH, CPUs: runtime.NumCPU()},
		IterationsDefault: *iterations,
		FixtureDB:         *fixture,
		Sections:          results,
	}

	if *jsonOut {
		enc := json.NewEncoder(os.Stdout)
		enc.SetIndent("", "  ")
		_ = enc.Encode(rep)
		return
	}
	printHuman(rep)
}

type multiFlag []string

func (m *multiFlag) String() string { return strings.Join(*m, ",") }
func (m *multiFlag) Set(v string) error {
	*m = append(*m, v)
	return nil
}

func runSection(name string, iterations uint64, fixture string) sectionResult {
	switch name {
	case "persist_yaml_load":
		return benchPersistLoad(iterations, fixture)
	case "persist_yaml_save":
		return benchPersistSave(iterations, fixture)
	case "macro_codec_encode":
		return benchMacroEncode(iterations)
	case "macro_codec_decode":
		return benchMacroDecode(iterations)
	case "match_direct", "match_fft", "match_multi_variant", "search_prep",
		"find_pixels", "ocr_preprocess":
		return runGocvSection(name, iterations)
	case "zpixmap_swizzle":
		reason := "no Go equivalent of Rust zpixmap_to_rgb (X11 capture used third-party screenshot)"
		return sectionResult{Name: name, Status: "skipped", SkipReason: &reason}
	default:
		reason := fmt.Sprintf("unknown section %s", name)
		return sectionResult{Name: name, Status: "error", SkipReason: &reason}
	}
}

func skipped(name, reason string) sectionResult {
	r := reason
	return sectionResult{Name: name, Status: "skipped", SkipReason: &r}
}

func failed(name, reason string) sectionResult {
	r := reason
	return sectionResult{Name: name, Status: "error", SkipReason: &r}
}

func note(s string) *string { return &s }

func gitOutput(args ...string) string {
	cmd := exec.Command("git", args...)
	out, err := cmd.Output()
	if err != nil {
		return "unknown"
	}
	return strings.TrimSpace(string(out))
}

func printHuman(rep report) {
	fmt.Printf("sqyre-bench-compare  impl=%s  rev=%s  cpus=%d\n",
		rep.ImplName, rep.GitDescribe, rep.Host.CPUs)
	fmt.Printf("%-24s %10s %12s %12s %10s %12s\n",
		"section", "status", "wall/iter", "cpu_user", "rss_kb", "io_rw")
	for _, s := range rep.Sections {
		fmt.Printf("%-24s %10s %12s %12s %10d %6d+%d\n",
			s.Name, s.Status, formatNs(s.WallNsPerIter), formatNs(s.CPUUserNs),
			s.PeakRSSKb, s.IOReadBytes, s.IOWriteBytes)
		if s.SkipReason != nil {
			fmt.Printf("  ↳ %s\n", *s.SkipReason)
		}
	}
}

func formatNs(ns uint64) string {
	switch {
	case ns >= 1_000_000_000:
		return fmt.Sprintf("%.2fs", float64(ns)/1e9)
	case ns >= 1_000_000:
		return fmt.Sprintf("%.2fms", float64(ns)/1e6)
	case ns >= 1_000:
		return fmt.Sprintf("%.1fµs", float64(ns)/1e3)
	default:
		return fmt.Sprintf("%dns", ns)
	}
}

func timed(name string, iterations uint64, notes string, body func() error) sectionResult {
	start := sampleNow()
	for i := uint64(0); i < iterations; i++ {
		if err := body(); err != nil {
			return failed(name, err.Error())
		}
	}
	end := sampleNow()
	wall := uint64(end.wall.Sub(start.wall))
	per := uint64(0)
	if iterations > 0 {
		per = wall / iterations
	}
	var n *string
	if notes != "" {
		n = &notes
	}
	return sectionResult{
		Name:          name,
		Status:        "ok",
		Iterations:    iterations,
		WallNsTotal:   wall,
		WallNsPerIter: per,
		CPUUserNs:     uint64(end.cpuUser - start.cpuUser),
		CPUSysNs:      uint64(end.cpuSys - start.cpuSys),
		PeakRSSKb:     end.hwmKb,
		RSSKb:         end.rssKb,
		IOReadBytes:   end.ioRead - start.ioRead,
		IOWriteBytes:  end.ioWrite - start.ioWrite,
		Notes:         n,
	}
}

func absFixture(path string) string {
	if filepath.IsAbs(path) {
		return path
	}
	wd, _ := os.Getwd()
	return filepath.Join(wd, path)
}

func ensureFile(path string) error {
	st, err := os.Stat(path)
	if err != nil {
		return err
	}
	if st.IsDir() {
		return fmt.Errorf("%s is a directory", path)
	}
	return nil
}
