//! Map Sqyre mouse calls onto accessibility gestures.
//!
//! An accessibility gesture is dispatched whole, so a press cannot stay open while
//! other actions run. Down records the press; up turns it into one gesture: a tap,
//! a long-press as long as the hold, or a swipe when the pointer moved in between.

use std::time::Instant;
use thiserror::Error;

/// Shortest stroke the shell dispatches for a tap.
pub const TAP_MS: u32 = 50;
/// Right click becomes a long-press of at least this long.
pub const LONG_PRESS_MS: u32 = 600;
/// Duration of the swipe used for one scroll step.
pub const SCROLL_MS: u32 = 250;
/// `GestureDescription.getMaxGestureDuration()`.
pub const MAX_GESTURE_MS: u32 = 60_000;

/// One accessibility gesture in display pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    /// Single-point stroke held for `duration_ms` (tap or long-press).
    Press { x: i32, y: i32, duration_ms: u32 },
    /// Straight stroke between two points.
    Swipe {
        from: (i32, i32),
        to: (i32, i32),
        duration_ms: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PointerError {
    #[error("{0}: not supported on Android")]
    Unsupported(&'static str),
    #[error("click before any move: no pointer position on Android")]
    NoPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Button {
    Left,
    Right,
}

fn parse_button(button: &str) -> Result<Button, PointerError> {
    match button {
        "right" => Ok(Button::Right),
        "middle" | "center" => Err(PointerError::Unsupported("middle click")),
        "scroll" => Err(PointerError::Unsupported("scroll-wheel click")),
        _ => Ok(Button::Left),
    }
}

#[derive(Debug, Clone, Copy)]
struct Press {
    button: Button,
    at: (i32, i32),
    since: Instant,
}

/// Last pointer position plus an open press waiting for its release.
#[derive(Debug, Default)]
pub struct PointerPlanner {
    pos: Option<(i32, i32)>,
    press: Option<Press>,
}

impl PointerPlanner {
    pub fn move_to(&mut self, x: i32, y: i32) {
        self.pos = Some((x, y));
    }

    pub fn position(&self) -> Option<(i32, i32)> {
        self.pos
    }

    /// Down returns `None`; up returns the gesture to dispatch for the whole press.
    pub fn click(
        &mut self,
        button: &str,
        down: bool,
        now: Instant,
    ) -> Result<Option<Gesture>, PointerError> {
        let button = parse_button(button)?;
        let pos = self.pos.ok_or(PointerError::NoPosition)?;
        if down {
            self.press = Some(Press {
                button,
                at: pos,
                since: now,
            });
            return Ok(None);
        }
        let Some(press) = self.press.take() else {
            return Ok(None);
        };
        let held = now
            .saturating_duration_since(press.since)
            .as_millis()
            .min(u128::from(MAX_GESTURE_MS)) as u32;
        let (x, y) = press.at;
        Ok(Some(match press.button {
            Button::Right => Gesture::Press {
                x,
                y,
                duration_ms: held.max(LONG_PRESS_MS),
            },
            Button::Left if pos != press.at => Gesture::Swipe {
                from: press.at,
                to: pos,
                duration_ms: held.max(TAP_MS),
            },
            Button::Left => Gesture::Press {
                x,
                y,
                duration_ms: held.max(TAP_MS),
            },
        }))
    }

    /// Drop an open press (run stopped before its release).
    pub fn cancel_press(&mut self) {
        self.press = None;
    }

    /// Swipe a quarter screen at the pointer (screen center before any move).
    ///
    /// Scrolling content up means the finger travels down.
    pub fn scroll(&self, up: bool, screen: (i32, i32)) -> Gesture {
        let (w, h) = (screen.0.max(1), screen.1.max(1));
        let (x, y) = self.pos.unwrap_or((w / 2, h / 2));
        let dist = (h / 4).max(1);
        let to_y = if up {
            (y + dist).min(h - 1)
        } else {
            (y - dist).max(0)
        };
        Gesture::Swipe {
            from: (x, y),
            to: (x, to_y),
            duration_ms: SCROLL_MS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(x: i32, y: i32) -> PointerPlanner {
        let mut p = PointerPlanner::default();
        p.move_to(x, y);
        p
    }

    #[test]
    fn quick_left_click_is_a_tap() {
        let mut p = at(10, 20);
        let t = Instant::now();
        assert_eq!(p.click("left", true, t), Ok(None));
        assert_eq!(
            p.click("left", false, t),
            Ok(Some(Gesture::Press {
                x: 10,
                y: 20,
                duration_ms: TAP_MS
            }))
        );
    }

    #[test]
    fn held_left_click_keeps_its_duration() {
        let mut p = at(1, 2);
        let t = Instant::now();
        p.click("left", true, t).expect("down");
        assert_eq!(
            p.click("left", false, t + Duration::from_millis(900)),
            Ok(Some(Gesture::Press {
                x: 1,
                y: 2,
                duration_ms: 900
            }))
        );
    }

    #[test]
    fn hold_is_capped_at_the_gesture_limit() {
        let mut p = at(0, 0);
        let t = Instant::now();
        p.click("left", true, t).expect("down");
        let g = p.click("left", false, t + Duration::from_secs(600));
        assert_eq!(
            g,
            Ok(Some(Gesture::Press {
                x: 0,
                y: 0,
                duration_ms: MAX_GESTURE_MS
            }))
        );
    }

    #[test]
    fn move_between_down_and_up_is_a_swipe() {
        let mut p = at(5, 5);
        let t = Instant::now();
        p.click("left", true, t).expect("down");
        p.move_to(50, 80);
        assert_eq!(
            p.click("left", false, t + Duration::from_millis(300)),
            Ok(Some(Gesture::Swipe {
                from: (5, 5),
                to: (50, 80),
                duration_ms: 300
            }))
        );
    }

    #[test]
    fn right_click_is_a_long_press() {
        let mut p = at(7, 8);
        let t = Instant::now();
        p.click("right", true, t).expect("down");
        assert_eq!(
            p.click("right", false, t),
            Ok(Some(Gesture::Press {
                x: 7,
                y: 8,
                duration_ms: LONG_PRESS_MS
            }))
        );
    }

    #[test]
    fn unsupported_buttons_and_missing_position() {
        let mut p = at(0, 0);
        let t = Instant::now();
        assert_eq!(
            p.click("middle", true, t),
            Err(PointerError::Unsupported("middle click"))
        );
        assert_eq!(
            p.click("center", true, t),
            Err(PointerError::Unsupported("middle click"))
        );
        assert_eq!(
            p.click("scroll", true, t),
            Err(PointerError::Unsupported("scroll-wheel click"))
        );
        let mut fresh = PointerPlanner::default();
        assert_eq!(fresh.click("left", true, t), Err(PointerError::NoPosition));
    }

    #[test]
    fn release_without_press_and_cancel_dispatch_nothing() {
        let mut p = at(3, 3);
        let t = Instant::now();
        assert_eq!(p.click("left", false, t), Ok(None));
        p.click("left", true, t).expect("down");
        p.cancel_press();
        assert_eq!(p.click("left", false, t), Ok(None));
    }

    #[test]
    fn scroll_swipes_a_quarter_screen_and_clamps() {
        let p = at(100, 400);
        assert_eq!(
            p.scroll(true, (1000, 2000)),
            Gesture::Swipe {
                from: (100, 400),
                to: (100, 900),
                duration_ms: SCROLL_MS
            }
        );
        assert_eq!(
            p.scroll(false, (1000, 2000)),
            Gesture::Swipe {
                from: (100, 400),
                to: (100, 0),
                duration_ms: SCROLL_MS
            }
        );
        let centered = PointerPlanner::default();
        assert_eq!(
            centered.scroll(true, (1000, 2000)),
            Gesture::Swipe {
                from: (500, 1000),
                to: (500, 1500),
                duration_ms: SCROLL_MS
            }
        );
        assert_eq!(
            at(10, 1990).scroll(true, (1000, 2000)),
            Gesture::Swipe {
                from: (10, 1990),
                to: (10, 1999),
                duration_ms: SCROLL_MS
            }
        );
    }
}
