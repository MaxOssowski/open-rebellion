# Tab steps through a task force's ships (P24)

Date: 2026-10-27. This follows `../2026-10-27-battle-clicks/`. The
manual's tactical keys list Tab and Shift+Tab for switching between the
ships of a task force. In the port only the panel's arrow buttons did
that.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository).

## The original

- **The key handler.** `FUN_0059b730` sends Tab (case 9) to the panel
  window (`+0x10c`) as a `WM_KEYDOWN`. It does not repeat while the key is
  held: bit 30 of the key data is checked.
- **The capital panel** (class `0x66d6e0`, constructor `FUN_005e4110`).
  Its previous and next buttons are `0xdd03` at (7, 10) and `0xdd04` at
  (132, 10), 11 × 53. These are the port's buttons at (489, 38) and
  (614, 38).
  - Its slot `+0x58` (`0x005e45a0`) presses `0xdd04` on Tab and `0xdd03`
    on Shift+Tab (`GetKeyState(VK_SHIFT)`), on key-down only.
  - `FUN_005e44d0` sends those buttons to `FUN_00595cc0` and
    `FUN_00595ce0`, the next and previous ship.
- **The squadron panel** (class `0x66d750`, constructor `FUN_005e7030`).
  It has the same two buttons, `0xdd02` and `0xdd03`, and Tab keys of its
  own (`0x005e74e0`).

## The port

`crates/rebellion-render/src/tactical_view.rs`:

- `tab_panel_control` gives `NextCapital` for Tab and `PreviousCapital`
  for Shift+Tab, while the selected-ship panel shows.
- `activate_tactical_command` runs that button, exactly as a click does.

Not ported: the squadron panel's previous and next buttons, and so its
Tab keys. The port's squadron panel has neither.

## Tests

- `tab_steps_through_the_capital_panel_only_while_it_shows`

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
