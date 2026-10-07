//! Android stand-in for the desktop point / search-area grab.
//!
//! Sqyre fills the phone and never sees touches meant for other apps, so recording
//! shows a still of the app used before Sqyre to place the point or area on. The
//! still is the projection backdrop (last frame seen while Sqyre was hidden); Retake
//! sends Sqyre to the back, captures the app that comes to the front, and returns.

use crate::theme;
use eframe::egui::{self, Color32, ColorImage, Order, Pos2, Rect, TextureHandle, TextureOptions};
use sqyre_android::{bridge, frames, AndroidError, Frame, Projection};
use sqyre_hotkeys::ScreenClickBridge;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

const LAYER: &str = "sqyre_screen_pick";
const BAR: &str = "sqyre_screen_pick_bar";
/// Time to answer the screen-recording prompt.
const CONSENT_WAIT: Duration = Duration::from_secs(60);
/// Time for the previous app to cover Sqyre.
const SWITCH_WAIT: Duration = Duration::from_secs(5);
/// Lets the switch animation finish before the still is taken.
const APP_SETTLE: Duration = Duration::from_millis(800);
const FRAME_WAIT: Duration = Duration::from_secs(3);
const POLL: Duration = Duration::from_millis(50);
const MARGIN: f32 = 12.0;
const BAR_PAD: i8 = 12;
const LOUPE_SIDE: f32 = 128.0;
const LOUPE_ZOOM: f32 = 4.0;
const RING_RADIUS: f32 = 10.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Point,
    Area,
}

/// What is placed on the still, in display pixels.
enum Selection {
    Point(Pos2),
    /// Drag start and current corner; `None` until the first drag.
    Area(Option<(Pos2, Pos2)>),
}

impl Selection {
    fn new(mode: Mode, size: egui::Vec2) -> Self {
        match mode {
            Mode::Point => Self::Point((size / 2.0).to_pos2()),
            Mode::Area => Self::Area(None),
        }
    }

    /// Where the loupe magnifies: the point, or the corner being dragged.
    fn focus(&self) -> Option<Pos2> {
        match self {
            Self::Point(p) => Some(*p),
            Self::Area(area) => area.map(|(_, corner)| corner),
        }
    }

    fn can_save(&self) -> bool {
        match self {
            Self::Point(_) => true,
            Self::Area(area) => area.is_some_and(|(a, b)| {
                let (a, b) = (round(a), round(b));
                a.0 != b.0 && a.1 != b.1
            }),
        }
    }
}

#[derive(Default)]
enum State {
    #[default]
    Idle,
    Capturing(Receiver<Result<Frame, String>>),
    Picking {
        texture: TextureHandle,
        /// Frame size in display pixels.
        size: egui::Vec2,
        selection: Selection,
    },
    Failed(String),
}

enum Choice {
    None,
    Save,
    Cancel,
    Retake,
}

#[derive(Default)]
pub struct ScreenPick {
    state: State,
}

impl ScreenPick {
    /// Call every frame. Runs only while a point or search-area recording is armed.
    pub fn sync(&mut self, ctx: &egui::Context, screen_click: &ScreenClickBridge) {
        let mode = if screen_click.peek_point_draft().is_some() {
            Mode::Point
        } else if screen_click.peek_search_area_draft().is_some() {
            Mode::Area
        } else {
            self.state = State::Idle;
            return;
        };
        if matches!(self.state, State::Idle) {
            self.state = match frames().backdrop() {
                Some((_, frame)) => picking(ctx, &frame, mode),
                None => retake(ctx),
            };
        }
        let choice = match &mut self.state {
            State::Idle => Choice::None,
            State::Capturing(rx) => match rx.try_recv() {
                Ok(Ok(frame)) => {
                    self.state = picking(ctx, &frame, mode);
                    Choice::None
                }
                Ok(Err(message)) => {
                    self.state = State::Failed(message);
                    Choice::None
                }
                Err(TryRecvError::Disconnected) => {
                    self.state = State::Failed("Taking the picture failed.".into());
                    Choice::None
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(POLL);
                    paint_bar(ctx, |ui| {
                        ui.label("Taking a picture of the previous app…");
                        cancel_row(ui)
                    })
                }
            },
            State::Picking {
                texture,
                size,
                selection,
            } => paint_pick(ctx, texture, *size, selection),
            State::Failed(message) => {
                let message = message.clone();
                paint_bar(ctx, |ui| {
                    ui.colored_label(theme::error_fg(), message);
                    let mut choice = cancel_row(ui);
                    if ui.button("Try again").clicked() {
                        choice = Choice::Retake;
                    }
                    choice
                })
            }
        };
        match choice {
            Choice::None => {}
            Choice::Save => {
                if let State::Picking { selection, .. } = &self.state {
                    save(screen_click, selection);
                }
                self.state = State::Idle;
            }
            Choice::Cancel => {
                screen_click.on_escape();
                self.state = State::Idle;
            }
            Choice::Retake => self.state = retake(ctx),
        }
    }
}

fn save(screen_click: &ScreenClickBridge, selection: &Selection) {
    match selection {
        Selection::Point(p) => {
            let (x, y) = round(*p);
            screen_click.on_left_click_at(x, y);
        }
        Selection::Area(Some((a, b))) => {
            let (ax, ay) = round(*a);
            let (bx, by) = round(*b);
            // First click sets the corner the area is anchored to, the second completes it.
            screen_click.on_left_click_at(ax, ay);
            screen_click.on_left_click_at(bx, by);
        }
        Selection::Area(None) => {}
    }
}

fn round(p: Pos2) -> (i32, i32) {
    (p.x.round() as i32, p.y.round() as i32)
}

fn picking(ctx: &egui::Context, frame: &Frame, mode: Mode) -> State {
    let size = [frame.width() as usize, frame.height() as usize];
    let image = ColorImage::from_rgba_unmultiplied(size, frame.rgba());
    let texture = ctx.load_texture(LAYER, image, TextureOptions::LINEAR);
    let size = egui::vec2(frame.width() as f32, frame.height() as f32);
    State::Picking {
        texture,
        size,
        selection: Selection::new(mode, size),
    }
}

/// Take a new still of the previous app on a worker thread.
fn retake(ctx: &egui::Context) -> State {
    let (tx, rx) = mpsc::channel();
    let ctx = ctx.clone();
    thread::spawn(move || {
        let result = capture_previous_app();
        // A failed send means the recording was cancelled meanwhile.
        let _ = tx.send(result);
        ctx.request_repaint();
    });
    State::Capturing(rx)
}

/// Worker thread: send Sqyre to the back, take the app now in front, then return.
fn capture_previous_app() -> Result<Frame, String> {
    ensure_projection()?;
    bridge::show_previous().map_err(|e| format!("Could not switch to the previous app: {e}."))?;
    if !wait_until(SWITCH_WAIT, || !frames().shell_visible()) {
        return Err("The previous app did not come to the front.".into());
    }
    thread::sleep(APP_SETTLE);
    let frame = settled_frame();
    if let Err(e) = bridge::show_shell() {
        // The still is ready either way; switching back to Sqyre by hand shows it.
        crate::log::warn(format_args!("could not bring Sqyre back: {e}"));
    }
    frame
}

/// Ask for screen recording when it is not running and wait for the answer.
fn ensure_projection() -> Result<(), String> {
    let store = frames();
    if store.projection() == Projection::Running {
        return Ok(());
    }
    store.rearm();
    bridge::request_projection()
        .map_err(|e| format!("Could not ask for screen recording: {e}."))?;
    let deadline = Instant::now() + CONSENT_WAIT;
    loop {
        match store.projection() {
            Projection::Running => return Ok(()),
            Projection::Stopped => {
                return Err("Screen recording was not allowed, so there is no screen to pick on. Allow it to record points and search areas.".into());
            }
            Projection::NotStarted if Instant::now() >= deadline => {
                return Err("Screen recording was not allowed in time.".into());
            }
            Projection::NotStarted => thread::sleep(POLL),
        }
    }
}

/// Newest frame; a screen that is not changing sends none, so the last one is used.
fn settled_frame() -> Result<Frame, String> {
    let deadline = Instant::now() + FRAME_WAIT;
    loop {
        match frames().fresh(Duration::from_millis(250)) {
            Ok(frame) => return Ok(frame),
            Err(AndroidError::NoFrame) if Instant::now() < deadline => {}
            Err(e) => return Err(format!("Could not take the screen: {e}.")),
        }
    }
}

fn wait_until(timeout: Duration, done: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while !done() {
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(POLL);
    }
    true
}

fn paint_pick(
    ctx: &egui::Context,
    texture: &TextureHandle,
    size: egui::Vec2,
    selection: &mut Selection,
) -> Choice {
    let viewport = ctx.viewport_rect();
    egui::Area::new(egui::Id::new(LAYER))
        .order(Order::Foreground)
        .fixed_pos(viewport.min)
        .constrain(false)
        .show(ctx, |ui| {
            let (rect, resp) =
                ui.allocate_exact_size(viewport.size(), egui::Sense::click_and_drag());
            let image = fit(size, rect);
            let px_per_point = size.x / image.width().max(1.0);
            let max = (size - egui::Vec2::splat(1.0)).to_pos2();
            let to_px =
                |pos: Pos2| (Pos2::ZERO + (pos - image.min) * px_per_point).clamp(Pos2::ZERO, max);
            match selection {
                Selection::Point(point) => {
                    if resp.dragged() {
                        *point += resp.drag_delta() * px_per_point;
                    } else if resp.clicked() {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            *point = to_px(pos);
                        }
                    }
                    *point = point.clamp(Pos2::ZERO, max);
                }
                Selection::Area(area) => {
                    if resp.drag_started() {
                        if let Some(start) = ui.input(|i| i.pointer.press_origin()) {
                            let start = to_px(start);
                            *area = Some((start, start));
                        }
                    }
                    if resp.dragged() {
                        if let (Some((_, corner)), Some(pos)) =
                            (area.as_mut(), resp.interact_pointer_pos())
                        {
                            *corner = to_px(pos);
                        }
                    }
                }
            }
            resp.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Other,
                    true,
                    match selection {
                        Selection::Point(_) => "Screen to pick a point on",
                        Selection::Area(_) => "Screen to draw a search area on",
                    },
                )
            });

            let painter = ui.painter();
            painter.rect_filled(rect, 0.0, Color32::BLACK);
            painter.image(texture.id(), image, unit_uv(), Color32::WHITE);
            let to_screen = |p: Pos2| image.min + p.to_vec2() / px_per_point;
            match selection {
                Selection::Point(point) => paint_crosshair(painter, image, to_screen(*point)),
                Selection::Area(Some((a, b))) => {
                    paint_area(painter, Rect::from_two_pos(to_screen(*a), to_screen(*b)));
                }
                Selection::Area(None) => {}
            }
            if let Some(focus) = selection.focus() {
                paint_loupe(
                    ctx,
                    painter,
                    texture,
                    size,
                    px_per_point,
                    focus,
                    to_screen(focus),
                );
            }
        });

    let (hint, coords) = match selection {
        Selection::Point(p) => {
            let (x, y) = round(*p);
            (
                "Tap to place the point. Drag to fine-tune it.",
                format!("X: {x}, Y: {y}"),
            )
        }
        Selection::Area(area) => (
            "Drag from one corner of the area to the opposite corner.",
            match area {
                Some((a, b)) => {
                    let ((ax, ay), (bx, by)) = (round(*a), round(*b));
                    format!(
                        "({}, {}) – ({}, {})",
                        ax.min(bx),
                        ay.min(by),
                        ax.max(bx),
                        ay.max(by)
                    )
                }
                None => "No area drawn yet".to_string(),
            },
        ),
    };
    let can_save = selection.can_save();
    paint_bar(ctx, |ui| {
        ui.label(egui::RichText::new("Screen of the previous app").strong());
        ui.label(hint);
        ui.weak(coords);
        let mut choice = Choice::None;
        ui.horizontal(|ui| {
            if ui
                .button("Retake")
                .on_hover_text("Switch to the previous app and take a new picture of it")
                .clicked()
            {
                choice = Choice::Retake;
            }
            match crate::widgets::save_cancel_row(ui, can_save) {
                crate::widgets::SaveCancel::Save => choice = Choice::Save,
                crate::widgets::SaveCancel::Cancel => choice = Choice::Cancel,
                crate::widgets::SaveCancel::None => {}
            }
        });
        choice
    })
}

/// Panel along the bottom of the safe area. Tooltip order keeps it above the still,
/// which moves to the top of its own order whenever it is touched.
fn paint_bar(ctx: &egui::Context, add: impl FnOnce(&mut egui::Ui) -> Choice) -> Choice {
    let content = ctx.content_rect();
    let width = (content.width() - MARGIN * 2.0).max(1.0);
    egui::Area::new(egui::Id::new(BAR))
        .order(Order::Tooltip)
        .pivot(egui::Align2::CENTER_BOTTOM)
        .fixed_pos(egui::pos2(content.center().x, content.bottom() - MARGIN))
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(theme::overlay_panel_fill())
                .stroke(egui::Stroke::new(1.0, theme::PRIMARY))
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::same(BAR_PAD))
                .show(ui, |ui| {
                    ui.set_width((width - f32::from(BAR_PAD) * 2.0 - 2.0).max(1.0));
                    add(ui)
                })
                .inner
        })
        .inner
}

fn cancel_row(ui: &mut egui::Ui) -> Choice {
    if crate::widgets::consume_escape(ui) || ui.button("Cancel").clicked() {
        Choice::Cancel
    } else {
        Choice::None
    }
}

/// Largest rect with the frame's aspect ratio centered in `area`.
fn fit(size: egui::Vec2, area: Rect) -> Rect {
    let scale = (area.width() / size.x.max(1.0)).min(area.height() / size.y.max(1.0));
    Rect::from_center_size(area.center(), size * scale)
}

fn unit_uv() -> Rect {
    Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0))
}

fn outline_strokes() -> [egui::Stroke; 2] {
    [
        egui::Stroke::new(3.0, Color32::BLACK),
        egui::Stroke::new(1.0, theme::PRIMARY),
    ]
}

fn paint_crosshair(painter: &egui::Painter, image: Rect, at: Pos2) {
    for stroke in outline_strokes() {
        painter.hline(image.x_range(), at.y, stroke);
        painter.vline(at.x, image.y_range(), stroke);
        painter.circle_stroke(at, RING_RADIUS, stroke);
    }
}

fn paint_area(painter: &egui::Painter, area: Rect) {
    painter.rect_filled(area, 0.0, theme::accent_dim());
    for stroke in outline_strokes() {
        painter.rect_stroke(area, 0.0, stroke, egui::StrokeKind::Middle);
    }
}

/// Magnified view around `point`, in the top corner away from it.
fn paint_loupe(
    ctx: &egui::Context,
    painter: &egui::Painter,
    texture: &TextureHandle,
    size: egui::Vec2,
    px_per_point: f32,
    point: Pos2,
    at: Pos2,
) {
    let content = ctx.content_rect().shrink(MARGIN);
    let side = egui::Vec2::splat(LOUPE_SIDE);
    let left = Rect::from_min_size(content.min, side);
    let loupe = if left.expand(RING_RADIUS).contains(at) {
        Rect::from_min_size(
            egui::pos2(content.right() - LOUPE_SIDE, content.top()),
            side,
        )
    } else {
        left
    };
    let span = LOUPE_SIDE / LOUPE_ZOOM * px_per_point;
    let uv = Rect::from_center_size(
        egui::pos2(point.x / size.x, point.y / size.y),
        egui::vec2(span / size.x, span / size.y),
    );
    painter.rect_filled(loupe, 4.0, Color32::BLACK);
    painter.image(texture.id(), loupe, uv, Color32::WHITE);
    paint_crosshair(painter, loupe, loupe.center());
    painter.rect_stroke(
        loupe,
        4.0,
        egui::Stroke::new(2.0, theme::PRIMARY),
        egui::StrokeKind::Outside,
    );
}
