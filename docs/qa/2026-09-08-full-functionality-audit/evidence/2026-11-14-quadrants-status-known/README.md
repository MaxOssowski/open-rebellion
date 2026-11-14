# Quadrant icons in hyperspace, Status "Attached:", the known System window, the bombardment summary (2026-11-14)

A0: the original REBEXE in the offline Windows 10 QEMU VM, the unsaved Alliance game of
2026-11-14-missions-transfer, days 541-711. Captures stay outside Git in
`.artifacts/a0-vm/2026-11-14-quadrants-status-known/` (README and SHA256SUMS there).
Hashes below are the first 16 hex digits of SHA-256.

## CMD-03-C014: members in hyperspace count at their target

| Capture | SHA-256 | What |
|---|---|---|
| 02-after-move-to-yavin-averam-defenses-icon-gone | `32cce37c8b6d83d6` | Adar Tallon, Averam's only member, Moved to Yavin: Averam's Defenses icon goes at once |
| 04-wedge-moved-yavin-to-averam-averam-defenses-icon-back | `0205883e5b9fe8a2` | Wedge Antilles Moved from Yavin to Averam: Averam's icon is lit again while he travels |
| 05-han-mission-to-coruscant-mission-icon-at-target-at-once | `e3ab866e8336808b` | Han Solo's mission onto Coruscant: its Missions icon (the Alliance's red) at once |

`FUN_00556390` reparents a moving member into the target container at once, so the source
loses it and the target gains it on the day of the order. The port already did this
(`SystemContents::members_with_arrivals`, 2026-11-04). The new unit test
`a_member_moving_away_counts_at_its_target_and_not_where_it_left` checks both systems
against captures 02 and 04. No code change.

## Status "Attached:" names the ship, not the fleet

| Capture | SHA-256 | What |
|---|---|---|
| 2026-11-14-missions-transfer/36-fleet-regiment-status | `f256fcf141273bbf` | a regiment aboard Fleet 1: "Attached: Medium Transport 1" |
| 21-fleet7-first-ywing-status-attached-escort-carrier-1 | `c3af2f33e3acc788` | Fleet 7's first Y-wing: "Alliance Escort Carrier 1" |
| 22-fleet7-second-ywing-status-attached-escort-carrier-3 | `d1cf713dec11ee19` | its second Y-wing: "Alliance Escort Carrier 3" |
| 32-chewbacca-enroute-status-attached-escort-carrier-1-eta-683 | `c5af6b3b0b40b5a8` | Chewbacca on his way into Fleet 7: "Alliance Escort Carrier 1", Enroute |
| 33-chewbacca-at-rest-day709-status-attached-escort-carrier-1 | `57ec1892a44d1947` | after arrival: the same |

`FUN_0044a2e0` writes the container's name (`FUN_004f62d0`). A unit in a fleet is a child of
a capital ship (`FUN_004fe160`), so the row names that ship. The port named the fleet
("Fleet 1") for regiments, squadrons, characters and special forces.

Fix: `fleet_window::aboard_label` names the ship the port seats the unit on. Regiments and
squadrons use `ship_cargo` and `ship_load`, the room rule of `FUN_00552300`. A squadron uses
its `FighterEntry::carrier`. A character uses its `Character::ship`, else the first ship. A
special force uses the first ship. The port names the fleet only when it seats the unit on
no ship.

Tests:
- These five failed before the change, each reading "Fleet 1":
  - `a_regiment_aboard_a_fleet_is_attached_to_the_ship_that_holds_it`;
  - `a_squadron_aboard_a_fleet_is_attached_to_the_ship_that_carries_it`;
  - `a_character_aboard_a_fleet_is_attached_to_the_ship_it_rides`;
  - `a_special_force_aboard_a_fleet_is_attached_to_its_ship`;
  - `a_member_on_its_way_aboard_a_fleet_is_attached_to_its_first_ship`.
- `a_squadron_on_its_way_into_a_fleet_is_attached_to_the_ship_it_boards` covers a squadron
  on its way in.
- The `fleet-character-en-route` case of `status-window.mjs` now expects the fleet's first
  ship from its Capital Ships list.

The fleet header's left count was checked against A0 missions-transfer/70, 74 and 78. It
reads 1, then 2, then 1, and the port matches. The unit test
`the_fleets_left_count_follows_a_regiment_boarding_and_leaving` covers it.

## OBJ-02-C010: an enemy System window with stale intelligence

| Capture | SHA-256 | What |
|---|---|---|
| 18-chandrila-army-regiment-status-enroute-eta-day171-on-day576-stale | `6a1269a1765f7c9b` | Chandrila (Imperial, no Alliance presence) on day 576: an Imperial Army Regiment reads "Enroute, ETA Destination Day 171" |
| 19-chandrila-tab-277 | `4468c4db2d6ba1ee` | the same System window on day 710: its Mines page lists four mines and eight unmined deposits |

A0 opens the enemy system's windows and fills them from the player's last intelligence,
a snapshot. The regiment's status is frozen from about day 170. This is the per-side view
of each object (`FUN_00539fd0`, `FUN_005844e0`: `+0x24 + side * 4`).

The port has no per-side snapshot. It shows the live contents when `opposing_contents_visible`
holds, and the port's fog never forgets a system. So for a system seen once, the port shows
the same pages as A0, but with current data, not stale data. Not changed: a per-side mirror
is an architecture change.

## CMD-11-C017: the bombardment summary

| Capture | SHA-256 | What |
|---|---|---|
| 41-kubindi-orbital-bombardment-summary-day709 | `235452d4be89e736` | "Orbital bombardment of Kubindi", "Alliance ships have conducted an orbital strike on the non-aligned system Kubindi" |
| 43-bombardment-alliance-forces-capital-ships | `00afcd5429a92c1e` | Alliance Forces: Capital Ships, two Escort Carriers operational, "No Casualties" |
| 44-tip-alliance-0 .. -5 | `2aa3f9542303bcb6`, `4aa0d77fb5f8be30`, `a84c6ef38772d198`, `008725c0bfa83b08`, `f606c366691d74d5`, `83cbdda24f94148b` | category tips: Capital Ships, Fighter Squadrons, Manufacturing, Defense Facilities, Trooper Regiments, Personnel |
| 44-tip-alliance-6 | `e60b94c61d099ca3` | Close: no tip |
| 44-tip-alliance-7 .. -10 | `c1682d4ff1d6e6a6`, `494c778ee97e5fff`, `36b9659bd9505543`, `335d25a20d5b3b1a` | rail tips: Summary, Alliance Forces, System Forces, Display |
| 45-bombardment-system-forces | `6fd6cbe0b3dd2a66` | System Forces: "None" / "No Casualties" |

The tip words match the port's TEXTSTRA ids (`rail_tooltip`, `category_tooltip`:
0x1944, 0x1937, 0x1943, 0x1939; 0x1928, 0x1929, 0x1835, 0x1946, 0x1930, 0x1931). The port
puts no tip on Close either. This run did not compare the two pixel by pixel.

## Runs

Browser runs used Arch Chromium 155.0.8059.84, not the pinned build. Both faction runs of
each gate passed:

- `status-window-2026-11-14T01-59-23-291Z-1224712`: every case on both sides, with
  `fleet-character-en-route` reading the fleet's first ship.
- `fleet-window-2026-11-14T02-03-32-482Z-1234772`: every case on both sides.

Before the fallback for unnumbered fixture ships, `fleet-character-en-route` failed with
"Fleet 1" against the fixture's "Mon Calamari Cruiser" and "Victory Destroyer".

Checks:
- `cargo mutants --in-place` on the diff: 42 mutants, 38 caught, 4 unviable, none missed.
- `provenance-scan check`: ok.
- `rebellion-render` lib tests: 1601 pass.
