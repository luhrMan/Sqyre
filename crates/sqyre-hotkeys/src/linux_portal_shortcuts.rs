//! Wayland macro chords through `org.freedesktop.portal.GlobalShortcuts`.
//!
//! BindShortcuts makes GNOME / KDE show their own "add shortcuts" dialog; the
//! compositor then owns those keys (the user can rebind them) and reports
//! Activated / Deactivated. ConfigureShortcuts (portal v2) reopens that editor.
//! Chords the desktop did not accept stay on the evdev matcher.

use crate::macro_hotkeys::{MacroHotkeyBinding, MacroHotkeyBridge, ShortcutEdge};
use crate::system_shortcuts::SystemShortcutsStatus;
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use ashpd::desktop::{ResponseError, Session};
use futures_util::{stream, StreamExt};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Same id as `sqyre-app` / `sqyre-capture` (`com.sqyre.app.desktop`).
const PORTAL_APP_ID: &str = "com.sqyre.app";
const TICK: Duration = Duration::from_millis(250);
/// Bindings must stay unchanged this long before the session is rebound.
const REBIND_QUIET: Duration = Duration::from_millis(750);

static STATUS: Mutex<SystemShortcutsStatus> = Mutex::new(SystemShortcutsStatus::Unavailable);
static CONFIGURABLE: AtomicBool = AtomicBool::new(false);
static OPEN_REQUESTED: AtomicBool = AtomicBool::new(false);
/// Current backend run; older threads stop writing once this moves on.
static RUN: AtomicU64 = AtomicU64::new(0);

pub fn system_shortcuts_status() -> SystemShortcutsStatus {
    STATUS.lock().clone()
}

pub fn system_shortcuts_configurable() -> bool {
    CONFIGURABLE.load(Ordering::SeqCst)
}

pub fn open_system_shortcuts() {
    OPEN_REQUESTED.store(true, Ordering::SeqCst);
}

/// Start the portal backend (replaces any previous run).
pub(crate) fn spawn(bridge: MacroHotkeyBridge, on_fire: Arc<dyn Fn(String) + Send + Sync>) {
    let run = Run(RUN.fetch_add(1, Ordering::SeqCst) + 1);
    let spawned = std::thread::Builder::new()
        .name("sqyre-shortcuts".into())
        .spawn(move || pollster::block_on(run.main(&bridge, &*on_fire)));
    if let Err(e) = spawned {
        eprintln!("sqyre-hotkeys: portal shortcuts thread: {e}");
    }
}

/// Stop the backend and hand every chord back to the evdev matcher.
pub(crate) fn stop(bridge: &MacroHotkeyBridge) {
    RUN.fetch_add(1, Ordering::SeqCst);
    bridge.set_system_owned(HashSet::new());
    *STATUS.lock() = SystemShortcutsStatus::Unavailable;
    CONFIGURABLE.store(false, Ordering::SeqCst);
}

/// What the portal sees of a binding; trigger mode only matters when firing.
type BindKey = Vec<(String, Vec<String>)>;

fn bind_key(bindings: &[MacroHotkeyBinding]) -> BindKey {
    bindings
        .iter()
        .map(|b| (b.macro_name.clone(), b.chord.clone()))
        .collect()
}

enum Outcome {
    Rebind,
    Declined,
}

enum Event {
    Edge(String, ShortcutEdge),
    Tick,
}

#[derive(Clone, Copy)]
struct Run(u64);

impl Run {
    fn alive(self) -> bool {
        RUN.load(Ordering::SeqCst) == self.0
    }

    fn set_status(self, status: SystemShortcutsStatus) {
        if self.alive() {
            *STATUS.lock() = status;
        }
    }

    fn set_owned(self, bridge: &MacroHotkeyBridge, owned: HashSet<String>) {
        if self.alive() {
            bridge.set_system_owned(owned);
        }
    }

    async fn main(self, bridge: &MacroHotkeyBridge, on_fire: &dyn Fn(String)) {
        if let Ok(app_id) = PORTAL_APP_ID.parse::<ashpd::AppID>() {
            // Fails harmlessly when capture already registered this connection.
            let _ = ashpd::register_host_app(app_id).await;
        }
        let proxy = match GlobalShortcuts::new().await {
            Ok(proxy) => proxy,
            Err(e) => {
                eprintln!("sqyre-hotkeys: GlobalShortcuts portal unavailable ({e}); using evdev");
                return;
            }
        };
        let version = proxy.get_property::<u32>("version").await.unwrap_or(1);
        if self.alive() {
            CONFIGURABLE.store(version >= 2, Ordering::SeqCst);
        }

        let mut declined: Option<BindKey> = None;
        while self.alive() {
            let bindings = bridge.bindings();
            let key = bind_key(&bindings);
            let reask = OPEN_REQUESTED.swap(false, Ordering::SeqCst);
            if bindings.is_empty() || (declined.as_ref() == Some(&key) && !reask) {
                if bindings.is_empty() {
                    self.set_owned(bridge, HashSet::new());
                    self.set_status(SystemShortcutsStatus::Active { bound: 0 });
                }
                async_io::Timer::after(TICK).await;
                continue;
            }
            match self
                .bind_and_listen(&proxy, bridge, on_fire, &bindings)
                .await
            {
                Ok(Outcome::Rebind) => declined = None,
                Ok(Outcome::Declined) => {
                    self.set_owned(bridge, HashSet::new());
                    self.set_status(SystemShortcutsStatus::Declined);
                    declined = Some(key);
                }
                Err(e) => {
                    eprintln!("sqyre-hotkeys: GlobalShortcuts failed ({e}); using evdev");
                    self.set_owned(bridge, HashSet::new());
                    self.set_status(SystemShortcutsStatus::Failed(e.to_string()));
                    return;
                }
            }
        }
    }

    async fn bind_and_listen(
        self,
        proxy: &GlobalShortcuts<'static>,
        bridge: &MacroHotkeyBridge,
        on_fire: &dyn Fn(String),
        bindings: &[MacroHotkeyBinding],
    ) -> Result<Outcome, ashpd::Error> {
        let activated = proxy.receive_activated().await?;
        let deactivated = proxy.receive_deactivated().await?;
        let session = proxy.create_session().await?;
        let shortcuts: Vec<NewShortcut> = bindings
            .iter()
            .map(|b| {
                NewShortcut::new(&b.macro_name, format!("Run macro “{}”", b.macro_name))
                    .preferred_trigger(xdg_trigger(&b.chord).as_deref())
            })
            .collect();
        self.set_status(SystemShortcutsStatus::Waiting);
        let response = proxy
            .bind_shortcuts(&session, &shortcuts, None)
            .await?
            .response();
        let bound = match response {
            Ok(bound) => bound,
            Err(ashpd::Error::Response(ResponseError::Cancelled)) => {
                let _ = session.close().await;
                return Ok(Outcome::Declined);
            }
            Err(e) => {
                let _ = session.close().await;
                return Err(e);
            }
        };
        let owned: HashSet<String> = bound
            .shortcuts()
            .iter()
            .map(|s| s.id().to_string())
            .collect();
        eprintln!(
            "sqyre-hotkeys: GlobalShortcuts bound {}/{} macro hotkeys",
            owned.len(),
            bindings.len()
        );
        self.set_status(SystemShortcutsStatus::Active { bound: owned.len() });
        self.set_owned(bridge, owned);

        let events = stream::select(
            stream::select(
                activated
                    .map(|a| Event::Edge(a.shortcut_id().to_string(), ShortcutEdge::Activated)),
                deactivated
                    .map(|d| Event::Edge(d.shortcut_id().to_string(), ShortcutEdge::Deactivated)),
            ),
            async_io::Timer::interval(TICK).map(|_| Event::Tick),
        );
        let mut events = std::pin::pin!(events);
        let bound_key = bind_key(bindings);
        let mut changed_since: Option<Instant> = None;
        let outcome = loop {
            let Some(event) = events.next().await else {
                break Outcome::Rebind;
            };
            match event {
                Event::Edge(id, edge) => bridge.on_system_shortcut(&id, edge, on_fire),
                Event::Tick => {
                    if !self.alive() {
                        break Outcome::Rebind;
                    }
                    if OPEN_REQUESTED.swap(false, Ordering::SeqCst) {
                        if !system_shortcuts_configurable() {
                            break Outcome::Rebind;
                        }
                        if let Err(e) = configure(proxy, &session).await {
                            eprintln!("sqyre-hotkeys: ConfigureShortcuts failed ({e})");
                        }
                    }
                    if bind_key(&bridge.bindings()) == bound_key {
                        changed_since = None;
                    } else if changed_since.get_or_insert_with(Instant::now).elapsed()
                        >= REBIND_QUIET
                    {
                        break Outcome::Rebind;
                    }
                }
            }
        };
        let _ = session.close().await;
        Ok(outcome)
    }
}

async fn configure(
    proxy: &GlobalShortcuts<'static>,
    session: &Session<'static, GlobalShortcuts<'static>>,
) -> ashpd::zbus::Result<()> {
    let options: HashMap<&str, ashpd::zvariant::Value<'_>> = HashMap::new();
    proxy
        .call_method("ConfigureShortcuts", &(session, "", options))
        .await
        .map(drop)
}

/// Sqyre chord → XDG "shortcuts" trigger (`CTRL+SHIFT+a`). `None` when the
/// chord has no single mappable key; the desktop then asks the user for one.
fn xdg_trigger(chord: &[String]) -> Option<String> {
    let mut mods: Vec<&str> = Vec::new();
    let mut key: Option<&str> = None;
    for k in chord {
        let modifier = match k.as_str() {
            "ctrl" => Some("CTRL"),
            "alt" | "ralt" => Some("ALT"),
            "shift" | "rshift" => Some("SHIFT"),
            "cmd" | "rcmd" | "super" => Some("LOGO"),
            _ => None,
        };
        if let Some(m) = modifier {
            if !mods.contains(&m) {
                mods.push(m);
            }
            continue;
        }
        if key.replace(xkb_keysym(k)?).is_some() {
            return None;
        }
    }
    mods.sort_by_key(|m| ["CTRL", "ALT", "SHIFT", "LOGO"].iter().position(|o| o == m));
    mods.push(key?);
    Some(mods.join("+"))
}

fn xkb_keysym(key: &str) -> Option<&str> {
    let sym = match key {
        "esc" => "Escape",
        "space" => "space",
        "enter" => "Return",
        "tab" => "Tab",
        "delete" => "BackSpace",
        "up" => "Up",
        "down" => "Down",
        "left" => "Left",
        "right" => "Right",
        "home" => "Home",
        "end" => "End",
        "pageup" => "Page_Up",
        "pagedown" => "Page_Down",
        "f1" => "F1",
        "f2" => "F2",
        "f3" => "F3",
        "f4" => "F4",
        "f5" => "F5",
        "f6" => "F6",
        "f7" => "F7",
        "f8" => "F8",
        "f9" => "F9",
        "f10" => "F10",
        "f11" => "F11",
        "f12" => "F12",
        "num0" => "KP_0",
        "num1" => "KP_1",
        "num2" => "KP_2",
        "num3" => "KP_3",
        "num4" => "KP_4",
        "num5" => "KP_5",
        "num6" => "KP_6",
        "num7" => "KP_7",
        "num8" => "KP_8",
        "num9" => "KP_9",
        "num_enter" => "KP_Enter",
        "num_plus" => "KP_Add",
        "num_minus" => "KP_Subtract",
        "num_asterisk" => "KP_Multiply",
        "num_slash" => "KP_Divide",
        "num_period" => "KP_Decimal",
        k if k.len() == 1
            && k.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) =>
        {
            k
        }
        _ => return None,
    };
    Some(sym)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|k| k.to_string()).collect()
    }

    #[test]
    fn trigger_orders_modifiers_and_maps_keysyms() {
        assert_eq!(
            xdg_trigger(&chord(&["a", "ctrl", "shift"])).as_deref(),
            Some("CTRL+SHIFT+a")
        );
        assert_eq!(
            xdg_trigger(&chord(&["cmd", "f9"])).as_deref(),
            Some("LOGO+F9")
        );
        assert_eq!(
            xdg_trigger(&chord(&["rshift", "shift", "num_plus"])).as_deref(),
            Some("SHIFT+KP_Add")
        );
    }

    #[test]
    fn trigger_none_without_single_key() {
        assert_eq!(xdg_trigger(&chord(&["ctrl", "shift"])), None);
        assert_eq!(xdg_trigger(&chord(&["a", "b"])), None);
        assert_eq!(xdg_trigger(&chord(&["ctrl", "mystery"])), None);
    }
}
