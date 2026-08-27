//! libei sender over the Remote Desktop portal `ConnectToEIS` fd.
//!
//! GNOME 46+ ignores `NotifyPointer*` once the session is on EIS. Absolute
//! pointer also needs ScreenCast streams on the same Remote Desktop session.

use crate::cap_log;
use crate::error::CaptureError;
use reis::ei::{self, button::ButtonState, keyboard::KeyState};
use reis::event::{Device, DeviceCapability, EiEvent, EiEventConverter};
use reis::handshake::EiHandshaker;
use reis::PendingRequestResult;
use sqyre_ports::AutomationError;
use std::io::ErrorKind;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub(crate) struct EisInput {
    context: ei::Context,
    converter: EiEventConverter,
    devices: Vec<DeviceSlot>,
    last_serial: u32,
    sequence: u32,
    start: Instant,
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
        let last_serial = converter.connection().serial();
        let _ = context.flush();
        let mut eis = Self {
            context,
            converter,
            devices: Vec::new(),
            last_serial,
            sequence: 0,
            start: Instant::now(),
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
                d.device.regions().iter().map(|r| {
                    format!(
                        "{}x{}+{}+{}@{}",
                        r.width, r.height, r.x, r.y, r.scale
                    )
                })
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
                "eis devices={} resumed={resumed} regions={regions_s}",
                eis.devices.len()
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
                self.last_serial = e.serial;
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
            EiEvent::KeyboardModifiers(e) => self.last_serial = e.serial,
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
        proto.start_emulating(self.last_serial, self.sequence);
        self.sequence = self.sequence.wrapping_add(1);
        self.devices[idx].emulating = true;
    }

    fn now_us(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }

    fn emit_frame(&self, idx: usize) {
        self.devices[idx]
            .device
            .device()
            .frame(self.last_serial, self.now_us());
        let _ = self.context.flush();
    }

    fn missing(cap: DeviceCapability) -> AutomationError {
        AutomationError::Backend(format!("EIS has no resumed {cap:?} device"))
    }

    /// `None` if the device has no regions (unrestricted) or `(x,y)` is inside one.
    fn outside_regions(device: &Device, x: i32, y: i32) -> Option<String> {
        let regions = device.regions();
        if regions.is_empty() {
            return None;
        }
        let inside = regions.iter().any(|r| {
            let rx = r.x as i32;
            let ry = r.y as i32;
            let rw = r.width as i32;
            let rh = r.height as i32;
            x >= rx && y >= ry && x < rx.saturating_add(rw) && y < ry.saturating_add(rh)
        });
        if inside {
            None
        } else {
            let detail = regions
                .iter()
                .map(|r| format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y))
                .collect::<Vec<_>>()
                .join(",");
            Some(detail)
        }
    }

    pub(crate) fn move_to(&mut self, x: i32, y: i32) -> Result<(), AutomationError> {
        self.drain();
        let idx = self
            .device_for(DeviceCapability::PointerAbsolute)
            .ok_or_else(|| Self::missing(DeviceCapability::PointerAbsolute))?;
        if let Some(detail) = Self::outside_regions(&self.devices[idx].device, x, y) {
            cap_log("INPUT", "fail", &format!("abs outside region pos={x},{y} {detail}"));
            return Err(AutomationError::Backend(format!(
                "EIS absolute pointer ({x},{y}) outside device regions ({detail})"
            )));
        }
        self.ensure_emulating(idx);
        let ptr = self.devices[idx]
            .device
            .interface::<ei::PointerAbsolute>()
            .ok_or_else(|| Self::missing(DeviceCapability::PointerAbsolute))?;
        ptr.motion_absolute(x as f32, y as f32);
        self.emit_frame(idx);
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
            if let Some(detail) = Self::outside_regions(&self.devices[ptr_idx].device, x, y) {
                cap_log(
                    "INPUT",
                    "fail",
                    &format!("click reseat outside region pos={x},{y} {detail}"),
                );
                return Err(AutomationError::Backend(format!(
                    "EIS click reseat ({x},{y}) outside device regions ({detail})"
                )));
            }
            self.ensure_emulating(ptr_idx);
            let ptr = self.devices[ptr_idx]
                .device
                .interface::<ei::PointerAbsolute>()
                .ok_or_else(|| Self::missing(DeviceCapability::PointerAbsolute))?;
            ptr.motion_absolute(x as f32, y as f32);
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
        self.emit_frame(idx);
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
        self.emit_frame(idx);
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
        self.emit_frame(idx);
        Ok(())
    }
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
