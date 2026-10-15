#!/usr/bin/env bash
# Verify cargo-sources.json is in sync with Cargo.lock (no network needed).
#
# Every crates.io package in the lockfile must have a matching archive entry
# (cargo/vendor/<name>-<version>) whose sha256 equals the lockfile checksum,
# and the file must not carry entries for packages the lockfile no longer has.
# On drift, regenerate with generate-cargo-sources.sh and commit the result.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$SCRIPT_DIR/../../../lib/repo-root.sh"

python3 - "$REPO_ROOT/Cargo.lock" "$SCRIPT_DIR/cargo-sources.json" <<'PY'
import json
import re
import sys

lock_path, sources_path = sys.argv[1], sys.argv[2]

# Parse [[package]] blocks (Cargo.lock is TOML, but a regex keeps this dep-free).
lock_pkgs = {}  # "name-version" -> sha256
name = version = checksum = source = None
block = re.compile(
    r'\[\[package\]\]\s*'
    r'name = "([^"]+)"\s*'
    r'version = "([^"]+)"\s*'
    r'source = "([^"]+)"\s*'
    r'checksum = "([0-9a-f]{64})"'
)
for m in block.finditer(open(lock_path, encoding='utf-8').read()):
    name, version, source, checksum = m.groups()
    if source.startswith('registry+'):
        lock_pkgs[f'{name}-{version}'] = checksum

sources = json.load(open(sources_path, encoding='utf-8'))
src_pkgs = {}  # "name-version" -> sha256
for entry in sources:
    if entry.get('type') != 'archive':
        continue
    dest = entry.get('dest', '')
    prefix = 'cargo/vendor/'
    if dest.startswith(prefix):
        src_pkgs[dest[len(prefix):]] = entry.get('sha256', '')

errors = []
for pkg, want in sorted(lock_pkgs.items()):
    got = src_pkgs.get(pkg)
    if got is None:
        errors.append(f'missing archive entry for {pkg}')
    elif got != want:
        errors.append(f'sha256 mismatch for {pkg}: lock={want} sources={got}')
for pkg in sorted(set(src_pkgs) - set(lock_pkgs)):
    errors.append(f'stale archive entry not in Cargo.lock: {pkg}')

if errors:
    print('cargo-sources.json is out of sync with Cargo.lock:', file=sys.stderr)
    for e in errors[:20]:
        print(f'  - {e}', file=sys.stderr)
    if len(errors) > 20:
        print(f'  ... and {len(errors) - 20} more', file=sys.stderr)
    print('Fix: scripts/linux/packaging/flatpak/generate-cargo-sources.sh, then commit.',
          file=sys.stderr)
    sys.exit(1)

print(f'cargo-sources.json OK: {len(src_pkgs)} crates match Cargo.lock')
PY
