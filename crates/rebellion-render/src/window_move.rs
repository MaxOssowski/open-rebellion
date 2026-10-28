//! Moving an original window by its own surface.
//!
//! `FUN_00606650`, the base window's handler, gives a left press (`0x201`)
//! to `FUN_00606ee0`, which holds the window when its `+0xb8` has bit 4. The
//! Fleet (`FUN_004a2630`), System (`FUN_00452fc0`), System Defenses
//! (`FUN_004a7790`) and Missions (`FUN_0049f130`) windows set that bit; the
//! sector window keeps its press (`FUN_004593e0` case `0x201`) and does not
//! move. A press on a child control (a button, list or tab) goes to that
//! control instead.
//!
//! While held the window follows the pointer, unbounded (`FUN_006071e0`;
//! `FUN_00606380` sets `+0xf4` to 1, a live move). The drop
//! (`FUN_00607010`) moves it back inside the bounds `FUN_006071a0` took
//! from the galaxy view, the rect the window opened in: to the left edge
//! when left of it, else to `right - width` when past it, and the same for
//! the height. `ghidra/notes/gid-markers.md` reads the same for the GID
//! legend.

use egui_macroquad::egui;

/// A move in progress: the window's logical position when it was pressed.
pub(crate) type MoveAnchor = Option<(i16, i16)>;

/// The pointer's travel since the press on `body`, in logical pixels, while
/// the body is held; `None` otherwise. A move not yet under way (`anchor`
/// unset, so the window still stands where it was pressed) starts only from
/// a press outside every rect of `controls`.
pub(crate) fn held_travel(
    ui: &egui::Ui,
    body: &egui::Response,
    controls: &[egui::Rect],
    anchor: MoveAnchor,
    scale: f32,
) -> Option<egui::Vec2> {
    if !body.dragged() {
        return None;
    }
    let (from, to) = ui.input(|input| {
        let pointer = &input.pointer;
        pointer.press_origin().zip(pointer.interact_pos())
    })?;
    if anchor.is_none() && controls.iter().any(|control| control.contains(from)) {
        return None;
    }
    Some((to - from) / scale.max(f32::EPSILON))
}

/// Move `position` by `travel` from where the press found it; `None` ends
/// a move, putting the window back with `inside` (the drop's bounds).
#[expect(
    clippy::cast_possible_truncation,
    reason = "Window positions are whole logical pixels, as the original's."
)]
pub(crate) fn follow(
    position: &mut (i16, i16),
    anchor: &mut MoveAnchor,
    travel: Option<egui::Vec2>,
    inside: impl FnOnce((i16, i16)) -> (i16, i16),
) {
    match travel {
        Some(travel) => {
            let (x, y) = *anchor.get_or_insert(*position);
            *position = (
                (f32::from(x) + travel.x).round() as i16,
                (f32::from(y) + travel.y).round() as i16,
            );
        }
        None => {
            if anchor.take().is_some() {
                *position = inside(*position);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // FUN_006071e0 moves the window live without bounds; FUN_00607010 puts
    // it back inside the bounds on the drop.
    #[test]
    fn a_held_window_follows_the_pointer_and_drops_inside_its_bounds() {
        let inside = |(x, y): (i16, i16)| (x.clamp(0, 100), y.clamp(0, 50));
        let mut position = (10, 10);
        let mut anchor = None;
        follow(
            &mut position,
            &mut anchor,
            Some(egui::vec2(5.0, 2.0)),
            inside,
        );
        assert_eq!(position, (15, 12));
        follow(
            &mut position,
            &mut anchor,
            Some(egui::vec2(200.0, -40.0)),
            inside,
        );
        assert_eq!(position, (210, -30));
        follow(&mut position, &mut anchor, None, inside);
        assert_eq!(position, (100, 0));
        assert_eq!(anchor, None);
        // No move under way: nothing changes.
        position = (300, 300);
        follow(&mut position, &mut anchor, None, inside);
        assert_eq!(position, (300, 300));
    }
}
