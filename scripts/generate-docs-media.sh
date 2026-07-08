#!/usr/bin/env bash
# Regenerate README screenshots and demo GIF from automated UI tests.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export SQYRE_UI_TEST=1
export SQYRE_NO_HOOK=1
export SQYRE_UPDATE_SCREENSHOTS=1
export GOFLAGS="-tags=gocv_specific_modules,nohook -buildvcs=false"

mkdir -p docs/images/frames

# Headless Fyne test driver (no xvfb): matches ./scripts/test.sh screenshot checks in CI.
env -u DISPLAY go test -v ./ui/ -run 'TestDocsScreenshots|TestDemoWorkflowFrames'

if command -v ffmpeg >/dev/null 2>&1; then
  ffmpeg -y -framerate 1 -i docs/images/frames/demo-macro-%03d.png \
    -vf "scale=800:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse" \
    docs/images/demo-macro.gif
  echo "Wrote docs/images/demo-macro.gif"
else
  echo "ffmpeg not installed; PNG frames updated in docs/images/ (install ffmpeg for GIF)" >&2
fi

echo "Done. Commit docs/images/ and update README embeds if needed."
