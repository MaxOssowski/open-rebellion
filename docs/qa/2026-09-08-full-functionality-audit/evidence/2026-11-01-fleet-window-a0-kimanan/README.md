# The Fleet window against A0, pixel for pixel (2026-11-01)

## A0 (VM Alliance game, day 318)

`menus/23-fleet1-{ships,fighters,troops,personnel}.png` (hashes in
`a0-sha256.txt`): the Kimanan Fleet window at (305, 86), 235 by 304.
Fleet 1, selected, holds Corellian Corvette 1, Medium Transport 1 and
two Alliance Fleet Regiments; Fleet 4, Corellian Corvette 2, is in
hyperspace to Kimanan and listed under it. Each capture shows one tab
after a click on it.

## The port's copy of that state

Fixture scenario `FleetWindowA0` (code 80, `interface_test_fixture.rs`
`place_kimanan`): the same fleets, names, cargo and destination, the
ships at full hull, the window opened at A0's (305, 86) with Fleet 1
selected. `fleet-window.mjs` case `alliance/a0-kimanan` clicks each tab
and compares the window with A0's 71 440 pixels
(`--a0-menus=DIR`, defaulting to `.artifacts/a0-vm/menus`).

## What the comparison found and changed

1. **Content indicators** (`FUN_004a66a0` flags, `FUN_004a67a0`): A0 marks
   Medium Transport 1 with the regiment indicator 10405 at (41, 23) in the
   right list, and Fleet 1's entry at (41, 33) in the left. The port drew
   only the no-hyperdrive mark 10430. It now draws 10404 (squadrons, x 25)
   and 10405 (regiments, x 41) under each ship in both lists, and the
   union of its ships' under the fleet entry (hyp: A0 shows it there;
   `FUN_004a3d40` draws the ships').
   port: the port keeps a fleet's regiments and squadrons as the fleet's;
   hyp: they fill the live ships' capacities in fleet order (CAPSHPSD:
   the transport takes 2 regiments, the corvette none). Personnel (flag 4,
   10406) are not placed on a ship.
2. **Disabled tabs take the click**: A0's Fighters and Personnel tabs,
   disabled (state `0x40`) over a fleet with neither aboard, still switch
   the list: an empty list, the Fighters counters "0" and "0", and the
   clicked button in its selected art (10417, 10419 matched within the
   cursor's pixels; Capital ships back to 10409 within 1 pixel). The port
   ignored the click; `tab_resource` now gives the shown tab its selected
   art even when disabled.

## Remaining differences

| Tab | Differing pixels | Where |
|---|---|---|
| Capital ships | 2066 | text, cursor |
| Fighters | 1091 | text, cursor |
| Troops | 2478 | text, cursor |
| Personnel | 1009 | text, cursor |

- Text: GDI hinting of the title, fleet names, counters and item names
  (the accepted text-width deviation).
- A0's cursor over the clicked tab.
- 19 pixels on Fleet 4's entry: A0 adds the 10424 overlay (`+0x50` bit
  `0x200`), the port's Fleet 4 lacks that state (its meaning is
  untraced, hyp: in combat).

The gate fails if a tab's count rises above 2200, 1200, 2600 or 1100.

## Tests

- `a_disabled_tab_takes_the_click_and_shows_its_empty_list_selected`
  (`fleet_window.rs`): fails without the change (the tab stays Capital
  ships).
- `a_ship_s_indicators_mark_the_regiments_it_holds_by_capacity_in_fleet_order`.
- `the_a0_kimanan_scenario_lists_fleet_1_loaded_and_fleet_4_en_route`
  (`interface_test_fixture.rs`).

## Run

`fleet-window-2026-11-01T18-09-50-001Z-2250971` (Arch Chromium 155, not
the pinned build): 73 cases pass, `alliance/a0-kimanan` at 2066, 1091,
2478 and 1009 differing pixels. Test WASM
`2692edb2fac1ca24a9760c030ff34d46b33dccb8764a8792a0d471be367a988b`.
