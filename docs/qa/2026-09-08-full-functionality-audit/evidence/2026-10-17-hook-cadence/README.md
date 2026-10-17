# The System hooks' cadence (F-037, step 3b)

Date: 2026-10-17.

Step 3a mapped the System hooks (`ghidra/notes/economy-cadence.md`, "The
hooks") and corrected bit 11. This step reads the side hook and ports the
cadence where it differs from the port's daily recompute.

## The original

- `FUN_00510d70`, the control word's hook, calls `FUN_004f6f40(holder)`
  when the holder changes. That stores the new side in `+0x24` bits 6-7
  and runs slot `+0xf0`, which for a System is `FUN_00510820`, the side
  hook.
- The side hook runs `FUN_0050ac80`, `FUN_0050b890`, `FUN_0050c350`, then
  `FUN_0050a4a0(0)`, which clears the uprising bit. It then sets the
  smuggling percent to 0 and recomputes it (`FUN_0050b230`), recomputes
  TroopRegRequired (`FUN_0050b5a0`), arms the informant timer `0x38e`
  (`FUN_0050c820`), and calls `FUN_0050ac10(1)`.
- With the slot table, the derived fields are recomputed by:
  - SmugglingPercent: the support, bit 11 and side hooks. Ships,
    squadrons or regiments coming or going (`FUN_00508660`) do not.
  - ProductionModifier (the port's "collection rate"): the support hook.
  - TroopRegRequired: the support, populated, uprising, bit 11 and side
    hooks. These are all its inputs.
  - The holder: the support, populated and regiment hooks. These are all
    its inputs.

## Port

- `SystemEconomy::hook_inputs` keeps the support, bit 11 and side seen on
  the last run. The smuggling percent is recomputed only when one of them
  changes. The collection rate is recomputed only when the support
  changes.
- `ActiveUprising::side` records the holder when the revolt begins. The
  revolt ends when the holder changes. Before, it went on under the new
  holder, whose support the revolt timer then wore down.
- TroopRegRequired and the holder keep their daily recompute, which gives
  the same values.
- port: a change made after the economy has run is seen on the next day's
  run.
- Not ported: the side hook's informant timer, since the port's informant
  incident has no source (F-029).

Save format: v62 -> v63 (`SystemEconomy::hook_inputs`,
`ActiveUprising::side`).

## Tests

- `economy.rs`:
  - `the_smuggling_percent_waits_for_the_support_bit_11_or_the_side_to_change`;
  - `the_collection_rate_waits_for_the_support_to_change`.
- `uprising.rs`: `a_change_of_holder_or_of_population_ends_the_revolt`.
- `save.rs`'s round trip also checks the revolt's side.
- Neutralized, each failing its test:
  - recomputing the smuggling percent every day;
  - recomputing the collection rate every day;
  - not watching the side;
  - keeping the revolt after a change of holder.

## Campaigns (`playtest-hook-cadence.json`)

`rebellion-playtest data/base --ticks 600 --dual-ai`, against `85f9de74`:

| Seed | Control changes | Uprisings begun / ended | Incidents | Revolt shifts | Smuggling events | Collection rate events |
|---|---|---|---|---|---|---|
| 1 | 17 -> 17 | 4/2 -> 4/2 | 18 -> 18 | 39 -> 39 | 4 -> 2 | 157 -> 140 |
| 2 | 17 -> 17 | 5/4 -> 5/4 | 8 -> 8 | 23 -> 23 | 3 -> 2 | 127 -> 112 |
| 3 | 42 -> 34 | 15/11 -> 13/10 | 39 -> 31 | 86 -> 59 | 35 -> 21 | 204 -> 157 |
| 7 | 18 -> 15 | 7/2 -> 8/3 | 34 -> 26 | 69 -> 59 | 26 -> 10 | 208 -> 166 |
| 13 | 20 -> 19 | 4/2 -> 6/4 | 15 -> 16 | 32 -> 32 | 43 -> 12 | 142 -> 128 |
| 42 | 18 -> 18 | 9/3 -> 11/3 | 34 -> 42 | 75 -> 84 | 20 -> 14 | 154 -> 153 |
| 47 | 22 -> 26 | 11/7 -> 14/8 | 38 -> 35 | 89 -> 84 | 28 -> 16 | 203 -> 176 |

- Leaving out the smuggling and collection rate events, seeds 1 and 2 play
  the same game. Their first difference is a snapshot's collection rate
  values. So those two fields change no gameplay, as expected, since only
  telemetry reads them.
- In the other seeds the first gameplay difference is a revolt ending at
  a change of holder. For example, in seed 3 Sluis Van revolts under the
  Empire on day 8 and passes to the Alliance on day 9, so the revolt now
  ends that day. Systems changing hands 3 or more times: 2/2/8/1/3/2/2
  -> 2/2/6/0/2/2/5.

## Browser

The Fleet window gate passes all 48 cases (interface WASM
`7d82ed1550feea5b9c34ab617be2d79bea994ed6e7e8f4e2fb662aa7d3225708`).

The release build ran with `agent-browser` (headless Chromium, 640 by
480): a new Empire campaign, 60 s at Very Slow, then 60 s at Fast.

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 22.4 ms |
| Fast | 3601 | 0.6 ms | 1.5 ms | 2.0 ms |

- No frame took over 50 ms.
- The console has 152 `[ai]` lines and 0 errors.
- `network.txt` lists 4 requests, all HTTP 200.
- Native GUI acceptance was not run.

## Gates

- `cargo test --workspace`: 2259 passed, 0 failed.
- `cargo test -p rebellion-data --tests -- --include-ignored`: only
  `fixed_fleet_tables_only_target_special_systems` fails, as before.
  `telemetry_coverage` passes.
- Seed-42 golden regenerated for this cause (the cadence and the save
  fields): initial `v1:0dfc4521e1bcaab5` -> `v1:156fbf23b95e317e`, final
  `v1:cdf8f5db1e744a60` -> `v1:653ff1b1bbff8904`. Two recordings are
  identical, and only the fingerprints changed in the manifest. The
  replay equivalence check passes with 0 browser errors. Release WASM
  SHA-256 `1d967cbeff7fc60588a7df458f64f3b93e39cd828f2d38aa4bc13cf0a47dfc10`.
- `cargo mutants --in-diff`: 14 mutants, 11 caught, 2 unviable, 1 missed.
  The missed one was the `||` -> `&&` before the holder check. An
  unpopulated system under the same holder now covers it (tried).
- `provenance-scan check`: 90 uncited items, unchanged.
- Clippy (pedantic) and rustfmt (edition 2021): no findings in the
  changed lines.

Hashes are in `SHA256SUMS`.
