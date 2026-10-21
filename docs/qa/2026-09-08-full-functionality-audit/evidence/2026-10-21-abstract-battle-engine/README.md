# The abstract battle engine (F-033, W2d)

Date: 2026-10-21. `crates/rebellion-core/src/abstract_battle.rs` steps a
tactical `BattleSession` the way the original resolves an abstract battle
(`ghidra/notes/abstract-battle.md`). The campaign does not use it yet.

## Ported from the original

| Rule | Source |
|---|---|
| One step is one second: `dt` = 1.0 | `FUN_0059a530`, `DAT_0066badc`; rates per second via `FUN_005a9de0` × 0.001 |
| Each task force with a target deals its summed rate × dt | `FUN_005d14d0`, `FUN_005cb650`, `FUN_005b54d0` |
| A unit's rate: strongest arc, family strengths and modifiers, maneuverability ratio, fighter hull scaling and truncation, torpedoes, recharge per second | `FUN_005b7780`, `FUN_005b4c60`, `FUN_005b1770`, `FUN_005b23a0`, `FUN_005b0400`, `FUN_005b9c00`, `FUN_005b0be0` |
| Shield and weapon recharge each step | `FUN_005b6320` |
| A side with no active unit is beaten, except the Empire while its Death Star stands | `FUN_005d14d0` |
| Strength, leaving out out-damaged ships; +20,000,000 for an operational Death Star | `FUN_005d0ce0`, `FUN_005b7a80`, `_DAT_0066d0f0` |
| 1200 seconds without a fall destroys the weaker side, or both on a tie | `FUN_005ae9c0`, `DAT_0066c3e0`, `FUN_005af7a0` |
| Winner: the only side with survivors | `FUN_0040c780` |

## Not the original yet

- `hyp:` targeting. A task force keeps a live target, otherwise it takes
  the first enemy of the class its leader's lasers favour
  (`FUN_005b7ce0`). The scoring `FUN_005ec740` / `FUN_005ed170` is next.
- `hyp:` a fighter group hit by `FUN_005b54d0` loses shields, then hull.
- `port:` a capital ship's integer hull and shields carry fractions of a
  point between steps.
- Not ported: the retreat decision (slots `+0x0c`, `+0x2c`) and the Death
  Star's abstract actions.

## Probe

`probe.patch` (not committed) resolved each strategic battle of seeds 1–3
(1500 ticks, dual AI) with `resolve_abstract`. `probe.txt` has one line per
battle. Results:

- all 67 battles ended;
- 28 attacker wins, 39 defender wins, no draw, no stalemate;
- 1 to 1008 steps, median 43.

The headless `step()` probe before it dealt no damage in any of the 67.

## Tests

22 unit tests in `abstract_battle.rs`, each citing its source. The
mutation results are in `mutants.txt`.
