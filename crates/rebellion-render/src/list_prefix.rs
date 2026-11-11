//! The original list's typed-name match (`FUN_00609650`), shared by the
//! Fleet, Personnel and Troop Finders (`ghidra/notes/troop-finder.md`).

/// The row `FUN_00609650` chooses for `text` among `names`, in list order.
///
/// The best row starts as the first. A row whose first letter matches
/// (`FUN_005f3430`, without case) counts the letters it shares with the
/// text; a whole-name match also counts the terminator, as the compare
/// runs up to the shorter length plus one. A longer count takes over, an
/// equal one keeps the earlier row, and a shorter one ends the scan. So an
/// empty text, or one no row begins, chooses the first row.
///
/// The length test (`FUN_005f3040`, the length plus one) is 0 only for a
/// name box that never held a string: `FUN_00604cf0` builds it without a
/// buffer (`FUN_005f2f50`), and any edit or set gives it one
/// (`FUN_005f3ad0`, `FUN_005f3c30`, `FUN_00604f90`). Either way the first
/// row is the answer for an empty name, as the A0 Troop Finder opens.
#[must_use]
pub fn best_prefix_row<'a>(names: impl IntoIterator<Item = &'a str>, text: &str) -> Option<usize> {
    let typed: Vec<char> = text.chars().map(|c| c.to_ascii_lowercase()).collect();
    let mut best: Option<(usize, usize)> = None;
    for (index, name) in names.into_iter().enumerate() {
        let letters: Vec<char> = name.chars().map(|c| c.to_ascii_lowercase()).collect();
        let Some((_, best_count)) = best else {
            best = Some((index, 0));
            if letters
                .first()
                .is_some_and(|first| typed.first() == Some(first))
            {
                best = Some((index, shared(&letters, &typed)));
            }
            continue;
        };
        if letters
            .first()
            .is_none_or(|first| typed.first() != Some(first))
        {
            continue;
        }
        let count = shared(&letters, &typed);
        if count > best_count {
            best = Some((index, count));
        } else if count < best_count {
            break;
        }
    }
    best.map(|(index, _)| index)
}

/// The letters two names share from the start, plus the terminator when
/// they are equal.
fn shared(name: &[char], typed: &[char]) -> usize {
    let common = name.iter().zip(typed).take_while(|(a, b)| a == b).count();
    if common == name.len() && common == typed.len() {
        common + 1
    } else {
        common
    }
}

/// The row an edit's match scrolls to: `FUN_00609650` chooses
/// (`FUN_00609500`) and scrolls to (`FUN_00609ed0`) its best row only when
/// that row is not chosen already (`+0x3c` bit 0, `0x60973f..0x609743`);
/// an edit that keeps the same best row leaves the list where it is.
#[must_use]
pub fn row_to_show(choice: Option<usize>, chosen_before: Option<usize>) -> Option<usize> {
    choice.filter(|&row| Some(row) != chosen_before)
}

/// A key's step through a Finder's list. A0 2026-11-05 (finder-keys/): in
/// the Fleet Finder, Down and Up move the chosen row by one and copy its
/// name into the name box, also while the box has the focus; with the
/// list's focus (after a click on a row), Home chooses the first row and
/// End the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListStep {
    Up,
    Down,
    First,
    Last,
}

/// The step this frame's keys ask for ([`step_of`]).
#[must_use]
pub fn key_step(ui: &egui_macroquad::egui::Ui, name_focused: bool) -> Option<ListStep> {
    use egui_macroquad::egui::Key;
    let keys = ui.input(|input| {
        [Key::ArrowDown, Key::ArrowUp, Key::Home, Key::End].map(|key| input.key_pressed(key))
    });
    step_of(keys, name_focused)
}

/// The step for the keys pressed (Down, Up, Home, End): Up and Down
/// always, Home and End only while the name box has not the focus (they
/// move its caret there).
#[must_use]
pub fn step_of([down, up, home, end]: [bool; 4], name_focused: bool) -> Option<ListStep> {
    if down {
        Some(ListStep::Down)
    } else if up {
        Some(ListStep::Up)
    } else if home && !name_focused {
        Some(ListStep::First)
    } else if end && !name_focused {
        Some(ListStep::Last)
    } else {
        None
    }
}

/// The row a `step` chooses in a list of `len` rows from the `current`
/// one: Down and Up move one row and stop at the ends. With no row chosen
/// the list's own current row is its first: A0 finder-keys/p0, p1 (the
/// Personnel Finder opened with none chosen) take Down to the second.
#[must_use]
pub fn stepped_row(current: Option<usize>, len: usize, step: ListStep) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(match step {
        ListStep::Down => (current.unwrap_or(0) + 1).min(len - 1),
        ListStep::Up => current.unwrap_or(0).saturating_sub(1),
        ListStep::First => 0,
        ListStep::Last => len - 1,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_edit_scrolls_only_to_a_row_not_chosen_already() {
        // FUN_00609650: `if ((best->+0x3c & 1) == 0)` select and scroll.
        use super::row_to_show;
        assert_eq!(row_to_show(Some(3), None), Some(3));
        assert_eq!(row_to_show(Some(3), Some(1)), Some(3));
        assert_eq!(row_to_show(Some(3), Some(3)), None);
        assert_eq!(row_to_show(None, Some(3)), None);
    }

    #[test]
    fn a_key_steps_the_chosen_row_and_stops_at_the_lists_ends() {
        use super::{step_of, stepped_row, ListStep};
        // A0 finder-keys/f1..f6: Fleet 1, Down to 2 and 3, Up to 2, End to
        // 4, Home to 1.
        assert_eq!(stepped_row(Some(0), 4, ListStep::Down), Some(1));
        assert_eq!(stepped_row(Some(2), 4, ListStep::Up), Some(1));
        assert_eq!(stepped_row(Some(1), 4, ListStep::Last), Some(3));
        assert_eq!(stepped_row(Some(3), 4, ListStep::First), Some(0));
        assert_eq!(stepped_row(Some(3), 4, ListStep::Down), Some(3));
        assert_eq!(stepped_row(Some(0), 4, ListStep::Up), Some(0));
        // A0 finder-keys/p0, p1: none chosen, Down takes the second row.
        assert_eq!(stepped_row(None, 4, ListStep::Down), Some(1));
        assert_eq!(stepped_row(None, 4, ListStep::Up), Some(0));
        assert_eq!(stepped_row(None, 1, ListStep::Down), Some(0));
        assert_eq!(stepped_row(Some(0), 0, ListStep::Down), None);
        // The name box keeps Home and End for its caret.
        assert_eq!(
            step_of([true, false, false, false], true),
            Some(ListStep::Down)
        );
        assert_eq!(
            step_of([false, true, false, false], true),
            Some(ListStep::Up)
        );
        assert_eq!(
            step_of([false, false, true, false], false),
            Some(ListStep::First)
        );
        assert_eq!(
            step_of([false, false, false, true], false),
            Some(ListStep::Last)
        );
        assert_eq!(step_of([false, false, true, false], true), None);
        assert_eq!(step_of([false, false, false, true], true), None);
        assert_eq!(step_of([false; 4], false), None);
    }

    use super::best_prefix_row;

    #[test]
    fn an_empty_name_or_one_no_row_begins_chooses_the_first_row() {
        // FUN_00609650 starts its best row at the first; the A0 Troop
        // Finder opens with Coruscant (its first row) white and the name
        // box empty (2026-10-29, VM, both tabs).
        let names = ["Coruscant", "Fleet 1", "Selonia"];

        assert_eq!(best_prefix_row(names, ""), Some(0));
        assert_eq!(best_prefix_row(names, "Nebulon"), Some(0));
        assert_eq!(best_prefix_row([], "Cor"), None);
    }

    #[test]
    fn the_longest_shared_beginning_wins_and_ties_keep_the_earlier_row() {
        let names = [
            "Calamari Cruiser",
            "Corellian Corvette",
            "Corellian Gunship",
        ];

        assert_eq!(best_prefix_row(names, "cor"), Some(1));
        assert_eq!(best_prefix_row(names, "CORELLIAN G"), Some(2));
        assert_eq!(best_prefix_row(names, "cx"), Some(0));
        assert_eq!(best_prefix_row(names, "Coz"), Some(1));
    }

    #[test]
    fn a_whole_name_beats_a_longer_one_and_a_shorter_match_ends_the_scan() {
        // FUN_00609650 compares up to the shorter length plus one, so the
        // terminator counts; a count below the best breaks the loop.
        assert_eq!(best_prefix_row(["Fleet 10", "Fleet 1"], "Fleet 1"), Some(1));
        assert_eq!(best_prefix_row(["Fa", "F", "Fab"], "Fab"), Some(0));
        // An equal count keeps scanning.
        assert_eq!(best_prefix_row(["Fax", "Fay", "Fa"], "Fa"), Some(2));
    }
}
