#!/usr/bin/env bash
# Fail when pure-crate line coverage drops below floors in coverage-floors.json.
# OS-specific crates (sqyre-capture, etc.) are intentionally excluded.
#
# Test code is stripped before measuring: files under tests/ dirs or named
# tests.rs / *_tests.rs / test_*.rs are dropped, and lines inside #[cfg(test)]
# items in the remaining files are ignored. Without this, test modules report
# themselves as covered and inflate every floor.
#
# Default: run a small instrumented llvm-cov pass over pure crates.
# Set COVERAGE_REPORT_LCOV to a prior `llvm-cov report --lcov` file (e.g.
# target/coverage/lcov.info from `make coverage`) to skip re-instrumenting.
set -euo pipefail

_here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/repo-root.sh
. "$_here/lib/repo-root.sh"
cd "$REPO_ROOT"

CARGO="${CARGO:-cargo}"
CONFIG="${COVERAGE_FLOORS_CONFIG:-$_here/coverage-floors.json}"

if [[ ! -f "$CONFIG" ]]; then
	echo "coverage floor config not found: $CONFIG" >&2
	exit 1
fi

REPORT_LCOV=""
CLEANUP_REPORT=0
if [[ -n "${COVERAGE_REPORT_LCOV:-}" ]]; then
	if [[ ! -f "$COVERAGE_REPORT_LCOV" ]]; then
		echo "COVERAGE_REPORT_LCOV not found: $COVERAGE_REPORT_LCOV" >&2
		exit 1
	fi
	REPORT_LCOV="$COVERAGE_REPORT_LCOV"
	echo "Reusing coverage report: $REPORT_LCOV"
else
	if ! "$CARGO" llvm-cov --version >/dev/null 2>&1; then
		echo "cargo-llvm-cov not found. Install with:" >&2
		echo "  rustup component add llvm-tools-preview" >&2
		echo "  cargo install cargo-llvm-cov --locked" >&2
		exit 1
	fi
	REPORT_LCOV="$(mktemp)"
	CLEANUP_REPORT=1
	trap '[[ "$CLEANUP_REPORT" -eq 1 ]] && rm -f "$REPORT_LCOV"' EXIT
	echo "Running llvm-cov for pure crates (sqyre-domain, sqyre-varref, sqyre-persist, sqyre-serialize, sqyre-validate, sqyre-executor)..."
	"$CARGO" llvm-cov \
		-p sqyre-domain \
		-p sqyre-varref \
		-p sqyre-persist \
		-p sqyre-serialize \
		-p sqyre-validate \
		-p sqyre-executor \
		--lcov --output-path "$REPORT_LCOV" -q
fi

python3 - "$REPORT_LCOV" "$CONFIG" <<'PY'
import json
import re
import sys

# Test-only files: anything under a tests/ dir, or basenames like tests.rs,
# *_tests.rs, test_*.rs, test_support.rs.
TEST_FILE_RE = re.compile(
    r"(?:^|/)tests?/|(?:^|/)(?:tests|test_support|[^/]*_tests|test_[^/]*)\.rs$"
)
# `#[cfg(test)]` or `#[cfg(all(test, ...))]` (but not cfg(not(test))).
CFG_TEST_RE = re.compile(r"#\s*\[\s*cfg\s*\(\s*(?:all\s*\(\s*)?test\b")


def test_region_lines(path):
    """1-based lines inside #[cfg(test)] items (attribute applies to the next
    item; extent found by naive brace matching — test items are balanced)."""
    try:
        with open(path, encoding="utf-8") as fh:
            lines = fh.read().splitlines()
    except OSError:
        return set()
    marked = set()
    i, n = 0, len(lines)
    while i < n:
        if not CFG_TEST_RE.search(lines[i]):
            i += 1
            continue
        depth = 0
        seen_open = False
        end = i
        while end < n:
            for ch in lines[end]:
                if ch == "{":
                    depth += 1
                    seen_open = True
                elif ch == "}":
                    depth -= 1
            # `#[cfg(test)] use foo;` — item without a block ends at `;`.
            if not seen_open and ";" in lines[end]:
                break
            if seen_open and depth == 0:
                break
            end += 1
        marked.update(range(i + 1, end + 2))
        i = end + 1
    return marked


def covered_lines(lcov_path):
    """Yield (filename, {line: hits}) per SF: record."""
    current = None
    lines = {}
    with open(lcov_path, encoding="utf-8") as fh:
        for raw in fh:
            raw = raw.strip()
            if raw.startswith("SF:"):
                if current is not None:
                    yield current, lines
                current = raw[3:]
                lines = {}
            elif raw.startswith("DA:"):
                lineno, hits = raw[3:].split(",")[:2]
                lines[int(lineno)] = int(hits)
    if current is not None:
        yield current, lines


def main() -> int:
    lcov_path, config_path = sys.argv[1:3]
    with open(config_path, encoding="utf-8") as fh:
        config = json.load(fh)

    per_file = []
    for filename, lines in covered_lines(lcov_path):
        if TEST_FILE_RE.search(filename):
            continue
        excluded = test_region_lines(filename)
        covered = sum(1 for ln, hits in lines.items() if hits > 0 and ln not in excluded)
        total = sum(1 for ln in lines if ln not in excluded)
        per_file.append((filename, covered, total))

    failed = False
    print("Coverage floors (line %, #[cfg(test)] code excluded):")
    for target in config["targets"]:
        name = target["name"]
        path = target["path"]
        floor = float(target["floor"])
        covered = total = 0
        for filename, fc, ft in per_file:
            if path in filename:
                covered += fc
                total += ft
        if total == 0:
            print(f"  FAIL {name}: no lines matched path {path!r}")
            failed = True
            continue
        pct = 100.0 * covered / total
        ok = pct >= floor
        if not ok:
            failed = True
        print(
            f"  {'ok' if ok else 'FAIL':4} {name}: {pct:5.1f}% ({covered}/{total}) — floor {floor:.1f}%"
        )

    if failed:
        print("One or more coverage floors were not met.", file=sys.stderr)
        return 1

    print("All coverage floors met.")
    return 0


raise SystemExit(main())
PY
