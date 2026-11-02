//! Runs every desktop-facing action through the real executor and Windows
//! backends (GDI capture, SendInput, Win32 focus, OCR) against a WinForms
//! fixture window (`fixtures/windows_action_fixture.ps1`).
//!
//! Needs an interactive desktop, so it is ignored by default:
//! `cargo nextest run -p sqyre-app --test windows_actions --run-ignored only`.
//! Pause / Navigate Select are not covered: the Win32 hook drops injected keys.
#![cfg(windows)]

use image::RgbaImage;
use sqyre_capture::{OsWindowFocuser, SharedRunCapturer};
use sqyre_domain::{grid_cell_rect, CoordinateRef, Macro, ScalarValue};
use sqyre_executor::{execute_macro_with, ExecDeps, OcrEngine};
use sqyre_ports::{
    CollectionArea, CoordinateResolver, DesktopRect, IconStore, ItemMeta, MacroLookup, PortError,
    ScreenCapturer, WindowFocuser,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Rect {
    fn from_yaml(v: &serde_yaml::Value) -> Self {
        let n = |k: &str| v[k].as_i64().unwrap_or_else(|| panic!("rect.{k}: {v:?}")) as i32;
        Self {
            x: n("x"),
            y: n("y"),
            w: n("w"),
            h: n("h"),
        }
    }
    fn center(self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
    fn contains(self, (x, y): (i32, i32)) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
    fn ltrb(self, pad: i32) -> (i32, i32, i32, i32) {
        (
            self.x - pad,
            self.y - pad,
            self.x + self.w + pad,
            self.y + self.h + pad,
        )
    }
}

struct Fixture {
    child: Child,
    dir: PathBuf,
    exe: String,
    title: String,
    rects: HashMap<String, Rect>,
}

impl Fixture {
    fn launch(dir: &Path) -> Self {
        let script =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/windows_action_fixture.ps1");
        let child = Command::new("powershell.exe")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&script)
            .arg("-Out")
            .arg(dir)
            .spawn()
            .expect("spawn fixture");
        let ready = dir.join("ready.json");
        let raw = wait_for(Duration::from_secs(60), || {
            std::fs::read_to_string(&ready).ok()
        })
        .expect("fixture window never became ready");
        let v: serde_yaml::Value = serde_yaml::from_str(&raw).expect("ready.json");
        let rects = [
            "form", "label", "swatch", "pattern", "text", "button", "grid",
        ]
        .into_iter()
        .map(|k| (k.to_string(), Rect::from_yaml(&v[k])))
        .collect();
        Self {
            child,
            dir: dir.to_path_buf(),
            exe: v["exe"].as_str().unwrap().to_string(),
            title: v["title"].as_str().unwrap().to_string(),
            rects,
        }
    }

    fn rect(&self, name: &str) -> Rect {
        self.rects[name]
    }

    fn text(&self) -> String {
        std::fs::read_to_string(self.dir.join("text.txt")).unwrap_or_default()
    }

    fn events(&self) -> Vec<String> {
        std::fs::read_to_string(self.dir.join("events.txt"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    fn wait_text(&self, want: &str) {
        let got = wait_for(Duration::from_secs(5), || {
            let t = self.text();
            (t == want).then_some(t)
        });
        assert!(
            got.is_some(),
            "TextBox: want {want:?}, got {:?}",
            self.text()
        );
    }

    fn wait_events(&self, want: &[&str]) {
        let ok = wait_for(Duration::from_secs(5), || {
            let ev = self.events();
            want.iter().all(|w| ev.iter().any(|e| e == w)).then_some(())
        });
        assert!(
            ok.is_some(),
            "events: want {want:?}, got {:?}",
            self.events()
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn wait_for<T>(timeout: Duration, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    let start = Instant::now();
    loop {
        if let Some(v) = f() {
            return Some(v);
        }
        if start.elapsed() > timeout {
            return None;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// `Fx~<control>` → fixture rects; General runtime points read `${foundX}` / `${CellX}`.
struct FixtureResolver<'a>(&'a Fixture);

fn var_int(macro_: &Macro, name: &str) -> Result<i32, PortError> {
    macro_
        .variables
        .get(name)
        .and_then(|v| v.as_display().parse::<f64>().ok())
        .map(|v| v as i32)
        .ok_or_else(|| PortError::invalid(format!("variable {name} unset")))
}

impl CoordinateResolver for FixtureResolver<'_> {
    fn resolve_point(&self, r: &CoordinateRef, macro_: &Macro) -> Result<(i32, i32), PortError> {
        match r.name() {
            "Image Search Reference" => {
                Ok((var_int(macro_, "foundX")?, var_int(macro_, "foundY")?))
            }
            "Cell Coordinates" => Ok((var_int(macro_, "CellX")?, var_int(macro_, "CellY")?)),
            name => self
                .0
                .rects
                .get(name)
                .map(|r| r.center())
                .ok_or_else(|| PortError::invalid(format!("unknown point {name}"))),
        }
    }

    fn resolve_search_area(
        &self,
        r: &CoordinateRef,
        _macro_: &Macro,
    ) -> Result<(i32, i32, i32, i32), PortError> {
        let rect = self
            .0
            .rects
            .get(r.name())
            .ok_or_else(|| PortError::invalid(format!("unknown area {}", r.name())))?;
        if let Some((r1, c1, r2, c2)) = r.cell_range() {
            return grid_cell_rect(rect.ltrb(0), 1, 2, r1, c1, r2, c2)
                .ok_or_else(|| PortError::invalid("cell out of range"));
        }
        Ok(rect.ltrb(8))
    }

    fn collection_grid(&self, _program: &str, _collection: &str) -> Result<(i32, i32), PortError> {
        Ok((1, 2))
    }

    fn collection_area(
        &self,
        r: &CoordinateRef,
        _macro_: &Macro,
    ) -> Result<CollectionArea, PortError> {
        let (left, top, right, bottom) = self.0.rect(r.name()).ltrb(0);
        Ok(CollectionArea {
            left,
            top,
            right,
            bottom,
            rows: 1,
            cols: 2,
        })
    }
}

struct PatternIcon(PathBuf);

impl IconStore for PatternIcon {
    fn variant_paths(&self, target: &str) -> Vec<PathBuf> {
        if target == "Fx~pattern" {
            vec![self.0.clone()]
        } else {
            Vec::new()
        }
    }
    fn mask_path(&self, _target: &str) -> Option<PathBuf> {
        None
    }
    fn item_meta(&self, target: &str) -> Option<ItemMeta> {
        Some(ItemMeta {
            name: target.to_string(),
            ..Default::default()
        })
    }
}

struct StderrLog;

impl sqyre_ports::ActionLogger for StderrLog {
    fn log(&self, _id: sqyre_domain::ActionId, message: String) {
        eprintln!("[exec] {message}");
    }
}

struct Macros(HashMap<String, Arc<Macro>>);

impl MacroLookup for Macros {
    fn get(&self, name: &str) -> Option<Arc<Macro>> {
        self.0.get(name).cloned()
    }
}

fn decode(name: &str, actions: &str) -> Macro {
    let body: String = actions.lines().map(|l| format!("    {l}\n")).collect();
    let yaml = format!(
        "name: {name}\nglobaldelay: 0\nkeyboarddelay: 20\nmousedelay: 40\nhotkey: []\nvariables: []\n\
         root:\n  type: loop\n  name: root\n  count: 1\n  subactions:\n{body}"
    );
    sqyre_serialize::decode_macro_from_yaml(&yaml).unwrap_or_else(|e| panic!("{name}: {e}\n{yaml}"))
}

struct Harness<'a> {
    fx: &'a Fixture,
    capturer: SharedRunCapturer,
    icons: PatternIcon,
    macros: Macros,
    ocr: Arc<sqyre_vision::OcrsEngine>,
    vars_dir: PathBuf,
}

impl Harness<'_> {
    fn run(&mut self, mut macro_: Macro) -> Macro {
        let mut automation = sqyre_input::OsAutomation::new().expect("OsAutomation");
        let resolver = FixtureResolver(self.fx);
        let focuser = OsWindowFocuser;
        let mut deps = ExecDeps::new(&mut automation)
            .capturer(&mut self.capturer)
            .resolver(&resolver)
            .icons(&self.icons)
            .macros(&self.macros)
            .window_focuser(&focuser)
            .logger(&StderrLog)
            .ocr(self.ocr.as_ref() as &dyn OcrEngine);
        deps.variables_dir = Some(&self.vars_dir);
        let name = macro_.name.clone();
        execute_macro_with(&mut macro_, deps).unwrap_or_else(|e| panic!("macro {name}: {e}"));
        macro_
    }
}

fn var(m: &Macro, name: &str) -> Option<String> {
    m.variables.get(name).map(ScalarValue::as_display)
}

fn cursor() -> (i32, i32) {
    sqyre_capture::desktop_cursor_position().expect("cursor position")
}

#[test]
#[ignore = "drives the real Windows desktop (mouse, keyboard, focus, screen)"]
fn every_desktop_action_runs_against_a_real_window() {
    sqyre_capture::enable_per_monitor_dpi_v2();
    let tmp = tempfile::tempdir().unwrap();
    let fx = Fixture::launch(tmp.path());

    let mut capturer = SharedRunCapturer(sqyre_capture::shared_capturer().expect("GDI capturer"));
    // Image Search template: the fixture's pattern box, cropped from a live capture.
    let p = fx.rect("pattern");
    // Retry until the pattern is painted: a flat template matches everywhere.
    let deadline = Instant::now() + Duration::from_secs(10);
    let template: RgbaImage = loop {
        let img: RgbaImage = capturer
            .capture_rect(DesktopRect {
                x: p.x + 8,
                y: p.y + 8,
                w: p.w - 16,
                h: p.h - 16,
            })
            .expect("capture pattern");
        let distinct: std::collections::HashSet<_> = img.pixels().map(|px| px.0).collect();
        if distinct.len() >= 4 {
            break img;
        }
        assert!(Instant::now() < deadline, "fixture pattern never painted");
        std::thread::sleep(Duration::from_millis(100));
    };
    let icon = tmp.path().join("pattern.png");
    template.save(&icon).unwrap();

    let vars_dir = tmp.path().join("variables");
    std::fs::create_dir_all(&vars_dir).unwrap();
    let child = decode("Child", "- {type: type, text: child, delayms: 0}");
    let mut h = Harness {
        fx: &fx,
        capturer,
        icons: PatternIcon(icon),
        macros: Macros(HashMap::from([("Child".to_string(), Arc::new(child))])),
        ocr: sqyre_vision::shared_ocrs().expect("OCR models"),
        vars_dir: vars_dir.clone(),
    };

    // Focus Window by exe path + title.
    h.run(decode(
        "focus",
        &format!(
            "- {{type: focuswindow, processpath: '{}', windowtitle: '{}'}}",
            fx.exe, fx.title
        ),
    ));
    let focused = OsWindowFocuser
        .focused()
        .unwrap()
        .expect("a focused window");
    assert_eq!(focused.window_title, fx.title);

    // Move (instant), Click, Type, Key tap and a held modifier.
    h.run(decode(
        "keyboard",
        r#"- {type: move, point: Fx~text, smooth: false}
- {type: click, button: left, state: tap}
- {type: type, text: hello sqyre, delayms: 5}
- {type: key, key: enter, state: tap}
- {type: key, key: shift, state: down}
- {type: key, key: a, state: tap}
- {type: key, key: shift, state: up}
- {type: key, key: enter, state: tap}"#,
    ));
    let t = fx.rect("text").center();
    let c = cursor();
    assert!(
        (c.0 - t.0).abs() <= 2 && (c.1 - t.1).abs() <= 2,
        "cursor {c:?} vs {t:?}"
    );
    fx.wait_text("hello sqyre\r\nA\r\n");

    // Smooth Move, every mouse button (tap and down/up), scroll both ways.
    h.run(decode(
        "mouse",
        r#"- {type: move, point: Fx~button, smooth: true}
- {type: click, button: left, state: tap}
- {type: click, button: right, state: tap}
- {type: click, button: middle, state: down}
- {type: click, button: middle, state: up}
- {type: click, button: scroll, state: up}
- {type: click, button: scroll, state: down}"#,
    ));
    assert!(
        fx.rect("button").contains(cursor()),
        "smooth move ended at {:?}",
        cursor()
    );
    fx.wait_events(&[
        "button:Left",
        "button:Right",
        "button:Middle",
        "wheel:1",
        "wheel:-1",
    ]);

    // Find Pixel (hit + else branch), OCR, Image Search → Move to the match.
    let m = h.run(decode(
        "vision",
        r#"- type: findpixel
  name: swatch
  searcharea: Fx~form
  targetcolor: 12ef34
  colortolerance: 4
  subactions:
  - type: setvariable
    assignments:
    - {variablename: pixelHit, value: 1}
    - {variablename: pixelX, value: '${foundX}'}
    - {variablename: pixelY, value: '${foundY}'}
- type: findpixel
  name: absent
  searcharea: Fx~form
  targetcolor: '010203'
  colortolerance: 0
  elseactions:
  - type: setvariable
    assignments:
    - {variablename: pixelMiss, value: 1}
- type: ocr
  name: label
  target: SQYRE
  searcharea: Fx~label
  outputvariable: ocrText
  subactions:
  - type: setvariable
    assignments:
    - {variablename: ocrHit, value: 1}
- type: imagesearch
  name: pattern
  targets: [Fx~pattern]
  searcharea: Fx~form
  tolerance: 0.9
  subactions:
  - {type: move, point: General~Image Search Reference, smooth: false}"#,
    ));
    assert_eq!(var(&m, "pixelHit").as_deref(), Some("1"), "find pixel hit");
    let px = (
        var(&m, "pixelX").unwrap().parse::<i32>().unwrap(),
        var(&m, "pixelY").unwrap().parse::<i32>().unwrap(),
    );
    assert!(fx.rect("swatch").contains(px), "pixel at {px:?}");
    assert_eq!(
        var(&m, "pixelMiss").as_deref(),
        Some("1"),
        "find pixel else"
    );
    assert_eq!(
        var(&m, "ocrHit").as_deref(),
        Some("1"),
        "ocr read {:?}",
        var(&m, "ocrText")
    );
    assert!(
        fx.rect("pattern").contains(cursor()),
        "image search moved to {:?}",
        cursor()
    );

    // Flow + variables: Loop / Conditional / Loop Jump, While, Wait, For Each Row,
    // Save Variable (file + clipboard paste), Run Macro, For Each Cell.
    let m = h.run(decode(
        "flow",
        r#"- type: setvariable
  assignments:
  - {variablename: i, value: 0}
  - {variablename: n, value: 0}
  - {variablename: clip, value: pasted}
- type: loop
  name: count
  count: 5
  subactions:
  - type: setvariable
    assignments:
    - {variablename: i, value: '${i}+1'}
  - type: conditional
    name: stop at 3
    match: all
    clauses:
    - {left: '${i}', operator: '==', right: '3'}
    subactions:
    - {type: loopjump, mode: break}
- type: while
  match: all
  clauses:
  - {left: '${n}', operator: '<', right: '4'}
  maxiterations: 10
  subactions:
  - type: setvariable
    assignments:
    - {variablename: n, value: '${n}+1'}
- {type: wait, time: 20}
- type: foreachrow
  name: rows
  sources:
  - {source: "r1\nr2", outputvar: line}
  subactions:
  - {type: savevariable, variablename: line, destination: rows.txt, append: true, appendnewline: true}
- {type: savevariable, variablename: clip, destination: clipboard}
- {type: move, point: Fx~text, smooth: false}
- {type: click, button: left, state: tap}
- {type: key, key: ctrl, state: down}
- {type: key, key: v, state: tap}
- {type: key, key: ctrl, state: up}
- {type: runmacro, macroname: Child}
- type: foreachcell
  name: cells
  cells: Fx~grid@1,1-1,2
  subactions:
  - {type: move, point: General~Cell Coordinates, smooth: false}
  - {type: click, button: left, state: tap}"#,
    ));
    assert_eq!(var(&m, "i").as_deref(), Some("3"), "loop jump break");
    assert_eq!(var(&m, "n").as_deref(), Some("4"), "while");
    let rows = std::fs::read_to_string(vars_dir.join("rows.txt")).unwrap();
    assert_eq!(rows.replace("\r\n", "\n"), "r1\nr2\n");
    fx.wait_text("hello sqyre\r\nA\r\npastedchild");
    fx.wait_events(&["cell1:Left", "cell2:Left"]);
}
