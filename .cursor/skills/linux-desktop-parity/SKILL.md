---
name: linux-desktop-parity
description: Verify and implement GNOME, Plasma, and Cosmic desktop parity for Sqyre on Linux. Use when working on Wayland capture, portal permissions, session detection, sqyre-probe, or desktop integration failures on pure Wayland.
---

# Linux desktop parity (GNOME / Plasma / Cosmic)

## Goal

Reach **full parity tier** on the user's graphical Linux session. Do not treat `open_or_skip` tests or a clean `cargo build` as success.

## Agent loop

1. Build probe: `make probe` (adds `--features portal-capture`; plain `cargo build -p sqyre-probe` leaves `capture.wayland_portal` pending).
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

Pure Wayland without XWayland needs `portal-capture` (libpipewire ≥ 1.0 dev headers — Ubuntu 24.04+). Linux Makefile targets (`make`, `make release`, `make probe`, bundles) enable it; plain `cargo build` does not.

## Portal rules

- UI thread: never call `shared_capturer()` when `shared_capturer_open_may_block()`; use `shared_capturer_nonblocking()` (returns `NotReady`) and let a worker/probe open it.
- Grants persist via restore tokens (`PersistMode::ExplicitlyRevoked`); `revoke_portal_grants()` clears them. App id `com.sqyre.app` must match the Flatpak id.

## Backend layout

```
crates/sqyre-capture/src/linux/
  session.rs          # X11 / XWayland / portal detection
  wayland/            # portal ScreenCast+PipeWire, RemoteDesktop EIS, foreign-toplevel, AT-SPI, layer-shell
crates/sqyre-hotkeys/src/linux_evdev.rs   # Wayland hotkeys (needs /dev/input access)
crates/sqyre-probe/   # structured JSON capability probe
```

Probe keys report `pending` when a backend is unavailable in the session or build: `capture.wayland_impl`, `capture.wayland_portal`, `windows.wayland_impl`, `input.wayland_impl`, `outline.wayland_impl`, `grab.wayland_impl`.

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

## Backend status (Wayland)

| Capability | Backend | Notes |
|------------|---------|-------|
| Session + probe | `session.rs`, `sqyre-probe` | done |
| Capture | portal ScreenCast + PipeWire (`portal-capture`) | done |
| Windows list/focus | foreign-toplevel, AT-SPI fallback | done; GNOME lacks foreign-toplevel |
| Input | portal RemoteDesktop EIS (shared ScreenCast session) | done; needs "Allow Remote Interaction". uinput fallback not implemented |
| Hotkeys | evdev | done; needs `input` group. Portal GlobalShortcuts not implemented |
| Outline / grab | wlr-layer-shell | done where compositor exposes it (not GNOME) |
| KWin D-Bus fast paths | — | not implemented (optional) |

After backend changes, re-run `./bin/sqyre-probe --json` and confirm the capability moves from `pending`/`fail` to `ok`.
