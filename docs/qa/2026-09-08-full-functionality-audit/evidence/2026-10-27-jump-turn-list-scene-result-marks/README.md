# Bow before the jump, the alert lists' paler scene, the results' damage marks (P24)

Date: 2026-10-27. Three faults the user reported after
`../2026-10-27-alert-scroll-bar/`:

1. A withdrawing ship did not turn its bow away before its jump; it
   snapped round as the jump began.
2. The Battle Alert's force lists stood on the summary's dark scene, which
   made the units hard to read.
3. The results' force pages drew no explosion cloud on damaged or lost
   units, and listed a unit that had jumped out as Destroyed.

## 1. The turn before the jump

- The original: `FUN_005b4d40` (the member's heading, slot `+0x80`)
  returns the side vector reversed (`FUN_005a7490`, `FUN_005b5090(-v)`)
  while its force's order (`+0x64`) is 3. It does not depend on where the
  ship stands, so a ship keeps turning on and past its exit line while it
  waits for the side's time. `FUN_005a6b40` then sets the forward to the
  same vector on each step of the jump.
- The port aimed the heading at the exit line's z. On the line that delta
  was 0, so the heading fell back to the current forward and the turn
  stopped; past the line the ship turned back toward it. The heading is
  now the side vector reversed.
- Test: `a_withdrawing_member_heads_for_its_exit` gains the on-the-line
  and past-the-line cases. It failed before the change.

## 2. The list tabs' scene

- `FUN_0044f860` builds surface `0x32` on the scene `0x29d8`/`0x29d9`
  (10712/10713, 400 by 310) and surface `0x33` on `0x29da`/`0x29db`
  (10714/10715, 410 by 280), both at (12, 13). `FUN_00450660` shows `0x32`
  on the summary and `0x33` on the Alliance, Imperial and System tabs.
- The original's capture (A0 VM `alert-imperial-forces.png`, SHA-256
  `ebf1693a1ae0797747727c80e15dc7913e07596a83285d6969c8871a5a33e590`)
  matches 10715 at (137, 65), the window's (12, 13): 89.5% of its first
  400 columns (the rest is the text and the list), 1% of its last 10,
  which the frame covers (100% frame).
- The port: `BattleAlertSkin.list_scene` and `battle_alert_scene`; the
  scene is cut to the frame's 400-wide aperture. `draw_original_bitmap`
  clamps a cut to its bitmap.
- Probe `battle-alert-list-scene`: on each list tab 10714/10715 matches
  pixel for pixel outside the title, the list and the covered columns
  (34,320 pixels).

## 3. The results' marks and escaped units

- `FUN_004724a0` draws each unit through `FUN_0042c3b0(.., 1, 1)`. With
  its last argument set, a damaged object (`+0x50` bit `0x200`) takes its
  GOKRES portrait + `0x2000` under the portrait (`fleet-window.md`):
  - a ship is damaged while its hull damage or a system nibble is not 0
    (`FUN_00501c00`);
  - a squadron while its losses (slot `+0x254`, `+0x60`) are not 0
    (`FUN_00503340`).
- A lost unit keeps the bit its full damage set, so the Destroyed column
  shows the mark too. This was a hypothesis until 2026-10-28, when the
  user checked it on the original, together with the turn before the
  jump and the launches from the hangar.
- `FUN_005a90c0` (event `0x2e`) ends a jump in state 9; destruction is
  state `0xb`. The port's results counted every unit that was no longer
  alive as Destroyed, so a ship (`retreated`) or a group whose jump had
  ended was listed there. They now stay Operational.
- Test: `an_escaped_unit_is_not_destroyed_and_a_damaged_or_lost_one_wears_its_damage_mark`.
- Not covered: the regiment and personnel pages draw no damage mark.

## Captures

Linux Chromium, 640x480. The alert runs are `2026-10-27T21-48-48-320Z-205650`
and the results runs are `2026-10-27T21-57-49-369Z-228240`.

| Capture | SHA-256 |
|---|---|
| alliance `battle-alert-tab-1.png` | `000652bf034ebccdef090b6772d5db55b2568c7b443df2e65fc3015be05e21e1` |
| alliance `battle-alert-tab-2.png` | `1496bb11fd3b9c23709841154b1603ae5fc6ea27a9f4109644410ffd79c704f1` |
| alliance `battle-alert-tab-3.png` | `bcaf5d77785429a6f955431c41cac8d3d46eb3b0059153786f4aefabd1ad37d2` |
| empire `battle-alert-tab-1.png` | `c1734c9a3c40cef6101b9fd3c22fc89afce75a633aa885d09bf5beb2603ea139` |
| empire `battle-alert-tab-2.png` | `2a24d13dccca31ac9f41667e963e9085e1dee5b1e608199afeb7ca1e8fcd4d9d` |
| empire `battle-alert-tab-3.png` | `99a486ef3f42e5d78cc0cfe925ee9777a3178d308fe4942ccf376889e1f32d52` |
| alliance `results-category-1.png` | `ae1972a9d89c3bba6cbcfd63e5ff968fb5f73fe4367883c1581353a39cb67cc1` |
| alliance `results-category-2.png` | `0bfefb29e56fdc388a03147319fcdc72b20379070aff7667d77e69fa721cab07` |
| empire `results-category-1.png` | `c3f56e9d91b70f93307344ac1db340dc9ead910d3612ecfbdb2b0b26fa38d801` |
| empire `results-category-2.png` | `7b91b66da5ab56a95b63035a998578531fb4cab1abfa682c9450f9f46efee399` |

The results captures show the cloud under the damaged operational Mon
Calamari Cruiser and A-wings and under the lost ones, and none under the
undamaged ones.

## Gates

- Browser `--battle --all`: 148/148 passed.
- Workspace: 3006 tests passed. Provenance scan: no uncited items.
- Replay equivalence and the replay manifest passed; the golden is
  unchanged.
- Mutants (43): 30 caught, 4 unviable, 9 missed, each explained:
  - two in the heading's x and y negation, which were `-0.0` on a side
    vector that lies along z; the heading now writes 0 for both;
  - seven in `draw_original_bitmap`, macroquad drawing that no unit test
    reaches; the `battle-alert-list-scene` probe fails if the cut is
    wrong (it did, at column 400, before the cut).
