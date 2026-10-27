# The camera target button looks at a fighter group too (P24)

Date: 2026-10-27. This follows `../2026-10-27-battle-tab/`. The manual
says the camera can centre on the selected ship, task force or fighter
group. The port's camera target button and Enter looked only at a
selected ship.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository).

## The original

- **The button.** `0xad21` (also Enter and NumPad .) calls `FUN_005c2990`
  with mode 9.
  - While a chase camera (`+0x644`) runs, mode 9 first calls mode 7. That
    removes the chase and restores the base camera.
  - Then the current camera's slot `+0x10` gets 9.
- **The free camera.** Its handler for 9 (`FUN_005d97c0`) sets its target
  (`+0x44`) to `FUN_00595be0`, and clears `+0x40`.
- **What `FUN_00595be0` gives.** It asks the selection state's slot `+4`
  (`FUN_00599c80`):

| State | Slot `+4` | Target |
|---|---|---|
| one unit `0x66bce8` | `0x005a5910` | its unit |
| enemy `0x66bba8` | `0x005a5680` | its unit |
| fighter group `0x66bbf8` | `0x005a4710` | its current squadron (`FUN_005c6070`, `+0xec`) |
| task force `0x66bc48` | `0x005a3960` | from the force (`+0x58`); not read further |
| navigation point `0x66bb58` | `0x005a55f0` | the point |

## The port

`crates/rebellion-render/src/tactical_view.rs`:

- `camera_target_of_selection` gives the camera button its object:
  - a selected ship, as before. Its id is the ship's index + 1, and it
    also covers a task force through its first ship.
  - for a fighter group, the first squadron that still flies, with id
    1001 + index as in the fighter projections.

Not exact:
- port: the camera looks at the target's position at the press. It does
  not keep tracking the target as the original's `+0x44` might.
- port: the task force uses its first selected ship, because
  `0x005a3960` is not traced past the force lookup.
- Not ported: navigation-point selection, so the camera never targets a
  point.

## Tests

- `the_camera_target_is_the_selected_ship_or_the_groups_squadron`

## Gates

This stage was gated together with its sibling (`../2026-10-27-battle-tab/`
and `../2026-10-27-camera-target/`) and committed with it.

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2958 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 39 tested: 17 caught, 21 unviable, 1 missed |
| Release WASM SHA-256 | `6c2a6e799285f5ce90b965b883d3abc9f63ed86ad2a9c363c69bdef707e5c09c` |

The missed mutant is the Shift test (`||` between the two Shift keys) in
`handle_original_tactical_controls`, which reads the keyboard through
macroquad. The key-to-button rule is `tab_panel_control`, and a test
catches it.
