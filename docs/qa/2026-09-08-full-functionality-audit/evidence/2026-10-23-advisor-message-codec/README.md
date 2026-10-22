# The advisor's message files decode and round-trip (P34)

Date: 2026-10-23. Stage 1 of the opening briefing
(`ghidra/notes/advisor-briefing.md`).

## Source

`FUN_004c2c70` (Alliance) and `FUN_004c0ba0` (Empire) open
`GDATA\C3POACT.SPT` and `GDATA\IMP22ACT.SPT` (REBEXE RCDATA strings
`0x618`, `0x61b`, `0x61c`) and read them with `FUN_0049c6f0` /
`FUN_004c52d0`.

## Change

`tools/dat-dumper/src/types/advisor_script.rs`: `AdvisorScriptFile`
(messages: id, `+0x1c`, `+0x24`, entries of record, flags, a, b),
registered for both files.

## Result on the owned files (not committed)

| File | SHA-256 | Messages | Briefing message | Its entries | Round trip |
|---|---|---|---|---|---|
| C3POACT.SPT | `7e8764deb2f3e8045d635d7b4e1ea766655497b9b09230201cbfedf4ebbaedba` | 87 | `0x29` | 34 | byte-exact, fully read |
| IMP22ACT.SPT | `87cda8ec7509801dd449495003febbb277559523ac0c3b5b2cfd3e6615c8ef93` | 85 | `0x4a` | 34 | byte-exact, fully read |

`dat-dumper -g data/base`: 53 of 53 files succeed.

## Gates

- Workspace 2691 passed, 0 failed.
- Mutants on the touched dat-dumper code: 26 tested, 18 caught,
  8 unviable, none missed (the registry gained a test after a first run
  left `build_registry` -> `HashMap::new()` alive).
- Provenance scan: 0 uncited items.
- No runtime code changes: the WASM build and the browser suites do not
  apply; `cargo check -p rebellion-app --target wasm32-unknown-unknown`
  passes.
