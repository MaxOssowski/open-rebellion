# Loyalty: a blockade's end (F-028, save v89)

Date: 2026-10-19. The user granted all permissions for this run, save
format changes included.

`ghidra/notes/betrayal-and-loyalty.md`. This is stage 7 of the tracker item
"Zdrada".

## The original

- A blockade's start (`System_vt220`, `FUN_00511300`) clears the system
  record's assault and bombardment counters (`+0x48..+0x58`) and stores
  the blockading side (`+0x60`).
- `System_vtd0` adds each regiment, ship or squadron destroyed there to its
  side's counter, by its class `+0x58`:
  - DestroyedReason `0xe` (assault);
  - DestroyedReason `0xf` (bombardment).
- At the blockade's end (`Event305_SystemBlockadeChanged_Handle` ->
  `FUN_00561280`) the side is the blockader, switched to the other side
  when the system stands and its holder is not the blockader.
- That side gets `FUN_0053ec10`'s `(L(other) - L(side)) / 50 +
  L(other) / 50` through `FUN_00561910`.

## Port

- `TroopClassDef::uprising_defense`, TROOPSD's class `+0x58`.
- `BombardEnv::losses` counts each bombardment's destroyed regiments and
  ships and each assault's lost regiments, by side. It counts them before
  `apply` removes them (`bombardment_losses`).
- `BlockadeState`:
  - `losses` (save v89), started with the blockader;
  - `record_losses`;
  - `take_ended` hands over an ended blockade.
- `loyalty::settle_blockade` (`FUN_00561280`) runs in the simulation's
  step 8 and the app's blockade step.
- The day's bombardment and assault losses are recorded in both loops and
  in the player's orders.

port:
- side 3's losses are not kept;
- a destroyed system's blockade is dropped with it;
- the losses count toward a blockade from the day the blockade step has
  seen it.

## Save v89

- `SAVE_VERSION` 88 -> 89.
- The seed-42 golden is regenerated for this cause:
  - initial `v1:a2c72fbdc17546ff` -> `v1:29ee3368cb892d9e`;
  - final `v1:55c055d53d7ca5b8` -> `v1:fb78593dbd336659`;
  - two recordings are identical, and only the fingerprints changed.

## Tests

- `loyalty.rs`:
  - `a_blockade_s_end_credits_the_holder_against_its_blockader`;
  - `side_losses_add_by_side_and_merge`.
- `blockade.rs`: `a_blockade_keeps_its_blockader_and_losses_until_it_ends`.
- `bombardment.rs`:
  `a_bombardment_counts_its_destroyed_regiments_and_ships_as_losses`.
- `assault.rs`: `an_assault_counts_its_lost_regiments_by_side`.
- These are new functions, so their tests could not pass before them.
- `cargo test --workspace`: 2465 passed, 0 failed.

## Campaigns (named cause)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:
- 17 of 24 are unchanged against stage 6;
- 25 loyalty shifts in all (battles, blockades, Coruscant and the HQ);
- `campaigns.txt` has the counts.

## Gates

- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `c38cc6c1193a4c9c665c9ffe23c9ea927c72cc8419edde710b6beaddc6fc833f`).
- The Fleet window gate passes all 48 cases (interface WASM
  `48ae0002373b90a4d82e7cd2fedf1b83a353986654861bd6de2b24f2b0b14e06`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.7 ms | 25.8 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.6 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`:
  - the first run: 41 mutants, 29 caught, 9 unviable, 3 missed;
  - after two test cases were added, `bombardment.rs` and `loyalty.rs`:
    27 mutants, 23 caught, 4 unviable;
  - the loader mutant is caught by the ignored data test (1 of 1).
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
