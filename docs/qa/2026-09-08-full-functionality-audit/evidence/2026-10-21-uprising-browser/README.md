# A revolt, its incident and a natural disaster, in the browser (F-026)

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty").

## The original

- A revolt (`+0x88` bit 2) never changes control (`FUN_0050a130`). Its
  incident timer (event `0x38d`, `FUN_0050d030`) fires every 30 to 100
  days (GNPRTB 7701/7702).
- The disaster (event `0x38f`, `FUN_00511930`) picks a random existing
  system. One with neither energy nor raw materials is spared; otherwise
  it erodes both (`FUN_00559e10`) and destroys each facility at 10
  percent (`ghidra/notes/uprising-incident.md`).
- F-026's port (2026-09-26) had unit tests only; its acceptance asks for
  a browser pass that shows a revolt, an incident and a disaster.

## The gate

- A new fixture scenario, `UprisingIncident` (75, code 76), is `FleetLoad`
  with three changes:
  - the primary system, the player's, is in revolt, its incident due on
    day 5;
  - the disaster is due on day 8;
  - port: a system with neither energy nor raw materials gets one energy,
    so the disaster's pick always strikes. The clock runs Fast.
- The observation adds:
  - whether the primary system is in revolt;
  - the player's last "Uprising incident at X" line;
  - the last "Natural disaster strikes X" line, of any audience, since
    the disaster may strike a neutral system.
- `fleet-window.mjs` `incident` checks, for both sides:
  - before: the primary system revolts and nothing has struck;
  - "Uprising incident at Akrit'tar", with the revolt still on;
  - a natural disaster strikes a system: Bothawui in the Alliance run,
    Talus in the Empire run (the pick is random);
  - 0 page errors, and 4 requests, all HTTP 200.
- The screenshots were inspected: Akrit'tar carries the revolt colour in
  the sector window, and the message counter rises after the incident.
  Their hashes are in `screenshots.sha256`; the checks are in
  `incident-browser.json`.
- The message lines are the port's texts; the original's Uprising
  Message (notification 3) and Natural Disaster (`0x154`, TEXTSTRA
  `0x7190`) texts are separate items.

## Tests

- New: `the_incident_scenario_revolts_the_primary_system_with_both_timers_due_soon`.
  It fails when the placement is skipped (`fail-without.txt`).
- `cargo mutants --in-diff` on the fixture:
  - the first run missed the `&&` of the empty-system test and the
    incident line's audience filter;
  - the test now covers both, and the rerun caught 15 of 17 (2
    unviable).

## Gates (`f20`)

- The Fleet window suite passes 54 cases (the 52 before, plus `incident`
  for both sides). The Status window gate passes.
- The 24 dual-AI campaigns and the replay equivalence check are
  unchanged (the release WASM is unchanged:
  `67cfa75bc3dac8337af3ca42dc9a37c14b7739991114811f48d42b75215ccb5e`).
- `provenance-scan check`: 51 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
