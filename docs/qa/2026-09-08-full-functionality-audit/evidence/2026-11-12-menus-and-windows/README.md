# Menus and windows against A0 (2026-11-12)

Cells: PRE-02-C002 (14 main-menu hotspots), PRE-03-C008 (corrupt save),
PRE-03-C009 (music slider), PRE-03-C010 (sound slider), CMD-02-C002
(sector window) and CMD-02-C003 (System window).

All browser runs are on Linux: Arch Chromium, headless, 640 by 480 (the
smoke at 1280 by 960). The A0 PNGs stay in the git-ignored
`.artifacts/a0-vm/`. Run folders are under `.artifacts/interface-parity/`.

## A0 used

| File | SHA-256 |
| --- | --- |
| `sector-windows/01-first.png` | `5ed39d9fa627fe5200905dfd7e3e4d203ba237568966e09fc29b498600142074` |
| `sector-windows/02-second.png` | `f20d1ce39f5a2eb25a14f2739ee950b577cdf4d060d2a2f7768219480b978195` |
| `sector-windows/03-flip.png` | `92f5607873c55eb98ef5fde284b2a2518c15a4f260261302e9e8927c7c297d60` |
| `sector-windows/04-left.png` | `d2212cadb3a3df678885d437673d078eeb7dfa0b274bb55240beb36c640b0fce` |
| `system-windows/07-icon-dbl.png` | `c8fe313bb07b6f7bff9527574838326d43a5b5804e56457ff47bf6f9e663a4b4` |
| `main-menu-states/01-menu.png` .. `12-load-options.png` | in `main-menu-states/a0-sha256.txt` |
| `tooltip/200-options.png` | `4466ded7e8b37c5120190f1e696f7d8beeed1d0f6026cfe711a098b950edce3e` |

## CMD-02-C003: the whole System window

- **A0.** `system-windows/07-icon-dbl`: Yavin's System window, Alliance,
  at (314, 86), 226 by 304.
- **The comparison.** The port's Akrit'tar window (fixture 306, the
  `open-system` case of `sector-quadrants.mjs`) against A0, pixel by
  pixel. Only the text objects are left out: the title and, in each band,
  its title, status and destination lines and its count. Their glyphs are
  Liberation's, and the names differ (Yavin, Akrit'tar). The status lines
  and counts are still checked by ink box, as before.
- **Before.** 41 of 46130 compared pixels differed (run
  `sector-quadrants-2026-11-12T01-50-51-028Z-3332065`). They were two port
  defects.
- **Defect 1: the title strip's right end.** A0's red title strip ends at
  x 223. Columns 224 and 225 are the window's navy (0, 0, 128) right
  border. The port drew the strip (STRATEGY 10299, 231 wide at x 2) to the
  window's edge at 226 (32 pixels).
  - Fix: `system_window.rs` clips the strip at `TITLE_STRIP_RIGHT`, 224.
    This also matches the 235-wide Fleet, Defenses and Missions windows,
    whose 231-wide strip at x 2 ends at their own 2-pixel border.
  - Test: `the_title_strip_ends_inside_the_windows_right_border`. Without
    the fix it fails (the clip ends at 226).
- **Defect 2: the buttons' and tabs' key pixels.** Each title button
  (10209, 10253, 10108) and tab (10311..10334) has one key pixel, its first
  DIB pixel at the bottom left. A0 shows the window's background 10297
  there: (9, 11, 32) at (3, 16), (195, 16) and (209, 16), and white at each
  tab's (x, 52). The port showed the key colour itself (green at the sector
  and minimize buttons, magenta at the tabs) or the title strip's red (the
  close box, which is already keyed) (9 pixels). The buttons and tabs are
  child controls blitted keyed over their parent (`FUN_00602d30` ->
  `FUN_005fd0f0`); the strip is a sibling, so it does not show through.
  - Fix: after each button and tab, the window repaints that one pixel
    from its background (`key_corner` in `draw_system_window`). This is
    local to the System window; the Defenses window shares the buttons
    and may need the same (not checked here).
  - Test: `the_buttons_and_tabs_key_corners_show_the_windows_background`.
    Without the fix it fails (only the whole background is painted).
- **After.** 0 of 46130 compared pixels differ (run
  `sector-quadrants-2026-11-12T02-14-46-397Z-3443969`, all 32 cases pass).
- **Gate.** `sector-quadrants.mjs --a0=.artifacts/a0-vm/system-windows`
  now asserts this in the Alliance `open-system` case (`checks.whole`).
  The port's capture is saved as `system-window-whole.png`.
- **The Empire side (one-off check, not yet a gate).** A0
  `build-selection2/11-stopped` (SHA-256 `84bf30f6...dbe30`): the Empire's
  Coruscant window at (203, 91), green title. The port's Akrit'tar window
  from the passing `build-selection` run
  (`build-selection-2026-11-12T02-17-31-763Z-3451382/empire/overview-screen.png`,
  at (225, 76)), with the same text masks and the Shipyards tab masked:
  0 of 44942 pixels differ. The Shipyards tab differs by state only: A0's
  Coruscant has no shipyard ("0:0"), so its tab shows the empty art; the
  fixture's system has one. `build-selection.mjs` is left to the agent
  working on the manufacturing windows; this check could join it.

## CMD-02-C002: the whole sector window

- **A0.** `sector-windows/01-first` to `09-third` (new Alliance game,
  Popular Support display, day 3 to 4).
- **A port defect found.** The original's sector window shows a shaded
  copy of the galaxy view under it (`FUN_004593e0`). That copy holds the
  view's caption, which `FUN_00425d00` draws into the view's picture. A0
  shows "Popular Support" there in (91, 67, 51), the shaded form of its red
  (255, 0, 0). It shows from x 252 to the left window's buttons, and from
  the right window's edge to x 342. The port drew the caption only on the
  bare galaxy view, so both windows covered it.
- **The colour.** Red is entry 249 of the starfield's palette (STRATEGY
  902 and 903). `FUN_005fddd0`'s table (`LAB_004ac550`: each channel
  c / 6 + 50) gives (92, 50, 50), whose nearest entry is (91, 67, 51). That
  is A0's colour exactly.
- **The fix.**
  - `lib.rs`: `gid_caption` returns the caption `draw_gid_caption` paints.
  - `sector_window.rs`: `set_gid_caption` takes it. Each window paints it
    over its shaded copy, in the shaded colour.
  - `bmp_cache.rs`: `shaded_palette_color` maps a colour through the
    starfield palette and the shading table.
- **Tests.** `the_window_shows_the_views_caption_under_it_shaded` fails
  without the paint call (no caption text). `a_colour_shades_through_its_nearest_palette_entry`
  pins the colour mapping.
- **Gate.** `sector-windows.mjs` now checks, in every step:
  - the shaded caption's ink box in each open window. A0 [252, 43, 263, 52]
    and [302, 43, 342, 55]; port [252, 44, 263, 52] and [302, 44, 343, 55].
    Before the fix the port had no caption ink at all.
  - In `04-left` (Churba, ten unexplored systems), the whole window. The
    title row and the ten names are left out (text). Run
    `sector-windows-b-20261112T031413`: 70323 pixels compared, 21 patches,
    557 pixels, the largest 15 by 25.
    The remaining patches are the galaxy markers behind the window, which
    are each game's own. The gate allows a patch up to 16 by 26 (two
    crosses stacked) and fewer than 700 pixels in all. A wrong shade,
    background or picture would make one large patch.
- **Earlier parts still pass.** Frames, buttons, places, Churba's planets
  and names, the whole Moddell window and the uninhabited tracks.
- **What a whole window cannot settle.** The fixture is another game than
  A0's, so the tracks, names' colours and markers of owned systems are
  compared piece by piece (earlier evidence), not in one frame. A fixture
  with A0's Sesswenna state would allow a whole Sesswenna comparison.

## PRE-02-C002: main-menu hotspots

- **Linux rerun.** `main-menu-states.mjs --a0=.artifacts/a0-vm/main-menu-states`
  passes (run `main-menu-states-b-20261112T024844`). Every compared region
  differs by 0 pixels. This includes the Intermediate monitor's keyed
  corners (Easy stays chosen) and Alliance's keyed corner (no game).
  `main-menu-rest.mjs` also passes (run `main-menu-rest-b-20261112T024918`).
- **Which controls A0 can still separate.** The A0 hit test only works
  where a 7 by 7 block of a bitmap's key colour lies inside its rectangle.
  Otherwise the QMP pointer's few-pixel error decides. I counted the key
  pixels in each frame of the remaining controls (COMMON.DLL):

  | Control | Rectangle | Bitmaps | Key pixels | 7 by 7 key blocks |
  | --- | --- | --- | --- | --- |
  | Game Type | (305, 333) 42 by 30 | 10158, 10159 | 326..327 | 0 |
  | Load/Options | (411, 232) 40 by 37 | 11241..11255, 10013 (40 by 36) | 166 | 0 |
  | Credits | (459, 242) 33 by 28 | 11271, 11272 | 214 | 0 |
  | Multiplayer | (67, 381) 51 by 61 | 11151..11180, 10005 | 1180 | 250 |

  - Game Type, Load/Options and Credits have only thin keyed edges. A0
    cannot tell their bitmap test from a rectangle test. The port's
    `hit_test_exact` already uses the bitmap for them.
  - The galaxy-size screens are pointer regions of the window
    (`FUN_00405050`), not bitmaps, so the rectangle is the original's rule.
  - Multiplayer is the one control left with a deep keyed area. Its top
    left (bitmap 8..14, 4..10) and its bottom right are keyed. The port
    takes no press there (`hit_test_exact`). This needs one A0 probe.

## PRE-03-C008: corrupt save

- **Linux rerun.** `game-options-smoke.mjs` passes on the production
  build (run `options-smoke-20261112T030101`). Both sides pass the corrupt
  load and overwrite-cancel checks, and the corrupt bytes stay in storage.
- **What the original does (decompile).**
  - Game Options reads each slot's header when it builds its rows
    (`FUN_00407180` -> `FUN_00401860` / `FUN_00401910` -> `FUN_00415cd0`).
  - `FUN_00415cd0` opens the file (`FUN_005f4a60`). It reads the header
    only if the open succeeded (`FUN_005f49b0` returns `DAT_006ac240`,
    which is 2 when the open failed). The header is a name string and six
    DWORDs (`FUN_00411900`), read with `istream::read` (`FUN_006173b0`).
  - The defaults are an empty name and six zeros (`FUN_00411730`). A short
    read leaves them so.
  - The Load button is enabled only when the header's DWORD at +0x18 is 1
    (single player) and `FUN_00401910` returns non-zero (the same field).
    Otherwise it is disabled (vtable +0x10).
  - The crest needs +0x18 = 2 (multiplayer, 10057) or a side of 1 or 2
    at +0x20 (10056 or 10055).
  - So an unreadable or zero-length save shows as an empty row: no name,
    no crest, Load disabled. A Save on it asks no overwrite question,
    since `FUN_00401910` gives 0 there (`FUN_00406b80` case 0x6f..0x74).
  - A save with a good header but a corrupt body loads through
    `FUN_004158e0` (a `Class_00658ba0` loader). I did not trace its error
    path.
- **The port differs.** `read_save_slots` (main.rs) lists an unreadable
  slot as "Unreadable save" / "Load to inspect error" with Load enabled.
  The overwrite question is asked. I did not change this: the decompile
  says what the header-level case should be, but A0 has not shown it.
- **A0 needed.** In the VM, put a zero-length file and a truncated save
  (header intact, body cut) in two slots. Capture Game Options with those
  rows, then press each row's Load and Save.

## PRE-03-C009 and PRE-03-C010: the sliders

- **Linux rerun.** `game-options.mjs --a0=.artifacts/a0-vm/tooltip`
  passes (run `game-options-b-20261112T024930`): A0 `200-options`
  compared over 201491 pixels with 0 differing, the sliders' thumbs and
  drags, and the reload.
- **Still open.** The ledger's remaining gap is the audible level. The
  transfer is traced (`FUN_00406b80`, `FUN_00407180`), but no A0 audio
  was recorded. An A0 recording would need the VM's sound output captured
  (QEMU's wav audiodev) at a few slider settings.

## Other gates run

- `build-selection.mjs` (the System window's bands): passes, run
  `build-selection-2026-11-12T02-17-31-763Z-3451382`.
- `sector-quadrants.mjs`: 32 of 32 cases pass, run
  `sector-quadrants-2026-11-12T02-14-46-397Z-3443969`.
- `sector-windows.mjs`: passes, run `sector-windows-b-20261112T031413`
  (all eight steps, the caption, Churba, Moddell and the uninhabited
  tracks).
- The WASM for these three runs was built from this change (interface-test
  build, `wasm_sha256` in each `result.json`).
- Not run here: `run.mjs` (another agent works on it). The batch should
  rerun its GID cases, since the sector windows now paint the caption.

## Proposed status

| Cell | Proposal | Why |
| --- | --- | --- |
| CMD-02-C003 | pass | The whole window matches A0 outside its text on both sides (Alliance in the gate, Empire one-off), and the text is within the accepted deviation. |
| CMD-02-C002 | pass | Every A0 step matches in frame, buttons, place, title, caption and planets, and the whole Churba and Moddell windows differ only by text glyphs and each game's own markers. |
| PRE-02-C002 | partial | All Linux checks pass, but Multiplayer's keyed area still needs one A0 probe. |
| PRE-03-C008 | partial | The Linux smoke passes, but the port's unreadable row differs from the traced original and A0 has not shown it. |
| PRE-03-C009, C010 | partial | A0 visuals and the Linux gate pass; the audible level has no A0 recording. |

## Unit tests and mutants

- `cargo test -p rebellion-render`: 1481 passed. `cargo test -p
  rebellion-app`: 156 passed. `provenance-scan check`: ok.
- `cargo mutants --in-place --in-diff` on the render diff: 26 mutants,
  23 caught, 2 unviable, 1 missed (`==` to `!=` in `gid_caption`'s
  viewer test). The new test
  `the_gid_caption_for_the_sector_windows_follows_the_viewer` catches it
  (rerun with `--re gid_caption`: 16 caught, 2 unviable). That rerun's one
  miss is in `draw_gid_marker` (macroquad's `dest_size`), code this change
  does not touch and no headless test can see.

## Save format

Unchanged.
