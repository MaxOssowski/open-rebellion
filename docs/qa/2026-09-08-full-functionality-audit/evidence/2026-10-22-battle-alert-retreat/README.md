# When the Battle Alert offers Retreat (F-006 / TAC-07)

Date: 2026-10-22. The reading is in `ghidra/notes/abstract-battle.md`,
under the player's Battle Alert.

## Change

- `abstract_battle::retreat_offered` is `FUN_00516730` as
  `FUN_004865b0` asks it for the dialog (`FUN_00488d20` stores the answer
  at `+0x50`; `FUN_0044f860` enables button `0x67` on it). Retreat is
  offered when all three hold:
  - no capital ship in the enemy's fleets at the system has a gravity
    well projector. Slot `+0x254` is the class's `+0xcc`, CAPSHPSD
    `+0xa4`; slot `+0x280` is 0. So an Interdictor bars it;
  - a unit of the viewer's side can move (slot `+0x34`): one of its
    fleets' capital ships, else one of its squadrons on the planet;
  - the autoroute search finds a system to go to.
- `tactical_view.rs` enables Retreat from it. The port's own rule (a
  player ship with a working hyperdrive, read from the battle session) is
  gone.

port: the search is `missions::nearest_place` without the battle's
system, as for the loser's withdrawal. It does not depend on the unit.

## Gates

- Tests: 1 new (core). Workspace 2637 passed, 0 failed (before the test
  gained its empty-entry case; it passes after).
- Mutants: the gate run caught 15 of 17 (1 unviable, 1 missed: `count >
  0`). The test now has an emptied squadron entry. A rerun on core
  caught all 16.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build):
  tactical `production-battle-alert-entry` passes 4 of 4 cases. The
  fixture has no Interdictor, so Retreat stays enabled: its rest art and
  the Retreat route are checked.
- Replay equivalence passed.
- Release WASM `0b54706074761818ff6a5f20254292d99424fe095cded9d8c120869ddcedb821`.
