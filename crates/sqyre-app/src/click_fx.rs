//! Click-effect feedback: a gold outline on the desktop where each automated
//! mouse press lands (Settings → Macros → "Flash a marker where clicks land").
//!
//! The outline lives on a [`sqyre_capture::ClickFxHost`] worker thread so a
//! press places the marker immediately — it does not wait for an egui frame,
//! and `ConfigureWindow` never blocks the UI under fullscreen XWayland games.

use sqyre_capture::ClickFxHost;
use sqyre_ports::SharedClickFx;

/// Owns the background click-marker host. UI thread only for start/stop.
#[derive(Default)]
pub struct ClickFxOverlay {
    host: Option<ClickFxHost>,
    host_failed: bool,
    /// Listener already wired onto [`SharedClickFx`].
    bound: bool,
}

impl ClickFxOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Keep the worker / listener in sync with the setting. Call once per UI frame.
    /// `flash_ms` is the marker lifetime in milliseconds (Settings → Playback).
    pub fn sync(&mut self, fx: &SharedClickFx, flash_ms: i32, enabled: bool) {
        if !enabled {
            if self.host.take().is_some() || self.bound {
                fx.clear_on_click();
                self.bound = false;
            }
            return;
        }

        if self.host.is_none() && !self.host_failed {
            match ClickFxHost::start() {
                Ok(host) => self.host = Some(host),
                Err(e) => {
                    self.host_failed = true;
                    crate::log::warn(format_args!("click effect host unavailable: {e}"));
                    return;
                }
            }
        }

        let Some(host) = self.host.as_ref() else {
            return;
        };
        host.set_flash_ms(flash_ms);
        if !self.bound {
            let host = host.clone();
            fx.set_on_click(Some(move |x, y| host.flash(x, y)));
            self.bound = true;
        }
    }
}
