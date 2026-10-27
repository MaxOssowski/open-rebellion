# The battle report says when a blockade was maintained or broken (P24)

Date: 2026-10-27. The port's battle report did not keep the report's
`+0x5c`, so it never read RCDATA `0x7015` "{2:4} forces have maintained
the blockade of {1:1}. " or `0x7016` "{2:4} forces have broken the
blockade of {1:1}. ". In the A0 run the original read `0x7015` where the
port read "is now under blockade".

## The original

- `+0x5c` is the system's last arrived side. `FUN_0049b720` copies it
  from the battle record's `+0x4c`. `FUN_0049c0d0`, called from the
  record's constructor `FUN_0049be60` at the battle's start, writes it as
  the system's `+0x78` bits 6..7. `FUN_0050a1b0`, called from
  `System_vtc8` (`FUN_00508660`), sets those bits to an entering fleet's
  or unit's side.
- The viewer's win (`FUN_0049b330`), when the system's side (`+0x24`
  bits 6..7) equals `+0x5c`, reads `0x7016` with the viewer's side if the
  system is the viewer's, else `0x7015` with the viewer's side. Otherwise
  it reads `0x7005` or `0x7006` as before, and `0x7007` at a neutral world.
- The viewer's loss (`FUN_0049b4d0`), in the same case, reads `0x7015`
  with the enemy's side if the system is the viewer's, else `0x7016` with
  the enemy's side.
- The TEXTSTRA.DLL RCDATA records as shipped:
  - `0x7015`: `|02 04| forces have maintained the blockade of |01 01|. `
  - `0x7016`: `|02 04| forces have broken the blockade of |01 01|. `

### The A0 run (Windows VM, Battle at Yavin)

The Alliance fleet had just arrived at Alliance-held Yavin, which the
Empire had blockaded. The Imperial viewer won the battle. The report
reads: "Imperial forces have maintained the blockade of Yavin. The
Alliance fleet has withdrawn."

| Capture (`.artifacts/a0-vm/2026-10-27-controls/`) | SHA-256 |
|---|---|
| `results-tactical-withdrawn.png` | `69e7309f04927c8837969443b31b281ed3bbc61c247b07fc026f1d620875417c` |

## The port

`BattleReport.last_arrival` is `+0x5c`, read from the system's
`last_arrival` (`battle_alert_sides`, the same field the Battle Alert
reads). `battle_report_lines` picks `0x7015` or `0x7016` as above.

Not exact:
- port: the port reads `last_arrival` when the report is drawn, not when
  the battle starts. No unit arrives during a battle. The withdrawal is a
  departure, which does not set it.

## Tests

`a_blockade_is_maintained_or_broken_when_the_systems_side_arrived_last`
fails without the change: the first case reads "Yavin is now under
blockade by Imperial forces."

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2977 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 12 tested: 11 caught, 1 unviable, 0 missed |
| Release WASM SHA-256 | `b116dcc16fd34b9c93926e2d48aa85ff7d92b65fd036753d5963a9bbc1b80fca` |
