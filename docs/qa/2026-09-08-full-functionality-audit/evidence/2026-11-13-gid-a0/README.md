# GID legend, uninhabited and mission marks, SpecForces typing against A0 (2026-11-13)

Surface ledger cells: CMD-02-C017 (matching legend), CMD-02-C020
(uninhabited), CMD-02-C023 (mission) and OBJ-03-C019 (SpecForces Finder
system-name search). All four pass against A0 in the Linux browser build.

## A0 captures

Original `REBEXE.EXE` in the offline Windows 10 VM, an Alliance game, days
243 to 264, 640 by 480 (`.artifacts/a0-vm/2026-11-13-gid/`, with
`NOTES.txt`). Hashes of the captures used (the folder's `SHA256SUMS` lists
all 37):

```
8294250a8bbbadc04368dd033f6dd95669863156740bd6638ae212bcbbecc4fc  A0-legend-button-tooltip.png
d39eba522dbbe3ea46009ba5f800e5ca3901f51c09aac5f8af85cda6004c5c21  A1-legend-single-press-held.png
d39eba522dbbe3ea46009ba5f800e5ca3901f51c09aac5f8af85cda6004c5c21  A2-legend-single-click-released-nothing.png
98003aa8a8f39d79f58bd6871c4aec6e5d6070e5dc0795573d599e71b90520c4  A3-legend-doubleclick-popular-support.png
ceb9585b53819664c2b23c4711bcd1e1eaf3cc9437013a1b259f6224ff888b9b  A4-idle-fleets-after-alt3-legend-closed.png
c0d1820c23915c6414719fc5757bbc8d348317d6b9459c5649b3a8b9e6ee80a6  A5-legend-doubleclick-idle-fleets.png
86705267acacb37e248192314e7ef144fb2497c9089b60b39754df8a35a203c7  A6-active-personnel-after-alt6.png
9099d335bed2a68554cdea95f4fa085c84cd168b40f3aa6976cdc5386012510f  A7-legend-doubleclick-active-personnel.png
ccf125cb5f41f02e876abca9a091db34e77d00d28756410a848c92734e59ccb0  A9-legend-doubleclick-uprisings.png
0a37114d3371adee12dd8a746a4356eba91fc9a12273fcb3c76bf285ab4800a9  B0-calaron-sector-before-day245.png
a0e91c2316c1127aa472b24ab59ce19e25e5c59f9569d46318ca70e14cac8e89  B2-popular-support-kubindi-explored-unpopulated-day258.png
2738442b8d9de624f80fb01eac076d563a0b560e1cff0202c1d30e2e9fe74098  C2-active-personnel-tallon-on-mission-at-averam-day245.png
5708c34a44528f4dbc0cb00eef3bbff9e0db119ff2d846c002fbae038cbb0922  E1-specforces-finder-alliance.png
d02d2db89c78726c2196ac78bfe32274d57eafb7d339e0c02d82864c17840f72  E2-specforces-finder-empire-empty.png
b4cdea9ed4cf399226bf171664ad98a8abd6537b3509ef52c18405d1eb84cdad  E3-specforces-name-box-clicked-empty.png
18cca921daf0190c70814c883b8a2fcee2ae9f9f65bb333ee9016678f0b123d5  E4-specforces-typed-Q.png
7a6cc0c41d178997711b47018df2365a59b1f0cfad197f4697d3263f4406b2c3  E5-specforces-typed-Y.png
238115ecc7aaec91a619c7ea20abcd6a574849bab4b04a32e564b613f8579f34  E6-specforces-typed-Ya.png
```

## What the trace says

- The legend button (`FUN_00427270`, command `0x134`, 30 by 30 at (55, 50)
  for the Alliance) is built by `FUN_00452ab0` with flags 3 at `+0x114`. Its
  window procedure `Class_00659e38_vt14` (`FUN_00452b70`) returns early on
  `WM_LBUTTONDOWN` and `WM_LBUTTONUP` while bit 2 is set and posts
  `WM_COMMAND` only on `WM_LBUTTONDBLCLK` (`0x203`). Bit 1 lets the button
  itself be dragged (XOR rectangle, then `MoveWindow` clamped to bounds
  starting at (55, 40)); A0 has no capture of that and the port does not
  port it.
- `FUN_00452630` composes the layout on a surface `FUN_005fbea0` makes zeroed
  (key: index 0, `FUN_005fd170`), blitting each bitmap item, kind 1 or 2,
  keyed (`FUN_005fd0f0`). It builds the shading table
  `FUN_005fddd0(LAB_004ac550)`.
- `FUN_00452240` (the legend's paint) copies the galaxy view's picture under
  the window (`FUN_005fcc30`), shades it (`FUN_005fe050`), then blits the
  composed layout keyed on index 0: black art pixels show the galaxy too.

Manual cross-check: "Double-click on the legend" (manual p. 24), "Double-click
on the Show Legend icon to bring up the legend" (p. 68, Fig. 3.5) and
"double-click on the Legend icon" (p. 71). Confirmed by the trace above; the
port was Partial (a click opened it) and is now Present.

## What changed in the port

- The legend button opens the legend on the second press of a Windows
  double click (500 ms, 2 pixels, both presses on the button), not on a
  click (`GidLegendWindow::press_button`). egui's double click could not be
  used: it pairs clicks anywhere on the screen.
- The legend shows the galaxy view under it shaded, through the sector
  window's copy, now shared (`sector_window::paint_shaded_view`).
- STRATEGY 10180..10182 and 10241..10245 key their blue first pixel; 10241
  and 10245 also key black (the composed surface's key).
- The SpecForces view lists the other side's special forces only where the
  player sees the other side's objects, as the Characters view does
  (hyp, `FUN_004f3d50` views). A0 E2 shows an empty Imperial tab; the
  port listed Coruscant.

## Browser gates (Linux, Arch Chromium 155, 640 by 480)

WASM `.artifacts/interface-parity/site/open-rebellion-test.wasm`
sha256 `0a19e1b78a0c374492697a9d45d720e98dc8f8353bc1e4bf285b7154bfa033e3`.

### `tools/interface-parity/gid-a0.mjs` (new; fixture GidA0, code 113)

Run `.artifacts/interface-parity/gid-a0-2026-11-13T01-15-28-370Z/`: pass.

| Check | A0 | Port |
|---|---|---|
| Kubindi (274, 92), Popular Support | B2: 10155 | 10155, 0 of 17 pixels off |
| Ithor, Kessel, Akrit'tar, Fwillsving | B2: 10158 | 10158 |
| A press held a second, a click | A1, A2: no legend | no legend |
| Averam (214, 146), Active Personnel | C2: 10146 (A6 before: 10149) | 10146, 0 of 53 pixels off |
| Unexplored systems, Active Personnel | A6: all 68 show 10154 | all 67 show 10154 |
| Alt+3, Alt+6, Alt+2 with the legend open | A4: legend closed | legend closed |

The legend windows at (150, 120), by pixel kind:

| A0 | Size | Art, frame, close box | Text ink boxes | Shaded galaxy differing | Port frame sha256 |
|---|---|---|---|---|---|
| A3 Popular Support | 180 x 135 | 0 of 1826 | within 1 pixel | 50 of 5296 | `c1f219c0c1bb0f90b8c555980808b708a3cd551172581643280b5e092302d344` |
| A5 Idle Fleets | 180 x 155 | 0 of 1919 | within 1 pixel | 28 of 6202 | `48def391c06fa0d211ccfa9127eb2fe239e8e03ac6011edf9fa01cc4e42e5b2f` |
| A7 Active Personnel | 180 x 95 | 0 of 1688 | within 1 pixel | 24 of 3436 | `4ad80997e2ec150030147dad0f27deab5fca27e7b6a11fe88cf208bcedbedc4b` |
| A9 Uprisings | 180 x 95 | 0 of 1688 | within 1 pixel | 21 of 3436 | `69d88e8874dd13b27aa9315d4f877f8dab1074654b1841cb8c58c0acdb3c5c43` |

The shaded pixels that differ are the systems' marks under the window,
which follow each game's state. Text ink boxes compare left, top and bottom
within a pixel; the right edge is the accepted text-width deviation.

Before the change the gate failed: a single click opened the legend, the
legend's interior was the unshaded galaxy, and 216 art pixels differed
(the blue around 10180, 10181, the separator and the side key).

### `tools/interface-parity/personnel-states.mjs` (SpecForcesYavin, code 114)

Run `.artifacts/interface-parity/personnel-states-2026-11-13T01-15-03-340Z/`:
pass (Alliance, Empire, Alliance SpecForces).

| Step | A0 | Port list outside text | Row |
|---|---|---|---|
| SpecForces view | E1 | 0 | grey, "Yavin" |
| Empire tab | E2: empty | no rows | |
| Name box clicked | E3 | 0 | grey |
| "Q" | E4 | 0 | white (chosen) |
| "Y" | E5 | 0 | white |
| "Ya" | E6 | 0 | white, no scroll |

Text deviation, accepted: the row name's ink is (105, 198)..(132, 207) in
A0 and (105, 200)..(129, 206) in the port (the port's anti-aliased glyphs
are 8 rows to A0's 10); A0's unchosen grey is the palette step
(117, 114, 114) of `0x787878`; the count's digit is masked in its red box.
A0 shows one row only, so the prefix choice among several rows rests on the
trace and `list_prefix` unit tests.

### Other gates on the same build

- GID gate `run.mjs` (Linux copy, `--all`), 38 scenarios, both sides,
  both viewports: 152 of 152 pass,
  `.artifacts/interface-parity/2026-11-13T01-07-27-524Z-1703829/`. The
  `matching-legend` probe now checks that a click opens nothing and
  double-clicks the button.
- `sector-windows.mjs --a0=.artifacts/a0-vm/sector-windows`:
  `.artifacts/interface-parity/sector-windows-2026-11-13T01-14-18-101Z/`,
  pass (the shared shaded copy).

### After rebasing on main (bfdbe30b), fixture codes 113 and 114

WASM sha256 `014b6c8971f36796efff601d0ec09b0a1d3a58a13ce2d28432121de7049a89cd`:
`gid-a0.mjs` pass (`.artifacts/interface-parity/gid-a0-2026-11-13T01-30-49-859Z/`),
`personnel-states.mjs` pass, all three runs
(`.artifacts/interface-parity/personnel-states-2026-11-13T01-31-05-185Z/`),
GID gate `matching-legend`, both sides and viewports, 4 of 4
(`.artifacts/interface-parity/2026-11-13T01-31-36-112Z-1897536/`).

## Unit tests and mutants

Each fix's test fails without it (checked by reverting the change):
`the_legend_button_opens_the_legend_only_on_a_double_click`,
`two_presses_on_the_button_pair_within_the_double_click_time_and_rectangle`,
`the_legend_shows_the_galaxy_view_shaded_under_it`,
`the_legend_takes_the_views_starfield_markers_and_caption`,
`the_gid_legends_art_keys_out_its_blue_first_pixel`,
`the_gid_legends_empire_and_neutral_keys_show_the_galaxy_through_their_black`,
`the_other_sides_specforces_list_only_where_the_player_sees_them`.
`cargo mutants` on `GidLegendWindow::press_button`, `set_view`,
`specforces_rows`, `decode_color_image` and `uses_blue_screen_transparency`:
26 mutants, 18 caught, 7 unviable, 1 missed (`set_view` replaced with `()`)
then caught by the added `set_view` test. Not mutated: the drawing code
(`draw_gid_legend`, `paint_shaded_view`) and the fixture bridge
(`shown_list`, `emit_finder_list`, `place_gid_a0`,
`place_specforces_yavin`), which the browser gates above cover.

## Open

- The legend button's own drag (`FUN_00452ab0` flags bit 1) is not ported;
  no A0 shows it.
- The SpecForces view's prefix choice among several rows has no A0 (the
  game had one Alliance row).
