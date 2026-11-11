//! One focus shared by the galaxy view's System, Fleet, Defenses and
//! Missions windows.
//!
//! The original's windows are child windows of the galaxy view. Each draws
//! its focused title strip only while it is the view's focused child
//! (`FUN_00457690`: `+0x14c` `+0xb8` equals the window's handle, else the
//! unfocused strip `+0x184`). A0 2026-11-11-manufacturing-focus/02: opening
//! Yavin's Defenses window beside Averam's System window turns Averam's
//! strip from (255, 0, 0) to the unfocused (179, 4, 4); /01: a press on a
//! sector window does not take the focus.
//!
//! port: each window kind keeps its own stack, the last window on top. A
//! kind stamps itself whenever it raises a window (opens, restores or
//! focuses one), and the kind with the latest stamp among those with a
//! window open holds the shared focus.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_STAMP: AtomicU64 = AtomicU64::new(1);

/// A window kind's claim on the shared focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FocusStamp {
    stamp: u64,
    holds: bool,
}

impl Default for FocusStamp {
    fn default() -> Self {
        Self {
            stamp: 0,
            holds: true,
        }
    }
}

impl FocusStamp {
    /// The kind raised one of its windows: it is now the latest.
    pub(crate) fn take(&mut self) {
        self.stamp = NEXT_STAMP.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn stamp(&self) -> u64 {
        self.stamp
    }

    /// Whether the kind's top window draws the focused strip.
    pub(crate) fn holds(&self) -> bool {
        self.holds
    }

    pub(crate) fn set_holds(&mut self, holds: bool) {
        self.holds = holds;
    }
}

/// Which kinds hold the shared focus: the latest raised of those with a
/// window open (`None` for a kind with no window open). At most one is
/// `true`; none when no window is open.
pub fn shared_focus<const N: usize>(stamps: [Option<u64>; N]) -> [bool; N] {
    let latest = stamps.iter().flatten().max().copied();
    stamps.map(|stamp| stamp.is_some() && stamp == latest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_latest_raised_kind_with_a_window_open_holds_the_focus() {
        assert_eq!(
            shared_focus([Some(3), Some(7), None, Some(5)]),
            [false, true, false, false]
        );
        assert_eq!(shared_focus([Some(3), None]), [true, false]);
        assert_eq!(shared_focus::<3>([None, None, None]), [false; 3]);
    }

    #[test]
    fn each_take_stamps_later_than_the_one_before() {
        let mut first = FocusStamp::default();
        let mut second = FocusStamp::default();
        assert!(first.holds());
        first.take();
        second.take();
        assert!(second.stamp() > first.stamp());
        first.take();
        assert!(first.stamp() > second.stamp());
        first.set_holds(false);
        assert!(!first.holds());
    }
}
