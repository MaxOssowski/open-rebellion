# Invention review, round 1 (2026-09-27)

Seven read-only review agents compared each strategic system in the port with
the REBEXE.EXE decompiles in `ghidra/notes/*.c`, starting from the
`tools/provenance-scan` report and `../../coverage-map.md`. Each report lists
the original function and line, the port location, a verdict, and a test that
would fail without the fix. The ledger records them as F-032 to F-039.

These are leads, not verified results. Check each row against its `.c` before
fixing it.

## Corrections made while ledgering

- `economy.md` row 9 is wrong and is excluded. It says the Empire doubling in
  `FUN_00559b60` applies to fleets. Its caller `FUN_0050b230` (lines 26–31)
  passes `FUN_00509020` as `param_6`, and `FUN_00509020` walks types
  `0x10..0x14` (`FUN_00504cc0`), which are regiments. The port doubles troops,
  which is correct.
- `missions.md` cites `FUN_0055cfb0`, `FUN_0055ae50`, `FUN_0055ae90`,
  `FUN_0055af50`, and `FUN_0055b0a0` from the community disassembly. That
  dump comes from a different build (`ghidra/notes/community-address-remap.md`),
  and none of these addresses starts a function in our binary. Remap them
  before citing them.
- `manufacturing.md` lists `FUN_0052b960` as needing a decompile. It is now in
  `ghidra/notes`, with `FUN_00525040`, `FUN_0053e150`, `FUN_0053e170`, and
  `FUN_0055e4d0`, all decompiled from our project.
