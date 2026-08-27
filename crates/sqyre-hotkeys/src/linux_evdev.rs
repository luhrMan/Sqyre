//! Wayland global input via evdev **without** `EVIOCGRAB`.
//!
//! `rdev::grab` exclusive-grabs every `/dev/input` node and replays through uinput.
//! That stalls the pointer for seconds on start/stop, and portal dialogs ignore the
//! synthetic clicks (so ScreenCast Share does nothing).

use rdev::{Button, Event, EventType, Key};
use std::fs::{read_dir, File};
use std::os::unix::fs::FileTypeExt;
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

const DEV_PATH: &str = "/dev/input";
const EPOLL_TIMEOUT_MS: i32 = 100;

pub fn watch_events(
    stop: Arc<AtomicBool>,
    mut on_event: impl FnMut(Event),
) -> Result<(), crate::HotkeyError> {
    let files =
        device_files().map_err(|e| crate::HotkeyError::Install(format!("list {DEV_PATH}: {e}")))?;
    if files.is_empty() {
        return Err(crate::HotkeyError::Install(format!(
            "no usable devices in {DEV_PATH}"
        )));
    }

    let epoll_fd = epoll::create(true)
        .map_err(|e| crate::HotkeyError::Install(format!("epoll create: {e}")))?;
    let mut devices = Vec::new();
    for file in files {
        let fd = file.as_raw_fd();
        let Ok(device) = evdev_rs::Device::new_from_fd(file) else {
            continue;
        };
        let idx = devices.len() as u64;
        let event = epoll::Event::new(epoll::Events::EPOLLIN, idx);
        if epoll::ctl(epoll_fd, epoll::ControlOptions::EPOLL_CTL_ADD, fd, event).is_err() {
            continue;
        }
        devices.push(device);
    }
    if devices.is_empty() {
        let _ = epoll::close(epoll_fd);
        return Err(crate::HotkeyError::Install(
            "could not open any evdev devices".into(),
        ));
    }

    let mut buf = [epoll::Event::new(epoll::Events::empty(), 0); 8];
    let mut x = 0.0_f64;
    let mut y = 0.0_f64;

    while !stop.load(Ordering::SeqCst) {
        let n = match epoll::wait(epoll_fd, EPOLL_TIMEOUT_MS, &mut buf) {
            Ok(n) => n,
            Err(_) => break,
        };
        for event in &buf[..n] {
            let idx = event.data as usize;
            let Some(device) = devices.get(idx) else {
                continue;
            };
            while device.has_event_pending() {
                let ev = match device.next_event(evdev_rs::ReadFlag::NORMAL) {
                    Ok((_, ev)) => ev,
                    Err(_) => break,
                };
                if let Some(event_type) = convert_event(&ev, &mut x, &mut y) {
                    on_event(Event {
                        time: SystemTime::now(),
                        name: None,
                        event_type,
                    });
                }
            }
        }
    }

    let _ = epoll::close(epoll_fd);
    Ok(())
}

fn device_files() -> std::io::Result<Vec<File>> {
    let mut out = Vec::new();
    for entry in read_dir(DEV_PATH)? {
        let entry = entry?;
        if !entry.file_type()?.is_char_device() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name == "mice" || name.starts_with("mouse") || name.starts_with("js") {
            continue;
        }
        if let Ok(file) = File::open(&path) {
            out.push(file);
        }
    }
    Ok(out)
}

fn convert_event(event: &evdev_rs::InputEvent, x: &mut f64, y: &mut f64) -> Option<EventType> {
    use evdev_rs::enums::{EventCode, EV_REL};

    match &event.event_code {
        EventCode::EV_KEY(key) => {
            if let Some(button) = mouse_button(key) {
                return Some(if event.value == 0 {
                    EventType::ButtonRelease(button)
                } else {
                    EventType::ButtonPress(button)
                });
            }
            let rdev_key = evdev_key(key)?;
            Some(if event.value == 0 {
                EventType::KeyRelease(rdev_key)
            } else {
                EventType::KeyPress(rdev_key)
            })
        }
        EventCode::EV_REL(EV_REL::REL_X) => {
            *x += event.value as f64;
            Some(EventType::MouseMove { x: *x, y: *y })
        }
        EventCode::EV_REL(EV_REL::REL_Y) => {
            *y += event.value as f64;
            Some(EventType::MouseMove { x: *x, y: *y })
        }
        _ => None,
    }
}

fn mouse_button(key: &evdev_rs::enums::EV_KEY) -> Option<Button> {
    use evdev_rs::enums::EV_KEY;
    match key {
        EV_KEY::BTN_LEFT => Some(Button::Left),
        EV_KEY::BTN_RIGHT => Some(Button::Right),
        EV_KEY::BTN_MIDDLE => Some(Button::Middle),
        _ => None,
    }
}

fn evdev_key(key: &evdev_rs::enums::EV_KEY) -> Option<Key> {
    use evdev_rs::enums::EV_KEY;
    Some(match key {
        EV_KEY::KEY_ESC => Key::Escape,
        EV_KEY::KEY_1 => Key::Num1,
        EV_KEY::KEY_2 => Key::Num2,
        EV_KEY::KEY_3 => Key::Num3,
        EV_KEY::KEY_4 => Key::Num4,
        EV_KEY::KEY_5 => Key::Num5,
        EV_KEY::KEY_6 => Key::Num6,
        EV_KEY::KEY_7 => Key::Num7,
        EV_KEY::KEY_8 => Key::Num8,
        EV_KEY::KEY_9 => Key::Num9,
        EV_KEY::KEY_0 => Key::Num0,
        EV_KEY::KEY_MINUS => Key::Minus,
        EV_KEY::KEY_EQUAL => Key::Equal,
        EV_KEY::KEY_BACKSPACE => Key::Backspace,
        EV_KEY::KEY_TAB => Key::Tab,
        EV_KEY::KEY_Q => Key::KeyQ,
        EV_KEY::KEY_W => Key::KeyW,
        EV_KEY::KEY_E => Key::KeyE,
        EV_KEY::KEY_R => Key::KeyR,
        EV_KEY::KEY_T => Key::KeyT,
        EV_KEY::KEY_Y => Key::KeyY,
        EV_KEY::KEY_U => Key::KeyU,
        EV_KEY::KEY_I => Key::KeyI,
        EV_KEY::KEY_O => Key::KeyO,
        EV_KEY::KEY_P => Key::KeyP,
        EV_KEY::KEY_LEFTBRACE => Key::LeftBracket,
        EV_KEY::KEY_RIGHTBRACE => Key::RightBracket,
        EV_KEY::KEY_ENTER => Key::Return,
        EV_KEY::KEY_LEFTCTRL => Key::ControlLeft,
        EV_KEY::KEY_A => Key::KeyA,
        EV_KEY::KEY_S => Key::KeyS,
        EV_KEY::KEY_D => Key::KeyD,
        EV_KEY::KEY_F => Key::KeyF,
        EV_KEY::KEY_G => Key::KeyG,
        EV_KEY::KEY_H => Key::KeyH,
        EV_KEY::KEY_J => Key::KeyJ,
        EV_KEY::KEY_K => Key::KeyK,
        EV_KEY::KEY_L => Key::KeyL,
        EV_KEY::KEY_SEMICOLON => Key::SemiColon,
        EV_KEY::KEY_APOSTROPHE => Key::Quote,
        EV_KEY::KEY_GRAVE => Key::BackQuote,
        EV_KEY::KEY_LEFTSHIFT => Key::ShiftLeft,
        EV_KEY::KEY_BACKSLASH => Key::BackSlash,
        EV_KEY::KEY_Z => Key::KeyZ,
        EV_KEY::KEY_X => Key::KeyX,
        EV_KEY::KEY_C => Key::KeyC,
        EV_KEY::KEY_V => Key::KeyV,
        EV_KEY::KEY_B => Key::KeyB,
        EV_KEY::KEY_N => Key::KeyN,
        EV_KEY::KEY_M => Key::KeyM,
        EV_KEY::KEY_COMMA => Key::Comma,
        EV_KEY::KEY_DOT => Key::Dot,
        EV_KEY::KEY_SLASH => Key::Slash,
        EV_KEY::KEY_RIGHTSHIFT => Key::ShiftRight,
        EV_KEY::KEY_LEFTALT => Key::Alt,
        EV_KEY::KEY_SPACE => Key::Space,
        EV_KEY::KEY_CAPSLOCK => Key::CapsLock,
        EV_KEY::KEY_F1 => Key::F1,
        EV_KEY::KEY_F2 => Key::F2,
        EV_KEY::KEY_F3 => Key::F3,
        EV_KEY::KEY_F4 => Key::F4,
        EV_KEY::KEY_F5 => Key::F5,
        EV_KEY::KEY_F6 => Key::F6,
        EV_KEY::KEY_F7 => Key::F7,
        EV_KEY::KEY_F8 => Key::F8,
        EV_KEY::KEY_F9 => Key::F9,
        EV_KEY::KEY_F10 => Key::F10,
        EV_KEY::KEY_F11 => Key::F11,
        EV_KEY::KEY_F12 => Key::F12,
        EV_KEY::KEY_RIGHTCTRL => Key::ControlRight,
        EV_KEY::KEY_RIGHTALT => Key::AltGr,
        EV_KEY::KEY_HOME => Key::Home,
        EV_KEY::KEY_UP => Key::UpArrow,
        EV_KEY::KEY_PAGEUP => Key::PageUp,
        EV_KEY::KEY_LEFT => Key::LeftArrow,
        EV_KEY::KEY_RIGHT => Key::RightArrow,
        EV_KEY::KEY_END => Key::End,
        EV_KEY::KEY_DOWN => Key::DownArrow,
        EV_KEY::KEY_PAGEDOWN => Key::PageDown,
        EV_KEY::KEY_INSERT => Key::Insert,
        EV_KEY::KEY_DELETE => Key::Delete,
        EV_KEY::KEY_LEFTMETA => Key::MetaLeft,
        EV_KEY::KEY_RIGHTMETA => Key::MetaRight,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_escape_and_left_click() {
        use evdev_rs::enums::{EventCode, EV_KEY};
        use evdev_rs::{InputEvent, TimeVal};

        let code = EventCode::EV_KEY(EV_KEY::KEY_ESC);
        let ev = InputEvent::new(&TimeVal::new(0, 0), &code, 1);
        assert!(matches!(
            convert_event(&ev, &mut 0.0, &mut 0.0),
            Some(EventType::KeyPress(Key::Escape))
        ));
        let code = EventCode::EV_KEY(EV_KEY::BTN_LEFT);
        let click = InputEvent::new(&TimeVal::new(0, 0), &code, 1);
        assert!(matches!(
            convert_event(&click, &mut 0.0, &mut 0.0),
            Some(EventType::ButtonPress(Button::Left))
        ));
    }
}
