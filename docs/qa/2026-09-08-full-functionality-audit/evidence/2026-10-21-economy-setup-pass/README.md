# The economy's setup pass runs once at game creation (FUN_00508250)

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty").

## The original

- Setup stage `0x17` (`FUN_005136d0` -> `FUN_0051b7f0`) calls each
  object's slot `+0xc0` once. For a system that is `FUN_00508250`, which
  computes every derived field: the resources, the smuggling percent, the
  production modifier, the holder, the garrison requirement and surplus,
  the summaries, and the uprising bits (`ghidra/notes/timer-scheduler.md`).
- After that a field changes only through its hooks and timers. A setter
  such as `FUN_00509ec0` (SmugglingPercent) notifies only on a change
  (`ghidra/notes/economy-cadence.md`).

## What the port did before

- No game started with the pass run. The first day's run found every
  field changed from its default and reported it.
- Over 24 seeds that was 1232 collection-rate, 382 support-tier, 48
  garrison and 37 smuggling-percent events on day 1.
- Since `ffb0ea29`, each of those smuggling events can post a "smuggling
  begins" message.

## Port

- `EconomySystem::setup` runs the day's computation once, at tick 0, and
  reports nothing.
- It runs at game creation in three places:
  - the app's new game;
  - the playtest runner;
  - the seed-42 replay fixture.
- port: a holder the pass would change is applied by the first day's run,
  which reports it again, as before. No seeded campaign has one.

## Tests

- New: `the_setup_pass_computes_the_fields_so_the_first_day_reports_no_change`.
  It fails when the pass does nothing (`fail-without.txt`).
- `cargo test --workspace`: 2492 passed, 0 failed.

## Golden

- The seed-42 golden is regenerated for this cause; the economy state is
  part of the fingerprint:
  - initial `v1:38af87970fe0c218` -> `v1:eb6515d682ec8a5d`;
  - the two tick-0 checkpoints `v1:e654ac40854bf110` ->
    `v1:41189c296d0bc0b5` and `v1:7f52d7f7f20b1eda` ->
    `v1:a4716f6b32ef437b`.
- From day 5 the checkpoints are unchanged, and the final state is still
  `v1:eea438fc7e6c6fbc`.
- Two recordings are identical, and only the fingerprints changed.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days.
Every seed is identical to the stage before, except for its 62 to 81
day-1 economy events, which are gone (`campaigns.txt`).

## Gates (`f21`)

- The replay equivalence check passes on the new golden with 0 browser
  errors (release WASM SHA-256
  `605aaa91428cb7afd47307f671f9410b96dec11e34411863c3064fa3f72f5075`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- `cargo mutants --in-diff`: 2 mutants, 1 caught, 1 unviable.
- `provenance-scan check`: 51 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
