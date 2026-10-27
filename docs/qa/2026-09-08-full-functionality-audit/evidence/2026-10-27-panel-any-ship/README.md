# The selected-ship panel shows any selected ship (P24)

Date: 2026-10-27. A review of the battle view's filters found that the
selected-ship panel stayed blank in two cases:
- our ship in no task force, for example after a withdrawal order or
  after a Ctrl click took it out of its force (`../2026-10-27-battle-clicks/`);
- ships selected from two forces.

The panel's background was drawn, but its name, art, meters, orders and
tactic were not. Tab did nothing either, because it used the same test.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository).

## The original

The one-unit state (`FUN_005a1550`), the several-unit state
(`FUN_005a2280`) and the enemy state (`FUN_005a1110`) show their unit the
same way. They frame it (`FUN_005c5f20`) and give it to the panel
(`FUN_0051c9b0`, which writes the panel's `+4`). None of them tests the
unit's group. The group number appears only as the panel's
"Task Force #" caption.

## The port

`crates/rebellion-render/src/tactical_view.rs`:

- `TacticalGroupHud::panel_ship_name` gives the selected ship whatever its
  group. The caption is still drawn only for a selected task force.
- The panel's ship is the clicked one (`selected_ship`) while it is still
  selected, else the first selected ship of ours, else the enemy ship.
  This follows `FUN_0051c9b0`, which gives the panel the clicked unit.
- The s139 test `the_panel_shows_a_grouped_or_an_enemy_ship` asserted that
  an ungrouped ship of ours showed no panel. That expectation came from
  the port's old behaviour, not from the original. It is now
  `the_panel_shows_any_selected_ship`, with the source above.

## Tests

- `the_panel_shows_any_selected_ship`
- `the_panel_shows_the_clicked_ship_of_a_mixed_selection`

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2964 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 7 tested: 6 caught, 1 unviable |
| Release WASM SHA-256 | `4bd26e85d431fff0b058bfa0498b6127d21597f815a6d9544ffe2d24e840ebc2` |
