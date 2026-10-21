# The invented support bands are gone (save v101, F-031)

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty").

## What the port had

- `economy.rs` sorted the holder's support into five bands, split at 20,
  30, 40 and 60 (`SupportTier::from_support_int`). It kept the last band
  per system (`SystemEconomy::last_support_tier`, saved since v8), and
  raised `SupportChanged` on each band change.
- Its only reader was the headless telemetry (`support_change`).
- The code itself said: "hyp: ... no recovered function shows the
  original announcing a support change at these edges" (F-031).
- It was labelled `EVT_SUPPORT_CHANGE` (`0x100`). In the original,
  `0x100` is `GameObjControlKindNotif` (`FUN_004fbbd0`), a control
  change. That change is ported in `dfb840f2` as System Joins /
  Declares Neutrality.

## Port

- `SupportTier`, `last_support_tier` and `SupportChanged` are removed,
  with the integrator's telemetry arm and the test of the band edges.
- New: `the_collection_rate_is_reported_when_it_changes_and_not_again`,
  for the neighbouring `CollectionRateChanged` telemetry. `cargo
  mutants` had found that branch untested.

## Save v101

- `SystemEconomy` loses the field, so `SAVE_VERSION` 100 -> 101.
- The seed-42 golden is regenerated for this cause; the economy state is
  part of every fingerprint:
  - initial `v1:eb6515d682ec8a5d` -> `v1:b77445f2a1a777c0`;
  - final `v1:eea438fc7e6c6fbc` -> `v1:4a994816eae0685d`.
- Two recordings are identical, and only the fingerprints changed.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days.
Every seed is identical apart from its `support_change` records; 1270 are
gone in all (`campaigns.txt`).

## Gates (`f34`)

- The replay equivalence check passes on the new golden with 0 browser
  errors (release WASM SHA-256
  `86df401cc73e92404f2059c7df36c021469e211a3bc6358dbfa7b2bb3a6a3084`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- `cargo test --workspace`: 2507 passed, 0 failed.
- `cargo mutants --in-diff`: after the new test, 5 caught, 1 unviable
  and 1 missed. The missed one, `>` -> `>=` on the collection rate's 0.01
  tolerance, is equivalent: it differs only at a difference of exactly
  0.01.
- `provenance-scan check`: 11 uncited items.

Hashes of the committed files are in `SHA256SUMS`.
