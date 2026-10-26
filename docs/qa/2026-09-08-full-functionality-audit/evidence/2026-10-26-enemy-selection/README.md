# A left click selects an enemy capital ship (P24)

Date: 2026-10-26. A playtest found that the port's tactical battle could
not select an opponent's ship: the left-click hit test skipped every ship
of the other side.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository).

## The original

The manual says "Left-click to select things, right-click to target
things" and "To select an individual capital ship, left-click on that
ship" (chapter 4, Tactical Display). It does not limit the click to the
player's own ships.

The interface's selection states pick the state for a clicked capital
ship by its side (`+0x168` against the player's side `+0x9ec`):

| Caller | Own ship | Enemy ship |
|---|---|---|
| no selection (`0x66be30` slot `+0xc`, `0x005a0d10`) | one-unit state `0x66bce8` (`FUN_005a1550`) | enemy state `0x66bba8` (`FUN_005a1110`) |
| click router `FUN_005a04f0` (capital, type 1) | the same | the same |
| enemy state (`0x66bba8` slot `+0xc`, `0x005a1240`) | one-unit state | enemy state |

- `FUN_005a1110` builds the same display as `FUN_005a1550`: the frame
  (`FUN_005c5f20`) and the panel's unit (`FUN_0051c9b0`). It also calls
  `FUN_005a11e0`, a sound while `+0x8c8` is 0 (not ported).
- The enemy state's order slot `+0x20`, slot `+0x24` and tactic slot
  `+0x28` are the empty `0x0048b440`, and so is slot `+0x14`. The enemy
  ship can be looked at, but it takes no order.
- From the enemy state, a Ctrl click (`0x005a1240`) starts a new one-unit
  selection. It never adds to the enemy's.
- An empty click (slot `+0x18`, `0x005a13a0`) goes back to no selection
  (`FUN_005a0c80`).

## The port

`crates/rebellion-render/src/tactical_view.rs`:

- The left-click hit test takes ships of both sides.
- `select_clicked_ship` holds the rule:
  - an own ship gathers with Ctrl, as before;
  - an enemy ship, or a Ctrl click while an enemy is selected, becomes the
    only selection.
- `selected_enemy_ship` lets `TacticalGroupHud` show the enemy ship's
  name, art, hull and shield meters, subsystems and contents in the
  selected-ship panel.
- port: an enemy force fills none of the player's group slots, so its
  panel has no "Task Force #" caption.
- Every order, tactic, escort, target and navigation path already filters
  the selection by the player's side, so the enemy ship takes no order.

## Tests

- `a_click_on_an_enemy_ship_selects_it_alone`
- `a_ctrl_click_from_an_enemy_selection_starts_a_new_one`
- `a_ctrl_click_on_an_own_ship_adds_it_to_the_selection`
- `an_enemy_selection_shows_its_ship_in_the_panel`: it fails without
  the enemy fallback in `TacticalGroupHud` (checked).
- `an_enemy_selection_takes_no_order`
- `an_enemy_selection_is_a_living_selected_enemy_ship`
- `the_panel_shows_a_grouped_or_an_enemy_ship`

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2920 passed, 0 failed (then render 957 passed after the mutant tests) |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed (`.artifacts/interface-parity/2026-10-26T22-49-26-016Z-4111880`) |
| Replay equivalence | passed |
| `cargo mutants` | 20 tested: 16 caught, 2 unviable, 2 missed (first run: 5 missed, three closed by the two tests) |
| Release WASM SHA-256 | `17ab794669e256d61e10c7424b71178ee46f36c1b958d262dae56c944fe8ced9` |

The two missed mutants replace `handle_combat_input` and
`draw_original_tactical_hud` with `()`. Both read the mouse or draw
through a live macroquad window, which unit tests do not have. The click
rule lives in `select_clicked_ship` and the panel rule in
`TacticalGroupHud::panel_ship_name`, and tests catch both.
