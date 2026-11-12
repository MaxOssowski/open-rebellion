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
