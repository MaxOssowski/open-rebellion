# Three tactical hypotheses settled from the executable (P24)

Date: 2026-10-27. These come from the review of the remaining `hyp:` tags
in the battle code. Each one is settled by static reading of REBEXE.EXE
(the decompile corpus, plus objdump for the vtable and the pushes). No
behaviour changes; only the comments change.

## A fighter group's installed hyperdrives (`+0x3ec`)

- `FUN_005b1bc0` returns `+0x3ec` less the hits (`+0x374`).
  `FUN_005b9c60`, the fighter object's constructor, writes `+0x3ec` from
  its stack argument.
- `FUN_0040be90`, the battle setup, builds that argument as
  `(slot +0x1ec != 0) + (slot +0x1f0 != 0)`.
- Read from REBEXE.EXE at the PE file offsets, the fighter vtable
  `0x0065def0` (type `0x1c`) holds:
  - slot `+0x1ec` = `0x557780`, which jumps to `0x557470` and returns
    `[[this+0x2c]+0x6c]`;
  - slot `+0x1f0` = `0x557790`, which jumps to `0x557480` and returns
    `[[this+0x2c]+0x70]`.
- The class record is the DAT record at `+0x28`, so these are FIGHTSD
  `0x44` `hyperdrive` and `0x48` `hyperdrive_if_damaged`.
- So the port's `has_hyperdrive` (either rating above 0) is the original's
  count above 0.

## A group bound for an escaped carrier

A test comment (`a_withdrawal_sends_the_fighters_to_their_carriers_and_away`)
said such a group "goes with it and keeps its craft (hyp)". The test's own
assertions, and the sourced production code, show the opposite.
- At the side's end, event `0x2f` (`FUN_005b2720`) destroys every unit of
  the side that is still in the fight without a hyperdrive.
- A group that docked first keeps its craft for the commit (`FUN_0040c780`,
  `ghidra/notes/planet-squadrons.md`).
- The comment now says so. The question of whether a lost carrier's docked
  squadrons die with it stays open in that note: it depends on whether
  `FUN_005f5500` still finds the destroyed unit.

## The list scroll bar's arrow strobe

- The bar's arrows (ids 100 and 101, made at `0x60b179` and `0x60b24e`)
  push 0x32 into `FUN_00602150`. That stores it at `+0xfc`, and stores
  `0xfa / 50` at `+0x104`.
- The strobe `FUN_0060f1e0` passes it to `FUN_00601f40(delay, 1, 10)`,
  which calls `timeSetEvent(delay, 10, .., TIME_PERIODIC)`.
- So the period is 50 ms, confirmed.

## Gates

| Gate | Result |
|---|---|
| `cargo check -p rebellion-render --tests` | clean |
| `provenance-scan check` | 0 uncited items, within the baseline |

Only comments changed, so the tests, the WASM and the browser matrix are
those of `212d7d69`.
