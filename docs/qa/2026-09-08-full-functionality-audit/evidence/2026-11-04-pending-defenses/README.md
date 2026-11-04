# Units being built in the Defenses window (2026-11-04)

This follows the unfinished facility (`2026-11-04-unfinished-facility/`) into the System Defenses window. The reading is in `ghidra/notes/unfinished-facility.md`, "The Defenses window".

## A0

Captured in the Windows 10 VM (the Alliance, day 563). The files are in `.artifacts/a0-vm/pending-facility/`, with hashes in `../2026-11-04-unfinished-facility/a0-sha256.txt`.
- **The page** (`23-tab0`). Pzob's Fighter Squadrons page lists six X-wings, then an A-wing being built at Pzob over the red grid.
- **The menu** (`24-awing-menu`). Move, Confirmed Move, Encyclopedia and Status are white, and Scrap is grey.
- **Status** (`25-awing-status`). "Fighter Squadron Status" with Attached: Pzob, Maintenance Cost 4, Squadron Size 12:12 and the class rows. It has no Status row. The picture is the A-wing over the red grid.

## Before

A unit being built stood only in its producer's band. The Defenses window listed nothing for it.

## Now

- **The rows.** `DefensesItem::Pending` lists the units queued for the system after its built objects:
  - regiments in training on the Trooper Regiments page;
  - squadrons of the shown side on the Fighter Squadrons page;
  - shields and batteries on theirs, and also the defense facilities travelling there.
- **The marks.** `FUN_0042c3b0` lays the unfinished mark last: 11570 for side 1, 11572 for side 2. A regiment keeps its backdrop 11513 under it. A defense facility en route takes 11509.
- **The menus.**
  - A squadron or a regiment opens Move, Confirmed Move, Encyclopedia, Status and a grey Scrap.
  - A defense facility opens Encyclopedia, Status and a grey Scrap.
- **Status.**
  - A squadron shows the planet squadron's rows over 11571 or 11573.
  - A regiment shows the Trooper Regiment Status with Status "Training" (`FUN_00445280`: `0x8771`, TEXTSTRA 34673), over its backdrop 11514 and 11571 or 11573.
  - A defense facility shows the Defense Facility Status, with "Under Construction", or "Enroute" with its ETA first. Its marks are 11571/11573, or 11510 en route.

## Tests

Unit tests:
- `units_being_built_for_the_system_follow_its_own_with_the_unfinished_mark`;
- `the_empires_unfinished_unit_takes_its_sides_mark`;
- `an_unfinished_battery_reads_under_construction_and_en_route_with_its_marks`;
- `an_unfinished_squadron_shows_the_planet_squadrons_status_over_its_mark`;
- `an_unfinished_squadron_lists_the_squadron_rows_with_a_grey_scrap`;
- `a_regiment_in_training_lists_on_the_regiment_page_over_its_backdrop_and_mark`;
- `a_regiment_in_training_reads_training_over_its_backdrop_and_mark`;
- `a_regiment_in_training_lists_the_regiment_rows_with_a_grey_scrap`.

The list tests fail with the pending rows left out.

`build-selection.mjs` (Linux, both sides): fixture 95 makes the secondary the player's and builds its one squadron and one regiment. Its Fighter Squadrons page shows the mini over the mark with 0 differing pixels. The check covers every pixel where either the mini or the mark is opaque. For the Alliance, the A0 `23-tab0` cell matches the same composite. The menu's row states, the Status title and the Status rows are checked too. For the Alliance, the Status picture is compared with A0 `25-awing-status`. The regiment's cell (mini, mark and backdrop), its menu and its Status word "Training" are checked the same way, with no A0.

## Open

- **port: grey Move.** The unfinished squadron's and regiment's Move rows are grey, where A0 offers them, because the unit is a queue entry.
- **Not listed.** A squadron or regiment en route.
- **No A0 for regiments.** There is no A0 capture of a regiment in training.
- **No A0 for defenses.** There is no A0 capture of a defense facility being built or en route.

## The built shipyard (CMD-09-C007)

- **A0** (`31-pzob-shipyards`). Pzob's Shipyards page, with the window at (145, 86), shows its built shipyard as STRATEGY 9006 at (9, 79). The opaque pixels are 0 of 2344 off, keyed on the bitmap's first byte.
- **The gate.** `build-selection.mjs` compares the page's first cell with 9006 on both sides. For the Alliance it now also compares A0 `31` with the same bitmap.
