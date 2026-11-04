# An unfinished facility's menu and Status (2026-11-04)

This covers OBJ-09-C002 ("building") and C003 ("en route") on the Status side, and CMD-09-C005 ("refinery building"). The reading is in `ghidra/notes/unfinished-facility.md`.

## A0

Captured in the Windows 10 VM (the Alliance, day 563). The files are in `.artifacts/a0-vm/pending-facility/`, with hashes in `a0-sha256.txt`.
- **The order.** Pzob's construction yard builds a refinery for Commenor. Commenor's Refineries page shows two refineries under construction in STRATEGY 9032, each 0 of 2145 pixels off (`11-commenor-refineries`).
- **The menu.** A right click opens Encyclopedia, Status and a grey Scrap (`12-menu`).
- **Status** (`13-status`). "Manufacturing Status" with Location Commenor, Status "Under Construction", Maintenance Cost 0, Standard Processing Rate 5 and Bombardment Value 2. The picture is the refinery over STRATEGY 11571.
- **On the way.** A Coruscant battle and the Alliance's capture of Coruscant played. The story movie MDATA.103 showed the caption "The Alliance has seized the Imperial capital system of Coruscant." (`02.png`, `03.png`).

## Before

A pending unit's page item stood for no object: its right click opened an empty menu, and it had no Status.

## Now

- **The menu.** `FacilityItem::Pending` carries the class, side and arrival, and opens `MenuObject::PendingFacility`. Its rows are Encyclopedia, Status and a grey Scrap.
- **Status.** `StatusObject::PendingFacility` shows "Manufacturing Status":
  - Location;
  - Status "Under Construction", or "Enroute" with the ETA after Location;
  - Maintenance Cost, Standard Processing Rate and Bombardment Value.

  The marks are 11571/11573 under construction, or 11506 en route. The HQ en route takes 11506 too.
- **Keys.** The marks are keyed (`bmp_cache`), so their key pixel shows the frame, as in A0.

## Tests

- Unit tests:
  - `an_unfinished_refinery_reads_under_construction_and_en_route_with_its_marks`;
  - `an_unfinished_facility_lists_encyclopedia_status_and_a_grey_scrap`;
  - the HQ en route test now checks mark 11506;
  - `the_status_pictures_marks_are_keyed` takes 11506, 11510, 11571 and 11573.
- `build-selection.mjs` (Linux, both sides): the queued refinery's cell opens Encyclopedia and Status, with Scrap grey. Its Status reads "Manufacturing Status", the five labels and "Under Construction". For the Alliance, against A0 `13-status`:
  - the picture and the buttons match exactly, 0 differing pixels (before the key fix, 1 pixel: the mark's key pixel);
  - the name inks within a pixel.

Run `build-selection-2026-11-04T16-23-48-813Z-2076065`: pass.

Full batch b57:
- the workspace tests (3372) and fixture tests (116) pass, and provenance-scan is clean;
- 19 of the 21 Linux gates passed on the first run. `status-window` and `build-selection` failed because a follow-up `cargo mutants --in-place` on `status_rows.rs` ran while the batch built the WASM, so the WASM carried a mutant.
- On a rebuilt WASM, `status-window`, `build-selection`, `sector-quadrants`, `personnel-states` and `fleet-window` pass.

Mutants (`rebellion-render` in-diff):
- 3 survived in `status_view`: the HQ-only ETA test, the side-1 mark and the side index. The Empire case and the "yard where the HQ is bound" case close them.
- On the rerun of `status_rows.rs`, 18 were caught and 3 were unviable.

## Open

- No A0 of a facility en route.
- The unfinished defense facility's Defenses window row.
