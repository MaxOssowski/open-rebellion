# A second click on the chosen maneuver cancels it (P24)

Date: 2026-10-27. The manual: "To cancel orders for the currently selected
ship, task force, or fighter group, bring up the Maneuvers or Mission
button, deselect any selected orders and confirm without choosing
anything else" (chapter 4). In the port a maneuver button stayed chosen
after a second click, so nothing could be deselected.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository).

## The original

- **The maneuver panel's constructor.** It creates the five maneuver
  buttons `0xdd10`..`0xdd14` (`FUN_00602150`, resources `0x46d`..`0x476`).
  Right after each one it calls `FUN_006030c0(8)`, which sets style bit 8
  in the button's `+0xc4`.
- **The button class (`FUN_00604170`).** When a button with style 8 is
  activated, it flips its pressed bit (`+0xc4 ^= 4`). So these are toggles.
- **A click on a maneuver (`FUN_005dd080` -> `FUN_005dd140`).** It releases
  every other maneuver button (`FUN_006030f0(4)`).
- **The commit (`FUN_005dd3a0`).** It takes the one button that is still
  pressed (`FUN_00603120(4)`) as orders 7 to 11, or order 0 when none is.
  Clicking the chosen maneuver again and confirming therefore cancels the
  order.
- **The Missions panel (`Class_0066d608` slot `+0x54`, `0x005df4f0`).** It
  keeps the order in `+0xc4` (`FUN_005df5f0`), which a click always sets.
  Its commit (`0xdd01` -> `FUN_00595c00(+0xc4)`) does not read the button
  state, so a mission cannot be deselected the same way.

## The port

`crates/rebellion-render/src/tactical_view.rs`:

- `toggled_maneuver`: clicking the pending maneuver gives `None`, and any
  other maneuver replaces it.
- Confirming with `None` gives the selection order `None`, through
  `assign_selected_command`.
- The Missions panel is unchanged.

## Tests

- `a_second_click_on_the_chosen_maneuver_cancels_it`

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2960 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 3 tested: 3 caught |
| Release WASM SHA-256 | `a074ed62edef9d6ba04887e89bbaa6c91e0b034edb3a09d89418046343da9318` |
