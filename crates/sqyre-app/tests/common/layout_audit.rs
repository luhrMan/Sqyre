use crate::common::build_docs_harness;
use eframe::egui::{self, accesskit::Role, Rect};
use egui_kittest::kittest::NodeT;
use fs2::FileExt;
use image::{Rgba, RgbaImage};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

const SIZE_CASES: &[SizeCase] = &[
    SizeCase::new("1920x1080", 1920.0, 1080.0, 1.0),
    SizeCase::new("1280x720", 1280.0, 720.0, 1.0),
    SizeCase::new("1024x640", 1024.0, 640.0, 1.0),
    SizeCase::new("800x600", 800.0, 600.0, 1.0),
    SizeCase::new("640x480", 640.0, 480.0, 1.0),
    SizeCase::new("480x360", 480.0, 360.0, 1.0),
    SizeCase::new("412x915", 412.0, 915.0, 1.0),
    SizeCase::new("915x412", 915.0, 412.0, 1.0),
    SizeCase::new("1280x720@1.5", 1280.0, 720.0, 1.5),
    SizeCase::new("1280x720@2.0", 1280.0, 720.0, 2.0),
];

#[derive(Clone, Copy)]
struct SizeCase {
    label: &'static str,
    size: [f32; 2],
    ui_scale: f32,
}

impl SizeCase {
    const fn new(label: &'static str, width: f32, height: f32, ui_scale: f32) -> Self {
        Self {
            label,
            size: [width / ui_scale, height / ui_scale],
            ui_scale,
        }
    }
}

#[derive(Clone)]
struct Finding {
    scene: String,
    size_label: String,
    kind: String,
    side: Option<String>,
    text: String,
    detail: Option<String>,
    rect: Rect,
    clip: Option<Rect>,
}

impl Finding {
    fn stable_key(&self) -> String {
        match &self.side {
            Some(side) => format!(
                "{}|{}|{}|{}|{}",
                self.scene, self.size_label, self.kind, side, self.text
            ),
            None => format!(
                "{}|{}|{}|{}",
                self.scene, self.size_label, self.kind, self.text
            ),
        }
    }
}

#[derive(Clone)]
struct NodeBounds {
    role: Role,
    label: String,
    rect: Rect,
    window_scope: Option<usize>,
}

#[derive(Clone)]
struct ScrollBounds {
    rect: Rect,
    horizontal: bool,
    vertical: bool,
}

struct AccessibilitySnapshot {
    interactive: Vec<NodeBounds>,
    windows: Vec<WindowBounds>,
    scroll_nodes: Vec<ScrollBounds>,
    scroll_metadata_nodes: usize,
    text_input_rects: Vec<Rect>,
}

#[derive(Clone)]
struct WindowBounds {
    role: Role,
    label: String,
    rect: Rect,
}

pub fn run_scene(scene: &str) {
    let started = Instant::now();
    let audit_render = std::env::var("SQYRE_LAYOUT_AUDIT").as_deref() == Ok("1");
    let mut findings = Vec::new();
    let mut scroll_metadata_nodes = 0;

    for size_case in SIZE_CASES {
        let mut harness = build_docs_harness(size_case.size, |app| {
            app.docs_settings_mut().ui_scale = size_case.ui_scale;
            setup_scene(app, scene);
        });
        let content_rect = harness.ctx.content_rect();
        let mut case_findings = Vec::new();
        if harness.try_run().is_err() {
            harness.run_steps(60);
            if let Err(error) = harness.try_run() {
                let cause_paths = error
                    .repaint_causes
                    .iter()
                    .map(|cause| cause.file)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join("; ");
                let detail = error
                    .repaint_causes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ");
                case_findings.push(Finding {
                    scene: scene.to_owned(),
                    size_label: size_case.label.to_owned(),
                    kind: "no_settle".into(),
                    side: None,
                    text: cause_paths,
                    detail: Some(detail),
                    rect: content_rect,
                    clip: None,
                });
            }
        }

        let content_rect = harness.ctx.content_rect();
        if size_case.ui_scale > 1.0 {
            println!(
                "layout_sweep zoom scene={scene} scale={} content_rect={:.1}x{:.1}",
                size_case.ui_scale,
                content_rect.width(),
                content_rect.height()
            );
        }
        let accessibility = inspect_accesskit(
            harness.root(),
            &harness.ctx,
            scene,
            size_case.label,
            content_rect,
            &mut case_findings,
        );
        scroll_metadata_nodes += accessibility.scroll_metadata_nodes;
        for clipped in &harness.output().shapes {
            inspect_shape(
                &clipped.shape,
                clipped.clip_rect,
                scene,
                size_case.label,
                content_rect,
                &accessibility,
                &mut case_findings,
            );
        }

        let before_windows = accessibility.windows;
        harness.run_steps(10);
        let after_windows = collect_windows(harness.root());
        for (before, after) in before_windows.iter().zip(after_windows.iter()) {
            if after.rect.width() > before.rect.width() + 1.0
                || after.rect.height() > before.rect.height() + 1.0
            {
                case_findings.push(Finding {
                    scene: scene.to_owned(),
                    size_label: size_case.label.to_owned(),
                    kind: "ratchet".into(),
                    side: None,
                    text: format!("{:?} {}", after.role, after.label),
                    detail: Some(format!(
                        "from {:.1}x{:.1} to {:.1}x{:.1}",
                        before.rect.width(),
                        before.rect.height(),
                        after.rect.width(),
                        after.rect.height()
                    )),
                    rect: after.rect,
                    clip: Some(content_rect),
                });
            }
        }

        if audit_render {
            render_case(
                &mut harness,
                scene,
                *size_case,
                &case_findings,
                content_rect,
            );
        }
        findings.extend(case_findings);
    }

    findings.sort_by_key(Finding::stable_key);
    write_outputs(scene, &findings);
    check_baseline(scene, &findings);

    let mut counts = BTreeMap::new();
    for finding in &findings {
        *counts.entry(finding.kind.as_str()).or_insert(0usize) += 1;
    }
    println!(
        "layout_sweep scene={scene} findings={} kinds={counts:?} scroll_metadata_nodes={scroll_metadata_nodes} elapsed={:.2}s",
        findings.len(),
        started.elapsed().as_secs_f32()
    );
}

fn setup_scene(app: &mut sqyre_app::SqyreApp, scene: &str) {
    match scene {
        "main" => {
            app.expand_all_branches_for_docs();
            app.open_macro_list_for_docs();
        }
        "main_no_list" => {
            app.expand_all_branches_for_docs();
            app.set_macro_list_open_for_docs(false);
        }
        "add_action_picker" => app.open_add_action_picker(),
        "command_palette" => app.open_command_palette_for_docs(),
        "data_editor_points_form" => app.open_data_editor(),
        "main_stress" => {
            app.apply_stress_fixture_for_docs();
            app.expand_all_branches_for_docs();
            app.open_macro_list_for_docs();
        }
        "logs_window" => app.open_logs_window_for_docs(),
        "yaml_builder" => app.open_yaml_builder_for_docs(),
        "ai_builder" => app.open_ai_builder_for_docs(),
        "delay_dialog" => app.open_delay_dialog_for_docs(),
        "edit_imagesearch_stress" => {
            app.apply_stress_fixture_for_docs();
            app.open_action_editor_for_docs("imagesearch");
        }
        "edit_type_stress" => {
            app.apply_stress_fixture_for_docs();
            app.open_action_editor_for_docs("type");
        }
        _ if scene.starts_with("settings_") => {
            let section = scene.strip_prefix("settings_").unwrap();
            let label = match section {
                "general" => "General",
                "macros" => "Macros",
                "sound" => "Sound",
                "permissions" => "Permissions",
                "data" => "Data",
                "updates" => "Updates",
                "appearance" => "Appearance",
                _ => panic!("unknown settings scene: {scene}"),
            };
            app.open_settings_section_for_docs(label);
        }
        _ if scene.starts_with("data_editor_") => {
            let tab = scene.strip_prefix("data_editor_").unwrap();
            app.open_data_editor_tab_for_docs(tab);
        }
        _ if scene.starts_with("variables_") => {
            let tab = scene.strip_prefix("variables_").unwrap();
            app.open_variables_panel_for_docs(tab);
        }
        _ if scene.starts_with("edit_") => {
            let type_key = scene.strip_prefix("edit_").unwrap();
            app.open_action_editor_for_docs(type_key);
        }
        _ => panic!("unknown layout scene: {scene}"),
    }
}

fn inspect_shape(
    shape: &egui::Shape,
    clip_rect: Rect,
    scene: &str,
    size_label: &str,
    content_rect: Rect,
    accessibility: &AccessibilitySnapshot,
    findings: &mut Vec<Finding>,
) {
    match shape {
        egui::Shape::Text(text_shape) => {
            let visual_rect = text_shape.visual_bounding_rect();
            let galley_text = text_shape.galley.job.text.as_str();
            if galley_text.trim().is_empty()
                || !visual_rect.is_finite()
                || !visual_rect.is_positive()
            {
                return;
            }
            let text = truncate_text(galley_text, 80);
            let visible = visual_rect.intersect(clip_rect);
            if visible.is_positive() {
                for (axis, side, amount, clip_edge, content_edge) in [
                    (
                        "h",
                        "left",
                        clip_rect.left() - visual_rect.left(),
                        clip_rect.left(),
                        content_rect.left(),
                    ),
                    (
                        "h",
                        "right",
                        visual_rect.right() - clip_rect.right(),
                        clip_rect.right(),
                        content_rect.right(),
                    ),
                    (
                        "v",
                        "top",
                        clip_rect.top() - visual_rect.top(),
                        clip_rect.top(),
                        content_rect.top(),
                    ),
                    (
                        "v",
                        "bottom",
                        visual_rect.bottom() - clip_rect.bottom(),
                        clip_rect.bottom(),
                        content_rect.bottom(),
                    ),
                ] {
                    if amount <= 1.0 {
                        continue;
                    }
                    let screen_cut = (clip_edge - content_edge).abs() <= 1.0;
                    let kind = match (axis, screen_cut) {
                        ("h", true) => "clipped_screen_h",
                        ("v", true) => "clipped_screen_v",
                        ("h", false) => "clipped_h",
                        ("v", false) => "clipped_v",
                        _ => unreachable!(),
                    };
                    let matching_scroll_clip = accessibility.scroll_nodes.iter().any(|scroll| {
                        rects_approx_equal(scroll.rect, clip_rect)
                            && ((axis == "h" && scroll.horizontal)
                                || (axis == "v" && scroll.vertical))
                    });
                    let text_input_clip = axis == "h"
                        && accessibility
                            .text_input_rects
                            .iter()
                            .any(|rect| rect.contains(visual_rect.left_top()));
                    if !screen_cut
                        && ((axis == "v" && accessibility.scroll_metadata_nodes == 0)
                            || matching_scroll_clip
                            || text_input_clip)
                    {
                        continue;
                    }
                    findings.push(Finding {
                        scene: scene.to_owned(),
                        size_label: size_label.to_owned(),
                        kind: kind.into(),
                        side: Some(side.into()),
                        text: text.clone(),
                        detail: Some(format!("{amount:.1}pt")),
                        rect: visual_rect,
                        clip: Some(clip_rect),
                    });
                }

                if !content_rect.intersects(visible) {
                    findings.push(Finding {
                        scene: scene.to_owned(),
                        size_label: size_label.to_owned(),
                        kind: "offscreen".into(),
                        side: None,
                        text: text.clone(),
                        detail: None,
                        rect: visible,
                        clip: Some(content_rect),
                    });
                }
            }

            for rows in text_shape.galley.rows.windows(2) {
                let [first, second] = rows else {
                    continue;
                };
                if first.ends_with_newline {
                    continue;
                }
                let Some(left_char) = first.row.glyphs.last().map(|glyph| glyph.chr) else {
                    continue;
                };
                let Some(right_char) = second.row.glyphs.first().map(|glyph| glyph.chr) else {
                    continue;
                };
                if !left_char.is_alphanumeric() || !right_char.is_alphanumeric() {
                    continue;
                }
                let left = first
                    .row
                    .glyphs
                    .iter()
                    .rev()
                    .take_while(|glyph| glyph.chr.is_alphanumeric())
                    .map(|glyph| glyph.chr)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<String>();
                let right = second
                    .row
                    .glyphs
                    .iter()
                    .take_while(|glyph| glyph.chr.is_alphanumeric())
                    .map(|glyph| glyph.chr)
                    .collect::<String>();
                findings.push(Finding {
                    scene: scene.to_owned(),
                    size_label: size_label.to_owned(),
                    kind: "word_split".into(),
                    side: None,
                    text: text.clone(),
                    detail: Some(format!("{left}{right}")),
                    rect: visual_rect,
                    clip: Some(clip_rect),
                });
            }
        }
        egui::Shape::Vec(shapes) => {
            for child in shapes {
                inspect_shape(
                    child,
                    clip_rect,
                    scene,
                    size_label,
                    content_rect,
                    accessibility,
                    findings,
                );
            }
        }
        _ => {}
    }
}

fn inspect_accesskit(
    root: egui_kittest::Node<'_>,
    ctx: &egui::Context,
    scene: &str,
    size_label: &str,
    content_rect: Rect,
    findings: &mut Vec<Finding>,
) -> AccessibilitySnapshot {
    let mut collector = AccessKitCollector {
        ctx,
        scene,
        size_label,
        content_rect,
        findings,
        snapshot: AccessibilitySnapshot {
            interactive: Vec::new(),
            windows: Vec::new(),
            scroll_nodes: Vec::new(),
            scroll_metadata_nodes: 0,
            text_input_rects: Vec::new(),
        },
        next_window_scope: 0,
    };
    for node in root.children() {
        collector.visit(node, None);
    }

    let snapshot = &collector.snapshot;
    let findings = &mut *collector.findings;
    for (index, first) in snapshot.interactive.iter().enumerate() {
        for second in &snapshot.interactive[index + 1..] {
            if first.window_scope != second.window_scope
                || !first.rect.is_positive()
                || !second.rect.is_positive()
                || first.rect.contains_rect(second.rect)
                || second.rect.contains_rect(first.rect)
            {
                continue;
            }
            let intersection = first.rect.intersect(second.rect);
            let smaller_area = first.rect.area().min(second.rect.area());
            if intersection.area() <= smaller_area * 0.25 {
                continue;
            }
            findings.push(Finding {
                scene: scene.to_owned(),
                size_label: size_label.to_owned(),
                kind: "overlap".into(),
                side: None,
                text: format!("{} ↔ {}", first.label, second.label),
                detail: Some(format!(
                    "{:?} / {:?}; {:.1}%",
                    first.role,
                    second.role,
                    intersection.area() / smaller_area * 100.0
                )),
                rect: intersection,
                clip: Some(content_rect),
            });
        }
    }

    collector.snapshot
}

struct AccessKitCollector<'a> {
    ctx: &'a egui::Context,
    scene: &'a str,
    size_label: &'a str,
    content_rect: Rect,
    findings: &'a mut Vec<Finding>,
    snapshot: AccessibilitySnapshot,
    next_window_scope: usize,
}

impl AccessKitCollector<'_> {
    fn visit(&mut self, node: egui_kittest::Node<'_>, inherited_window_scope: Option<usize>) {
        let accesskit_node = node.accesskit_node();
        let role = accesskit_node.role();
        let is_window = matches!(role, Role::Window | Role::Dialog | Role::AlertDialog);
        let window_scope = if is_window {
            let scope = self.next_window_scope;
            self.next_window_scope += 1;
            Some(scope)
        } else {
            inherited_window_scope
        };
        let scroll_x_min = accesskit_node.scroll_x_min();
        let scroll_x_max = accesskit_node.scroll_x_max();
        let scroll_y_min = accesskit_node.scroll_y_min();
        let scroll_y_max = accesskit_node.scroll_y_max();
        let has_scroll_metadata = role == Role::ScrollView
            || accesskit_node.scroll_x().is_some()
            || scroll_x_min.is_some()
            || scroll_x_max.is_some()
            || accesskit_node.scroll_y().is_some()
            || scroll_y_min.is_some()
            || scroll_y_max.is_some();
        if has_scroll_metadata {
            self.snapshot.scroll_metadata_nodes += 1;
        }
        if accesskit_node.bounding_box().is_some() {
            let rect = node.rect();
            let label = accesskit_node
                .label()
                .filter(|label| !label.is_empty())
                .unwrap_or_else(|| format!("{role:?}"));

            let horizontal = scroll_x_min
                .zip(scroll_x_max)
                .is_some_and(|(min, max)| max > min);
            let vertical = scroll_y_min
                .zip(scroll_y_max)
                .is_some_and(|(min, max)| max > min);
            if has_scroll_metadata && (horizontal || vertical) {
                self.snapshot.scroll_nodes.push(ScrollBounds {
                    rect,
                    horizontal,
                    vertical,
                });
            }

            if matches!(
                role,
                Role::TextInput
                    | Role::MultilineTextInput
                    | Role::SearchInput
                    | Role::ComboBox
                    | Role::EditableComboBox
            ) {
                self.snapshot.text_input_rects.push(rect);
            }

            if is_interactive(role) && rect.is_positive() {
                // AccessKit bounds are the raw widget rect, not clipped by the
                // parent Ui (e.g. a ScrollArea viewport). A scrolled-off widget
                // then reports a rect that overlaps siblings below the scroll.
                // `Response::interact_rect` is rect ∩ parent clip — the part that
                // is actually visible — so use it when it exists.
                let visible = interact_rect(self.ctx, &node).unwrap_or(rect);
                self.snapshot.interactive.push(NodeBounds {
                    role,
                    label: label.clone(),
                    rect: visible,
                    window_scope,
                });
            }

            if is_window {
                self.snapshot.windows.push(WindowBounds {
                    role,
                    label: label.clone(),
                    rect,
                });
                let overflow = overflow_sides(rect, self.content_rect);
                if !overflow.is_empty() {
                    self.findings.push(Finding {
                        scene: self.scene.to_owned(),
                        size_label: self.size_label.to_owned(),
                        kind: "window_overflow".into(),
                        side: None,
                        text: format!("{role:?} {label}"),
                        detail: Some(overflow),
                        rect,
                        clip: Some(self.content_rect),
                    });
                }
            }
        }

        for child in node.children() {
            self.visit(child, window_scope);
        }
    }
}

fn collect_windows(root: egui_kittest::Node<'_>) -> Vec<WindowBounds> {
    let mut windows = Vec::new();
    fn visit(node: egui_kittest::Node<'_>, windows: &mut Vec<WindowBounds>) {
        let accesskit_node = node.accesskit_node();
        let role = accesskit_node.role();
        if matches!(role, Role::Window | Role::Dialog | Role::AlertDialog)
            && accesskit_node.bounding_box().is_some()
        {
            windows.push(WindowBounds {
                role,
                label: accesskit_node
                    .label()
                    .unwrap_or_else(|| format!("{role:?}")),
                rect: node.rect(),
            });
        }
        for child in node.children() {
            visit(child, windows);
        }
    }
    for child in root.children() {
        visit(child, &mut windows);
    }
    windows
}

/// The clip-intersected rect for the egui widget behind an AccessKit node.
fn interact_rect(ctx: &egui::Context, node: &egui_kittest::Node<'_>) -> Option<Rect> {
    // Consumer `NodeId` packs (accesskit id << 64) | tree index, and egui's
    // accesskit id is the widget `Id` value.
    let bits = (u128::from(node.accesskit_node().id()) >> 64) as u64;
    if bits == 0 {
        return None;
    }
    // SAFETY: `bits` came from egui's own `Id::accesskit_id()`, so it is a
    // non-zero `Id` value produced by egui this frame.
    let id = unsafe { egui::Id::from_high_entropy_bits(bits) };
    ctx.read_response(id).map(|response| response.interact_rect)
}

fn rects_approx_equal(first: Rect, second: Rect) -> bool {
    [
        (first.left(), second.left()),
        (first.right(), second.right()),
        (first.top(), second.top()),
        (first.bottom(), second.bottom()),
    ]
    .into_iter()
    .all(|(a, b)| (a - b).abs() <= 1.0)
}

fn is_interactive(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::DefaultButton
            | Role::CheckBox
            | Role::RadioButton
            | Role::TextInput
            | Role::MultilineTextInput
            | Role::SearchInput
            | Role::DateInput
            | Role::DateTimeInput
            | Role::WeekInput
            | Role::MonthInput
            | Role::TimeInput
            | Role::EmailInput
            | Role::NumberInput
            | Role::PasswordInput
            | Role::PhoneNumberInput
            | Role::UrlInput
            | Role::ComboBox
            | Role::EditableComboBox
            | Role::Slider
            | Role::SpinButton
            | Role::Link
            | Role::Tab
            | Role::Switch
            | Role::MenuItem
            | Role::MenuItemCheckBox
            | Role::MenuItemRadio
            | Role::MenuListOption
            | Role::ListBoxOption
            | Role::DisclosureTriangle
            | Role::ColorWell
            | Role::TreeItem
    )
}

fn overflow_sides(rect: Rect, content: Rect) -> String {
    [
        ("left", content.left() - rect.left()),
        ("right", rect.right() - content.right()),
        ("top", content.top() - rect.top()),
        ("bottom", rect.bottom() - content.bottom()),
    ]
    .into_iter()
    .filter(|(_, amount)| *amount > 1.0)
    .map(|(side, amount)| format!("{side} by {amount:.1}pt"))
    .collect::<Vec<_>>()
    .join(", ")
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let mut result = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        result.push('…');
    }
    result
}

fn render_case(
    harness: &mut egui_kittest::Harness<'_, sqyre_app::SqyreApp>,
    scene: &str,
    size_case: SizeCase,
    findings: &[Finding],
    _content_rect: Rect,
) {
    let image_dir = audit_dir().join(scene);
    fs::create_dir_all(&image_dir).expect("create layout image directory");
    let image = harness.render().expect("render layout audit case");
    let image_path = image_dir.join(format!("{}.png", size_case.label));
    image.save(&image_path).expect("save layout audit image");

    let mut annotated = image;
    let pixels_per_point = harness.ctx.pixels_per_point();
    for finding in findings {
        draw_rect(&mut annotated, finding.rect, pixels_per_point);
    }
    let annotated_path = image_dir.join(format!("{}.annotated.png", size_case.label));
    annotated
        .save(&annotated_path)
        .expect("save annotated layout audit image");
}

fn draw_rect(image: &mut RgbaImage, rect: Rect, pixels_per_point: f32) {
    let red = Rgba([255, 0, 0, 255]);
    let width = image.width() as i32;
    let height = image.height() as i32;
    let left = (rect.left() * pixels_per_point).floor() as i32;
    let right = (rect.right() * pixels_per_point).ceil() as i32;
    let top = (rect.top() * pixels_per_point).floor() as i32;
    let bottom = (rect.bottom() * pixels_per_point).ceil() as i32;

    for offset in 0..2 {
        for x in left..right {
            set_pixel(image, x, top + offset, red, width, height);
            set_pixel(image, x, bottom - offset - 1, red, width, height);
        }
        for y in top..bottom {
            set_pixel(image, left + offset, y, red, width, height);
            set_pixel(image, right - offset - 1, y, red, width, height);
        }
    }
}

fn set_pixel(image: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>, width: i32, height: i32) {
    if (0..width).contains(&x) && (0..height).contains(&y) {
        image.put_pixel(x as u32, y as u32, color);
    }
}

fn write_outputs(scene: &str, findings: &[Finding]) {
    let output_dir = audit_dir();
    fs::create_dir_all(&output_dir).expect("create layout audit directory");
    let scene_path = output_dir.join(format!("{scene}.jsonl"));
    let mut scene_file = BufWriter::new(File::create(&scene_path).expect("create scene findings"));
    for finding in findings {
        writeln!(scene_file, "{}", finding_json(finding)).expect("write scene finding");
    }
    scene_file.flush().expect("flush scene findings");

    let lock_path = output_dir.join("findings.lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)
        .expect("open findings lock");
    FileExt::lock_exclusive(&lock).expect("lock aggregate findings");
    let mut scene_paths = fs::read_dir(&output_dir)
        .expect("read layout audit directory")
        .map(|entry| entry.expect("read layout audit entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "jsonl")
                && path
                    .file_name()
                    .is_some_and(|name| name != "findings.jsonl")
        })
        .collect::<Vec<_>>();
    scene_paths.sort();
    let aggregate_tmp = output_dir.join(format!("findings.{}.tmp", std::process::id()));
    let mut aggregate =
        BufWriter::new(File::create(&aggregate_tmp).expect("create aggregate findings"));
    for path in scene_paths {
        let source = File::open(path).expect("open scene findings");
        for line in BufReader::new(source).lines() {
            writeln!(aggregate, "{}", line.expect("read scene finding"))
                .expect("write aggregate finding");
        }
    }
    aggregate.flush().expect("flush aggregate findings");
    fs::rename(aggregate_tmp, output_dir.join("findings.jsonl"))
        .expect("replace aggregate findings");
    FileExt::unlock(&lock).expect("unlock aggregate findings");
}

fn finding_json(finding: &Finding) -> String {
    serde_json::to_string(&json!({
        "scene": finding.scene,
        "size_label": finding.size_label,
        "kind": finding.kind,
        "side": finding.side,
        "text": finding.text,
        "detail": finding.detail,
        "stable_key": finding.stable_key(),
        "rect": rect_values(finding.rect),
        "clip": finding.clip.map(rect_values),
    }))
    .expect("serialize layout finding")
}

fn rect_values(rect: Rect) -> [f32; 4] {
    [rect.left(), rect.top(), rect.right(), rect.bottom()]
}

fn check_baseline(scene: &str, findings: &[Finding]) {
    let baseline_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("layout_baseline.txt");
    let current_keys = findings
        .iter()
        .map(Finding::stable_key)
        .collect::<BTreeSet<_>>();
    let scene_prefix = format!("{scene}|");

    if std::env::var("SQYRE_UPDATE_LAYOUT_BASELINE").as_deref() == Ok("1") {
        update_scene_baseline(&baseline_path, &scene_prefix, &current_keys);
        return;
    }

    if !baseline_path.exists() {
        println!("layout baseline missing; findings for {scene}:");
        for key in &current_keys {
            println!("  {key}");
        }
        return;
    }

    let baseline = fs::read_to_string(&baseline_path).expect("read layout baseline");
    let scene_keys = baseline
        .lines()
        .filter(|key| key.starts_with(&scene_prefix))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let new_findings = current_keys
        .difference(&scene_keys)
        .cloned()
        .collect::<Vec<_>>();
    let fixed_findings = scene_keys
        .difference(&current_keys)
        .cloned()
        .collect::<Vec<_>>();
    if !fixed_findings.is_empty() {
        println!("layout baseline keys no longer found for {scene}: {fixed_findings:?}");
    }
    if !new_findings.is_empty() {
        panic!("layout findings not in baseline for {scene}: {new_findings:?}");
    }
}

fn update_scene_baseline(
    baseline_path: &Path,
    scene_prefix: &str,
    current_keys: &BTreeSet<String>,
) {
    let lock_path = audit_dir().join("baseline.lock");
    fs::create_dir_all(audit_dir()).expect("create baseline lock directory");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)
        .expect("open baseline lock");
    FileExt::lock_exclusive(&lock).expect("lock layout baseline");
    let mut keys = if baseline_path.exists() {
        fs::read_to_string(baseline_path)
            .expect("read layout baseline")
            .lines()
            .map(str::to_owned)
            .filter(|key| !key.starts_with(scene_prefix))
            .collect::<BTreeSet<_>>()
    } else {
        BTreeSet::new()
    };
    keys.extend(current_keys.iter().cloned());
    let content = keys.into_iter().collect::<Vec<_>>().join("\n");
    let temp_path = baseline_path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&temp_path, format!("{content}\n")).expect("write updated layout baseline");
    fs::rename(temp_path, baseline_path).expect("replace layout baseline");
    FileExt::unlock(&lock).expect("unlock layout baseline");
}

fn audit_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("target")
        .join("layout-audit")
}
