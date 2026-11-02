# Subdue Uprising in Create Mission against A0 (2026-11-02)

Surface ledger OBJ-11 C009 "Subdue Uprising".

## A0

The A0 run used the Windows 10 VM and the Empire's "resource turn 14" save, days 203..204. The captures are in `.artifacts/a0-vm/kinds2/`, with hashes in `a0-sha256.txt`.

1. Jerjerrod was in Fleet 3 at Selonia, the Empire's, in uprising against it. He was reached through the Personnel Finder's Display.
2. His menu (`60`), then Mission, then a click on Selonia's planet in its Fleet window, opened Create Mission (`61`):
   - Subdue Uprising was shown;
   - "Target" was Selonia.
3. The drop-down (`62`, `63-1..5`, scrolled by clicks) listed Subdue Uprising, then Espionage. That is MISSNSD 0x80 above 0x13, last record first again (see `2026-11-02-character-kinds-a0`).
4. Subdue Uprising's icon, GOKRES 7296 (TEXTSTRA 11392 & 0xfff, + 0x1000), at the window's (70, 86): 0 of 8,320 pixels differing.
5. No research kind was offered. Selonia showed no idle yard, and Jerjerrod's design skill was not read.

## The port's case

The new scenario `UprisingTargeting` (90, code 91) is `MissionTargeting` with a changed target system:

- the system is the player's;
- it carries `ControlKind::Uprising` and an active uprising.

`mission-dialog.mjs` `uprising` releases the agent's Mission on it on each side. It checks:

- Subdue Uprising heads the list;
- the side's icon (3200 or 7296) is at (70, 86) exactly.

## Tests

- `the_uprising_scenario_makes_the_target_the_players_and_in_uprising` (`interface_test_fixture.rs`).

## Run

Batch `b22` passed every gate: workspace 3326, fixtures 115.

`mission-dialog-2026-11-02T10-53-13-301Z-4112421` (Linux Chromium 155, test WASM `2e69ad2e…`):

- Both sides list SubdueUprising, Recruitment, Espionage.
- The icon is at (70, 86), 0 pixels differing.
- The fixture's agent can also recruit, which A0's Jerjerrod did not list.
