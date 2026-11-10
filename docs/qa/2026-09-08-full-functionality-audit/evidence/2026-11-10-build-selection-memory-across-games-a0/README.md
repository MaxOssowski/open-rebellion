# Build Selection remembers its class across games

Date: 2026-11-10. Adds to OBJ-08-C007 (available).

## A0

Windows 10 VM, original REBEXE, one run of the program:

1. In an Empire game, Coruscant's facilities band confirmed an Orbital
   Shipyard (`09a-before-confirm-maintenance-103`,
   `09-coruscant-shipyard-ordered-maintenance-90`).
2. Game Options' Restart went back to the main menu. Two new Empire
   games followed. In the first, nothing was confirmed. The second is
   below.
3. In that game, on day 16, the first Build on Chandrila's facilities
   band opened on **Orbital Shipyard**, the sixth class, not on the
   first (`13-chandrila-reopens-on-orbital-shipyard-day16`).

`FUN_00437f80` reopens on the class last confirmed by the same kind of
manager, read from `DAT_006b289c` (facilities), `DAT_006b28a0` (troops)
and `DAT_006b28a4` (ships). These are the executable's globals, and
`FUN_00438980` sets them at Confirm. Nothing resets them for a new game.

The captures are in `.artifacts/a0-vm/build-selection-empire2/`, with
their SHA-256 in `SHA256SUMS`.

## The change

The port kept this memory in `BuildSelectionState` and replaced the
whole state with its default on a new game and on a load. A new game
therefore always opened on the first class.

`BuildSelectionState::for_next_game` keeps the remembered classes and
drops the open window. The new-game and load paths in `main.rs` now call
it.

The unit test `the_last_confirmed_class_outlives_the_game` checks both
sides of the change:

- a state carried by `for_next_game` reopens on the second class
  confirmed before;
- a fresh state, as the old reset made, opens on the first class.

A remembered ship class is a slotmap key. A new game loads the same DAT
files in the same order, so the key names the same class. When a class
is not in the new list, the window opens on the first class, as before.

## Also seen: mission kinds on the side's own system

`16-needa-chandrila-kinds-*`: Needa's Mission on Chandrila, the
Empire's own system with a new Orbital Shipyard, offered Diplomacy and
Espionage. Darth Vader's offered Recruitment, Diplomacy and Espionage.
Neither offered a research mission, so OBJ-11-C015..C017 still have no
A0 capture.

## Batch b166

23 of 23 interface gates on Linux; workspace tests 3498 passed, fixture
tests 139. The diff's mutant (`for_next_game`) was caught, 1 of 1.
Tactical matrix `2026-11-10T04-09-43-521Z-3786995`: 147 of 148. The one
failure was the trench run's movie fetch, the known flake; rerun alone,
`2026-11-10T04-23-09-758Z-3817720`, it passes 4 of 4.
