# The original's enemy and group panels, observed in the Windows VM (P24)

Date: 2026-10-27. These are observations of the original REBEXE.EXE
running in the Windows 10 VM (`~/VMs/rebellion-win10`, QMP screendumps
at the game's 640×480). The captures are in the ignored store
`.artifacts/a0-vm/2026-10-27-controls/`; only their hashes are kept here.

The scenario: save `save_res1` (Empire, day 156). Imperial Fleet 1 (a
Victory Destroyer, the Death Star, a TIE squadron and a regiment) is sent
from Averam to Yavin. An Alliance fleet intercepts and the Battle Alert
"Battle at Yavin" opens. Take Command starts a battle, which the Alliance
leaves within seconds.

## What the original shows

| Capture | SHA-256 | Observation |
|---|---|---|
| `alert-summary.png` | `d1d8ddfa…ef7a` | "Battle at Yavin": "The Imperial fleet has entered the Yavin system. Alliance forces have been detected on an intercept course." |
| `alert-alliance-forces.png` | `939ebd7c…c60b` | The enemy tab lists "Fleet 8" and its ship "Swift" |
| `alert-imperial-forces.png` | `ebf1693a…e590` | "Fleet 1", "Victory Destroyer 2", then indented under it "TIE Fighter" and "Imperial Fleet Regiment", then "Death Star"; the list has a scroll bar |
| `enemy-selected-haste.png` | `239dd10f…fec4` | A left click selects the enemy ship "Haste": red frame (255, 0, 0); caption "Task Force Unassigned"; order line "Orders Unavailable", no tactic line; no Missions or Maneuvers buttons; the previous and next arrows are shown |
| `own-selected-victory-destroyer.png` | `ecb56e4f…c49d` | Our "Victory Destroyer 1": yellow frame, "Task Force # 1", "No Orders" and "Tactics: Stand Off", both order buttons |
| `red-group-panel.png` | `fe0ebf92…49d2` | The Red group's panel: "Red Group", "No Orders", "Tactics: Stand Off", "Fighters: 6 72/72", and no previous or next buttons |
| `camera-target-red-group-in-hangar.png` | `34a318b7…7b2d` | With the Red group selected before any launch, the camera target button looks at its carrier, the Victory Destroyer |
| `results-summary-sim.png` | `8e5e208e…5c3c` | A simulated run: "The Imperial fleet is victorious. Yavin is now under blockade…" |
| `results-tactical-withdrawn.png` | `69e7309f…417c` | The commanded run: "…maintained the blockade of Yavin. The Alliance fleet has withdrawn." |
| `after-tactical-yavin-system.png` | `a76acbeb…a859` | After the battle, the Yavin system window shows Fleet 1 with its fighter and troop icons; no squadron is left at the system |

## The code behind the panel, `FUN_005e4b40`

This is the capital panel's update (class `0x66d6e0`):

- **The caption (`+0xac`).** `FUN_005b1480()` gives the unit's group. With
  none, `DAT_006bd568` is drawn, which is TEXTTACT `0xdd9f` "Task Force
  Unassigned". Otherwise `DAT_006bd544` (`0xdd9c` "Task Force #") is drawn
  with the number.
- **The order line.** When the unit's side `+0x168` is the viewer's, the
  order text appears (`0xdd8f` "No Orders" and the others). Otherwise
  `0xddbf` "Orders Unavailable" appears.
- **The tactic line (`+0xb4`).** It appears only for the viewer's unit.
- **Buttons `0xdd01` and `0xdd02`.** They show only when `FUN_00596830`
  accepts the unit, which an enemy's is not.

## The port

`crates/rebellion-render/src/tactical_view.rs`:

- `TacticalGroupHud::panel_caption` gives "Task Force # n" for our ship
  in group n. It gives "Task Force Unassigned" for our ship in no group
  and for an enemy ship. Before this stage, the s139 enemy panel showed no
  caption, and so did the s150 ungrouped panel.
- `panel_orders_line` gives "Orders Unavailable" for an enemy ship. Its
  tactic line is no longer drawn.
- The Missions and Maneuvers buttons are not drawn for an enemy ship.
  They could not be clicked before either.
- An enemy ship's selection frame is red, (1, 0, 0).
- `camera_target_of_selection` gives a fighter group with every squadron
  in its hangar the carrier, which the camera then tracks.

Settled by these observations, with no change needed:
- The enemy list in the Battle Alert shows the enemy's ships, as the port
  does.
- After the battle the squadron is in its fleet, not at the system, as in
  the port. This run had no launch, so a launched squadron's place after
  a battle is still unobserved.
- A fighter group's panel has no previous and next buttons, as in the
  port.

## Tests

- `the_panel_caption_and_orders_follow_the_unit_and_its_side`
- `the_camera_target_of_a_group_in_its_hangar_is_the_carrier`
- `an_enemy_selection_frame_is_red`

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2967 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | first run 38 tested: 4 missed. After `enemy_selection_frame_color` and its test: 40 tested, 17 caught, 20 unviable, 3 missed |
| Release WASM SHA-256 | `2574e16a6aee4db64dc89634e1257452d69a855ffa15fd962488451b4488f8c0` |

Three mutants survive, all in the drawing functions, which need a live
macroquad window:
- `draw_original_tactical_hud` replaced with `()`;
- the `!` before `selected_enemy` that hides the tactic line;
- `alive && selected` against `||` in the frame loop of
  `draw_tactical_view`.

Their rules sit in `TacticalGroupHud` (`panel_caption`,
`panel_orders_line`, `selected_enemy`) and in
`enemy_selection_frame_color`, and tests catch those.
