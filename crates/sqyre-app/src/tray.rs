//! System tray: Hide application (checkable) / Quit (GSR-style).
//!
//! The title-bar close button quits Sqyre. Hide the window from the tray menu only.

use egui::{Context, ViewportCommand};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

enum TrayCommand {
    SetVisible(bool),
    Quit,
}

/// Retains the OS tray icon for the process lifetime.
pub struct SystemTray {
    cmd_rx: Option<Receiver<TrayCommand>>,
    wake_poller: Mutex<Option<(Arc<AtomicBool>, JoinHandle<()>)>>,
    #[cfg(not(target_arch = "wasm32"))]
    root_window: Option<Arc<winit::window::Window>>,
    #[cfg(not(target_arch = "wasm32"))]
    application_hidden: AtomicBool,
    #[cfg(target_os = "linux")]
    _handle: Option<ksni::blocking::Handle<LinuxTray>>,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    _icon: Option<tray_icon::TrayIcon>,
}

impl SystemTray {
    /// Install the tray. Failures are logged; the UI keeps running.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn install(ctx: Context, root_window: Option<Arc<winit::window::Window>>) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        match install_inner(ctx, cmd_tx, cmd_rx, root_window) {
            Ok(tray) => tray,
            Err(err) => {
                crate::log::warn(format_args!("system tray unavailable: {err}"));
                Self::inactive()
            }
        }
    }

    /// True after the user hid Sqyre from the tray menu.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn application_hidden(&self) -> bool {
        self.application_hidden.load(Ordering::SeqCst)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn application_hidden(&self) -> bool {
        let _ = self;
        false
    }

    /// Apply tray menu actions on the egui/UI thread (`App::logic`, including while hidden).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn poll_commands(&self, ctx: &Context, frame: &eframe::Frame) {
        let Some(rx) = &self.cmd_rx else {
            return;
        };
        let window = frame.winit_window().or(self.root_window.as_ref());
        loop {
            match rx.try_recv() {
                Ok(TrayCommand::SetVisible(visible)) => {
                    set_application_visible(ctx, window, visible);
                    self.application_hidden.store(!visible, Ordering::SeqCst);
                    if visible {
                        self.stop_wake_poller();
                    } else {
                        self.start_wake_poller(ctx);
                    }
                    #[cfg(all(feature = "native-runtime", not(target_arch = "wasm32")))]
                    sqyre_capture::event_log(
                        "SQYRE_TRAY",
                        &[("visible", if visible { "yes" } else { "no" })],
                    );
                }
                Ok(TrayCommand::Quit) => {
                    self.stop_wake_poller();
                    quit_app(ctx, window);
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn poll_commands(&self, _ctx: &Context, _frame: &eframe::Frame) {}

    fn inactive() -> Self {
        Self {
            cmd_rx: None,
            wake_poller: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            root_window: None,
            #[cfg(not(target_arch = "wasm32"))]
            application_hidden: AtomicBool::new(false),
            #[cfg(target_os = "linux")]
            _handle: None,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            _icon: None,
        }
    }

    /// While tray-hidden, egui may not paint; keep `App::logic` alive for show/quit.
    fn start_wake_poller(&self, ctx: &Context) {
        let mut poller = self.wake_poller.lock().expect("tray wake lock");
        if poller.is_some() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let wake = ctx.clone();
        let stop_flag = Arc::clone(&stop);
        let join = thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                wake.request_repaint();
                thread::sleep(Duration::from_millis(250));
            }
        });
        *poller = Some((stop, join));
    }

    fn stop_wake_poller(&self) {
        let mut poller = self.wake_poller.lock().expect("tray wake lock");
        if let Some((stop, join)) = poller.take() {
            stop.store(true, Ordering::Relaxed);
            let _ = join.join();
        }
    }
}

impl Default for SystemTray {
    fn default() -> Self {
        Self::inactive()
    }
}

impl Drop for SystemTray {
    fn drop(&mut self) {
        self.stop_wake_poller();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn show_window(ctx: &Context, window: Option<&Arc<winit::window::Window>>) {
    if let Some(win) = window {
        #[cfg(target_os = "windows")]
        let _ = win.set_minimized(false);
        let _ = win.set_visible(true);
    }
    ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(ViewportCommand::Focus);
    ctx.request_repaint();
}

#[cfg(not(target_arch = "wasm32"))]
fn hide_application(ctx: &Context, window: Option<&Arc<winit::window::Window>>) {
    if let Some(win) = window {
        #[cfg(target_os = "windows")]
        let _ = win.set_minimized(true);
        // Unmap. Do not resize to 1×1 / move off-screen — on sessions where
        // set_visible is ignored that left a vertical Alt-Tab skeleton.
        let _ = win.set_visible(false);
    }
    ctx.send_viewport_cmd(ViewportCommand::Visible(false));
    #[cfg(all(feature = "native-runtime", not(target_arch = "wasm32")))]
    sqyre_capture::event_log("SQYRE_TRAY", &[("hide", "unmap")]);
    ctx.request_repaint();
}

#[cfg(not(target_arch = "wasm32"))]
fn set_application_visible(
    ctx: &Context,
    window: Option<&Arc<winit::window::Window>>,
    visible: bool,
) {
    if visible {
        show_window(ctx, window);
    } else {
        hide_application(ctx, window);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn quit_app(ctx: &Context, window: Option<&Arc<winit::window::Window>>) {
    if let Some(win) = window {
        let _ = win.set_visible(true);
    }
    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(ViewportCommand::Close);
    ctx.request_repaint();
}

fn load_tray_rgba(size: u32) -> Result<(Vec<u8>, u32, u32), String> {
    crate::assets::app_icon_rgba(size).ok_or_else(|| "rasterize tray icon from SVG".into())
}

fn send_tray_command(tx: &Sender<TrayCommand>, wake: &Context, cmd: TrayCommand) {
    if tx.send(cmd).is_ok() {
        wake.request_repaint();
    }
}

#[cfg(target_os = "linux")]
fn install_inner(
    wake: Context,
    cmd_tx: Sender<TrayCommand>,
    cmd_rx: Receiver<TrayCommand>,
    root_window: Option<Arc<winit::window::Window>>,
) -> Result<SystemTray, String> {
    use ksni::blocking::TrayMethods;

    let (rgba, w, h) = load_tray_rgba(32)?;
    let mut argb = rgba;
    for pixel in argb.chunks_exact_mut(4) {
        pixel.rotate_right(1); // RGBA → ARGB
    }
    let icon = ksni::Icon {
        width: w as i32,
        height: h as i32,
        data: argb,
    };

    let tray = LinuxTray {
        icon,
        application_hidden: false,
        cmd_tx,
        wake,
    };
    let handle = tray
        .spawn()
        .map_err(|e| format!("StatusNotifierItem: {e}"))?;

    Ok(SystemTray {
        cmd_rx: Some(cmd_rx),
        wake_poller: Mutex::new(None),
        root_window,
        application_hidden: AtomicBool::new(false),
        _handle: Some(handle),
    })
}

#[cfg(target_os = "linux")]
struct LinuxTray {
    icon: ksni::Icon,
    application_hidden: bool,
    cmd_tx: Sender<TrayCommand>,
    wake: Context,
}

#[cfg(target_os = "linux")]
impl LinuxTray {
    fn toggle_hide(&mut self) {
        self.application_hidden = !self.application_hidden;
        send_tray_command(
            &self.cmd_tx,
            &self.wake,
            TrayCommand::SetVisible(!self.application_hidden),
        );
    }

    fn show_from_tray(&mut self) {
        if !self.application_hidden {
            return;
        }
        self.application_hidden = false;
        send_tray_command(&self.cmd_tx, &self.wake, TrayCommand::SetVisible(true));
    }
}

#[cfg(target_os = "linux")]
impl ksni::Tray for LinuxTray {
    fn id(&self) -> String {
        "sqyre".into()
    }

    fn title(&self) -> String {
        "Sqyre".into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "Sqyre".into(),
            ..Default::default()
        }
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![self.icon.clone()]
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::{CheckmarkItem, MenuItem, StandardItem};
        vec![
            CheckmarkItem {
                label: "Hide application".into(),
                checked: self.application_hidden,
                activate: Box::new(|this: &mut Self| this.toggle_hide()),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|this: &mut Self| {
                    send_tray_command(&this.cmd_tx, &this.wake, TrayCommand::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.show_from_tray();
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn install_inner(
    wake: Context,
    cmd_tx: Sender<TrayCommand>,
    cmd_rx: Receiver<TrayCommand>,
    root_window: Option<Arc<winit::window::Window>>,
) -> Result<SystemTray, String> {
    use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, TrayIconBuilder};

    let (rgba, w, h) = load_tray_rgba(32)?;
    let icon = Icon::from_rgba(rgba, w, h).map_err(|e| format!("tray icon: {e}"))?;

    let menu = Menu::new();
    let hide_item = Box::leak(Box::new(CheckMenuItem::new(
        "Hide application",
        true,
        false,
        None,
    )));
    let quit_item = MenuItem::new("Quit", true, None);
    let hide_id = hide_item.id().clone();
    let quit_id = quit_item.id().clone();
    menu.append(hide_item)
        .map_err(|e| format!("tray menu: {e}"))?;
    menu.append(&PredefinedMenuItem::separator())
        .map_err(|e| format!("tray menu: {e}"))?;
    menu.append(&quit_item)
        .map_err(|e| format!("tray menu: {e}"))?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Sqyre")
        .with_icon(icon)
        .build()
        .map_err(|e| format!("tray build: {e}"))?;

    std::mem::forget(quit_item);

    let application_hidden = Arc::new(AtomicBool::new(false));
    let hidden_flag = application_hidden.clone();
    let wake_thread = wake.clone();
    std::thread::Builder::new()
        .name("sqyre-tray-menu".into())
        .spawn(move || {
            let rx = MenuEvent::receiver();
            while let Ok(event) = rx.recv() {
                if event.id == hide_id {
                    let now_hidden = !hidden_flag.load(Ordering::SeqCst);
                    hidden_flag.store(now_hidden, Ordering::SeqCst);
                    send_tray_command(
                        &cmd_tx,
                        &wake_thread,
                        TrayCommand::SetVisible(!now_hidden),
                    );
                    hide_item.set_checked(now_hidden);
                } else if event.id == quit_id {
                    send_tray_command(&cmd_tx, &wake_thread, TrayCommand::Quit);
                }
            }
        })
        .map_err(|e| format!("tray menu thread: {e}"))?;

    Ok(SystemTray {
        cmd_rx: Some(cmd_rx),
        wake_poller: Mutex::new(None),
        root_window,
        application_hidden: AtomicBool::new(false),
        _icon: Some(tray_icon),
    })
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn install_inner(
    _wake: Context,
    _cmd_tx: Sender<TrayCommand>,
    _cmd_rx: Receiver<TrayCommand>,
    _root_window: Option<Arc<winit::window::Window>>,
) -> Result<SystemTray, String> {
    Err("system tray not supported on this platform".into())
}

#[cfg(test)]
mod tests {
    #[test]
    fn tray_icon_rgba_loads() {
        let (rgba, w, h) = super::load_tray_rgba(32).expect("tray svg");
        assert_eq!(w, 32);
        assert_eq!(h, 32);
        assert_eq!(rgba.len(), 32 * 32 * 4);
    }
}
