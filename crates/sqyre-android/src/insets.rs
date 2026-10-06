//! System bar and display cutout insets reported by the shell.

use std::sync::atomic::{AtomicU64, Ordering};

/// Edges of the window covered by system bars or cutouts, in physical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Insets {
    pub left: u16,
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
}

impl Insets {
    /// Clamps each edge to `0..=u16::MAX`.
    pub fn from_px(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        let edge = |v: i32| u16::try_from(v.max(0)).unwrap_or(u16::MAX);
        Self {
            left: edge(left),
            top: edge(top),
            right: edge(right),
            bottom: edge(bottom),
        }
    }

    fn pack(self) -> u64 {
        u64::from(self.left)
            | (u64::from(self.top) << 16)
            | (u64::from(self.right) << 32)
            | (u64::from(self.bottom) << 48)
    }

    fn unpack(bits: u64) -> Self {
        let edge = |shift: u32| (bits >> shift) as u16;
        Self {
            left: edge(0),
            top: edge(16),
            right: edge(32),
            bottom: edge(48),
        }
    }
}

/// Latest insets; all four edges update together so readers never see a mix.
pub struct InsetStore(AtomicU64);

impl InsetStore {
    pub const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    pub fn set(&self, insets: Insets) {
        self.0.store(insets.pack(), Ordering::Relaxed);
    }

    pub fn get(&self) -> Insets {
        Insets::unpack(self.0.load(Ordering::Relaxed))
    }
}

impl Default for InsetStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_starts_empty_and_keeps_the_latest() {
        let store = InsetStore::new();
        assert_eq!(store.get(), Insets::default());
        store.set(Insets::from_px(1, 132, 3, 63));
        store.set(Insets::from_px(0, 140, 0, 48));
        assert_eq!(
            store.get(),
            Insets {
                left: 0,
                top: 140,
                right: 0,
                bottom: 48
            }
        );
    }

    #[test]
    fn edges_round_trip_independently() {
        let insets = Insets {
            left: 1,
            top: u16::MAX,
            right: 0,
            bottom: 300,
        };
        assert_eq!(Insets::unpack(insets.pack()), insets);
    }

    #[test]
    fn from_px_clamps_out_of_range_edges() {
        assert_eq!(
            Insets::from_px(-5, 70_000, 12, i32::MIN),
            Insets {
                left: 0,
                top: u16::MAX,
                right: 12,
                bottom: 0
            }
        );
    }
}
