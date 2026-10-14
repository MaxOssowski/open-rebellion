# The AI's world model: fleet, system and sector records (F-036)

Date: 2026-10-14.

The production modules (`ghidra/notes/ai-production-modules.md`) choose
sites by querying records that phase 4 of the AI tick (`FUN_00418500`)
rebuilds. `ghidra/notes/ai-galaxy-records.md` gives their layout. This
step ports them into `crates/rebellion-core/src/ai_records.rs`, next to
the class records of the previous commit:

- **Side constants** (`side_constants`, `FUN_0041bd20`): `+0x364..+0x390`
  = 3, 3, 2, 1, 1, 0, 0, 3, 9, 0, 6, 12.
- **Fleet records** (`FleetRecord::build`, `FUN_00432e40`,
  `FUN_00433620` over `FUN_00484630`): the flags `+0x38` and 30 values
  from `+0x50`. Each ship is added by `FUN_00433780`, along with the
  people aboard (`Planner::flags`, `FUN_00401d20`).
- **System records** (`SystemRecord::build`, `FUN_00433e40`): state
  `+0x28`, summary `+0x2c`, kind `+0x30`, 60 values from `+0x48`. The
  record holds the regiments, fleets, defenses, yards, mines, the support
  term, and the shield, battery, garrison and room goals.
- **Sector records** (`SectorRecord::build`, `add_system`,
  `FUN_004321e0`, `FUN_00432350`): candidates `+0x24`, state, summary,
  90 values from `+0x70`.
- **Queries** (`Galaxy::sector_query`, `system_query`; `ranked`, `pick`;
  `FUN_00419a00`, `FUN_00431830`, `FUN_004357b0`): masks, keys, order,
  and the coin on a tie.

The port's assumptions carry `port:` and `hyp:` tags in the doc comments.
Examples: every unit is active, the AI sees the whole galaxy, and the
`+0x40` thresholds with no known writer read as 0.

Nothing calls `Galaxy::build` yet. Module 1 will be its first reader, so
the simulation, the save format and the seed-42 golden are unchanged.

## Gates

- `cargo test --workspace`: 1821 passed, 0 failed. `ai_records` has 58
  tests. Each one that asserts original behaviour names its function.
- `cargo test -p rebellion-data --test replay_manifest -- --ignored`:
  passes; golden unchanged (`v1:33233aedba63db5c`).
- `cargo mutants` on `ai_records.rs` and `Planner::flags`:
  - Full run: 1254 mutants, 1200 caught, 44 missed, 6 unviable, 4
    timeouts.
  - Some survivors came from a source copy older than the last tests.
    After tests for the gaps were added, two reruns of the matching
    mutants on the current code leave 13 missed, all equivalent
    (`mutants-equivalent.txt`):
    - `v[23..25]`: squadrons on a planet (`FUN_005039d0`) have no port
      counterpart, so these values are always 0 in a system record. That
      covers the `v[25]` sums and flags and the `less` choice for
      `v[38]`/`v[53]`.
    - `f30 & 0x3_0000`: no system record sets these kind bits, so the
      garrison check always runs.
    - `f30 & 2`: a system neither own nor the opponent's is always
      neutral.
    - `below - v[0] > 0` against `>=`: at equality the term is 9 / 10 = 0.
    - `v[1] < empire + 3 + 3` against `<=`: at equality both give 3
      batteries.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy: no findings in `rebellion-core`'s touched files. rustfmt:
  `ai_records.rs` is clean; no findings in the other changed lines.

Still open: the distance queries (`FUN_00419b80`, `FUN_0041a400`), the
Scrap order, and wiring `Galaxy` into `AISystem::pass` for module 1.

Hashes are in `SHA256SUMS`.
