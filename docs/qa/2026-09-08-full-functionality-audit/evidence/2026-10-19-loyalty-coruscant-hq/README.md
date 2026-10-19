# Loyalty: Coruscant taken and the Rebel HQ lost (F-028)

Date: 2026-10-19.

`ghidra/notes/betrayal-and-loyalty.md`. This is stage 4 of the tracker item
"Zdrada".

## The original

`FUN_00561910(amount, side)` moves every character's base loyalty `+0x66`:
- up for the characters whose class serves `side`, down for the others;
- clamped to `0..=100`;
- never for a class with `+0xb4` (`is_unable_to_betray`).

Two of its four callers:
- **Coruscant** (`0x109`) changes side (`FUN_00561670` -> `FUN_005616a0`).
  The new side gains `rand(SDPRTB 2561) + SDPRTB 2562` = `0..=5`.
- **The Alliance HQ** (family `0x20..0x21`) is lost (`FUN_00561470` ->
  `FUN_00561780`). The Alliance gets `rand(GNPRTB 2592) + GNPRTB 2593` =
  `0..=-5`.

The manual's "Traitors" names both: "the destruction of the Rebel HQ; the
Empire gaining or losing control of Coruscant".

## Port

`rebellion-core/src/loyalty.rs`:
- `shift` (`FUN_00561910`, `FUN_004ef1e0`);
- `watch` and `settle`: the day's start against its end;
- `simulation.rs` step 13e and the app's day loop call them;
- a draw is taken only when its event happens;
- the telemetry is `loyalty_shift`;
- `SdprtbParams::contains`, so that a table the port lacks falls back to
  the shipped value.

port: two changes in one day count once.

hyp: the side words' bits `0x30` are read as clear.

## Tests

- `loyalty.rs`:
  - `a_side_s_gain_raises_its_characters_and_lowers_the_others`;
  - `the_empire_s_gain_raises_its_characters`;
  - `the_shifts_read_the_side_s_sdprtb_and_the_gnprtb`;
  - `coruscant_taken_or_the_rebel_headquarters_lost_shift_loyalty`.
- `integrator.rs`: `each_loyalty_shift_is_recorded_with_its_side_and_amount`.
- `simulation.rs`: `a_roll_draws_evenly_below_the_bound`.
- These are new functions, so their tests could not pass before them.
- `cargo test --workspace`: 2455 passed, 0 failed before the mutant tests
  were added.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:
- 23 of 24 are unchanged event for event against stage 2.
- Seed 9's Alliance loses its HQ: one `loyalty_shift`, Alliance -5.
- Coruscant changes hands in no campaign.
- The seed-42 golden is unchanged.

## Gates

- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `ce9ffba92b7ba508c90a9bc7ffcb5f01c9a05d8b80f02ef8d52afdf55d428e2c`).
- The Fleet window gate passes all 48 cases (interface WASM
  `381207dfea75f1f593c6ff73baf467161fbcd1e40e5fe78af85bc9394f4770d8`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 22.8 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.8 ms |

- One earlier run read 0.9 and 1.8 ms at the 99th percentile. It
  overlapped a mutants run, and the run above was repeated alone.
- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - 55 mutants, 49 caught, 5 unviable, 1 missed (`SdprtbParams::contains`
    `==` -> `!=`);
  - a further assertion catches that one (3 of 3).
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
