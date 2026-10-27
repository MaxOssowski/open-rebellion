# A lost ship's characters take the injury roll (P24)

Date: 2026-10-27. This follows `../2026-10-27-character-ships/` (save
v107), which set a lost ship's characters Adrift. The original then rolls
each one an injury, which can kill a minor character. This stage ports
that roll for a battle's losses.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository). The GNPRTB values below were read from
GNPRTB.DAT with `dat-dumper`.

## The original

- `CapShip_vt11c` (`0x00502840`), for a destruction reason from 1 to
  `0x10` (`cmp eax,1 / jl`, `cmp eax,0x10 / jg`), walks the ship's
  children of families `0x30..0x3c` (`FUN_004f25c0`). It skips those en
  route (`+0x50` bit 5) and calls slot `+0x2e8` on the rest. A battle's
  loss has reason `0xc` (`FUN_0040cfc0` -> `FUN_00501ae0`).
- `Character_vt2e8` (`FUN_004ef640`) calls
  `FUN_0053ea00(effective combat, &injury)` and then
  `+0x2f0(injury, 10, ...)`:
  ```
  uVar2 = FUN_0053e130(Gnprtb_2568,100 - param_1);
  iVar3 = FUN_0053e2f0(uVar2);
  uVar1 = Gnprtb_2570;
  if (iVar3 != 0) {
    iVar3 = FUN_0053e290(uVar2);
    iVar4 = FUN_0053e290(uVar1);
    *param_2 = iVar4 + iVar3 + Gnprtb_2569;
  }
  ```
  The chance (`uVar2`) is drawn before the spread (`uVar1`).
- `Character_vt2f0` (`FUN_004ef6d0`): when the injury is above 0, it is
  added to `+0x94` (`FUN_004ee2d0`). Slot `+0x2e0` then decides death:
  - a minor character (`0x00665be0`) dies with GNPRTB 2577 (25%, through
    `FUN_0056f160`), and `+0xac` destroys it with reason 10;
  - a major character never dies this way (`FUN_006158b0` returns 0).
- GNPRTB.DAT as shipped, all difficulty columns:
  - 2568: 1, the smallest chance;
  - 2569: 1, the base;
  - 2570: 29, the spread.

## The port

- `missions::injure_passengers(world, characters, rolls)` does the roll.
  It reuses `uprising::roll_injury`, the same shape as `FUN_0053e990`,
  then `Character::injure`, then `mission_detection::dies_of_injury` and
  `kill_character`.
- The abstract battle (`integrator::apply_space_combat_result_inner`)
  injures the characters that `strand_lost_ships_passengers` returned.
  Its `passenger_rolls` are the battle's `combat_rolls` after the first,
  which seeds the abstract battle. The rest of those 256 rolls went unused
  before, so the campaign's random stream is unchanged.
- The tactical battle (`tactical_flow::apply_results`) draws from a copy
  of the session's `tactical_rng`.

Not exact:
- port: the tactical rolls continue the session's MSVC `rand` stream
  (`FUN_0061a310`), mapped to fractions for `Rolls`. The original draws
  `FUN_0053e290` from its own generator.
- The bombardment's and sabotage's ship losses (`scrap::destroy_ship`)
  set the characters Adrift but do not roll injuries yet. Those callers
  have no rolls at that point.

The seed-42 replay golden is unchanged: its 25 days lose no ship with a
character aboard.

## Tests

- `a_lost_ships_passengers_take_the_injury_roll_and_a_minor_one_may_die`
  (missions)
- `an_injury_of_nothing_draws_no_death_roll` (missions)
- `a_lost_ships_characters_go_adrift_and_the_fleet_keeps_the_rest`
  (integrator, extended with the roll)
- `each_character_of_the_lost_ships_draws_its_own_injury`
  (tactical_flow)

The first and the integrator test fail without the roll.

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2986 passed, 0 failed |
| `replay_manifest -- --ignored` (original data) | 2 passed; the golden is unchanged |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | first run 14 tested: 7 caught, 2 unviable, 5 missed. After two tests were added and two corrected, 11 caught, 2 unviable, 1 missed |
| Release WASM SHA-256 | `1a1f7e351d49ad01fcf2552a8d1e6b4ec53e7ad011b840a735e06727ffd7c954` |

The missed mutant replaces `PerceptionIntegrator::apply_space_combat`
with `()`. Nothing in the workspace calls that method. It only gained the
rolls parameter, so that it cannot drop them.
