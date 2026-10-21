# The abstract battle's targeting (F-033, W2e)

Date: 2026-10-21. The abstract battle engine
(`crates/rebellion-core/src/abstract_battle.rs`) now chooses targets the
way the original's side commanders do. Before, it used a `hyp:` rule. The
campaign does not use the engine yet.

## Ported

| Rule | Source |
|---|---|
| Enemy list: active units, highest priority first; priority = 15 × strength − 0.01 × (shield-generator ratio + hull ratio) | `FUN_005d1a70`, `FUN_005ed8e0`, `FUN_005b1710`, `FUN_005b1640` |
| Fighter list (fighters, then ships) for task forces whose leader carries lasers; ship list (ships, fighters, ships) for the others | `FUN_005c9200`, `FUN_005b7ce0` |
| A task force keeps an eligible target while the side's task forces on it score under 300 | `FUN_005ed5f0`, `DAT_0066da84` |
| Eligible: active, with weapon recharge | `FUN_005ed550` |
| Score: (hull × 0.65 + shields) ÷ (running rate − target shield recharge), plus 0.001 × max(1, travel seconds); −1 when the rate cannot win | `FUN_005ed170`, `DAT_0066da8c`, `DAT_0066da98`, `DAT_0066da9c` |
| Pairing, per target: alone in 0..180; otherwise from the tail up with the head's rate counted, linking the tail through the first that fits (the head itself is not linked); then the idle take the first eligible target | `FUN_005ec740`, `DAT_0066cbac`, `DAT_0066da58` |

`hyp:` items remain:

- The travel speed is the leader's engine power. The original reads slot
  `+0x34` of the task force object `+0x54`.
- A fighter group's shield-generator ratio is taken from its hull.
- The priority's game-list terms are taken as empty when there is no
  display.

## Probe

`probe.txt` comes from the same hook as the W2d probe
(`../2026-10-21-abstract-battle-engine/probe.patch`), seeds 1–3:

- all 67 battles end;
- 26 attacker wins, 41 defender wins, no stalemate;
- median 43 steps;
- 55 of 67 battles leave the same survivors as with the `hyp:` rule.

## Tests

31 tests in `abstract_battle.rs`. The test "weak task forces combine and
the head moves on" fails when the head is linked too (checked by hand).
The mutation results are in `mutants.txt`.
