# The Defenses window's title-button key pixels (2026-11-12)

Cell OBJ-13-C008 (Coruscant controlled) is partial. Its note ended: "the
title buttons' bottom-left key pixel is unsettled in A0 (black here, green
or red elsewhere)". The `sector-quadrants.mjs` case `defenses-other-side`
skipped those three pixels when it compared the Empire's Defenses window of
an Alliance-held system with A0 `coruscant-676/14-after-movie`.

This round traces what the original draws under those pixels. The port
already draws it, so no port code changes. The gate now compares the three
pixels too.

## A0 used

The PNGs stay in the git-ignored `.artifacts/a0-vm/` of the main checkout.

| File | SHA-256 | What it shows |
| --- | --- | --- |
| `coruscant-676/14-after-movie.png` | `c09a8f4832b492eb073ce68710912f1f6a7eb2c5be25078de3b565472b9e56ec` | The Empire's Defenses window of the Alliance-held Coruscant, red strip, at (200, 91) |
| `garrison-clean/test-day483-yavin-regiments.png` | `95003b7e9f8448a70596a3cc3943e0e29171f44912ccdf92928caf05474007a1` | The Alliance's own Defenses window (Yavin) |
| `status/17-def-tab.png` | `08616b81a885e4458aaa268bf254f02af5b338ebe61762ae6093f7da84dedea6` | The Empire's own Defenses window (Coruscant), green strip |
| `fleet-aboard/00-coruscant-fleet1-ships.png` | `55cb1d67f4c9111edcfeccd6374129d4cde8cbd21facb370e384c9a65a6a8635` | A Fleet window, green key pixels |
| `status/22-battery-menu.png` | `7f4c3cff8729282d9b479918c7e46b34db3f47477e85d3158d977bfb97e0660d` | A Defenses window (black) and a Fleet window (green) side by side |
| `menus2/11-mission-dialog-own-cell.png` | `ca5a223af655144352f3ddc71182266d111fc4b44698a0e92e190d913d30329f` | The Create Mission dialog, red at its close box's corner |

## What A0 shows

`coruscant-676/14`, window origin (200, 91), at window points:

| Point | A0 | Neighbours |
| --- | --- | --- |
| (3, 16), sector button | (0, 0, 0) | (2, 16) is the strip's red; (3, 15) is the button's white |
| (204, 16), minimize | (0, 0, 0) | |
| (218, 16), close | (0, 0, 0) | |

The same three pixels are black in `garrison-clean` (Alliance, own side)
and `status/17` (Empire, own side). So for the Defenses window the pixel is
black whatever the strip's colour.

The "green or red elsewhere" captures are other windows:

- **Green.** `fleet-aboard/00..02` and the right-hand window of `status/22`
  are Fleet windows. `status/22` shows green at window x 569 and 583, y 106,
  under a red strip. The Fleet window blits its buttons plain
  (`FUN_004a4b10`; settled in `2026-11-12-window-frames`, port
  `KeyPixel::Colour`).
- **Red.** `menus2/11` shows red (255, 0, 0) at (410, 54), the close box's
  corner of the Create Mission dialog, a different window class. It is not
  traced here.

## The trace

All functions are in `~/Games/rebexe-decompile/decompiled-typed/`.

1. `FUN_004a8790` (Defenses window create, class `0065be30` vt38) loads
   `0x2951` (10577) with `FUN_005fbd20` and hands it to `FUN_006075b0`.
   That stores it at the window's `+0xa4`, the window surface. There is no
   side branch: one id for every side.
2. The same function creates the three title buttons with `FUN_00606c90`
   (`0x277c/0x277d`, `0x280d/0x280e`, `0x27e1/0x27e0`).
3. `FUN_00606c90` passes each button `FUN_006073d0()`, which returns the
   window's `+0xa4`, and the button's x and y. `FUN_00602150` stores them
   as the button's backdrop and its `+0x10c`, `+0x110`.
4. `FUN_00602d30` (the button paint) copies the backdrop from that surface
   at (`+0x10c`, `+0x110`) into its composite `+0xa8` (`FUN_005fcc30`).
   Then it blits the art over it keyed on its first DIB pixel
   (`FUN_005fd0f0`). So the key pixel, the art's bottom-left corner,
   shows 10577 at (x, 16).
5. The title strip is painted by `FUN_004a9800` (vt44). It picks the strip
   by focus (`+0x14c` `+0xb8` against `+0x18`) and blits it with SRCCOPY
   (`FUN_005fc140`) straight to the paint DC. It never writes into
   `+0xa4`. `FUN_00457690` (the System window's paint) does the same.

So the key pixel shows the window background 10577 at the button's
corner, whatever the strip's side or focus. 10577 is black at (3, 16),
(204, 16) and (218, 16), which is what all three Defenses captures show.

## The port

`defenses_window.rs` already repaints each button's key pixel from 10577
(`paint_key_pixel`, `KeyPixel::Background(BACKGROUND)`), with no side or
focus condition. That fix came from round 5 of `2026-11-12-window-frames`.
The 2026-11-11 b191 run that left the pixel unsettled predates it.

- New test:
  `defenses_window::tests::the_title_buttons_key_pixels_show_the_background_whatever_the_strips_side_and_focus`.
  It opens an Empire-held system for the Alliance (the other side),
  unfocused, checks that side 2's unfocused strip 10302 is painted, and
  asserts all three key pixels repaint from 10577. It pins the traced
  behaviour; there is no code change for it to fail without. With the
  `paint_key_pixel` call removed it would fail, as the existing
  `the_title_buttons_key_pixels_show_the_windows_background` does.

## The gate

`sector-quadrants.mjs`, case `defenses-other-side`: the `buttonKeys` skip
is removed. The Empire case now compares every title-row pixel outside the
name's label (x 19..198) with A0 `coruscant-676/14`, the three key pixels
included.

Run on Linux, Arch Chromium, headless, 640 by 480, with the interface-test
WASM of this tree (`wasm_sha256`
`4d38223d394a41201f901010c77ac4d6051b9533cc9c7ca4cc3dede5c16c4dd5`):

- `sector-quadrants.mjs --no-build --only=empire/defenses-other-side
  --only=alliance/defenses-other-side`: 2 of 2 cases pass. Run
  `sector-quadrants-2026-11-12T11-58-57-217Z-946660` (in the agent
  worktree's `.artifacts/interface-parity/`, `result.json` SHA-256
  `432ee4ede300664fcf47ee70ab7312bee7334adb27dd1bc8cb09c59313cd5a4a`).
- Empire case against A0: 816 pixels compared (813 before, plus the three
  key pixels), 0 differ. The name's ink starts at (20, 5) in both and ends
  at y 14 in both ("Coruscant" in A0, the fixture's shorter name in the
  port).
- The A0 file was copied into the worktree's `.artifacts/a0-vm/` for the
  run; without it the case skips the A0 comparison.

## Checks

- `cargo test -p rebellion-render`: 1494 passed, 6 ignored.
- `cargo check --workspace`: ok.
- `rustfmt --check` on `defenses_window.rs`: nothing reported for the new
  test.
- Not run: the full batch, `run.mjs` and the tactical gates (other agents).

## Still open for OBJ-13-C008

- The fixture system is not Coruscant. No browser case opens Coruscant
  itself after it changes hands.
- The Create Mission dialog's red key pixel (`menus2/11`) is another
  window and is not traced here.

## Save format

Unchanged.

## Round 2: Coruscant itself after it changes hands

This round closes the first open item above: a browser case now opens
Coruscant's own Defenses window after the Alliance takes it.

### The fixture

`interface_test_fixture.rs`, new scenario `CoruscantSeized` (index 106,
fixture code 107 | side << 8, so 619 for the Empire). It copies the state
that A0 `coruscant-676/14` shows:

- Coruscant is held by the Alliance, explored and populated.
- It has one Alliance Army Regiment (`0x1000_0001`) and nothing else of the
  Alliance's. Its defence facilities are cleared. Every fleet, regiment,
  special force and character that stood there moves to a system in another
  sector.
- Both fogs reveal Coruscant (`reveal`). In A0 the Empire sees the
  regiment: its Trooper Regiments tab shows as full.
- Its sector window is open. The quadrant setup and watch use the new
  `quadrant_systems`, which makes Coruscant the second system and the
  sector's first other system the primary. Only `Quadrants` places the
  sector marks.

`main.rs` is unchanged.

Unit test:
`interface_test_fixture::tests::the_seized_coruscant_scenario_leaves_the_alliance_one_regiment_there_in_the_empires_sight`.
With the `place_coruscant_seized` call removed it fails on the side. With
the Empire fog's reveal removed it fails on the counts (`[0, 0, 0, 0, 0]`
against `[0, 1, 0, 0, 0]`).

### The gate case

`sector-quadrants.mjs`, case `empire/coruscant-seized`. A case may now name
its own `scenario` and `factions`. Only the Empire runs this one, because A0
is the Empire's game. The case double-clicks Coruscant's defenses quadrant
and checks the following:

- The window shows side 1 on the Personnel page, with counts
  `[0, 1, 0, 0, 0]`, no rows and no garrison line.
- The five tabs match STRATEGY.DLL 10571 (Personnel, pressed), 10563
  (Regiments, full), 10558, 10555 and 10552 (empty). Each tab compares
  1187 pixels and 0 differ.
- The whole 235 by 304 window matches A0 `14-after-movie.png` at (200, 91).
  Every pixel is compared except three masks:
  - the title's name, (19, 2, 180, 16);
  - the page name, (2, 54, 231, 13);
  - A0's mouse pointer, (118, 157, 26, 26).

  64881 pixels are compared and 0 differ. Without the pointer mask, 179
  pixels differ, all in x 120..141 and y 159..180, which is the pointer
  only. The list is empty in both windows, so the 10577 backdrop under it
  is compared too. This includes the red strip, the title buttons and
  their black key pixels, the tab row and the frame.
- The name's black ink is at A0 (20, 5)..(81, 14) and at port
  (20, 6)..(81, 14), width ratio 1.00. Both name Coruscant.
- The page name's white ink is at (89, 54)..(144, 63) in both, ratio 1.00.
- The port opens the window at (200, 91), where A0's stood.

Runs on Linux (Arch Chromium, headless, 640 by 480). The interface-test
WASM `wasm_sha256` is
`b2d60208ea455efb50ff77178eb6b8a01a1a065b92f885228a2cac57937b427f` and the
runtime pack is `3f3f6876a255da49d10ec37220f6b492e5a8239b2555497f0cc84ac7e175f98c`.
The run directories are in the agent worktree's
`.artifacts/interface-parity/`.

| Run | Cases | Result |
| --- | --- | --- |
| `sector-quadrants-2026-11-12T12-12-06-892Z-1223448` | `--only=empire/coruscant-seized` | 1 of 1 pass; `result.json` SHA-256 `9d2b98c487da20556cc6e3584e465f6f1aa746387e466964ecc3007920e708cc`; screen `coruscant-screen.png` SHA-256 `9ef933b8c4761dbbc64e62da8eab423dc03b363c8026158b1fc9d83ad127c419` (inspected) |
| `sector-quadrants-2026-11-12T12-12-19-417Z-1228710` | the whole gate | 33 of 33 pass, including `empire/coruscant-seized` (0 of 64881) and `empire/defenses-other-side` (0 of 816); `result.json` SHA-256 `9c1fe4026b41cc4f9d0de9c278176bea6785960a138eed950d0045601beb28de` |

Two earlier runs failed while the case was being tuned. The pointer mask
was too small in `...T12-11-05-778Z-1195361`, where 16 pixels differed. The
compared-count threshold was too high in `...T12-11-54-173Z-1217209`, where
0 differed but the check required more than 65000 compared pixels.

The A0 file (`c09a8f48...`) was copied into the worktree's
`.artifacts/a0-vm/coruscant-676/` for these runs. `result.json` records the
comparison, so it did run. Without the file the case returns
`a0: "skipped: A0 file missing"`.

### Checks

- `cargo test -p rebellion-app`: 159 passed, 3 ignored.
- `cargo check --workspace`: ok.
- `rustfmt --check`: none of the reported diffs are in the new code.
- `cargo mutants` on the new code in `interface_test_fixture.rs`
  (`place_coruscant_seized`, `quadrant_systems`, `reveal`, `quadrant_setup`,
  decode arm 106, the `apply` hook): every mutant in it is caught. The first
  pass missed three:
  - the sector test that picks the system the contents move to;
  - the character-move filter;
  - the `Quadrants`-only marks.

  The test now checks each of them. The regex also matched two mutants in
  `FleetLoadWatch::next` (6566, 6569), which is earlier code outside this
  change, and both survive.
- Not run: the full batch, `run.mjs` and the tactical gates (other agents).

### Still open for OBJ-13-C008

- The Create Mission dialog's red key pixel (`menus2/11`) is another
  window and is not traced here.
- The fixture places A0's state directly. The browser does not play the
  seizure itself, the battle and the 0x7171 movie. The port's
  SeatOfPower and story-message path for it is covered by unit tests only.

## Batch b199

The full gate batch b199 (2026-11-12 13:25-15:05) passed 28 of 30 gates, the workspace (3631 tests) and fixture tests (159), and 172 tactical battle checks, 0 failed. fleet-window (move-and-carry: the regiment did not board in time) and status-window (fighter: Status did not open in time) failed under a load average near 24 from three agents' builds; rerun alone they pass (fleet-window-2026-11-12T14-08-10-312Z-2322992 2/2, status-window-2026-11-12T14-08-44-749Z-2323466 whole gate).
