# The battle report names the viewer's withdrawal destination (P24)

Date: 2026-10-27. The port's battle report and Display page left out
where the viewer's withdrawn fleet went. They read "has withdrawn"
(`0x700c`) instead of "has withdrawn to {2:1}" (`0x700d`), and dropped
" to {3:1}" from `0x7011`, "because the destination is decided after the
report". This stage gives the report its destination.

## The original

- `FUN_0049ac00` builds the report. `FUN_0049b720` fills `+0x48`: for each
  of the viewer's (`+0x20`) withdrawing fleets (flag `0x20`), while
  `+0x48` is empty, it stores the fleet's destination system.
  `Class_0065bc08_vt20` (`FUN_0049b060`) takes the winner (`+0x50`) and
  the withdrawn side (`+0x54`) from the system's `+0x78`.
- The viewer's loss, `FUN_0049b4d0`: when `+0x54` is set, the body reads
  `0x700d` with the destination's name, and the Display line reads
  `0x7011` with it as `{3:1}`. When no destination is found, the function
  returns without the body.
- No winner, `FUN_0049b150`: when the withdrawn side is the viewer's
  (`+0x54 == +0x58`), it reads `0x700d`, `0x700b` for the enemy, and
  `0x7011`. The enemy's withdrawal reads `0x700c` and `0x7010`, with no
  destination.
- TEXTSTRA.DLL RCDATA `0x700d` as shipped: "The {1:4} fleet has withdrawn
  to {2:1}. " (bytes `7c 02 01` before ". ").

### The A0 run (Windows VM, Battle at Yavin)

In `results-tactical-withdrawn.png`, the Imperial viewer won and the
Alliance withdrew. The report reads "The Alliance fleet has withdrawn.",
with no destination: `0x700c` for the enemy, as above. The viewer's own
withdrawal was not captured.

| Capture (`.artifacts/a0-vm/2026-10-27-controls/`) | SHA-256 |
|---|---|
| `results-tactical-withdrawn.png` | `69e7309f04927c8837969443b31b281ed3bbc61c247b07fc026f1d620875417c` |

## The port

- `withdrawal_destination` in `crates/rebellion-core/src/abstract_battle.rs`
  is `withdraw_loser`'s search (`FUN_005545e0`, `nearest_place` without
  the battle's system). `withdraw_loser` now calls it for each fleet.
- `battle_report` in `crates/rebellion-render/src/tactical_view.rs` asks
  for it when the viewer's side withdrew. `BattleReport.destination` is
  `+0x48`.
- `battle_report_lines` reads `0x700d` with the destination for the
  viewer's withdrawal, and `0x700c` for the enemy's.
- `battle_display_lines` fills `0x7011`'s `{3:1}`.

Not exact:
- port: the destination comes from the same search the withdrawal uses,
  not from the moving fleet's record. The search reads only system
  control, blockades and fleets at other systems, and the battle does not
  change those.
- port: when no destination is found, the port still writes the body,
  using `0x700c` and `0x7011` without " to {3:1}". The original writes no
  body.

## Tests

- `the_viewers_withdrawn_fleet_is_reported_at_its_destination`
- `the_report_finds_the_viewers_withdrawal_destination_in_the_world`
- `the_display_page_tells_the_viewer_where_its_fleet_is` (extended)

All three fail without the change: the body reads "has withdrawn." and the
destination is `None`.

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2976 passed, 0 failed |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 22 tested: 13 caught, 8 unviable, 1 missed |
| Release WASM SHA-256 | `7b1f99cb7ef1e1d18fbed41bcfcb461795691c48daddbdf6b6252e53dc6bf216` |

The missed mutant replaces `draw_battle_result_summary` with `()`. That
function draws into a live macroquad window, which the unit tests cannot
open; the lines it draws are tested through `battle_result_summary_lines`.
