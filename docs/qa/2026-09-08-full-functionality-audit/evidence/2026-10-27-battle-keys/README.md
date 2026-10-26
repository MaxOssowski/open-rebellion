# The tactical battle's keys follow the original (P24)

Date: 2026-10-27. A check of the manual's tactical chapter against the
port found several keys missing or different:
- Space did nothing;
- 1 to 8 did nothing;
- the camera keys were missing;
- F1 to F12 also changed the selection.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository, and `ghidra/notes/FUN_0059b730.c`).

## The original

### The key handler, `FUN_0059b730`

The handler takes `WM_KEYDOWN` and `WM_SYSKEYDOWN`. Most keys press a HUD
button through `FUN_0059ca90(id, 1)`, which runs that button's
`FUN_0059b130` case. The key-up messages release the camera buttons with
`FUN_0059ca90(id, 0)`.

| Virtual key | Button |
|---|---|
| `0x0d` Enter, `0x6e` NumPad . | `0xad21` camera target |
| `0x21` Page Up, `0x69` NumPad 9, `0x6b` NumPad + | `0xad1b` zoom in |
| `0x22` Page Down, `0x63` NumPad 3, `0x6d` NumPad - | `0xad1c` zoom out |
| `0x23` End, `0x61` NumPad 1 | `0xad1a` recall |
| `0x24` Home, `0x67` NumPad 7 | `0xad19` memorize |
| `0x25` Left, `0x64` NumPad 4 | `0xad1d` |
| `0x26` Up, `0x68` NumPad 8 | `0xad1f` |
| `0x27` Right, `0x66` NumPad 6 | `0xad1e` |
| `0x28` Down, `0x62` NumPad 2 | `0xad20` |
| `0x31`..`0x38` (1 to 8) | `0xad02`..`0xad09`, the task-force buttons |

The other keys:
- **1 to 8:** before the press, `FUN_0059b430` and `FUN_0059b600` release
  the eight task-force buttons and the four fighter-group buttons.
- **F1 to F8 and F9 to F12:** these press no button. When that task force
  or fighter group exists (`FUN_00595ba0`, `FUN_00595bc0`), the camera
  follows it (`FUN_005c2990` mode `0xb`). The selection stays.
- **Space (`0x20`):** `FUN_00595b10(0x20)` queues the key as a command
  (`FUN_005a0ad0`, class `0x66bd48`). The command switch `FUN_0059fd40`
  passes code `0x20` to the selection state's slot `+0x1c`. It does so
  only while the interface takes commands: the context's first word, set
  to 1 once the battle is loaded (`FUN_005aacb0`, state 2 to 4).
- **Tab:** it goes to the panel window (`+0x10c`). This is not traced here.

The main row's `=` and `-` are not keys of the original.

### Space, slot `+0x1c`

- One ship (`0x66bce8`, `0x005a21e0`): the point is the ship's position
  (`FUN_005c6070`, then `FUN_005a2260` copies `+0x30`..`+0x3c`).
- Several ships (`0x66bc98`, `0x005a3250`) and a task force (`0x66bc48`,
  `0x005a4180`): the point is the members' centroid. Each member adds 1
  to the count (`x - (-1)`, `DAT_0066bb50`), and the sum is scaled by
  1 / count (`DAT_0066bb54`); the decompiled sign is wrong.
- From the point, `FUN_005aa970` takes the nearest unit of the other side
  (`FUN_005a61c0`) that is still in the fight (`FUN_005ad870`), with the
  type `+0x20` 0 (a capital ship). When there is none, it searches again
  with type 1 (a squadron). The first unit found keeps a tie.
- The state's slot `+0xc` then gets that unit with 0, as a right click
  without Ctrl does. This sets the target.
- The enemy state (`0x66bba8`), the fighter state (`0x66bb58`) and no
  selection (`0x66be30`) have the empty `0x006158b0`.

## The port

`crates/rebellion-render/src/tactical_view.rs`:

- `activate_tactical_hud_control` runs a HUD button for the mouse and the
  keys alike. `TACTICAL_BUTTON_KEYS` holds the table above.
- port: a held key acts once, when it is pressed.
- F1 to F12 set only the camera follow, and only for a group with a
  living unit (`task_force_exists`, `fighter_group_exists`).
- `attack_nearest_enemy` holds the Space rule. It uses
  `BattleSession::nearest_unit_of_side` (`FUN_005aa970`) and
  `assign_selected_focus_target`, which can now also take a squadron.
- port: a fighter group's Space waits for the right-click targets of
  fighter groups.
- The browser probe of the group buttons now presses 1 (task force 1). It
  checks that F12 leaves the selection alone, where before it expected
  F12 to select the gold group.
- That journey now logs two camera states, where it used to log three:
  the initial one and F12's follow. Key 1 presses the task-force button,
  which does not move the camera (`FUN_0059b450`).

## Tests

- `space_targets_the_enemy_capital_nearest_the_selected_ship`
- `space_searches_from_the_selected_ships_centroid`
- `space_falls_back_to_the_nearest_enemy_squadron`
- `space_with_an_enemy_or_no_selection_does_nothing`
- `each_battle_key_presses_its_original_button`
- `a_camera_follow_key_needs_a_group_with_a_unit_left`
- `the_nearest_unit_uses_the_squared_distance_over_three_axes`
- `the_nearest_unit_keeps_the_first_of_a_tie_and_skips_other_units`
- `space_aims_from_the_exact_centroid_on_every_axis`
- `f_keys_follow_task_forces_then_fighter_groups`
- `the_hud_buttons_pause_zoom_and_toggle`

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2933 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | first run 144 passed, 4 failed: `production-group-presentation` still expected three camera states. After that expectation was corrected, the group and navigation-camera scenarios ran again: 8 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | first run 80 tested: 31 missed. After the tests above, 82 tested: 8 missed, and then 30 tested on `attack_nearest_enemy` and `camera_follow_for_key`: 28 caught, 2 unviable |
| Release WASM SHA-256 | `3ee19e8d1ae64be9c506021682a78a437f526857dd1bb5b4868e66cf315af62b` |

Four mutants still survive. Each sits in a line that reads the keyboard
through macroquad, which unit tests cannot drive:
- `!is_key_pressed` in the follow-key loop;
- the Ctrl test (`||`) of the task-force button;
- its `changed > 0`, in two forms.

That button code moved unchanged from the click handler into
`activate_tactical_hud_control`. The browser group probe covers it: task
force 8 is selected by mouse and task force 1 by key 1.
