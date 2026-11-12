# Defenses, Fleet and Missions windows against A0 (2026-11-12)

Round 5 compared the whole System window with A0 and found two defects
(evidence `2026-11-12-menus-and-windows`). This round does the same for the
other three 235 by 304 windows: Defenses, Fleet and Missions.

All browser runs are on Linux: Arch Chromium, headless, 640 by 480. The A0
PNGs stay in the git-ignored `.artifacts/a0-vm/`. Run folders are under
`.artifacts/interface-parity/`. The WASM is the interface-test build of this
change (`wasm_sha256` in each `result.json`).

## A0 used

| File | SHA-256 |
| --- | --- |
| `garrison-clean/test-day483-yavin-regiments.png` | `95003b7e9f8448a70596a3cc3943e0e29171f44912ccdf92928caf05474007a1` |
| `status/17-def-tab.png` | `08616b81a885e4458aaa268bf254f02af5b338ebe61762ae6093f7da84dedea6` |
| `fleet2/umgul-empty.png` | `8570ffca27327d4ebdff5a9049b8e206836d9c08aceebf76b662a35de7fcb8eb` |
| `fleet2/vjun-fleet5-admiral-ships.png` | `aff84cfb62822b1b165ecd5d3b4056c0fdfa65fc0e4fb54ce19720d515c438f6` |
| `system-windows/09-fleet.png` | `395e1db4b69d0247f058471665acf09c6b979499a4b986ab0f0bc5bc2375a1c5` |
| `mission-reports/missions-window-enroute-day276.png` | `3d9969f367d2af01d75456cd37fa6e76e45cd47d98cd166aee94a9d47d9a11d6` |

## How the windows were compared

- Each port window is compared with A0 pixel by pixel.
- Only two kinds of boxes are left out:
  - text objects (title, page lines, names), whose glyphs are Liberation's;
  - what each game's own state draws (other units, other targets).
- The pixel at each title button's and tab's key point, the bottom-left
  corner, is always compared.

## Defenses window

- **A0.** `garrison-clean/test-day483-yavin-regiments`: the Alliance's
  Yavin, Trooper Regiments page, at (305, 86).
- **Port.** The Akrit'tar window of `sector-quadrants.mjs` case
  `alliance/defenses`, Trooper Regiments page, at (68, 86).
- **Left out.** The title, the page name and garrison lines, the first two
  list cells (A0 has two Army regiments, the fixture one Fleet regiment).
  Also the inside of the Personnel, Squadrons and Batteries tabs: their
  empty art differs by state. Their bottom row stays in. The tab art itself
  is still checked against its bitmap (`compareTabs`).
- **Before.** 7923 pixels differed over the whole window. After the masks,
  two port defects were left (run
  `sector-quadrants-2026-11-12T03-06-05-439Z-3715352`, the round 5 run).
- **Defect 1: the title buttons' key pixels.** A0 shows the background
  10577's black at (3, 16), (204, 16) and (218, 16). The port showed the
  key colour green (10209, 10253) or the title strip's red (10108, which is
  already keyed). The buttons are children blitted keyed over the window
  (`FUN_004a8790` makes them; `FUN_00602d30` -> `FUN_005fd0f0`).
  - Fix: `defenses_window.rs` repaints each button's key pixel from 10577
    (`paint_key_pixel`, shared in `fleet_window.rs`).
  - Test: `defenses_window::tests::the_title_buttons_key_pixels_show_the_windows_background`.
    Without the fix it fails.
- **Defect 2: the tabs' key colour.** The port keyed the tabs 10550..10576
  on pure blue. Each tab holds 36 or 62 pure blue pixels and one magenta
  key pixel, its first DIB pixel (10562's key is white). A0 shows the
  selected Regiments tab 10564 with its blue right columns (98..99, 21..52),
  and the background's white at each tab's (x, 52). The port showed black
  and white in those columns and magenta at each corner (61 + 5 pixels).
  - Fix: `bmp_cache.rs` moves 10550..10576 from the blue key to the
    first-pixel key, as the other CoolStrobeButtons.
  - Test: `bmp_cache::tests::the_defenses_windows_tabs_key_their_first_pixel_and_keep_their_blue`.
    Without the fix it fails.
  - The old test `the_defenses_windows_tabs_frames_and_rail_icons_key_out_their_blue_matte`
    asserted the blue key for 10550, 10563 and 10576. A0 shows that was
    wrong. Those three ids moved to the new test; the row frames and rail
    icons keep their blue-key assertion
    (`the_defenses_windows_row_frames_and_rail_icons_key_out_their_blue_matte`).
- **After.** Run `sector-quadrants-2026-11-12T04-47-04-280Z-4101713`: 0 of 49497 compared pixels differ.
- **Gate.** `sector-quadrants.mjs` case `alliance/defenses` now asserts the
  whole window (`pageLabels.whole`). `compareTabs` now skips only each
  tab's first-pixel colour, not blue.
- **The Empire side (one-off check, not a gate).** A0 `status/17-def-tab`: the Empire's
  Coruscant, Planetary Shields page, at (196, 91), the pointer over the
  Shields tab. The port's Empire window from the same run
  (`empire-defenses/shields-page-screen.png`, at (128, 91)), with the text
  boxes, the Personnel, Regiments and Squadrons tabs (state) and the
  pointer (136..185, 30..60) left out: 0 of 58147 pixels differ. Before the
  fix, the round 5 run showed the same three green button corners and the
  tab defects.

## Fleet window

- **A0.** `fleet2/umgul-empty` (Alliance, Umgul, no fleets, at (305, 86)).
  `system-windows/09-fleet` and the `fleet2/vjun-fleet5-admiral-*` tabs show
  the same title row.
- **Port.** `fleet-window.mjs` case `alliance/a0-empty` (fixture 340).
- **Before.** 233 pixels differed (run
  `fleet-window-2026-11-12T02-58-05-059Z-3697148`): 232 are the title's
  glyphs, and one is a port defect.
- **Defect: the close box's key pixel.** In the Fleet window A0 shows the
  key colour green (0, 255, 0) at all three buttons' (x, 16), the close
  box's (218, 16) too. The port keys 10108 everywhere, so it showed the
  title strip's red. The Fleet window blits its buttons plain
  (`FUN_004a4b10` builds them over the surface `+0xa4`). The Defenses,
  Missions and System windows key them.
  - Fix: `fleet_window.rs` paints the close box's key pixel in its green
    (`KeyPixel::Colour(CLOSE_KEY)`).
  - Test: `fleet_window::tests::the_close_boxs_key_pixel_shows_its_green`.
    Without the fix it fails.
- **After.** Run `fleet-window-2026-11-12T04-57-24-672Z-4163419`, case `alliance/a0-empty`: 0 of 71440 pixels differ outside the title box.
- **Gate.** `fleet-window.mjs` case `a0-empty` now masks only the title
  label's box (19, 2, 175 by 16) and allows 0 differing pixels (it allowed
  300 before).
- **Open: two tab pixels.** In the `a0-vjun` tabs, A0 shows black at (130,
  96) and (226, 122). The port shows the strip's backdrop there. Tabs 1 and 4
  (10409, 10412 and their selected art) are 30 pixels wide in 31-wide
  controls, so column 30 is outside the bitmap. Why A0 shows black at just
  one pixel of that column is not traced. The `a0-vjun` cases keep their
  ceilings.

## Missions window

- **A0.** `mission-reports/missions-window-enroute-day276`: the Empire's
  "Mission at Chandrila" at (153, 91).
- **Port.** The F'tral window of `sector-quadrants.mjs` case
  `empire/missions-en-route`, at (318, 91).
- **Left out.** The title, "Target:", the target's and the agent's names
  (text), the mission list (A0 lists two missions, the second selected) and
  the target's picture (another planet).
- **Before.** 6492 pixels differed over the whole window. After the masks,
  5 were left: three title-button key pixels and two tab key pixels.
- **Defect: the title buttons' key pixels.** A0 shows the background
  11165's grey (90, 88, 94) at (3, 16), (204, 16), (218, 16). The port showed
  green. Same cause and fix as the Defenses window.
  - Test: `missions_window::tests::the_title_buttons_key_pixels_show_the_windows_background`.
    Without the fix it fails.
- **Open: the tabs' key pixels.** Under the Agents and Decoys tabs' key
  pixels A0 shows (166, 202, 240) at (105, 142) and (90, 88, 94) at
  (166, 142). The port shows the background 11165's black. The tab strip
  (`FUN_0060d590`, created in `FUN_0049f540`) has its own backdrop, which
  is not traced. The gate allows exactly these two pixels and no others.
- **After.** Run `sector-quadrants-2026-11-12T04-47-04-280Z-4101713`,
  case `empire/missions-en-route`: of 49018 compared pixels only the two
  open tab pixels differ.

## Gate runs

- `sector-quadrants.mjs --a0=.artifacts/a0-vm/system-windows`: 32 of 32
  cases pass, run `sector-quadrants-2026-11-12T04-47-04-280Z-4101713`. The
  System window's whole check still finds 0 of 46130 pixels differing.
- `fleet-window.mjs`: 88 of 88 cases pass, run `fleet-window-2026-11-12T04-57-24-672Z-4163419`. The `chrome` cases now compose the close box unkeyed (`composeExpected`). A first run (`fleet-window-2026-11-12T04-31-55-015Z-3879652`) failed `a0-empty` by this one pixel: a filled 1 by 1 rect draws nothing at scale 1, so the fix paints a mesh.
- Not run here: `run.mjs` (another agent) and the Game Options gates.

## Proposed status

| Cell | Proposal | Why |
| --- | --- | --- |
| OBJ-02-C006 (troop tab) | pass | The whole Trooper Regiments window now matches A0 garrison-clean outside text and state, after the two fixes. |
| OBJ-02-C012 (friendly) | pass | The same Alliance window, red title strip, matches A0 pixel for pixel outside text and state. |
| OBJ-05-C001 (empty fleet) | pass | The empty Fleet window matches A0 umgul-empty in every pixel outside the title text. |
| OBJ-12-C001 (hyperspace) | pass, with a note | The Missions window matches A0 outside text and state except two tab key pixels, an open finding. |
| OBJ-12-C015 (hyperspace indicator) | pass (unchanged) | The en route mini still matches A0 exactly; the window frame is now checked too. |

## Unit tests and mutants

- `cargo test -p rebellion-render`: 1486 passed, 6 ignored.
- Each of the four new tests fails with its fix removed (checked by
  reverting the code only).
- `provenance-scan check`: ok.
- `cargo mutants --in-place --in-diff` on the render diff: 9 mutants, 9
  caught.

## Save format

Unchanged.

## Round 7: the two open tab findings

Both open tab findings above are now traced and fixed. Both come from the
same CoolStrobeButton paint, `FUN_00602d30`. Each paint first copies the
button's backdrop from its surface `+0xa4` at (`+0x10c`, `+0x110`) into
the composite `+0xa8` (31 by 28 for a Fleet tab). Then it blits the art
over it, keyed on its first DIB pixel (`FUN_005fd0f0` -> `FUN_005fc710`).
`FUN_005fc710` copies only the art's own width. So a 30-wide art leaves
column 30 of a 31-wide control showing the backdrop.

### Missions window: the tabs' key pixels

- **Trace.** `FUN_0049f540` creates the strip (`FUN_0060d590`, 122 by 16
  at (105, 127)). It then creates the two tabs with `FUN_00602150` at
  (0, 0) and (61, 0) in the strip. It passes each tab the window's own
  surface `+0xa4` (`FUN_006073d0`), which holds 11165, not the strip's.
  `FUN_00602150` stores the tab's x and y in the strip as `+0x10c` and
  `+0x110`. So `FUN_00602d30` reads the backdrop from 11165 at the tab's
  strip position, not at its window position.
  - The key pixel, the art's bottom-left corner, sits at window (105, 142)
    and (166, 142).
  - It shows 11165 at (0, 15), which is (166, 202, 240), palette index 9.
  - It shows 11165 at (61, 15), which is (90, 88, 94).
  - That is exactly what A0 shows. 11165 is black at (105, 142) and
    (166, 142), which is what the port showed.
- **Fix.** `missions_window.rs` repaints each tab's key pixel from 11165 at
  (tab x, 15) (`tab_key_pixel`). The fix uses a new `KeyPixel::BackgroundAt`
  in `fleet_window.rs`.
- **Test.**
  `missions_window::tests::the_tabs_key_pixels_show_the_background_at_their_strip_position`.
  It fails without the fix (checked by making `tab_key_pixel` return the
  plain background).
- **Gate.** `sector-quadrants.mjs` case `empire/missions-en-route` no longer
  allows any pixel. It now asserts `differing === 0`.
- **After.** Run `sector-quadrants-2026-11-12T06-55-34-591Z-210823`: 32 of
  32 cases pass. The whole Missions window has 0 of 49018 compared pixels
  differing from A0.

### Fleet window: column 30 of the 30-wide tabs

- **Trace.** `FUN_004a4b10` creates the two strips (`FUN_0060d590`, 131 by
  28 at (99, 96)) and their tabs. It passes each tab the strip's surface
  `+0x98`. Whenever a strip is shown, `FUN_004a6980` calls
  `FUN_0060da10(strip, pane, x - 0x61, y - 0x1d)`, that is
  `(strip, pane, 2, 67)`, with the side's pane art (`+0x180` 10407 or
  `+0x184` 10457). That call:
  - replaces the strip's surface with a copy of the pane read from
    (2, 67), so the strip shows the pane where it stands;
  - and for every tab sets `+0xa4` to that surface and **both
    `+0x10c` and `+0x110` to 0**.
- So every tab, whatever its x, reads its backdrop from the strip's
  (0, 0). Its column c shows the pane at window (99 + c, 96 + r).
  - The 30-wide art of the Capital Ships and Personnel tabs (10409, 10412,
    10415, 10416, 10419, and Empire 10459...) leaves column 30 showing the
    pane at window x 129. At x 129 the pane is black for rows 96..122 and
    red at row 123.
  - A0 `fleet2/vjun-fleet5-admiral-*` shows exactly that: black at
    (130, 96) and (226, 122), where the pane itself is (59, 59, 59) and
    (30, 29, 24). It shows red at (130, 123) and (226, 123).
  - The port showed the pane at the column's own position.
- **Fix.** `fleet_window.rs` paints each tab's backdrop under its art: the
  pane, clipped to the tab, placed at (97 + tab x, 29)
  (`tab_backdrop_origin`).
- **Test.**
  `fleet_window::tests::each_tab_shows_the_pane_from_the_strips_left_edge_under_its_art`.
  It fails without the fix (checked by keeping the pane at its own origin).
- **Gate.** `fleet-window.mjs`:
  - `composeExpected` now composes the same backdrop. Without that, case
    `chrome` failed by these 2 pixels per faction (run
    `fleet-window-2026-11-12T06-58-40-614Z-217942`).
  - The `a0-vjun` ceilings are now the measured counts: ships 1578,
    fighters 1125, troops 1125, personnel 1486. Before, they were 1700,
    1250, 1250 and 1600. Every run since 2026-11-11 measured 1580, 1127,
    1127 and 1488, so each tab is 2 pixels closer to A0.
- **Other A0 cases.** In the same run every other A0 comparison in
  `fleet-window.mjs` dropped by 2 pixels, or by 1 where only one 30-wide tab
  shows: a0-destination, a0-kimanan, a0-selonia, a0-umgul and a0-yavin. No
  count rose.

### Round 7 gate runs

All runs used Linux Arch Chromium, headless, with the interface-test WASM
of this change (`wasm_sha256`
`cd9541ea5c7bad530f8ad38091e87a4eb14dbf8384467dafa17393f56fe8f7c7`). The
run folders are in the agent worktree's `.artifacts/interface-parity/`.

- `sector-quadrants.mjs --a0=<abs>/.artifacts/a0-vm/system-windows`:
  32 of 32 cases pass. Run `sector-quadrants-2026-11-12T06-55-34-591Z-210823`.
- `fleet-window.mjs`: 88 of 88 cases pass. Run
  `fleet-window-2026-11-12T07-07-52-309Z-239957`. The `a0-vjun` counts are
  1578, 1125, 1125 and 1486, each at its new ceiling.
- Not run here: `briefing.mjs` and the input and modality gates (other
  agents).

### Round 7 unit tests and mutants

- `cargo test -p rebellion-render`: 1490 passed, 6 ignored.
- `provenance-scan check`: ok.
- `cargo mutants --in-place -p rebellion-render --in-diff` on the render
  diff: 27 mutants, 26 caught, 1 unviable.
  - A first pass missed 4 mutants in `paint_key_pixel`'s
    `BackgroundAt` offset (`-` to `+` or `/`).
  - The Missions test now also asserts that both repaints place 11165 at
    the strip's corner (105, 127). With that, all 4 are caught.

### Round 7 proposed status

| Cell | Proposal | Why |
| --- | --- | --- |
| OBJ-12-C001 (hyperspace) | pass, the note removed | The whole Missions window now matches A0 `missions-window-enroute-day276` in every pixel outside text and state, the two tab key pixels included. |
| OBJ-12-C015 (hyperspace indicator) | pass (unchanged) | The en route mini and the whole window still match A0 exactly. |
| OBJ-05-C002 (populated) | pass (strengthened) | The `a0-vjun` tabs no longer differ from A0 outside text and A0's cursor. The ceilings are now the measured counts. |
| OBJ-05-C001 (empty fleet) | pass (unchanged) | No tabs are shown there. `a0-empty` still has 0 differing pixels outside the title. |

### Still open

- The strip's own column 130, at window x 229, is the zero-filled black
  past the pane copy (130 columns). The port shows 10770 there, which is
  black in every row 96..123, so the two agree. This is noted only
  because it is the one strip pixel the port does not draw from the pane.
- The other A0 Fleet cases (a0-destination, a0-kimanan, a0-selonia,
  a0-umgul, a0-yavin) keep their wider ceilings. Each has 1 or 2 pixels
  fewer to spend now. Their remaining differences are text and state,
  and they were not re-audited here.
