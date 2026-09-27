#!/usr/bin/env bash
# Run the Rust comparative harness → target/bench-compare/rust-latest.json
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${BENCH_COMPARE_OUT:-$ROOT/target/bench-compare}"
mkdir -p "$OUT_DIR"
cd "$ROOT"

ITER="${BENCH_ITERATIONS:-40}"
ISOLATE="${BENCH_ISOLATE:-1}"
EXTRA=()
if [[ "$ISOLATE" == "1" ]]; then
  EXTRA+=(--isolate)
fi

echo "Building sqyre-bench-compare (release)…"
cargo build -p sqyre-bench-compare --release

BIN="$ROOT/target/release/sqyre-bench-compare"
"$BIN" --json --iterations "$ITER" "${EXTRA[@]}" \
  --fixture-db "$ROOT/scripts/bench-compare/fixtures/db.yaml" \
  | tee "$OUT_DIR/rust-latest.json" >/dev/null

# Also print human summary
"$BIN" --iterations "$ITER" "${EXTRA[@]}" \
  --fixture-db "$ROOT/scripts/bench-compare/fixtures/db.yaml" \
  | tee "$OUT_DIR/rust-latest.txt"

echo "Wrote $OUT_DIR/rust-latest.json"
