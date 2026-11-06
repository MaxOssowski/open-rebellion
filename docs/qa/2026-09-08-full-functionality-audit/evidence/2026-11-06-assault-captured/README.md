# The Assault Summary's Personnel page and its captives (2026-11-06)

This covers the Assault Summary's Personnel page (CMD-11-C017 surface, OBJ-12-C009 "captured" context). It closes the open item "Veers under Captured".

## A0

`.artifacts/a0-vm/messages-day633/assault-coruscant-alliance-tab4` shows the Alliance's Assault on Coruscant, Personnel page:
- Survivors "None", Captured Veers, Killed "No Casualties";
- the heads are centred in 110-wide columns;
- Veers's mini lies on its backdrop, with the damage monitor at its left and the captive's bars over it.

## The source

From the disassembly (`ghidra/notes/assault-summary-window.md`, "Captured and the rows"):
- `FUN_00496770` files captives aboard the attacking ships under the attacker's Captured (`+0x90`).
- `FUN_00496f50` files captives on the planet under `+0x90` or `+0x98` by their own side.
- `FUN_0044edb0` centres the picture in the whole cell.

## Before

There were three faults:
- The port's characters carry their DAT record id (Mon Mothma `0x240`) with no family byte, so `push_character` dropped every character. In a real game the Personnel page was always empty.
- Captives were never filed under Captured.
- The heads were centred in 165 px, and the picture sat in the cell less the name's height, 6 px too high.

## Now

- **`assault_report`.** Characters are always Personnel. Captives go under Captured by who holds them. `ReportUnit` carries `captive` and `injured`.
- **`assault_summary`.** `unit_layers` draws the backdrop 11500, the damage mark 11502, the mini, then the captive mark (mini + 0x3000). The picture is centred in the whole cell, and the name sits under it. The heads are as wide as their columns.

## Tests

**Unit tests.**
- `captives_go_under_captured_by_who_holds_them` uses a real-data record id (0x2a5), so it fails without the family fix.
- `a_hurt_captive_lies_on_its_backdrop_and_damage_mark_behind_bars`.
- `only_a_captive_character_takes_the_captive_mark`.
- `the_category_buttons_and_the_arrows_pick_the_category` now checks the heads' centring; it fails at 165 px.

**Mutants.** `assault_report`: 20 caught, 2 unviable. `assault_summary`: 6 caught, 4 unviable.

**Browser.** `fleet-window.mjs` `assault-taken` (Linux Chromium, run with `--a0=.artifacts/a0-vm/messages-day633`) carries the other side's general as a hurt captive.
- The Alliance's Personnel page lists Veers under Captured; the Empire's lists Jan Dodonna.
- Veers's mini (rows 145..173, x 168..235 of the window) differs from A0 in **0 pixels**.
- The name below is text, the accepted text deviation.

## Capital ships and squadrons (later the same day)

CAPSHPSD's and FIGHTSD's class ids carry no family byte either (`status_rows::ship_family`). Because of that, the report dropped every capital ship and squadron in a real game, the same fault as the characters'.

**Fix.** `push_as` now files them under Capital Ships and Fighter Squadrons by their source, not by their id. The unit test fixture now uses record ids (0x01, 0x05) without the family byte. With the old squadron push, `the_report_lists_the_fleets_ships_squadrons_and_cargo_as_the_attackers` fails.

**A0.** `assault-coruscant-alliance-tab0` lists Haste and Agile under the Alliance's Capital Ships.

**Browser.** `fleet-window.mjs` `assault-taken` now checks that the Attackers' Capital Ships lists the fleet's ships. The run lists five Mon Calamari Cruisers; the fixture's copies are added after load, before the first day numbers them.
