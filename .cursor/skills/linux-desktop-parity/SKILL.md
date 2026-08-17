---
name: linux-desktop-parity
description: Verify and implement GNOME, Plasma, and Cosmic desktop parity for Sqyre on Linux. Use when working on Wayland capture, portal permissions, session detection, sqyre-probe, or desktop integration failures on pure Wayland.
---

# Linux desktop parity (GNOME / Plasma / Cosmic)

## Goal

Reach **full parity tier** on the user's graphical Linux session. Do not treat `open_or_skip` tests or a clean `cargo build` as success.

## Agent loop

1. Build probe: `make probe` (or `cargo build -p sqyre-probe`).
2. Run: `./bin/sqyre-probe --json` (add `--human` for stderr summary).
3. Parse JSON: check `parity_tier`, `capabilities`, `permissions_needed`.
4. If permissions are missing, tell the user the exact DE settings path (see below) and re-run with `--wait-permissions 120`.
5. Fix the failing backend module indicated by `backend` / `error` fields.
6. Re-run until exit code `0` or document an unfixable DE limitation.

Required caps (default `--require`): `capture.open`, `capture.rect`, `windows.list`, `input.open`, `hotkeys.start`, `outline.open`, `grab.open`.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All required capabilities `ok` or `skip` |
| 1 | One or more required capabilities failed |
| 2 | Probe infrastructure error (serialize, bad args) |

## Log markers (grep `~/.sqyre/diag.log` with `SQYRE_DIAG=1`)

```
SQYRE_SESSION type=… desktop=… compositor=… backend=…
SQYRE_CAP=ok backend=x11 rect=… checksum=…
SQYRE_CAP=fail error=…
SQYRE_PORTAL=ok interface=ScreenCast
SQYRE_PORTAL=denied interface=ScreenCast
SQYRE_INPUT=ok|fail …
SQYRE_HOTKEY=ok|fail …
SQYRE_FOCUS=ok list count=…
SQYRE_PROBE parity_tier=… ms=…
```

## Permission hints by DE

| Issue | GNOME | Plasma/KDE | Cosmic |
|-------|-------|------------|--------|
| Screen recording | Settings → Privacy → Screen Recording | System Settings → Privacy & Security → Screen Recording | COSMIC Settings → Privacy → Screen Capture |
| Synthetic input | `sudo usermod -aG input $USER` then re-login | same | same |
| Global shortcuts | Portal / Settings → Keyboard | Portal / System Settings | Portal (evolving) |

Pure Wayland without XWayland: enable `portal-capture` at build time (`cargo build -p sqyre-app --features portal-capture`; needs libpipewire ≥ 1.0 dev headers — Ubuntu 24.04+). `make release` on Linux enables it automatically.

## Backend layout

```
crates/sqyre-capture/src/linux/
  session.rs          # X11 / XWayland / portal detection
  wayland/            # portal capture, foreign-toplevel, layer-shell (incremental)
crates/sqyre-probe/   # structured JSON capability probe
```

Wayland stubs report `pending` until implemented. Probe keys: `capture.wayland_impl`, `capture.wayland_portal`, `outline.wayland_impl`, `grab.wayland_impl`, `input.wayland_impl`.

## Tests

```bash
# CI-safe (no display required)
cargo test -p sqyre-probe

# Host with graphical session
cargo test -p sqyre-probe --test linux_desktop_parity -- --ignored --nocapture
```

## Do not treat as success

- `open_or_skip` passing in headless CI
- App launch without `platform_warning` on XWayland-only (hybrid, not native Wayland)
- File dialogs or tray working (portal/DBus — unrelated to capture parity)
- `libwayland-dev` present in devcontainer (link dep only)

## Implementation order

1. Session detection + probe (done)
2. Portal + PipeWire capture (`portal_capture_implemented()` → true)
3. Foreign-toplevel window focus
4. uinput input backend
5. Portal GlobalShortcuts hotkeys
6. wlr-layer-shell outline + grab
7. Optional KWin D-Bus fast paths for Plasma

After each step, re-run `./bin/sqyre-probe --json` and confirm the relevant capability moves from `pending`/`fail` to `ok`.
