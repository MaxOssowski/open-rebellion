# Yavin Under Blockade, and the defender's assault row, against A0

Date: 2026-11-08. Closes EVT-01-C008. Adds to CMD-08-C014.

## A0

These come from the same Alliance game in the Windows 10 VM as
`2026-11-08-luke-jedi-portrait-a0`. The captures are in
`.artifacts/a0-vm/`:

| File | SHA-256 |
|---|---|
| `events-day673/00-index-day673.png` | `7c20c3eb3bf0e45aaffe4ea6218b30da8f61b4d47d4374aaf1d6b09c9736036d` |
| `events-day673/02-yavin-under-blockade.png` | `58f3c74fc42cc81efa0f85faec34caa8beac7ffa0ba315735098b9d45b99816f` |
| `events-day673/03-bpfassh-joins-enemy.png` | `28b4b56a68c758915ad2f432ac1657b6da1ab55972db8f0f2c1e0da501748f47` |
| `events-day673/04-fleet-arrives-sluis-van.png` | `391fb289da04d0c0ee8903920988c765162422c35e3c5ec5b5c7ce3cce6dade4` |
| `events-day673/06-saboteurs-strike-yavin.png` | `5597a842c10f726a455ea3bd54688de8122620bdf93028fc3e0d8a612d0fd9d8` |
| `assault-defender-day902/00-index-day902.png` | `618fd01d8c1d32dfbee1c804f79ccd41291759ac75125bae810737c57e957b8a` |
| `assault-defender-day902/01-assault-on-yavin-chosen.png` | `e58bde59d44dd46b775fded1390c6145fe0360f78fe3919a01d6b54d929463a6` |

- **"Yavin Under Blockade"** (Alliance reader). The text is "Imperial ships
  have been detected at Yavin.  The world is under enemy blockade." This is
  the holder's `0x70a2` / `0x70a3` (`FUN_004960f0`). The message did not
  appear after the battle at Yavin on day 592. It came later, which matches
  the rule that a blockade left by a battle is silent.
- **"Assault on Yavin"** (Alliance defender) is chosen in the index, and
  Display is lit. `FUN_00497530` gives the defender kind 3; the attacker's
  summary opens at once (kind 7). The port already does both
  (`AssaultReport::opens_at_once`), so no change was needed.
- **Bpfassh Joins Enemy, Fleet Arrives at Sluis Van, Saboteurs Strike at
  Yavin** were captured for later comparisons.

## Port

The event-message fixture (Alliance) blockades Yavin through the app's own
`blockade_started_message` (`interface_test_fixture::blockaded_system`). The
message lists last on the Conflict rail, under Saboteurs Strike.

`message-display.mjs` (Linux, Alliance) checks the exact text, and the
picture, title image and rail (Display lit) are each 0 pixels from A0
`02-yavin-under-blockade`.

The other side's "Fleet Initiates Blockade" (`0x70a0` / `0x70a1`) and
Evacuation Losses have no A0 capture.

## Tests

`the_event_messages_scenario_blockades_yavin_for_the_alliance`.
