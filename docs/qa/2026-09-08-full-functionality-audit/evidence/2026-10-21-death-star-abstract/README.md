# The Death Star in the abstract battle (F-033, W2i)

Date: 2026-10-21. The reading is in `ghidra/notes/abstract-battle.md`, "The
Death Star in the abstract battle" (commit `824a072c`).

## Change

`AbstractBattle::death_star_step` runs first in each side's step
(`FUN_005d14d0`).

- The Empire, with an operational Death Star, charges the superlaser by
  `ABSTRACT_DEATH_STAR_CHARGE_PER_STEP` = 1.0 x 1/3 x 0.001
  (`FUN_005ba7f0`). At 100 it destroys the first capital ship of its enemy
  list (`FUN_005d1a70`, `ranked_enemies`) and empties the charge
  (`FUN_005ba710`). A full charge takes 300,000 steps, so in a battle it
  never fires.
- The Alliance, with more active fighter units than the Empire
  (`FUN_005c5eb0`) and an attackable Death Star (`FUN_005ba450`), sends its
  first fighter task force on the trench run (`trench_run`,
  `FUN_005aefc0`):
  - members are visited in turn, up to 100 visits, after which every
    member left is destroyed;
  - a roll of 1..10 over the maneuverability (slot `+0xbc`) hits for
    (1..hull / 8) x 8, destroying a member with 1 or less to roll or no
    hull left;
  - a member left with hull rolls 1..100 against (torpedoes x 0.5333 x
    hull / full hull + 1.0) x 0.5;
  - success empties the Death Star's hull and charge (`FUN_005ba8a0`), and
    the port's Death Star capital ship falls, so its fleet loses the Death
    Star.
- `hyp:` the leader kind 4 is a fighter group, and no person commands
  (the commander term is 1.0).
- `port:` the units the Death Star carries (`+0x38`, `+0x3c`) are
  unknown and none falls with it.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against HEAD `824a072c`. 23 of 24 are identical; seed 3 changes
(`campaigns.json`). On day 1340 at Orto, an Alliance fleet of 5 ships and
15 squadrons meets the Death Star with 13 Imperial squadrons. The trench
run destroys the Death Star.

- Empire capital ships lost: 6 -> 7.
- Alliance wins: 741 -> 755.
- Empire wins: 1351 -> 1337.

## Gates

- Tests: 11 new tests.
  - The charge and fire, the trench run's hit and miss, the hit roll's
    threshold, the eights of damage, the shot's threshold at its exact
    boundary, the 100 visits, the turns, and the conditions.
  - The step calls the Death Star part.
  - With the Death Star part neutralized, 4 of the first tests fail.
  - Workspace: 2553 passed, 0 failed. Replay goldens pass; the only
    failure is the long-standing
    `seeds::tests::fixed_fleet_tables_only_target_special_systems`.
- Mutants on the diff:
  - The first run left 26 survivors. They were closed by RNG-pinned tests
    and by removing three unreachable guards: `operational` already holds
    hull and `+0x74`; a fighter task force holds only fighters; a live
    member has full hull.
  - Final run: 57 caught, 2 unviable, 0 missed (`mutants.txt`).
- `provenance-scan check`: 0 uncited.
- Browser, Arch Chromium 155.0.8059.84:
  - Fleet window 58 of 58;
  - replay equivalence passed (`v1:0174e1ddb8a28d5b` ->
    `v1:00f7cdfb6462bcb2`, no errors, 4 requests);
  - frame timing: very slow p99 0.7 ms, fast p99 1.6 ms, 0 frames over
    50 ms.
- Release WASM: `9b73d8fc5eb1a9fd0e4e81d0be6b0fc7e35231222fb8406081bfa30dae275ee3`.
