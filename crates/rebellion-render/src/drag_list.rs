//! The selection rules of the `CoolDragList` control (vtable `0x0066e0f0`,
//! window procedure `FUN_006083c0`), which the Fleet window's two lists
//! (`FUN_004a4b10`, flags 0) and the System Defenses window's list
//! (`FUN_004a8790`, flags `0xa0000`) share. Neither sets `0x10000`, the
//! single-selection flag, so both keep several items selected
//! (`ghidra/notes/drag-list.md`).
//!
//! A press (`WM_LBUTTONDOWN` or `WM_RBUTTONDOWN`) on an item:
//!
//! - already selected: changes nothing yet (flag `0x10`), so a drag carries
//!   the whole selection; the left release applies the click below, unless
//!   the release was a drop (`0x29a`);
//! - with Ctrl (`MK_CONTROL`): toggles the item;
//! - with Shift (`MK_SHIFT`): adds every item whose cell meets the rectangle
//!   from the last press point to this one (`FUN_00609590`);
//! - otherwise: selects the item alone (`FUN_006094b0`, then bit 0).
//!
//! A plain press on no item clears the selection; a Ctrl press there does
//! nothing. Every press then becomes the anchor (`+0xf8`).

/// The keys held with a press (`wParam`'s `MK_CONTROL` and `MK_SHIFT`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
}

/// A point or cell in the list's content pixels: the client point plus
/// the first visible pixel (`FUN_00609d80`).
pub type ContentPoint = (f32, f32);

/// A list's selected items (each item's `+0x3c` bit 0), in the order they
/// were selected, with the press state `FUN_006083c0` keeps.
#[derive(Debug, Clone, PartialEq)]
pub struct ListSelection<K> {
    items: Vec<K>,
    /// The last press point (`+0xf8`), valid once a press has landed
    /// (`+0xf4` bit 1).
    anchor: Option<ContentPoint>,
    /// The press landed on a selected item (`+0xf4` bit `0x10`): the
    /// release applies the click.
    deferred: bool,
}

impl<K> Default for ListSelection<K> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            anchor: None,
            deferred: false,
        }
    }
}

impl<K: Copy + PartialEq> ListSelection<K> {
    /// A selection of `item` alone, as a select from outside makes it
    /// (`FUN_00609500`).
    #[must_use]
    pub fn only(item: K) -> Self {
        Self {
            items: vec![item],
            ..Self::default()
        }
    }

    #[must_use]
    pub fn items(&self) -> &[K] {
        &self.items
    }

    #[must_use]
    pub fn contains(&self, item: K) -> bool {
        self.items.contains(&item)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The one selected item, or `None` for none or several.
    #[must_use]
    pub fn single(&self) -> Option<K> {
        match self.items.as_slice() {
            [item] => Some(*item),
            _ => None,
        }
    }

    /// The first selected item in `cells` order, as `FUN_00609410` walks
    /// the list.
    #[must_use]
    pub fn first_in<'a>(&self, cells: impl IntoIterator<Item = &'a K>) -> Option<K>
    where
        K: 'a,
    {
        cells.into_iter().copied().find(|item| self.contains(*item))
    }

    /// Clear the selection, as a refill of the list does.
    pub fn clear(&mut self) {
        self.items.clear();
        self.deferred = false;
    }

    /// Keep only the items `keep` accepts: an item the list no longer
    /// holds is no longer selected.
    pub fn retain(&mut self, keep: impl FnMut(&K) -> bool) {
        self.items.retain(keep);
    }

    /// A left or right press at `point` over `hit`, among `cells` (each
    /// item with its cell's top-left and size). Returns whether the
    /// selection changed.
    pub fn press(
        &mut self,
        hit: Option<K>,
        point: ContentPoint,
        modifiers: Modifiers,
        cells: &[(K, ContentPoint, (f32, f32))],
    ) -> bool {
        let before = self.items.clone();
        if hit.is_some_and(|item| self.contains(item)) {
            self.deferred = true;
        } else {
            self.click(hit, point, modifiers, cells);
        }
        self.anchor = Some(point);
        self.items != before
    }

    /// The left release at `point` over `hit` when it is no drop: a press
    /// held back on a selected item applies now (`FUN_006083c0`, case
    /// `0x202`). Returns whether the selection changed.
    pub fn release(
        &mut self,
        hit: Option<K>,
        point: ContentPoint,
        modifiers: Modifiers,
        cells: &[(K, ContentPoint, (f32, f32))],
    ) -> bool {
        if !std::mem::take(&mut self.deferred) {
            return false;
        }
        let before = self.items.clone();
        self.click(hit, point, modifiers, cells);
        self.items != before
    }

    /// A release that dropped the selection (`0x29a`) clears the held
    /// click.
    pub fn dropped(&mut self) {
        self.deferred = false;
    }

    fn click(
        &mut self,
        hit: Option<K>,
        point: ContentPoint,
        modifiers: Modifiers,
        cells: &[(K, ContentPoint, (f32, f32))],
    ) {
        if modifiers.ctrl {
            if let Some(item) = hit {
                if let Some(index) = self.items.iter().position(|known| *known == item) {
                    self.items.remove(index);
                } else {
                    self.items.push(item);
                }
            }
        } else if modifiers.shift {
            let anchor = self.anchor.unwrap_or(point);
            for (item, _, _) in cells
                .iter()
                .filter(|(_, origin, size)| meets(anchor, point, *origin, *size))
            {
                if !self.contains(*item) {
                    self.items.push(*item);
                }
            }
        } else {
            self.items.clear();
            if let Some(item) = hit {
                self.items.push(item);
            }
        }
    }
}

/// `FUN_00609590`: a cell at `origin`, `size` wide and high (the list's
/// `+0xcc`/`+0xd0`), meets the rectangle spanned by `from` and `to`, edges
/// included.
fn meets(from: ContentPoint, to: ContentPoint, origin: ContentPoint, size: (f32, f32)) -> bool {
    let (left, right) = (from.0.min(to.0), from.0.max(to.0));
    let (top, bottom) = (from.1.min(to.1), from.1.max(to.1));
    origin.0 <= right && left <= origin.0 + size.0 && origin.1 <= bottom && top <= origin.1 + size.1
}

/// The keys held now, as `wParam` reports them with a press.
#[must_use]
pub fn modifiers(ctx: &egui_macroquad::egui::Context) -> Modifiers {
    ctx.input(|input| Modifiers {
        ctrl: input.modifiers.ctrl || input.modifiers.mac_cmd,
        shift: input.modifiers.shift,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A vertical list of four 50-pixel rows, as the Fleet window's right
    /// list lays its 125 by 50 items out (`FUN_004a4b10`).
    fn rows() -> Vec<(u8, ContentPoint, (f32, f32))> {
        (0..4u8)
            .map(|row| (row, (0.0, f32::from(row) * 50.0), (125.0, 50.0)))
            .collect()
    }

    fn row_point(row: u8) -> ContentPoint {
        (60.0, f32::from(row) * 50.0 + 25.0)
    }

    const PLAIN: Modifiers = Modifiers {
        ctrl: false,
        shift: false,
    };
    const CTRL: Modifiers = Modifiers {
        ctrl: true,
        shift: false,
    };
    const SHIFT: Modifiers = Modifiers {
        ctrl: false,
        shift: true,
    };

    #[test]
    fn a_plain_press_selects_the_item_alone() {
        // FUN_006083c0 0x201: FUN_006094b0 clears, then bit 0 is set.
        let cells = rows();
        let mut selection = ListSelection::only(0);
        assert!(selection.press(Some(2), row_point(2), PLAIN, &cells));
        assert_eq!(selection.items(), &[2]);
    }

    #[test]
    fn a_ctrl_press_toggles_the_item_and_keeps_the_rest() {
        // FUN_006083c0 0x201 with MK_CONTROL: bit 0 ^= 1.
        let cells = rows();
        let mut selection = ListSelection::only(0);
        selection.press(Some(2), row_point(2), CTRL, &cells);
        selection.press(Some(3), row_point(3), CTRL, &cells);
        assert_eq!(selection.items(), &[0, 2, 3]);
        // A selected item's press is held to the release, which toggles it.
        assert!(!selection.press(Some(2), row_point(2), CTRL, &cells));
        assert!(selection.release(Some(2), row_point(2), CTRL, &cells));
        assert_eq!(selection.items(), &[0, 3]);
    }

    #[test]
    fn a_shift_press_adds_the_items_from_the_last_press_to_this_one() {
        // FUN_00609590(anchor, point): every cell meeting the rectangle,
        // added to what is already selected.
        let cells = rows();
        let mut selection = ListSelection::default();
        selection.press(Some(0), row_point(0), PLAIN, &cells);
        selection.press(Some(2), row_point(2), SHIFT, &cells);
        assert_eq!(selection.items(), &[0, 1, 2]);
        // The anchor is now row 2's press; a Ctrl press elsewhere keeps the
        // rows and moves it.
        selection.press(Some(3), row_point(3), CTRL, &cells);
        assert_eq!(selection.items(), &[0, 1, 2, 3]);
    }

    #[test]
    fn a_shift_range_takes_cells_whose_edge_touches_the_rectangle() {
        // FUN_00609590 compares with <=: a cell whose bottom edge is the
        // rectangle's top is in.
        let cells = rows();
        let mut selection = ListSelection::default();
        // A press between the items' row 0 and row 1 on no item...
        selection.press(None, (10.0, 50.0), PLAIN, &cells);
        selection.press(Some(1), (10.0, 99.0), SHIFT, &cells);
        assert_eq!(selection.items(), &[0, 1]);
    }

    #[test]
    fn a_press_on_a_selected_item_keeps_the_selection_until_the_release() {
        // Flag 0x10: the press changes nothing, so a drag carries every
        // selected item; a release that is no drop selects the item alone.
        let cells = rows();
        let mut selection = ListSelection::default();
        selection.press(Some(0), row_point(0), PLAIN, &cells);
        selection.press(Some(1), row_point(1), CTRL, &cells);
        assert!(!selection.press(Some(0), row_point(0), PLAIN, &cells));
        assert_eq!(selection.items(), &[0, 1]);
        assert!(selection.release(Some(0), row_point(0), PLAIN, &cells));
        assert_eq!(selection.items(), &[0]);
        // A second release has nothing held.
        assert!(!selection.release(Some(1), row_point(1), PLAIN, &cells));
    }

    #[test]
    fn a_drop_forgets_the_held_click() {
        // 0x29a clears flag 0x10 before the release would apply it.
        let cells = rows();
        let mut selection = ListSelection::default();
        selection.press(Some(0), row_point(0), PLAIN, &cells);
        selection.press(Some(1), row_point(1), CTRL, &cells);
        selection.press(Some(1), row_point(1), PLAIN, &cells);
        selection.dropped();
        assert!(!selection.release(Some(1), row_point(1), PLAIN, &cells));
        assert_eq!(selection.items(), &[0, 1]);
    }

    #[test]
    fn a_plain_press_on_no_item_clears_and_a_ctrl_press_there_does_nothing() {
        // FUN_006083c0: no item and no key, FUN_006094b0; with MK_CONTROL
        // and no item, nothing.
        let cells = rows();
        let mut selection = ListSelection::only(1);
        assert!(!selection.press(None, (60.0, 230.0), CTRL, &cells));
        assert_eq!(selection.items(), &[1]);
        assert!(selection.press(None, (60.0, 230.0), PLAIN, &cells));
        assert!(selection.is_empty());
    }

    #[test]
    fn single_and_first_read_the_selection() {
        let cells = rows();
        let mut selection = ListSelection::only(2);
        assert_eq!(selection.single(), Some(2));
        selection.press(Some(1), row_point(1), CTRL, &cells);
        assert_eq!(selection.single(), None);
        assert_eq!(
            selection.first_in(cells.iter().map(|(item, _, _)| item)),
            Some(1)
        );
        selection.retain(|item| *item != 1);
        assert_eq!(selection.items(), &[2]);
    }
}
