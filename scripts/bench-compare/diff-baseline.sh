#!/usr/bin/env bash
# Re-run Rust harness and diff against a saved baseline.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${BENCH_COMPARE_OUT:-$ROOT/target/bench-compare}"
NAME="${1:-${BENCH_BASELINE:-default}}"
BASE="$OUT_DIR/baselines/${NAME}.json"

if [[ ! -f "$BASE" ]]; then
  echo "Baseline not found: $BASE" >&2
  echo "Create one with: make bench-baseline-save BENCH_BASELINE=$NAME" >&2
  exit 1
fi

"$ROOT/scripts/bench-compare/run-rust.sh"
python3 "$ROOT/scripts/bench-compare/compare.py" \
  --label-left "baseline($NAME)" \
  --label-right current \
  --fail-on-regress \
  "$BASE" "$OUT_DIR/rust-latest.json" \
  | tee "$OUT_DIR/baseline-diff-${NAME}.txt"
