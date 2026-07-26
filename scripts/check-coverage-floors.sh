#!/usr/bin/env bash
# Instrument pure crates with llvm-cov and fail when line coverage drops below floors.
# OS-specific crates (sqyre-capture, etc.) are intentionally excluded.
set -euo pipefail

_here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/repo-root.sh
. "$_here/lib/repo-root.sh"
cd "$REPO_ROOT"

CARGO="${CARGO:-cargo}"
CONFIG="${COVERAGE_FLOORS_CONFIG:-$_here/coverage-floors.json}"

if ! "$CARGO" llvm-cov --version >/dev/null 2>&1; then
	echo "cargo-llvm-cov not found. Install with:" >&2
	echo "  rustup component add llvm-tools-preview" >&2
	echo "  cargo install cargo-llvm-cov --locked" >&2
	exit 1
fi

if [[ ! -f "$CONFIG" ]]; then
	echo "coverage floor config not found: $CONFIG" >&2
	exit 1
fi

REPORT_JSON="$(mktemp)"
trap 'rm -f "$REPORT_JSON"' EXIT

echo "Running llvm-cov for pure crates (sqyre-domain, sqyre-varref, sqyre-persist, sqyre-executor)..."
"$CARGO" llvm-cov \
	-p sqyre-domain \
	-p sqyre-varref \
	-p sqyre-persist \
	-p sqyre-executor \
	--json --summary-only --output-path "$REPORT_JSON" -q

python3 - "$REPORT_JSON" "$CONFIG" <<'PY'
import json
import sys


def line_pct(files, path_substr: str):
    covered = total = 0
    for entry in files:
        if path_substr in entry["filename"]:
            summary = entry["summary"]["lines"]
            covered += summary["covered"]
            total += summary["count"]
    if total == 0:
        return None, 0, 0
    return 100.0 * covered / total, covered, total


def main() -> int:
    report_path, config_path = sys.argv[1:3]
    with open(report_path, encoding="utf-8") as fh:
        report = json.load(fh)
    with open(config_path, encoding="utf-8") as fh:
        config = json.load(fh)

    files = report["data"][0]["files"]
    failed = False

    print("Coverage floors (line %):")
    for target in config["targets"]:
        name = target["name"]
        path = target["path"]
        floor = float(target["floor"])
        pct, covered, total = line_pct(files, path)
        if pct is None:
            print(f"  FAIL {name}: no lines matched path {path!r}")
            failed = True
            continue
        ok = pct >= floor
        status = "ok" if ok else "FAIL"
        if not ok:
            failed = True
        print(
            f"  {status:4} {name}: {pct:5.1f}% ({covered}/{total}) — floor {floor:.1f}%"
        )

    if failed:
        print("One or more coverage floors were not met.", file=sys.stderr)
        return 1

    print("All coverage floors met.")
    return 0


raise SystemExit(main())
PY
