#!/usr/bin/env bash
# Run Rust + Go harnesses and print a side-by-side summary.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${BENCH_COMPARE_OUT:-$ROOT/target/bench-compare}"
mkdir -p "$OUT_DIR"

"$ROOT/scripts/bench-compare/run-rust.sh"
GO_OK=1
if ! "$ROOT/scripts/bench-compare/run-go.sh"; then
  GO_OK=0
  echo "Go harness failed — Rust results are in $OUT_DIR/rust-latest.json" >&2
fi

if [[ "$GO_OK" -eq 1 && -f "$OUT_DIR/go-latest.json" ]]; then
  python3 "$ROOT/scripts/bench-compare/compare.py" \
    --label-left go --label-right rust \
    "$OUT_DIR/go-latest.json" "$OUT_DIR/rust-latest.json" \
    | tee "$OUT_DIR/go-vs-rust.txt"
else
  echo "Skipping Go↔Rust table (Go results unavailable)."
fi
