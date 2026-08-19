---
name: debug-from-logs
description: >-
  Debug Sqyre from existing logs and stable diag markers before guessing from
  code. Use when investigating bugs, crashes, hangs, black frames, capture,
  hotkeys, overlays, focus, probe failures, or any unexpected runtime behavior;
  when the user mentions logs, diag.log, crash.log, last_site, SQYRE_DIAG, or
  stderr; or when adding logging to diagnose an issue.
---

# Debug from logs

Do not diagnose from source inspection alone. Read or produce logs, then cite them.

## Required loop

1. Locate the failure (crate, OS path, last user action).
2. **Read existing evidence** (files below, terminal stderr, probe JSON). Quote the matching lines in the diagnosis.
3. If logs are missing, empty, or too coarse: **add instrumentation**, rebuild, reproduce, read again.
4. Fix from that evidence. Do not land a speculative fix and skip the log step.
5. After the fix, keep useful `event_log` / `cap_log` / `mark_site` calls; remove one-off noise.

If the user has not reproduced with `SQYRE_DIAG=1`, ask them to (or run it yourself when a display/session is available) before concluding.

## Where logs live

Default dir: `~/.sqyre` (`sqyre_capture::diag::log_dir()`, set at app startup).

| File | When written | Use for |
|------|----------------|---------|
| `last_site.txt` | Always (`mark_site`) | Hard abort / hang: last code site |
| `crash.log` | Panic hook | Panic location, payload, backtrace, last site |
| `diag.log` | Only if `SQYRE_DIAG=1`/`true`/`yes` | Timeline of `note` / `event_log` / `cap_log` |

Stderr always gets `sqyre: …` lines from `diag::note` and `crate::log::warn`. Terminals folder and the user's run output count as logs.

Desktop/capture issues: also run `./bin/sqyre-probe --json` (see linux-desktop-parity skill).

Macro/action issues: executor `ActionLogger` lines (`ctx.log`, `log_timing`, `log_image`) in the app action log UI — not `diag.log`.

## APIs (use these, not `println!` / `tracing` / `log`)

Native / X11 / Win32 / overlay / probe:

```rust
use sqyre_capture::{cap_log, event_log, mark_site, note};

mark_site("preview:finish_texture:before_gpu"); // abort breadcrumb; always disk
note("overlay: grab arm failed");               // stderr; diag.log if SQYRE_DIAG
event_log("SQYRE_SESSION", &[("backend", "x11"), ("display", "yes")]);
cap_log("CAP", "fail", "error=portal-denied");  // → SQYRE_CAP=fail error=…
```

Desktop shell user warnings: `crate::log::warn(...)`.

Executor run path: `ctx.log` / `log_with` / `log_timing` / `timed_step` — never spam `note` on the hot path of every action.

Site strings: `area:verb:phase` (e.g. `x11:get_active_window:before_open`, `overlay:click:{id}`). Prefixes for agents: `SQYRE_<CATEGORY>=ok|fail|…` plus `key=value` fields (no spaces in values).

## Adding logs

Add a log at each boundary that could explain the bug: session/backend choice, Result Err, skip/deny, size/format, permission outcome, before/after FFI or GPU.

- Failures: `cap_log` or `note` with the error; success paths that are rare (first open, backend pick) may log once.
- Hard-crash suspects: `mark_site` immediately before the risky call and again after success.
- Do not log every frame, mouse move, or egui paint.
- Do not introduce `env_logger`, `tracing`, or a second log crate.

## Do not treat as diagnosis

- “Looks like it could be X” with no log line
- Green unit tests in headless CI for display/input bugs
- Reading only the function that returns `Err` without the stderr/`diag.log` that led there
