#!/usr/bin/env bash
# Save the current Rust JSON report as a named baseline for later diffs.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${BENCH_COMPARE_OUT:-$ROOT/target/bench-compare}"
NAME="${1:-${BENCH_BASELINE:-default}}"
mkdir -p "$OUT_DIR/baselines"

if [[ ! -f "$OUT_DIR/rust-latest.json" ]]; then
  echo "No $OUT_DIR/rust-latest.json — running Rust harness first…"
  "$ROOT/scripts/bench-compare/run-rust.sh"
fi

dest="$OUT_DIR/baselines/${NAME}.json"
cp -f "$OUT_DIR/rust-latest.json" "$dest"
# Stamp metadata sidecar
meta="$OUT_DIR/baselines/${NAME}.meta.json"
python3 - <<PY
import json, time
from pathlib import Path
rep = json.loads(Path("$dest").read_text())
meta = {
  "name": "$NAME",
  "saved_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
  "git_rev": rep.get("git_rev"),
  "git_describe": rep.get("git_describe"),
  "impl_name": rep.get("impl_name"),
}
Path("$meta").write_text(json.dumps(meta, indent=2) + "\n")
print(f"Saved baseline '{meta['name']}'  rev={meta['git_describe']} → $dest")
PY
