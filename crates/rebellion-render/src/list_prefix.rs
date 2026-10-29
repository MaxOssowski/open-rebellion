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
/// hyp: the name box always holds a string buffer, so its length test
/// (`FUN_005f3040`, the length plus one) never stops the match; the A0
/// Troop Finder opens on its first row with an empty name box.
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

#[cfg(test)]
mod tests {
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
