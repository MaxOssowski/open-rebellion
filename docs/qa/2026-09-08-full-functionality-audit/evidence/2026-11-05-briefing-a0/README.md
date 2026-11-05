# The opening briefing against A0 (2026-11-05)

This covers PRE-05 C001..C003, C006 and C007, and PRE-02 C009 and C010. The reading is in `ghidra/notes/advisor-briefing.md`.

## A0

The Windows 10 VM, new games. The frames are in `.artifacts/a0-vm/`, with hashes in `a0-sha256.txt`.
- **Without the briefing DLLs** (`new-alliance-game/`). The GOG install ships `ALBRIEF.DLL` and `EMBRIEF.DLL` renamed `.OLD`. A new Alliance game plays MDATA.003 and opens on the galaxy with no briefing. The Message Index then shows its Advice Messages (`n045`). This settles the note's open question: with slot `0xd` unloadable, no briefing plays.
- **With `ALBRIEF.DLL` restored** (`briefing-alliance/`, a screen every 1.5 s, then every 3 s). The briefing plays over the galaxy view: the displays, the marks, the systems' highlights and the sector lights. The view then brightens, and the Message Index opens on the Advice Messages with Victory Conditions chosen (`b121`).
- **With `EMBRIEF.DLL` restored** (`briefing-empire/`, every 2 s). The same flow for the Empire. The index's chosen row is green (`e094`).

## The port

It loads the briefing DLLs from the build's data, so the briefing plays.

## Tests

`briefing.mjs` `full` (Linux, both sides):
- **The actions.** After each map action of message `0x29` / `0x4a`, the port's screenshot is compared with every A0 frame over the galaxy view. The best match holds at least 0.985 of pixels within 32 per channel:
  - Alliance: 0.989 to 0.999 over 16 actions;
  - Empire: 0.993 to 0.999.

  The two games are separate new games, so a few of the systems' sides and marks differ (A0 `b010` against the port's `0xe`).
- **The index.** After the briefing, the Message Index's window holds at least 0.95: Alliance 0.961 (`b121`), Empire 0.964 (`e094`).
  - The rows are the same nine advice titles in the same order.
  - The readouts at the top, the droids' frames and the row font differ: the port draws the advice rows bold, A0 draws them regular.

## Open

- **Narration and droids.** The narration's timing and the droid clips' frames are not compared with A0 (PRE-05-C003 and C004).
- **The skip.** The short form is not compared with A0 (PRE-05-C005).
- **Bold rows.** The port's bold advice rows against A0's regular ones are not traced.
