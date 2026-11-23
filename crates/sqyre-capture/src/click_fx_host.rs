//! Dedicated-thread click marker: places a hollow gold square as soon as the
//! executor reports a press, without waiting for the egui frame.
//!
//! Under fullscreen XWayland/Proton games, `ConfigureWindow` on the UI thread
//! both delays the flash (until the next paint) and hitches the shell. This
//! host owns a [`SelectionOutline`] on a private thread so place/clear never
//! touch the UI event loop.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::{cap_log, event_log, mark_site, note, CaptureError, SelectionOutline};

/// Hollow square edge length (virtual-desktop pixels).
const FLASH_PX: i32 = 48;

enum Cmd {
    Flash { x: i32, y: i32 },
}

/// Background click-marker host. Clone freely; dropping every clone stops the worker.
#[derive(Clone)]
pub struct ClickFxHost {
    tx: mpsc::Sender<Cmd>,
    flash_ms: Arc<AtomicI32>,
}

impl ClickFxHost {
    /// Open the outline on a worker thread and return a handle.
    pub fn start() -> Result<Self, CaptureError> {
        let (tx, rx) = mpsc::channel::<Cmd>();
        let flash_ms = Arc::new(AtomicI32::new(360));
        let flash_ms_worker = Arc::clone(&flash_ms);
        thread::Builder::new()
            .name("sqyre-clickfx".into())
            .spawn(move || worker_loop(rx, flash_ms_worker))
            .map_err(|e| CaptureError::Message(format!("clickfx worker spawn: {e}")))?;
        Ok(Self { tx, flash_ms })
    }

    pub fn set_flash_ms(&self, ms: i32) {
        self.flash_ms.store(ms.max(1), Ordering::Relaxed);
    }

    /// Show the marker at `(x, y)` immediately (newest press wins).
    pub fn flash(&self, x: i32, y: i32) {
        // Unbounded channel + low press rate: never blocks the executor.
        if self.tx.send(Cmd::Flash { x, y }).is_err() {
            note("clickfx: worker gone");
        }
    }
}

fn worker_loop(rx: mpsc::Receiver<Cmd>, flash_ms: Arc<AtomicI32>) {
    mark_site("clickfx:worker:open");
    let open_start = Instant::now();
    let mut outline = match SelectionOutline::open() {
        Ok(o) => o,
        Err(e) => {
            note(&format!("clickfx: outline open failed: {e}"));
            // Drain until disconnect so senders do not hang forever.
            while rx.recv().is_ok() {}
            return;
        }
    };
    event_log(
        "SQYRE_CLICKFX",
        &[
            ("op", "open"),
            ("open_ms", &open_start.elapsed().as_millis().to_string()),
        ],
    );

    let mut deadline: Option<Instant> = None;
    loop {
        let cmd = match deadline {
            None => match rx.recv() {
                Ok(c) => c,
                Err(_) => break,
            },
            Some(until) => {
                let left = until.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    outline.clear();
                    deadline = None;
                    event_log("SQYRE_CLICKFX", &[("op", "clear"), ("reason", "expire")]);
                    continue;
                }
                match rx.recv_timeout(left) {
                    Ok(c) => c,
                    Err(RecvTimeoutError::Timeout) => {
                        outline.clear();
                        deadline = None;
                        event_log("SQYRE_CLICKFX", &[("op", "clear"), ("reason", "expire")]);
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        };

        // Coalesce: if more flashes are already queued, keep only the newest.
        let Cmd::Flash { mut x, mut y } = cmd;
        while let Ok(Cmd::Flash { x: nx, y: ny }) = rx.try_recv() {
            x = nx;
            y = ny;
        }

        let half = FLASH_PX / 2;
        let t0 = Instant::now();
        outline.set_rect(x - half, y - half, x + half, y + half);
        let set_ms = t0.elapsed().as_millis();
        if set_ms >= 16 {
            cap_log("CLICKFX", "slow", &format!("set_rect_ms={set_ms}"));
        }
        event_log(
            "SQYRE_CLICKFX",
            &[
                ("op", "show"),
                ("xy", &format!("{x},{y}")),
                ("set_rect_ms", &set_ms.to_string()),
            ],
        );
        let ms = flash_ms.load(Ordering::Relaxed).max(1) as u64;
        deadline = Some(Instant::now() + Duration::from_millis(ms));
    }

    outline.clear();
    mark_site("clickfx:worker:exit");
}
