# Facility menus: the System window's pages open them; every facility offers Scrap (P06, P11)

Date: 2026-10-28. The reading is in
`ghidra/notes/manufacturing-build-selection.md`, "Facility menus". This
covers the "commands" part of P06's open "system-window item compositions
and commands".

## The original

- A right release in the System window opens the object pop-up
  (`FUN_004ac5c0`) for slot 22's selection (`FUN_00453ee0`). On a facility
  page that is the list's selected objects; over empty list space, nothing.
- Each facility class keeps the base order list (`FUN_005399e0`, built by
  `FUN_00539c20`): Scrap (`0x200`). Encyclopedia and Status are always added
  (`FUN_0051d990`). This holds for defense facilities, yards, mines and
  refineries alike.
- The page label is font 4.

## The port

Before:
- the System window opened no menu on a facility page (a code comment said
  so);
- a Defenses window facility's menu lacked Scrap ("untraced");
- the page label was a 10-pixel proportional font.

Now:
- `MenuObject::ManufacturingFacility` and `MenuObject::ProductionFacility`
  exist, with their Status objects and Encyclopedia class ids;
- every facility's menu offers Scrap, Encyclopedia and Status. Scrap stays
  disabled, as it is everywhere in the port until scrapping is ported;
- the System window's facility pages open the menu on the right release:
  for the selected yard, mine or refinery, and for nothing over empty
  space. port: a pending unit's and an empty deposit's objects are
  untraced, so their menu stands for nothing;
- the page label is `original_font(4)`.

## Tests

- `a_facility_lists_scrap_encyclopedia_and_status` replaces
  `a_defense_facility_lists_only_encyclopedia_and_status`. The old test
  held the port's "untraced" empty list.
- `a_right_click_on_a_yard_opens_its_menu` (new).
- `a_facility_page_shows_its_label_pictures_and_selected_frame` now
  expects:
  - the right release on a deposit to open a menu for nothing (it held "opens
    nothing");
  - the page label in font 4.

## Gates

- Workspace: 3036 tests passed. Provenance scan: no uncited items.
- Mutants (`rebellion-render`, the diff): 12 mutants, 8 caught, 4
  unviable.
- Browser:
  - `sector-quadrants.mjs`: 12 of 12 (run `sector-quadrants-2026-10-28T18-56-28-312Z-3861355`);
  - `fleet-window.mjs` `chrome`: 2 of 2;
  - GID: 152 of 152 (run `2026-10-28T18-57-10-812Z-3864106`);
  - `build-selection.mjs`: only its known `opened` 6-pixel failure, which
    predates these stages.
- rustfmt: `system_window.rs` and `manufacturing_window.rs` are clean;
  `object_menu.rs` and `encyclopedia.rs` keep only their baseline
  differences.
- Release WASM SHA-256 `ae7312c0310c1254c25ef058f08c9aa3abfbcb7600fd0a94fe73eec0f8036414`.
