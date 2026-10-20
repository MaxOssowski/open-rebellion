# The invented KDY production modifier is gone (F-023, save v99)

Date: 2026-10-20 (overnight run, "Ekonomia i incydenty").

## What was there

`economy.rs` step 4 computed
`clamp(100 - capital ships * GNPRTB 7684 - squadrons * GNPRTB 7685)` as a
"KDY production modifier" (`SystemEconomy::production_modifier`).

- It applied the blockade's troop-withdraw formula (`FUN_0050b310` +
  `FUN_0055a020`, ported in `blockade.rs` under F-021) to production.
- No recovered code does that.
- Only the integrator's telemetry read the value.

The real ProductionModifier (`+0x70`, `FUN_0050b2c0`) is a separate
field. It is stored in `collection_rate` and lengthens mine and refinery
cycles.

## Port

- The field, its two GNPRTB constants, its computation and its two tests
  are removed.
- The campaign snapshot's `production_modifier` key now reports the real
  `+0x70` (`economy::production_modifier_at`).

## Save v99

- `SystemEconomy` loses a field, so `SAVE_VERSION` 98 -> 99.
- The seed-42 golden is regenerated for this cause:
  - initial `v1:44a4ae74f038e3ea` -> `v1:fe91e55dd0f9a2af`;
  - final `v1:a2218c2164fce8ed` -> `v1:b51db94b116e8495`;
  - two recordings are identical, and only the fingerprints changed.

## Campaigns

All 24 dual-AI campaigns are identical to the stage before once the
snapshot's `production_modifier` value is set aside. The simulation did
not change; only that telemetry value did.

## Gates (`f17`)

- The replay equivalence check passes on the new golden with 0 browser
  errors (release WASM SHA-256
  `fad8387ac579e7df3a4cd6cd7816b3c43d1a5ecd3b9509e291274b018ae3fb03`).
- The Fleet window suite passes 52 cases; the Status window gate passes.
- `cargo test --workspace`: 2488 passed, 0 failed.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- `cargo mutants --in-diff`: 2 mutants, 1 caught, 1 unviable.
- `provenance-scan`: 51 uncited items. The two KDY constants go, and
  `CombatSystem::snapshot_fleet` loses one literal (the family-`0x34`
  test dropped with the battle shield). The baseline is rewritten to 51.

Hashes of the committed files are in `SHA256SUMS`.
