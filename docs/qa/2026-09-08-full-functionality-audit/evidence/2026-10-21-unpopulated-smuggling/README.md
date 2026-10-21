# A held unpopulated system has a smuggling percent too

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty").

## The original

- `FUN_0050b230` reads only the system's holder (`+0x24` bits 6..7). It
  has no populated test (`+0x88` bit 0).
- A system a side holds by its regiments alone therefore gets its
  smuggling percent (`FUN_00559c40`) like any other.
- Notification `0x145` then reaches the Smuggling message as usual.

## What the port did before

The economy's daily run skipped unpopulated systems, so their smuggling
percent stayed 0. `economy-cadence.md` noted it as "port:".

## Port

- Step 1 (the hooks' inputs and the smuggling percent) is now a helper,
  `refresh_smuggling`.
- The economy runs it for unpopulated systems too, beside the holder
  (`07943894`).
- port: the other derived fields still run only for populated systems.

## Tests

- New: `an_unpopulated_held_system_smuggles_too`. It fails without the
  change (`fail-without.txt`).
- Changed, with the evidence above: `unpopulated_system_skipped_in_advance`
  becomes `an_unheld_unpopulated_system_reports_nothing`. It still
  expects no events, but no longer that the system has no economy
  record.
- `cargo test --workspace`: 2510 passed, 0 failed.
- `cargo mutants --in-diff`: 6 mutants, 3 caught, 3 unviable.

## Golden and campaigns

- The seed-42 golden is regenerated for this cause: unpopulated systems
  now carry an economy record, which is in the fingerprint.
  - Initial `v1:b77445f2a1a777c0` -> `v1:e01dc3b9b044c28e`.
  - Final `v1:4a994816eae0685d` -> `v1:7737eb96955b5cd7`.
  - Two recordings are identical, and only the fingerprints changed.
- `rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days
  (`campaigns.txt`):
  - every event is identical;
  - the campaign snapshots keep every earlier system row and add rows
    for unpopulated systems (234 to 348 per campaign).
  - No campaign holds an unpopulated system, so no smuggling record
    changes.

## Gates (`f37`)

- The replay equivalence check passes on the new golden with 0 browser
  errors (release WASM SHA-256
  `aed4562e2632be692cd3461215b43fe95bc5a31ac4262257f0ff9920b24ea015`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.

Hashes of the committed files are in `SHA256SUMS`.
