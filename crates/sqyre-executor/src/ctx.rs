//! Per-run executor state and injected backends ([`ExecDeps`]).

use crate::backends::{
    AutomationBackend, ContinueKeyWaiter, CoordinateResolver, IconStore, MacroLookup, MoveOptions,
    OcrEngine, ScreenCapturer, WindowFocuser,
};
use crate::error::{FlowSignal, Result};
use sqyre_domain::{Action, ActionId, Macro, ScalarValue};
use sqyre_ports::{ActionHighlighter, ActionLogger, ClickFxSink, FocusedWindow, RuntimeVarSink};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Executor: injected backends ([`ExecDeps`]) plus per-run mutable state.
pub struct Executor<'a> {
    pub deps: ExecDeps<'a>,
    /// Set when a `Stop` action or `Break`/`Continue`-style flow requests halt.
    pub stop_requested: bool,
    /// Keys still held from [`PressState::Down`] / navigate hold (released on run end).
    pub(crate) held_keys: BTreeSet<String>,
    /// Mouse buttons still held from down/hold (released on run end).
    pub(crate) held_buttons: BTreeSet<String>,
    /// Nested [`ActionKind::RunMacro`] call stack (includes the top-level macro name).
    pub(crate) run_macro_stack: Vec<String>,
    /// [`VariableStore::revision`] last handed to [`ExecDeps::runtime_vars`];
    /// `None` until the first publish. Skips re-publishing unchanged variables
    /// after every action.
    pub(crate) published_vars_revision: Option<u64>,
    /// When true, the next image-search capture waits for a newer portal frame.
    /// Cleared after that capture; set again by mouse/keyboard/scroll input so
    /// nested searches without intervening input reuse the cache (fast) while
    /// post-click nested searches still get a fresh frame.
    pub(crate) capture_dirty: bool,
    /// Window that had focus before the last Focus Window switched away from it.
    pub(crate) previous_focus: Option<FocusedWindow>,
    /// Last pointer target this run commanded. Click-effect positions prefer it
    /// over a live query — compositor cursor metadata can lag a just-sent move.
    pub(crate) last_pointer: Option<(i32, i32)>,
}

/// Hard default for nested RunMacro depth when callers omit a custom budget.
pub const DEFAULT_RUN_MACRO_MAX_DEPTH: usize = 32;

/// Hard default for While when `max_iterations` ≤ 0 and callers omit a custom budget.
pub const DEFAULT_WHILE_MAX_ITERATIONS: i32 = 100_000;

impl<'a> Executor<'a> {
    pub fn new(automation: &'a mut dyn AutomationBackend) -> Self {
        Self {
            deps: ExecDeps::new(automation),
            stop_requested: false,
            held_keys: BTreeSet::new(),
            held_buttons: BTreeSet::new(),
            run_macro_stack: Vec::new(),
            published_vars_revision: None,
            // First capture of the run should not reuse a pre-run portal cache.
            capture_dirty: true,
            previous_focus: None,
            last_pointer: None,
        }
    }

    /// Screen may have changed; the next image search should wait for a new frame.
    pub(crate) fn mark_capture_dirty(&mut self) {
        self.capture_dirty = true;
    }

    /// Consume the dirty flag (or honor a wait/repeat forced fresh) for one capture.
    pub(crate) fn take_capture_fresh(&mut self, force: bool) -> bool {
        if force || self.capture_dirty {
            self.capture_dirty = false;
            true
        } else {
            false
        }
    }

    /// Move the pointer and remember the target so presses can report where they
    /// landed even when the backend cannot read the cursor back.
    pub(crate) fn move_pointer_to(&mut self, x: i32, y: i32, opts: MoveOptions) {
        self.deps.automation.move_to(x, y, opts);
        self.last_pointer = Some((x, y));
    }

    pub(crate) fn input_click_down(&mut self, button: &str) -> Result<()> {
        self.mark_capture_dirty();
        self.deps.automation.click(button, true)?;
        self.held_buttons.insert(button.to_string());
        self.emit_click_fx();
        Ok(())
    }

    pub(crate) fn input_click_up(&mut self, button: &str) -> Result<()> {
        self.mark_capture_dirty();
        self.deps.automation.click(button, false)?;
        self.held_buttons.remove(button);
        Ok(())
    }

    /// Press+release with `hold_ms` between edges (macro mouse delay).
    /// Zero-hold taps are intermittently dropped by Wayland/EIS compositors
    /// even when both edges return Ok.
    pub(crate) fn input_click_tap(&mut self, button: &str, hold_ms: i32) -> Result<()> {
        self.input_click_down(button)?;
        if hold_ms > 0 {
            self.interruptible_sleep(hold_ms)?;
        }
        self.input_click_up(button)
    }

    /// Report where a press landed so the shell can flash the click marker.
    /// Prefers the run's commanded pointer target; falls back to a live backend
    /// read for presses with no preceding move.
    fn emit_click_fx(&mut self) {
        let Some(sink) = self.deps.click_fx else {
            return;
        };
        let pos = self
            .last_pointer
            .or_else(|| self.deps.automation.cursor_position());
        if let Some((x, y)) = pos {
            sink.click_at(x, y);
        }
    }

    pub(crate) fn input_key_down(&mut self, key: &str) -> Result<()> {
        self.mark_capture_dirty();
        self.deps.automation.key_down(key)?;
        self.held_keys.insert(key.to_string());
        Ok(())
    }

    pub(crate) fn input_key_up(&mut self, key: &str) -> Result<()> {
        self.mark_capture_dirty();
        self.deps.automation.key_up(key)?;
        self.held_keys.remove(key);
        Ok(())
    }

    /// Best-effort release of any keys/buttons still held when the macro ends.
    pub(crate) fn release_held_inputs(&mut self) {
        let keys = std::mem::take(&mut self.held_keys);
        for key in keys {
            let _ = self.deps.automation.key_up(&key);
        }
        let buttons = std::mem::take(&mut self.held_buttons);
        for button in buttons {
            let _ = self.deps.automation.click(&button, false);
        }
    }

    pub fn check_stopped(&self) -> Result<()> {
        if self.stop_requested
            || self
                .deps
                .stop_flag
                .is_some_and(|f| f.load(Ordering::SeqCst))
        {
            Err(FlowSignal::Stopped.into())
        } else {
            Ok(())
        }
    }

    /// Sleep in ≤50ms chunks, aborting with [`FlowSignal::Stopped`].
    pub fn interruptible_sleep(&mut self, ms: i32) -> Result<()> {
        let mut left = ms.max(0);
        while left > 0 {
            self.check_stopped()?;
            let chunk = left.min(50);
            self.deps.automation.milli_sleep(chunk);
            left -= chunk;
        }
        self.check_stopped()
    }

    /// Push the macro's variables to the live-variables sink, unless they are
    /// unchanged since the last publish (this runs after every action).
    pub(crate) fn publish_runtime_vars(&mut self, macro_: &Macro) {
        let Some(sink) = self.deps.runtime_vars else {
            return;
        };
        let revision = macro_.variables.revision();
        if self.published_vars_revision == Some(revision) {
            return;
        }
        let pairs: Vec<(String, String)> = macro_
            .variables
            .iter()
            .map(|(n, v)| (n.to_string(), v.as_display()))
            .collect();
        sink.publish(&pairs);
        self.published_vars_revision = Some(revision);
    }

    pub fn log(&self, action_id: ActionId, message: impl Into<String>) {
        if let Some(logger) = self.deps.logger {
            logger.log(action_id, message.into());
        }
    }

    /// Build a log line only when a logger is attached.
    pub fn log_with(&self, action_id: ActionId, f: impl FnOnce() -> String) {
        if let Some(logger) = self.deps.logger {
            logger.log(action_id, f());
        }
    }

    pub fn log_images_enabled(&self) -> bool {
        self.deps
            .logger
            .map(|l| l.log_images_enabled())
            .unwrap_or(false)
    }

    pub fn log_verbose_enabled(&self) -> bool {
        self.deps
            .logger
            .map(|l| l.log_verbose_enabled())
            .unwrap_or(false)
    }

    /// Log a line whose *value* is expensive to produce. `f` runs only when
    /// verbose diagnostics are on.
    pub fn log_verbose(&self, action_id: ActionId, f: impl FnOnce() -> String) {
        let Some(logger) = self.deps.logger else {
            return;
        };
        if !logger.log_verbose_enabled() {
            return;
        }
        logger.log(action_id, f());
    }

    /// Log a labeled capture. Returns the stored [`LogImage`] (post-downscale)
    /// so callers can attach it to find contexts.
    pub fn log_image(
        &self,
        action_id: ActionId,
        label: impl Into<String>,
        image: &sqyre_match::ImageBuf,
    ) -> Option<sqyre_ports::LogImage> {
        self.log_image_annotated(action_id, label, image, Vec::new())
    }

    /// Log a labeled capture carrying in-image annotations (e.g. every OCR
    /// word box) so the Logs zoom view can click-inspect them.
    pub fn log_image_annotated(
        &self,
        action_id: ActionId,
        label: impl Into<String>,
        image: &sqyre_match::ImageBuf,
        annotations: Vec<sqyre_ports::ImageAnnotation>,
    ) -> Option<sqyre_ports::LogImage> {
        let logger = self.deps.logger?;
        if !logger.log_images_enabled() {
            return None;
        }
        let mut image = crate::log_draw::image_buf_to_log_image(
            label.into(),
            image,
            self.deps.log_image_max_edge,
        )?;
        image.annotations = annotations;
        logger.log_image(action_id, &image);
        Some(image)
    }

    /// Log a text line carrying interactive find markers (hover/click "where
    /// found" in the Logs window).
    pub fn log_finds(&self, action_id: ActionId, text: String, finds: Vec<sqyre_ports::LogFind>) {
        if let Some(logger) = self.deps.logger {
            logger.log_finds(action_id, text, finds);
        }
    }

    /// Start a new collapsible section in the action's log — the Logs window
    /// renders one group per detection attempt.
    pub fn log_section(&self, action_id: ActionId, label: String) {
        if let Some(logger) = self.deps.logger {
            logger.log_section(action_id, label);
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "pipeline payload: title, summary, images, detail lines, and find markers"
    )]
    pub fn log_item_pipeline(
        &self,
        action_id: ActionId,
        title: impl Into<String>,
        summary: impl Into<String>,
        thumbnail: &sqyre_match::ImageBuf,
        steps: &[(&str, &sqyre_match::ImageBuf)],
        details: Vec<String>,
        finds: Vec<sqyre_ports::LogFind>,
    ) {
        let Some(logger) = self.deps.logger else {
            return;
        };
        if !logger.log_images_enabled() {
            return;
        }
        let title = title.into();
        let max_edge = self.deps.log_image_max_edge;
        let Some(thumbnail) =
            crate::log_draw::image_buf_to_log_image(format!("Item — {title}"), thumbnail, max_edge)
        else {
            return;
        };
        let steps: Vec<_> = steps
            .iter()
            .filter_map(|(label, img)| {
                crate::log_draw::image_buf_to_log_image((*label).to_string(), img, max_edge)
            })
            .collect();
        logger.log_item_pipeline(
            action_id,
            title,
            summary.into(),
            &thumbnail,
            &steps,
            details,
            finds,
        );
    }

    /// Record how long a named step took (shown in the action logs UI).
    pub fn log_timing(&self, action_id: ActionId, step: &str, elapsed: Duration) {
        self.log(
            action_id,
            format!("timing: {step} {:.1}ms", elapsed.as_secs_f64() * 1000.0),
        );
    }

    /// Time a fallible step and log its duration even when it errors.
    pub fn timed_step<T>(&self, action_id: ActionId, step: &str, f: impl FnOnce() -> T) -> T {
        let started = Instant::now();
        let out = f();
        self.log_timing(action_id, step, started.elapsed());
        out
    }
}

/// Dependencies for a full macro run.
pub struct ExecDeps<'a> {
    pub automation: &'a mut dyn AutomationBackend,
    pub capturer: Option<&'a mut dyn ScreenCapturer>,
    /// Spatial dedup distance for image-search peaks; `0` uses the library default.
    pub close_matches_distance: i32,
    /// When true, stop trying later icon variants after one hits on the same search.
    pub variant_exit_early: bool,
    /// Release keys/buttons still held when the macro ends (success, stop, or error).
    pub release_held_inputs: bool,
    /// Safety budget for While actions with `max_iterations` ≤ 0.
    pub while_max_iterations: i32,
    /// Max nested RunMacro depth (including the top-level macro).
    pub run_macro_max_depth: usize,
    /// Long-edge cap for images stored in the action log.
    pub log_image_max_edge: usize,
    pub resolver: Option<&'a dyn CoordinateResolver>,
    pub icons: Option<&'a dyn IconStore>,
    pub macros: Option<&'a dyn MacroLookup>,
    pub continue_waiter: Option<&'a dyn ContinueKeyWaiter>,
    pub window_focuser: Option<&'a dyn WindowFocuser>,
    pub ocr: Option<&'a dyn OcrEngine>,
    pub stop_flag: Option<&'a AtomicBool>,
    pub logger: Option<&'a dyn ActionLogger>,
    pub highlighter: Option<&'a dyn ActionHighlighter>,
    pub runtime_vars: Option<&'a dyn RuntimeVarSink>,
    pub variables_dir: Option<&'a Path>,
    /// Receives each mouse-press position for the desktop click-effect flash.
    pub click_fx: Option<&'a dyn ClickFxSink>,
}

impl<'a> ExecDeps<'a> {
    /// Backends with only automation wired; everything else absent.
    pub fn new(automation: &'a mut dyn AutomationBackend) -> Self {
        Self {
            automation,
            capturer: None,
            close_matches_distance: 0,
            variant_exit_early: true,
            release_held_inputs: true,
            while_max_iterations: DEFAULT_WHILE_MAX_ITERATIONS,
            run_macro_max_depth: DEFAULT_RUN_MACRO_MAX_DEPTH,
            log_image_max_edge: crate::log_draw::DEFAULT_LOG_IMAGE_MAX_EDGE,
            resolver: None,
            icons: None,
            macros: None,
            continue_waiter: None,
            window_focuser: None,
            ocr: None,
            stop_flag: None,
            logger: None,
            highlighter: None,
            runtime_vars: None,
            variables_dir: None,
            click_fx: None,
        }
    }

    pub fn capturer(mut self, c: &'a mut dyn ScreenCapturer) -> Self {
        self.capturer = Some(c);
        self
    }

    pub fn resolver(mut self, r: &'a dyn CoordinateResolver) -> Self {
        self.resolver = Some(r);
        self
    }

    pub fn icons(mut self, i: &'a dyn IconStore) -> Self {
        self.icons = Some(i);
        self
    }

    pub fn ocr(mut self, o: &'a dyn OcrEngine) -> Self {
        self.ocr = Some(o);
        self
    }

    pub fn logger(mut self, l: &'a dyn ActionLogger) -> Self {
        self.logger = Some(l);
        self
    }

    pub fn stop_flag(mut self, f: &'a AtomicBool) -> Self {
        self.stop_flag = Some(f);
        self
    }

    pub fn continue_waiter(mut self, w: &'a dyn ContinueKeyWaiter) -> Self {
        self.continue_waiter = Some(w);
        self
    }

    pub fn window_focuser(mut self, f: &'a dyn WindowFocuser) -> Self {
        self.window_focuser = Some(f);
        self
    }

    pub fn highlighter(mut self, h: &'a dyn ActionHighlighter) -> Self {
        self.highlighter = Some(h);
        self
    }

    pub fn macros(mut self, m: &'a dyn MacroLookup) -> Self {
        self.macros = Some(m);
        self
    }

    pub fn close_matches_distance(mut self, d: i32) -> Self {
        self.close_matches_distance = d;
        self
    }

    pub fn variant_exit_early(mut self, enabled: bool) -> Self {
        self.variant_exit_early = enabled;
        self
    }

    pub fn click_fx(mut self, s: &'a dyn ClickFxSink) -> Self {
        self.click_fx = Some(s);
        self
    }

    pub fn release_held_inputs(mut self, enabled: bool) -> Self {
        self.release_held_inputs = enabled;
        self
    }

    pub fn while_max_iterations(mut self, max: i32) -> Self {
        self.while_max_iterations = max;
        self
    }

    pub fn run_macro_max_depth(mut self, max: usize) -> Self {
        self.run_macro_max_depth = max;
        self
    }

    pub fn log_image_max_edge(mut self, edge: usize) -> Self {
        self.log_image_max_edge = edge;
        self
    }

    pub fn runtime_vars(mut self, v: &'a dyn RuntimeVarSink) -> Self {
        self.runtime_vars = Some(v);
        self
    }

    pub fn variables_dir(mut self, dir: &'a Path) -> Self {
        self.variables_dir = Some(dir);
        self
    }
}

/// Runs a list of child actions — injected into flow/navigate so those modules
/// do not import `run` and recreate a cycle.
pub(crate) type RunChildrenFn = fn(&mut Executor<'_>, &[Action], &mut Macro) -> Result<()>;

/// Set `monitorNWidth` / `monitorNHeight` builtins.
pub(crate) fn apply_monitor_sizes(macro_: &mut Macro, sizes: &[(i32, i32)]) {
    if sizes.is_empty() {
        macro_.variables.set("monitor1Width", ScalarValue::Int(0));
        macro_.variables.set("monitor1Height", ScalarValue::Int(0));
        return;
    }
    for (i, (w, h)) in sizes.iter().enumerate() {
        let n = i + 1;
        macro_
            .variables
            .set(format!("monitor{n}Width"), ScalarValue::Int(i64::from(*w)));
        macro_
            .variables
            .set(format!("monitor{n}Height"), ScalarValue::Int(i64::from(*h)));
    }
}
