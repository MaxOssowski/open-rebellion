# The Fleet window against A0: squadrons aboard and an empty window (2026-11-02)

## A0 (VM Alliance game, days 766..955)

Captures under `.artifacts/a0-vm/fleet2/` (hashes in `a0-sha256.txt`),
each Fleet window at (305, 86), 235 by 304:

- `umgul-empty` (day 766): Umgul's Fleet window after its fleets
  withdrew. No entry, no pane, no tabs: the window's background and title
  only.
- `selonia-fleet4-{tab}` (day 775): Fleet 4, Alliance Escort Carrier 2
  with an X-wing and a Y-wing squadron (the Fighters counters "2" and
  "6"), selected; Fleet 1, Corellian Corvette 1, expanded and still in
  hyperspace from Umgul (engine glow; its orders grey, `08-`).
- `yavin-fleet11-{tab}`, `yavin-imperial-unselected` (day 955): the
  Empire's Fleet 11 at Yavin seen by the Alliance: three Star Destroyers
  with 17 TIE squadrons (capacity 18) and 3 War Droid Regiments (capacity
  9). Not yet copied in a scenario; see below.
- `07-` the corvette's menu after a drag onto Fleet 4 did nothing (every
  order grey while Fleet 1 was in hyperspace), `09-` Fleet 4's menu.

## The port's copies

- `FleetWindowSelonia` (code 83, `place_selonia`): Fleet 4 with the
  carrier and one X-wing and one Y-wing squadron; Fleet 1 with the
  corvette at hull 240 ordered from another system of the sector to
  Selonia; Fleet 4 selected, Fleet 1 expanded.
- `FleetWindowEmpty` (code 84): Umgul emptied (`clear_for_a0`), its
  window open.

`fleet-window.mjs` cases `alliance/a0-selonia` and `alliance/a0-empty`.
No rendering code changed: the port already matched both states.

## Differences

| State | Differing pixels | Ceiling | Where |
|---|---|---|---|
| Selonia ships / fighters / troops / personnel | 1732 / 1547 / 1161 / 1063 | 1900 / 1700 / 1300 / 1200 | text |
| Umgul empty | 233 | 300 | the title |

## Open: which ship carries the cargo

A0's Fleet 11 marks regiments (10455, the Empire's 10405) under its third
Star Destroyer, Devastator, not its first. The port fills ships'
capacities in fleet order (hyp, `ship_contents`), which would mark
Exterminator. The original keeps each unit in a ship's own contents; how a
unit chooses its ship is untraced, so the rule is left unchanged until it
is.

## Tests

- `the_a0_selonia_scenario_selects_fleet_4_with_two_squadrons`,
  `the_a0_empty_scenario_opens_umgul_with_no_fleet_there`
  (`interface_test_fixture.rs`).

## Run

`fleet-window-2026-11-02T00-06-36-359Z-2679430` (Arch Chromium 155): 77
cases pass. Test WASM
`8f929ca13a26e59a1048f86e318728a75f2ce76acd15631295b731fd12d25d12`. The
batch's other gates pass; the workspace passes 3302 tests and the
fixtures 108. No render code changed, so no mutants were run.
