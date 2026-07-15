//! System tray: hide-on-close, Show / Quit menu.

use egui::{Context, ViewportCommand};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Retains the OS tray icon for the process lifetime.
pub struct SystemTray {
    active: bool,
    quit_requested: Arc<AtomicBool>,
    #[cfg(target_os = "linux")]
    _handle: Option<ksni::blocking::Handle<LinuxTray>>,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    _icon: Option<tray_icon::TrayIcon>,
}

impl SystemTray {
    /// Install the tray. Failures are logged; the UI keeps running without hide-on-close.
    pub fn install(ctx: Context) -> Self {
        match install_inner(ctx) {
            Ok(tray) => tray,
            Err(err) => {
                eprintln!("system tray unavailable: {err}");
                Self::inactive()
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// True after the tray Quit item was chosen — close should exit, not hide.
    pub fn quit_requested(&self) -> bool {
        self.quit_requested.load(Ordering::SeqCst)
    }

    fn inactive() -> Self {
        Self {
            active: false,
            quit_requested: Arc::new(AtomicBool::new(false)),
            #[cfg(target_os = "linux")]
            _handle: None,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            _icon: None,
        }
    }
}

impl Default for SystemTray {
    fn default() -> Self {
        Self::inactive()
    }
}

fn show_window(ctx: &Context) {
    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(ViewportCommand::Focus);
    ctx.request_repaint();
}

fn quit_app(ctx: &Context, quit_requested: &AtomicBool) {
    quit_requested.store(true, Ordering::SeqCst);
    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(ViewportCommand::Close);
    ctx.request_repaint();
}

fn load_tray_rgba(size: u32) -> Result<(Vec<u8>, u32, u32), String> {
    let img = image::load_from_memory(crate::assets::APP_ICON_PNG)
        .map_err(|e| format!("decode tray icon: {e}"))?
        .resize(size, size, image::imageops::FilterType::Lanczos3)
        .into_rgba8();
    let (w, h) = img.dimensions();
    Ok((img.into_raw(), w, h))
}

#[cfg(target_os = "linux")]
fn install_inner(ctx: Context) -> Result<SystemTray, String> {
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

    let quit_requested = Arc::new(AtomicBool::new(false));
    let tray = LinuxTray {
        ctx,
        icon,
        quit_requested: quit_requested.clone(),
    };
    let handle = tray
        .spawn()
        .map_err(|e| format!("StatusNotifierItem: {e}"))?;

    Ok(SystemTray {
        active: true,
        quit_requested,
        _handle: Some(handle),
    })
}

#[cfg(target_os = "linux")]
struct LinuxTray {
    ctx: Context,
    icon: ksni::Icon,
    quit_requested: Arc<AtomicBool>,
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
        use ksni::menu::{MenuItem, StandardItem};
        vec![
            StandardItem {
                label: "Show".into(),
                activate: Box::new(|this: &mut Self| show_window(&this.ctx)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|this: &mut Self| {
                    quit_app(&this.ctx, &this.quit_requested);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        show_window(&self.ctx);
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn install_inner(ctx: Context) -> Result<SystemTray, String> {
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, TrayIconBuilder};

    let (rgba, w, h) = load_tray_rgba(32)?;
    let icon = Icon::from_rgba(rgba, w, h).map_err(|e| format!("tray icon: {e}"))?;

    let menu = Menu::new();
    let show_item = MenuItem::new("Show", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let show_id = show_item.id().clone();
    let quit_id = quit_item.id().clone();
    menu.append(&show_item)
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

    // Keep menu items alive for the tray lifetime.
    std::mem::forget(show_item);
    std::mem::forget(quit_item);

    let quit_requested = Arc::new(AtomicBool::new(false));
    let quit_flag = quit_requested.clone();
    let ctx_clone = ctx.clone();
    std::thread::Builder::new()
        .name("sqyre-tray-menu".into())
        .spawn(move || {
            let rx = MenuEvent::receiver();
            while let Ok(event) = rx.recv() {
                if event.id == show_id {
                    show_window(&ctx_clone);
                } else if event.id == quit_id {
                    quit_app(&ctx_clone, &quit_flag);
                }
            }
        })
        .map_err(|e| format!("tray menu thread: {e}"))?;

    Ok(SystemTray {
        active: true,
        quit_requested,
        _icon: Some(tray_icon),
    })
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn install_inner(_ctx: Context) -> Result<SystemTray, String> {
    Err("system tray not supported on this platform".into())
}

#[cfg(test)]
mod tests {
    #[test]
    fn tray_icon_rgba_loads() {
        let (rgba, w, h) = super::load_tray_rgba(32).expect("tray png");
        assert_eq!(w, 32);
        assert_eq!(h, 32);
        assert_eq!(rgba.len(), 32 * 32 * 4);
    }
}
