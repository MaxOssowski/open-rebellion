# The Smuggling message speaks when smuggling begins or ends (0x145)

Date: 2026-10-20 (overnight run, "Ekonomia i incydenty").

## The original

- `FUN_0050b230` -> `FUN_00559c40` gives a system's SmugglingPercent
  (`+0x6c`). `FUN_00509ec0` stores it and notifies `0x145`
  (`SystemSmugglingPercentNotif`) only when it changes.
- The story class `FUN_00499130` builds the Smuggling message from the
  old and the new value. It speaks only when one of them is 0: smuggling
  begins (the new value is not 0) or ends.
- The system must be held by a side (`+0x24` bits 6..7):
  - an Alliance system speaks when the old value is at least 0;
  - an Empire system speaks when the old value is below 1.
- On the viewer's own system it reports a loss (TEXTSTRA `0x7060` /
  `0x7064` on beginning, `0x7061` / `0x7065` on ending); on the other
  side's system it reports a benefit (`0x7062` / `0x7066`, `0x7063` /
  `0x7067`).
- Its category mask is `0x001`, Popular Support (`FUN_00499030`,
  `FUN_0040f340`). The background is `0x3ec`
  (`ghidra/notes/story-message-classes.md`).

## What the port did before

`SmugglingPercentChanged` (save v61) carried only the new value, and no
reader turned it into a message.

## Port

- `EconomyEvent::SmugglingPercentChanged` gains `previous`, the stored
  value before the change. It is an event, not saved state, so the save
  format is unchanged.
- `story_messages::smuggling_message` is `FUN_00499130`. The app pushes
  it into the message log on the Popular Support rail, for the viewer's
  side only.
- port: the pictures are not drawn.

## Tests

- New: `smuggling_speaks_when_it_begins_or_ends_by_whose_system_it_is`.
  It covers each side, each viewer, beginning and ending, a change between
  two non-zero values, a neutral system, and the Empire boundary (old
  value 1).
- `cargo test --workspace`: 2490 passed, 0 failed.

## Campaigns and gates (`f19`)

- `rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days: all
  24 are identical to the stage before. The message log is not simulation
  state.
- The replay equivalence check passes on the unchanged golden (initial
  `v1:38af87970fe0c218`, final `v1:eea438fc7e6c6fbc`) with 0 browser
  errors (release WASM SHA-256
  `67cfa75bc3dac8337af3ca42dc9a37c14b7739991114811f48d42b75215ccb5e`).
- The Fleet window suite passes 52 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- `cargo mutants --in-diff`:
  - the first run missed `<` -> `<=` in the Empire condition;
  - the boundary assertion was added, and the rerun caught 11 of 12
    (1 unviable).
- `provenance-scan check`: 51 uncited items, unchanged.
- Not checked: a browser screenshot of the message itself. No fixture
  scenario yet changes a system's smuggling percent.

Hashes of the committed files are in `SHA256SUMS`.
