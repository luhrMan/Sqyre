//! libei sender over the Remote Desktop portal `ConnectToEIS` fd.
//!
//! GNOME 46+ ignores `NotifyPointer*` once the session is on EIS. Absolute
//! pointer also needs ScreenCast streams on the same Remote Desktop session.

use crate::cap_log;
use crate::error::CaptureError;
use reis::ei::{self, button::ButtonState, keyboard::KeyState};
use reis::event::{Device, DeviceCapability, EiEvent, EiEventConverter, Region};
use reis::handshake::EiHandshaker;
use reis::PendingRequestResult;
use sqyre_ports::{cosmic_desktop_session, AutomationError};
use std::io::ErrorKind;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub(crate) struct EisInput {
    context: ei::Context,
    converter: EiEventConverter,
    devices: Vec<DeviceSlot>,
    sequence: u32,
    /// Absolute warps sent this session. Diagnostic only.
    move_n: u32,
    last_move_log: Option<Instant>,
}

struct DeviceSlot {
    device: Device,
    resumed: bool,
    emulating: bool,
}

impl EisInput {
    pub(crate) fn connect(fd: OwnedFd, stop: &AtomicBool) -> Result<Self, CaptureError> {
        cap_log("INPUT", "eis", "handshake start");
        let stream = UnixStream::from(fd);
        stream
            .set_nonblocking(true)
            .map_err(|e| CaptureError::Message(format!("EIS nonblocking: {e}")))?;
        let context = ei::Context::new(stream)
            .map_err(|e| CaptureError::Message(format!("EIS context: {e}")))?;
        let resp = handshake_with_timeout(&context, Duration::from_secs(4), stop)?;
        let converter = EiEventConverter::new(&context, resp);
        let _ = context.flush();
        let mut eis = Self {
            context,
            converter,
            devices: Vec::new(),
            sequence: 0,
            move_n: 0,
            last_move_log: None,
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if stop.load(Ordering::SeqCst) {
                return Err(CaptureError::Message("EIS handshake aborted".into()));
            }
            eis.drain();
            if eis.devices.iter().any(|d| {
                d.resumed
                    && (d.device.has_capability(DeviceCapability::PointerAbsolute)
                        || d.device.has_capability(DeviceCapability::Keyboard)
                        || d.device.has_capability(DeviceCapability::Button))
            }) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let resumed = eis.devices.iter().filter(|d| d.resumed).count();
        let regions: Vec<String> = eis
            .devices
            .iter()
            .filter(|d| d.resumed)
            .flat_map(|d| {
                d.device
                    .regions()
                    .iter()
                    .map(|r| format!("{}x{}+{}+{}@{}", r.width, r.height, r.x, r.y, r.scale))
            })
            .collect();
        let regions_s = if regions.is_empty() {
            "none".to_string()
        } else {
            regions.join(",")
        };
        cap_log(
            "INPUT",
            if resumed == 0 { "fail" } else { "ok" },
            &format!(
                "eis devices={} resumed={resumed} regions={regions_s} layout={}",
                eis.devices.len(),
                if eis.cosmic_layout() { "cosmic" } else { "abs" },
            ),
        );
        if resumed == 0 {
            return Err(CaptureError::Message(
                "Remote Desktop EIS connected but no input device resumed".into(),
            ));
        }
        Ok(eis)
    }

    pub(crate) fn drain(&mut self) {
        for _ in 0..32 {
            match poll_readable_timeout(&self.context, Duration::ZERO) {
                Ok(true) => {}
                _ => break,
            }
            match self.context.read() {
                Ok(0) => break,
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        while let Some(result) = self.context.pending_event() {
            match result {
                PendingRequestResult::Request(ev) => {
                    let _ = self.converter.handle_event(ev);
                }
                PendingRequestResult::ParseError(_) | PendingRequestResult::InvalidObject(_) => {}
            }
        }
        while let Some(ev) = self.converter.next_event() {
            self.on_event(ev);
        }
        let _ = self.context.flush();
    }

    fn on_event(&mut self, ev: EiEvent) {
        // Compositor replies (pause/resume/region rebuild) explain a warp the
        // client thought it sent. Rare; not the per-move hot path.
        cap_log("INPUT", "eis-ev", &event_detail(&ev));
        match ev {
            EiEvent::SeatAdded(e) => {
                e.seat.bind_capabilities(
                    DeviceCapability::Pointer
                        | DeviceCapability::PointerAbsolute
                        | DeviceCapability::Keyboard
                        | DeviceCapability::Scroll
                        | DeviceCapability::Button
                        | DeviceCapability::Text,
                );
                let _ = self.context.flush();
            }
            EiEvent::DeviceAdded(e) => {
                self.devices.push(DeviceSlot {
                    device: e.device,
                    resumed: false,
                    emulating: false,
                });
            }
            EiEvent::DeviceRemoved(e) => {
                self.devices.retain(|d| d.device != e.device);
            }
            EiEvent::DeviceResumed(e) => {
                if let Some(d) = self.devices.iter_mut().find(|d| d.device == e.device) {
                    d.resumed = true;
                    d.emulating = false;
                }
            }
            EiEvent::DevicePaused(e) => {
                if let Some(d) = self.devices.iter_mut().find(|d| d.device == e.device) {
                    d.resumed = false;
                    d.emulating = false;
                }
            }
            _ => {}
        }
    }

    fn device_for(&self, cap: DeviceCapability) -> Option<usize> {
        self.devices
            .iter()
            .position(|d| d.resumed && d.device.has_capability(cap))
    }

    fn ensure_emulating(&mut self, idx: usize) {
        if self.devices[idx].emulating {
            return;
        }
        let proto = self.devices[idx].device.device().clone();
        // `last_serial` is the newest serial the compositor has sent. The
        // converter updates it on every event.
        proto.start_emulating(self.converter.connection().serial(), self.sequence);
        self.sequence = self.sequence.wrapping_add(1);
        self.devices[idx].emulating = true;
    }

    /// End the current emulation sequence so the next warp is a fresh
    /// `start_emulating`, matching the first move of a new app.
    fn restart_emulating(&mut self, idx: usize) {
        if self.devices[idx].emulating {
            let proto = self.devices[idx].device.device().clone();
            proto.stop_emulating(self.converter.connection().serial());
            self.devices[idx].emulating = false;
        }
        self.ensure_emulating(idx);
    }

    fn emit_frame(&self, idx: usize) -> bool {
        self.devices[idx]
            .device
            .device()
            .frame(self.converter.connection().serial(), monotonic_us());
        match self.context.flush() {
            Ok(()) => true,
            Err(e) => {
                cap_log("INPUT", "fail", &format!("flush {e}"));
                false
            }
        }
    }

    fn log_move(&mut self, started: bool, detail: &str) {
        self.move_n = self.move_n.saturating_add(1);
        let now = Instant::now();
        // Smooth moves step many times per gesture. Log the start of each
        // gesture (a gap, or a fresh emulation) and then at most a few times
        // a second so a later macro run is visible.
        let due = self.move_n <= 4
            || started
            || self
                .last_move_log
                .is_none_or(|t| now.duration_since(t) >= Duration::from_millis(200));
        if !due {
            return;
        }
        self.last_move_log = Some(now);
        cap_log("INPUT", "move", &format!("n={} {detail}", self.move_n));
    }

    fn missing(cap: DeviceCapability) -> AutomationError {
        AutomationError::Backend(format!("EIS has no resumed {cap:?} device"))
    }

    /// Cosmic advertises `virtual pointer` and `virtual absolute pointer`.
    /// The first absolute warp after `start_emulating` moves the cursor.
    /// Later warps in that same emulation do not, so each warp stops and
    /// starts emulation again. A smooth move's 1px steps are not sent here;
    /// the caller warps once to the endpoint.
    fn cosmic_layout(&self) -> bool {
        cosmic_desktop_session()
            || self.devices.iter().any(|d| {
                d.device
                    .name()
                    .is_some_and(|n| n.eq_ignore_ascii_case("virtual absolute pointer"))
            })
    }

    /// Start emulation on the relative pointer before an absolute warp.
    /// Returns `start`, `keep`, or `none`.
    fn arm_relative_pointer(&mut self) -> &'static str {
        let Some(idx) = self.devices.iter().position(|d| {
            d.resumed
                && d.device.has_capability(DeviceCapability::Pointer)
                && !d.device.has_capability(DeviceCapability::PointerAbsolute)
        }) else {
            return "none";
        };
        if self.devices[idx].emulating {
            return "keep";
        }
        self.ensure_emulating(idx);
        let _ = self.context.flush();
        "start"
    }

    pub(crate) fn move_to(&mut self, x: i32, y: i32) -> Result<(), AutomationError> {
        self.drain();
        let idx = self
            .device_for(DeviceCapability::PointerAbsolute)
            .ok_or_else(|| Self::missing(DeviceCapability::PointerAbsolute))?;
        let (lx, ly) =
            region_coords(self.devices[idx].device.regions(), x, y).map_err(|detail| {
                cap_log(
                    "INPUT",
                    "fail",
                    &format!("abs outside region pos={x},{y} {detail}"),
                );
                AutomationError::Backend(format!(
                    "EIS absolute pointer ({x},{y}) outside device regions ({detail})"
                ))
            })?;
        let cosmic = self.cosmic_layout();
        let rel = if cosmic {
            self.arm_relative_pointer()
        } else {
            "skip"
        };
        let started = if cosmic {
            self.restart_emulating(idx);
            true
        } else {
            let started = !self.devices[idx].emulating;
            self.ensure_emulating(idx);
            started
        };
        let ptr = self.devices[idx]
            .device
            .interface::<ei::PointerAbsolute>()
            .ok_or_else(|| Self::missing(DeviceCapability::PointerAbsolute))?;
        ptr.motion_absolute(lx, ly);
        let flushed = self.emit_frame(idx);
        let cursor = portal_cursor_label();
        self.log_move(
            started,
            &format!(
                "kind=abs px={x},{y} logical={lx},{ly} dev={idx} {} emu={} rel={rel} serial={} flush={} cursor={cursor}",
                device_brief(&self.devices[idx].device),
                if started { "start" } else { "keep" },
                self.converter.connection().serial(),
                if flushed { "ok" } else { "fail" },
            ),
        );
        Ok(())
    }

    pub(crate) fn click(
        &mut self,
        button: u32,
        down: bool,
        reseat: Option<(i32, i32)>,
    ) -> Result<(), AutomationError> {
        self.drain();
        let idx = self
            .device_for(DeviceCapability::Button)
            .ok_or_else(|| Self::missing(DeviceCapability::Button))?;
        self.ensure_emulating(idx);

        // Re-assert absolute position with the button edge. GNOME/Mutter intermittently
        // drops orphaned button events when the pointer device has not framed recently
        // (common after Image Search / kick / rapid Tap sequences).
        if let Some((x, y)) = reseat {
            let ptr_idx = if self.devices[idx]
                .device
                .interface::<ei::PointerAbsolute>()
                .is_some()
            {
                idx
            } else {
                self.device_for(DeviceCapability::PointerAbsolute)
                    .ok_or_else(|| Self::missing(DeviceCapability::PointerAbsolute))?
            };
            let (lx, ly) =
                region_coords(self.devices[ptr_idx].device.regions(), x, y).map_err(|detail| {
                    cap_log(
                        "INPUT",
                        "fail",
                        &format!("click reseat outside region pos={x},{y} {detail}"),
                    );
                    AutomationError::Backend(format!(
                        "EIS click reseat ({x},{y}) outside device regions ({detail})"
                    ))
                })?;
            self.ensure_emulating(ptr_idx);
            let ptr = self.devices[ptr_idx]
                .device
                .interface::<ei::PointerAbsolute>()
                .ok_or_else(|| Self::missing(DeviceCapability::PointerAbsolute))?;
            ptr.motion_absolute(lx, ly);
            if ptr_idx != idx {
                self.emit_frame(ptr_idx);
            }
        }

        let btn = self.devices[idx]
            .device
            .interface::<ei::Button>()
            .ok_or_else(|| Self::missing(DeviceCapability::Button))?;
        let state = if down {
            ButtonState::Press
        } else {
            ButtonState::Released
        };
        btn.button(button, state);
        let flushed = self.emit_frame(idx);
        let edge = if down { "down" } else { "up" };
        let reseat_s = reseat
            .map(|(x, y)| format!("{x},{y}"))
            .unwrap_or_else(|| "none".into());
        cap_log(
            "INPUT",
            "click",
            &format!(
                "edge={edge} button={button} reseat={reseat_s} dev={idx} {} flush={}",
                device_brief(&self.devices[idx].device),
                if flushed { "ok" } else { "fail" },
            ),
        );
        Ok(())
    }

    pub(crate) fn scroll(&mut self, up: bool) -> Result<(), AutomationError> {
        self.drain();
        let idx = self
            .device_for(DeviceCapability::Scroll)
            .ok_or_else(|| Self::missing(DeviceCapability::Scroll))?;
        self.ensure_emulating(idx);
        let scroll = self.devices[idx]
            .device
            .interface::<ei::Scroll>()
            .ok_or_else(|| Self::missing(DeviceCapability::Scroll))?;
        let discrete = if up { -120 } else { 120 };
        let px = if up { -15.0 } else { 15.0 };
        scroll.scroll_discrete(0, discrete);
        scroll.scroll(0.0, px);
        let flushed = self.emit_frame(idx);
        cap_log(
            "INPUT",
            "scroll",
            &format!(
                "dir={} dev={idx} {} flush={}",
                if up { "up" } else { "down" },
                device_brief(&self.devices[idx].device),
                if flushed { "ok" } else { "fail" },
            ),
        );
        Ok(())
    }

    pub(crate) fn key(&mut self, evdev: u32, down: bool) -> Result<(), AutomationError> {
        self.drain();
        let idx = self
            .device_for(DeviceCapability::Keyboard)
            .ok_or_else(|| Self::missing(DeviceCapability::Keyboard))?;
        self.ensure_emulating(idx);
        let kb = self.devices[idx]
            .device
            .interface::<ei::Keyboard>()
            .ok_or_else(|| Self::missing(DeviceCapability::Keyboard))?;
        let state = if down {
            KeyState::Press
        } else {
            KeyState::Released
        };
        kb.key(evdev, state);
        let _ = self.emit_frame(idx);
        Ok(())
    }
}

fn caps_label(device: &Device) -> String {
    let mut parts = Vec::new();
    if device.has_capability(DeviceCapability::Pointer) {
        parts.push("rel");
    }
    if device.has_capability(DeviceCapability::PointerAbsolute) {
        parts.push("abs");
    }
    if device.has_capability(DeviceCapability::Button) {
        parts.push("btn");
    }
    if device.has_capability(DeviceCapability::Scroll) {
        parts.push("scroll");
    }
    if device.has_capability(DeviceCapability::Keyboard) {
        parts.push("key");
    }
    if device.has_capability(DeviceCapability::Touch) {
        parts.push("touch");
    }
    if device.has_capability(DeviceCapability::Text) {
        parts.push("text");
    }
    if parts.is_empty() {
        "none".to_string()
    } else {
        parts.join("+")
    }
}

fn regions_label(regions: &[Region]) -> String {
    if regions.is_empty() {
        return "none".to_string();
    }
    regions
        .iter()
        .map(|r| format!("{}x{}+{}+{}@{}", r.width, r.height, r.x, r.y, r.scale))
        .collect::<Vec<_>>()
        .join(",")
}

fn device_brief(device: &Device) -> String {
    let name = device.name().unwrap_or("-").replace(' ', "_");
    format!(
        "name={name} caps={} regions={}",
        caps_label(device),
        regions_label(device.regions())
    )
}

fn event_detail(ev: &EiEvent) -> String {
    match ev {
        EiEvent::Disconnected(_) => "disconnected".into(),
        EiEvent::SeatAdded(_) => "seat-added".into(),
        EiEvent::SeatRemoved(_) => "seat-removed".into(),
        EiEvent::DeviceAdded(e) => format!("added {}", device_brief(&e.device)),
        EiEvent::DeviceRemoved(e) => format!("removed {}", device_brief(&e.device)),
        EiEvent::DevicePaused(e) => {
            format!("paused serial={} {}", e.serial, device_brief(&e.device))
        }
        EiEvent::DeviceResumed(e) => {
            format!("resumed serial={} {}", e.serial, device_brief(&e.device))
        }
        EiEvent::KeyboardModifiers(e) => format!("mods serial={} group={}", e.serial, e.group),
        EiEvent::Frame(e) => format!("frame serial={}", e.serial),
        EiEvent::DeviceStartEmulating(e) => {
            format!("start-emulating serial={} seq={}", e.serial, e.sequence)
        }
        EiEvent::DeviceStopEmulating(e) => format!("stop-emulating serial={}", e.serial),
        EiEvent::PointerMotion(_) => "rel-motion".into(),
        EiEvent::PointerMotionAbsolute(_) => "abs-motion".into(),
        EiEvent::Button(_) => "button".into(),
        EiEvent::ScrollDelta(_) => "scroll-delta".into(),
        EiEvent::ScrollStop(_) => "scroll-stop".into(),
        EiEvent::ScrollCancel(_) => "scroll-cancel".into(),
        EiEvent::ScrollDiscrete(_) => "scroll-discrete".into(),
        EiEvent::KeyboardKey(_) => "key".into(),
        EiEvent::TouchDown(_) => "touch-down".into(),
        EiEvent::TouchUp(_) => "touch-up".into(),
        EiEvent::TouchMotion(_) => "touch-motion".into(),
        EiEvent::TouchCancel(_) => "touch-cancel".into(),
        EiEvent::TextKeysym(_) => "text-keysym".into(),
        EiEvent::TextUtf8(_) => "text-utf8".into(),
    }
}

fn portal_cursor_label() -> String {
    super::portal_session::portal_cursor_position()
        .map(|(cx, cy)| format!("{cx},{cy}"))
        .unwrap_or_else(|| "none".into())
}

/// Desktop pixel → EIS logical coordinates.
///
/// Regions are logical with a per-region scale (COSMIC at 200%:
/// `1440x960+0+0@2` for a 2880×1920 output); Sqyre works in physical pixels.
/// The physical rect is taken as the logical rect × scale. No regions means
/// the device is unrestricted and takes desktop pixels as-is.
fn region_coords(regions: &[Region], x: i32, y: i32) -> Result<(f32, f32), String> {
    if regions.is_empty() {
        return Ok((x as f32, y as f32));
    }
    let (fx, fy) = (f64::from(x), f64::from(y));
    regions
        .iter()
        .find_map(|r| {
            let s = if r.scale > 0.0 {
                f64::from(r.scale)
            } else {
                1.0
            };
            let (px, py) = (f64::from(r.x) * s, f64::from(r.y) * s);
            let (pw, ph) = (f64::from(r.width) * s, f64::from(r.height) * s);
            (fx >= px && fy >= py && fx < px + pw && fy < py + ph).then(|| {
                (
                    (f64::from(r.x) + (fx - px) / s) as f32,
                    (f64::from(r.y) + (fy - py) / s) as f32,
                )
            })
        })
        .ok_or_else(|| {
            regions
                .iter()
                .map(|r| format!("{}x{}+{}+{}@{}", r.width, r.height, r.x, r.y, r.scale))
                .collect::<Vec<_>>()
                .join(",")
        })
}

fn handshake_with_timeout(
    context: &ei::Context,
    limit: Duration,
    stop: &AtomicBool,
) -> Result<reis::handshake::HandshakeResp, CaptureError> {
    let mut handshaker = EiHandshaker::new("sqyre", ei::handshake::ContextType::Sender);
    let deadline = Instant::now() + limit;
    loop {
        if stop.load(Ordering::SeqCst) {
            return Err(CaptureError::Message("EIS handshake aborted".into()));
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            cap_log("INPUT", "fail", "eis handshake timeout_ms=4000");
            return Err(CaptureError::Message(
                "EIS handshake timed out (ConnectToEIS fd never spoke ei protocol)".into(),
            ));
        }
        match poll_readable_timeout(context, left.min(Duration::from_millis(250))) {
            Ok(false) => continue,
            Ok(true) => {}
            Err(e) => {
                return Err(CaptureError::Message(format!("EIS poll: {e}")));
            }
        }
        match context.read() {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(e) => {
                return Err(CaptureError::Message(format!("EIS handshake read: {e}")));
            }
        }
        while let Some(result) = context.pending_event() {
            let ev = match result {
                PendingRequestResult::Request(ev) => ev,
                PendingRequestResult::ParseError(e) => {
                    return Err(CaptureError::Message(format!("EIS handshake parse: {e}")));
                }
                PendingRequestResult::InvalidObject(_) => continue,
            };
            match handshaker.handle_event(ev) {
                Ok(Some(resp)) => return Ok(resp),
                Ok(None) => {
                    let _ = context.flush();
                }
                Err(e) => {
                    return Err(CaptureError::Message(format!("EIS handshake: {e}")));
                }
            }
        }
        let _ = context.flush();
    }
}

/// `ei_device.frame` time, in microseconds of `CLOCK_MONOTONIC`.
fn monotonic_us() -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid out-pointer and `CLOCK_MONOTONIC` is always
    // available on Linux.
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    if rc != 0 {
        return 0;
    }
    let sec = u64::try_from(ts.tv_sec).unwrap_or(0);
    let nsec = u64::try_from(ts.tv_nsec).unwrap_or(0);
    sec.saturating_mul(1_000_000) + nsec / 1_000
}

fn poll_readable_timeout(fd: impl AsFd, timeout: Duration) -> std::io::Result<bool> {
    let mut pfd = libc::pollfd {
        fd: fd.as_fd().as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let ms = timeout.as_millis().min(i32::MAX as u128) as libc::c_int;
    loop {
        // SAFETY: `pfd` is a single valid pollfd for `fd` for the duration of this call.
        let n = unsafe { libc::poll(&mut pfd, 1, ms) };
        if n < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == ErrorKind::Interrupted {
                continue;
            }
            return Err(err);
        }
        return Ok(n > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(x: u32, y: u32, width: u32, height: u32, scale: f32) -> Region {
        Region {
            x,
            y,
            width,
            height,
            scale,
            mapping_id: None,
        }
    }

    #[test]
    fn scaled_region_maps_physical_to_logical() {
        let regions = [region(0, 0, 1440, 960, 2.0)];
        assert_eq!(region_coords(&regions, 1440, 960), Ok((720.0, 480.0)));
        assert_eq!(region_coords(&regions, 2879, 0), Ok((1439.5, 0.0)));
        assert!(region_coords(&regions, 2880, 0).is_err());
    }

    #[test]
    fn frame_timestamp_tracks_monotonic_clock() {
        let a = monotonic_us();
        std::thread::sleep(Duration::from_millis(2));
        let b = monotonic_us();
        assert!(b > a);
        // Not "time since the EIS thread started" (that clock begins at 0).
        let uptime: f64 = std::fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|s| s.split_whitespace().next().and_then(|n| n.parse().ok()))
            .unwrap_or(0.0);
        if uptime > 1.0 {
            assert!(a > 1_000_000, "frame time {a} is not CLOCK_MONOTONIC");
        }
    }

    #[test]
    fn unscaled_and_unrestricted() {
        let regions = [
            region(0, 0, 1920, 1080, 1.0),
            region(1920, 0, 2560, 1440, 1.0),
        ];
        assert_eq!(region_coords(&regions, 2000, 10), Ok((2000.0, 10.0)));
        assert_eq!(region_coords(&[], 5, 6), Ok((5.0, 6.0)));
    }
}
