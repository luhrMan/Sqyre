//! Pure overlay-omit gate transitions (no mutex / hooks).
//!
//! [`super::overlay_omit`] wraps this in process statics; unit tests exercise
//! the transitions here without parking_lot or OS callbacks.

use web_time::Instant;

/// Idle before restoring buttons after a detection grab when no macro hide is held.
pub const RESTORE_IDLE_MS: u64 = 150;

/// Process-neutral omit gate state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmitGate {
    /// When true, detection captures include overlay buttons.
    pub include: bool,
    pub omitted: bool,
    pub depth: u32,
    /// Bumped whenever a restore is scheduled or cancelled.
    pub generation: u64,
    pub restore_at: Option<Instant>,
}

impl Default for OmitGate {
    fn default() -> Self {
        Self {
            include: true,
            omitted: false,
            depth: 0,
            generation: 0,
            restore_at: None,
        }
    }
}

/// Side-effect requested by a transition (caller runs the omit hook).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmitEffect {
    None,
    /// Hide buttons now (`hook(true)`).
    Hide,
    /// Show buttons now (`hook(false)`).
    Show,
    /// Schedule a deferred show at `restore_at` (wake the restore worker).
    ScheduleRestore,
}

impl OmitGate {
    /// Publish the user setting. `macro_hide_held` is the current macro-hide span.
    pub fn set_include(&mut self, include: bool, macro_hide_held: bool) -> OmitEffect {
        if self.include == include {
            return OmitEffect::None;
        }
        self.include = include;
        if include && self.omitted && self.depth == 0 {
            self.begin_restore();
            OmitEffect::Show
        } else if !include && !self.omitted && self.depth == 0 && macro_hide_held {
            if self.arm_omit() {
                OmitEffect::Hide
            } else {
                OmitEffect::None
            }
        } else {
            OmitEffect::None
        }
    }

    /// Enter a detection grab. Returns whether the caller should hide buttons now.
    pub fn enter_detection(&mut self) -> OmitEffect {
        if self.include {
            return OmitEffect::None;
        }
        self.depth += 1;
        if self.arm_omit() {
            OmitEffect::Hide
        } else {
            OmitEffect::None
        }
    }

    /// Leave a detection grab.
    pub fn leave_detection(&mut self, macro_hide_held: bool, now: Instant) -> OmitEffect {
        self.depth = self.depth.saturating_sub(1);
        self.release_omit(macro_hide_held, now)
    }

    /// Macro hide span entered while detection omits buttons.
    pub fn enter_macro_hide(&mut self) -> OmitEffect {
        if self.include {
            return OmitEffect::None;
        }
        if self.arm_omit() {
            OmitEffect::Hide
        } else {
            OmitEffect::None
        }
    }

    /// Macro hide span left (`macro_hide_held` is false after the leave).
    pub fn leave_macro_hide(&mut self, now: Instant) -> OmitEffect {
        self.release_omit(false, now)
    }

    /// Cancel a pending show and mark omitted. Returns true when newly omitted.
    pub fn arm_omit(&mut self) -> bool {
        self.restore_at = None;
        self.generation = self.generation.wrapping_add(1);
        if self.omitted {
            false
        } else {
            self.omitted = true;
            true
        }
    }

    /// Show now when include is on; otherwise schedule idle restore unless macro hide holds.
    pub fn release_omit(&mut self, macro_hide_held: bool, now: Instant) -> OmitEffect {
        if self.depth > 0 || !self.omitted {
            return OmitEffect::None;
        }
        if macro_hide_held && !self.include {
            return OmitEffect::None;
        }
        if self.include {
            self.begin_restore();
            return OmitEffect::Show;
        }
        self.generation = self.generation.wrapping_add(1);
        self.restore_at = Some(now + std::time::Duration::from_millis(RESTORE_IDLE_MS));
        OmitEffect::ScheduleRestore
    }

    /// Marks a restore in progress.
    pub fn begin_restore(&mut self) {
        self.omitted = false;
        self.restore_at = None;
        self.generation = self.generation.wrapping_add(1);
    }

    /// Worker tick: restore when idle elapsed and nothing still holds omit.
    pub fn try_idle_restore(
        &mut self,
        generation: u64,
        macro_hide_held: bool,
        now: Instant,
    ) -> OmitEffect {
        if self.depth == 0
            && self.omitted
            && !macro_hide_held
            && self.generation == generation
            && self.restore_at.is_some_and(|at| now >= at)
        {
            self.begin_restore();
            OmitEffect::Show
        } else {
            OmitEffect::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn nested_detection_stays_omitted_until_outer_leaves() {
        let mut g = OmitGate::default();
        assert_eq!(g.set_include(false, false), OmitEffect::None);
        assert_eq!(g.enter_detection(), OmitEffect::Hide);
        assert!(g.omitted);
        assert_eq!(g.depth, 1);
        assert_eq!(g.enter_detection(), OmitEffect::None);
        assert_eq!(g.depth, 2);
        let now = Instant::now();
        assert_eq!(g.leave_detection(false, now), OmitEffect::None);
        assert_eq!(g.depth, 1);
        assert!(g.omitted);
        assert_eq!(g.leave_detection(false, now), OmitEffect::ScheduleRestore);
        assert!(g.restore_at.is_some());
    }

    #[test]
    fn macro_hide_blocks_restore_between_grabs() {
        let mut g = OmitGate::default();
        let _ = g.set_include(false, false);
        assert_eq!(g.enter_macro_hide(), OmitEffect::Hide);
        let now = Instant::now();
        assert_eq!(g.enter_detection(), OmitEffect::None);
        assert_eq!(g.leave_detection(true, now), OmitEffect::None);
        assert!(g.omitted);
        assert_eq!(g.leave_macro_hide(now), OmitEffect::ScheduleRestore);
    }

    #[test]
    fn include_on_shows_immediately_when_idle() {
        let mut g = OmitGate {
            include: false,
            omitted: true,
            depth: 0,
            generation: 1,
            restore_at: Some(Instant::now()),
        };
        assert_eq!(g.set_include(true, false), OmitEffect::Show);
        assert!(!g.omitted);
        assert!(g.restore_at.is_none());
    }

    #[test]
    fn idle_restore_fires_after_deadline() {
        let mut g = OmitGate::default();
        let _ = g.set_include(false, false);
        let now = Instant::now();
        assert_eq!(g.enter_detection(), OmitEffect::Hide);
        assert_eq!(g.leave_detection(false, now), OmitEffect::ScheduleRestore);
        let gen = g.generation;
        let at = g.restore_at.expect("scheduled");
        assert_eq!(
            g.try_idle_restore(gen, false, at - Duration::from_millis(1)),
            OmitEffect::None
        );
        assert_eq!(g.try_idle_restore(gen, false, at), OmitEffect::Show);
        assert!(!g.omitted);
    }
}
