# Character command: the leadership readers (F-038, stage 4a)

Date: 2026-10-19.

`ghidra/notes/character-command.md`. This is stage 4a of the tracker item
"Dowodzenie i jego utrata przy ranie". Stage 4b (the decoy roll) and 4c
(the AI records' rank bits) follow.

## The original

- `FUN_004fd790(fleet, kind)`: the first character of the fleet's side
  aboard the fleet whose command (`+0x96`) is `kind`.
- `FUN_00509330(system, kind)`: the first one of the system's side held by
  the system.
- Slot `+0x1f4` (`FUN_004edc50`) returns leadership `+0x88`.
- Bombardment (`bombardment-order.md`):
  - the batteries hit with `(general / G0x600 + 1) * attack`
    (`FUN_0058dc10`, `FUN_0055e930`), where the general is the system's
    (`FUN_00509330(3)`);
  - each bombarding fleet fires `(admiral / G0x601 + 1) * bombardment`
    (`FUN_0058df60`, `FUN_0055e950`), where the admiral is that fleet's own
    (`FUN_004fd790(2)`).
- Planetary Assault (`planetary-assault.md`):
  - each duel reads the system's general and the attacking regiment's
    fleet's general (`FUN_0058bec0`);
  - `FUN_0055eb00` adds each one's leadership over G`0x201` to its side;
  - the push order at `0x0058c01c..0x0058c029` is `(attack, fleet
    general, defense, system general)`;
  - the fleet is the one the walk `FUN_0058ba10` hands the functor with
    the regiment and its ship.

## Port

- `character_command.rs`:
  - `fleet_holder`;
  - `system_holder` (none for a neutral or contested system);
  - `leadership` (`Character::effective_skill`, else 0).
- `bombardment.rs`: the batteries' general and each fleet's own admiral.
  Before, both multipliers were 1, and the admiral's was computed once for
  all fleets.
- `assault.rs`:
  - `attackers` returns each regiment with its fleet;
  - `AssaultRequest::troop_fleets`;
  - the duel adds both generals.
  - The unused `attacking_troops` is folded into `attackers`.
- `status_rows.rs`: the Fleet Status rows use `fleet_holder`.

## Tests

- `character_command.rs`:
  `a_fleet_s_or_a_planet_s_officer_is_its_own_side_s_holder_of_the_rank`.
- `bombardment.rs`:
  `a_fleet_s_admiral_and_the_planet_s_general_magnify_their_side_s_fire`.
- `assault.rs`: `each_side_s_general_adds_its_leadership_over_thirty_to_the_duel`.
- All three fail when `leadership` reads 0 (`fail-without.txt`).

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all
24 runs are unchanged event for event against stage 3b. The AI gives no
command yet (module `0xc`).

## Gates

- `cargo test --workspace`: 2465 passed, 0 failed.
- The seed-42 golden is unchanged (`v1:094e77fcd3b4b666` ->
  `v1:beb63be8a01b2699`).
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `d7a2f9a0a5bbedbbef7f4f5699020469221bddfba668fd25f0cf429121ebb5ba`).
- The Fleet window gate passes all 48 cases (interface WASM
  `3f9f3e697b9d3629c38d702453a3078a9cbb6589ec4fec535c170753d924c437`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.5 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.6 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-core`, `rebellion-render`):
  - 54 mutants;
  - after the test was tightened, all caught but one:
    `assault.rs` `!attackers.is_empty() && !defenders.is_empty()` with
    `||`. That guard is older code. The duel loop below it tests both
    lists again, so the mutant behaves the same (equivalent).
- `provenance-scan check`: 85 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
