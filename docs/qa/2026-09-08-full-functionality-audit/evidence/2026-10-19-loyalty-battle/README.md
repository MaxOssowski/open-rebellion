# Loyalty: a battle's end (F-028)

Date: 2026-10-19.

`ghidra/notes/betrayal-and-loyalty.md`. This is stage 6 of the tracker item
"Zdrada".

## The original

- Each destroyed regiment, ship or squadron adds its class `+0x58` to its
  side's battle counter on the system record (`System_vtd0`,
  DestroyedReason `0xc`).
- The counter clears at the battle's start.
- At the battle's end (`FUN_00517270` -> `FUN_00560d00`) the winner `w`
  gets `(L(other) - L(w)) / SDPRTB 2560 + L(other) / SDPRTB 2560`
  (`FUN_0053ec10`; SDPRTB 2560 = 50, C's truncating division), through
  `FUN_00561910`.
- A battle without a winner shifts nothing.

## Port

`loyalty::battle_watch` and `settle_battle` wrap every battle resolution:
- the simulation's step 3 (`resolve_system_space_combat`);
- the app's AI-against-AI auto-resolve;
- the app's Auto Resolve button;
- the played tactical battle's `tactical_flow::apply_results`.

The losses are the capital ships' class `+0x58` (`uprising_defense`, 3..42
in CAPSHPSD), counted before against after.

port: a regiment lost aboard is not counted. Fighters are worth 0, as
FIGHTSD ships them.

## Tests

- `loyalty.rs`:
  `a_battle_s_winner_gains_its_enemy_s_losses_less_its_own_over_fifty`.
  It covers:
  - two Alliance fleets;
  - a partial Imperial loss;
  - the Empire's negative shift;
  - a draw.
- `cargo test --workspace`: 2460 passed, 0 failed.

## Campaigns (named cause: battles shift loyalty)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:
- 13 of 24 are unchanged event for event against stage 4;
- 15 battles shift loyalty;
- `campaigns.txt` has the counts.

The seed-42 golden is unchanged.

## Gates

- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `edd706066b09b3105edb1f4d8890561c634872ec10c4715c9ea89a11e1ee3e24`).
- The Fleet window gate passes all 48 cases (interface WASM
  `b473bcbb9049ce78eb1d4efc5260b475d5ddb581eaab77a21ed9781fb62cae3e`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.6 ms | 0.8 ms | 23.4 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.6 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - the first run: 26 mutants, 20 caught, 2 unviable, 4 missed. The test's
    numbers hid the Alliance's losses;
  - after the test was changed, `loyalty.rs`: 24 mutants, 23 caught,
    1 unviable.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
