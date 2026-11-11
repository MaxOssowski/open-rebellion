# Three fixes from the 2026-11-11 A0 captures

Date: 2026-11-11. Cells CMD-03-C018 (quadrant icon right-click menu), the
Encyclopedia Topic box (OBJ-01, see `2026-11-11-finder-prefix-search`),
PRE-01-C009 and END-01-C011 (the missing-movie box). The A0 is in
`2026-11-11-icon-menu-and-topic-box-a0` (captures in
`.artifacts/a0-vm/2026-11-11-icon-menu-topic/`).

## 1. A unit flying in greys the defenses icon's Move and Scrap

**A0.** Umgul's defenses icon with Piett has Move, Confirmed Move and
Scrap white (`12`). While a TIE squadron flies in, all three are grey
(`30`), and the squadron's own Move is grey (`31`). After it lands they
are white again (`32`, `33`).

**Source.** The original moves a travelling object into its target's
container at once with its en-route bit (`+0x50` bit 4); the character
transit does the same (`mission-lifecycle.md`). The defenses icon's walks
(`FUN_00504c40` regiments, `FUN_005039d0` squadrons) use mode 3, any
existing object (`object-state-flags.md`), so the team holds the unit in
flight. Its move command (`FUN_00578c00`) and its scrap command refuse it
while it travels, so the whole order is disabled (`FUN_0053c100`).

**The port.**
- `TroopTransportState::surface_arrival_pending(world, system, alliance)`
  (`rebellion-core/src/troop_transport.rs`): a regiment transit with the
  leg `Surface(system)` or a squadron transit to `Planet(system)` of the
  side.
- `main.rs`: `defenses_move_enabled`, `issue_defenses_move` (the drop and
  the targeted Move refuse with the "Move refused" line) and the Scrap gate
  of a defenses icon (`order_gates`) check it.

**Coruscant's grey icon.** Coruscant's icon was grey on days 12 to 17
with every visible member free (`00`, `10`, `16`, `18`). An object en
route there is the likely cause, but none was seen.

Deliveries from production now count, as of the stage after b188. The
trace in `ghidra/notes/build-delivery.md` shows the travelling product
already in its destination's container:
- deployment (`FUN_004fb4b0`, event `0x308`) runs `FUN_005150c0`;
- `FUN_005150c0` calls `FUN_00556430`, which makes the product travel to
  "the container it already sits in";
- en route (bit 4) is recomputed by `FUN_004f8240`.

So a built regiment, squadron or special force on its way is in the
defenses icon's team. The port holds such products in `DeliveryState`
until they arrive (the hyp in `delivery.rs`). Now:
- `DeliveryState::surface_arrival_pending(system, side)` answers for a
  regiment, fighter squadron or special force of the side going to the
  system;
- `defenses_move_enabled`, `issue_defenses_move` and the defenses icon's
  Scrap gate (`order_gates`, which now takes the delivery state) check it
  beside the transit check.

The Coruscant case itself stays inferred, since the Empire's deliveries
on days 12 to 17 were not seen in A0.

## 2. A freshly opened Encyclopedia gives the keys to the Topic box

**A0.** `20`, `21`: the Encyclopedia opened from the console, then `cor`
typed with no press anywhere. The box reads `cor` and Corellia is chosen.

**The port.** `open_original_encyclopedia_at` now sets
`original_box_focused`. The trace found no `SetFocus` on the opening path
(`FUN_00429f30`, `FUN_0045d400`, `FUN_0045ddc0`); which call gives the box
the focus is untraced (`ghidra/notes/encyclopedia-window.md`, updated).

The test `typing_reaches_the_topic_box_only_once_it_is_pressed` asserted
the opposite, from that trace. It is replaced, on this A0 evidence, by
`typing_reaches_a_freshly_opened_topic_box_and_chooses_the_best_row`.

The other A0 rows agree with the port and need no change: the chosen row
is scrolled to the list's bottom after an edit (`21`, `22`) and to its top
after a category change (`26`), the minimal reveal of
`reconcile_original_index_state`.

## 3. The missing-movie box takes A0's look

**A0** `53`, measured in 640 by 480 client pixels:
- frame (173, 171)..(472, 319), 300 by 149; raised edge: face
  `255,251,240` and white on the top and left, `160,160,164` and
  `128,128,128` on the right and bottom;
- title (176, 174)..(469, 195), flat `166,202,240`; the caption from x
  201; a close box (448, 176)..(467, 193);
- white body to y 274; the stop icon, 32 square at (197, 220); the path
  from x 239;
- face band y 275..317; OK (380, 285)..(454, 307), 75 by 23, with a dark
  outline.

**The port.** `movie_error_box.rs` drops the Windows 10 hyp:
`MovieErrorBox::layout` gives A0's rects for A0's 200-pixel path (test
`the_box_takes_a0s_rects_for_a0s_path`), and `paint` draws the classic
frame, the title, the close box, the icon and the default button in A0's
colours.

hyp: a longer or shorter path keeps A0's centre and its 100 pixels
beyond the text. The box's font is egui's proportional font at 12
pixels, not the system font, so the text widths differ (accepted
deviation, `acceptance-linux-browser`).

## Tests

- `delivery::tests::a_regiment_squadron_or_special_force_on_its_way_is_pending_where_it_goes`
  (9 of 9 mutants of `surface_arrival_pending` caught).
- `fleet_move_tests::a_defenses_icons_move_and_scrap_are_disabled_while_a_built_regiment_is_on_its_way`
  fails without the check: `left: (true, true)`, `right: (false, false)`.
- `troop_transport::tests::a_regiment_or_squadron_flying_to_a_planet_is_pending_there_for_its_side_only`
- `fleet_move_tests::a_defenses_icons_move_and_scrap_are_disabled_while_a_squadron_flies_in`
  fails without the change: `left: (true, true)`, `right: (false, false)`.
- `encyclopedia::tests::typing_reaches_a_freshly_opened_topic_box_and_chooses_the_best_row`
  fails without the change.
- `movie_error_box::tests::the_box_takes_a0s_rects_for_a0s_path` and
  `a_caption_wider_than_the_text_widens_the_box_past_its_close_box`.

Mutants (in place, scoped):
- `surface_arrival_pending`: 9 of 9 caught;
- `MovieErrorBox::layout` and `open_original_encyclopedia_at`: the first
  run missed 4 in the caption's minimum width; the caption test closes
  them (`layout` 27 caught, 2 unviable).

## The browser gate

`cutscenes.mjs` has a new case, `missing-movie-box`. Its server answers
404 for `data/video/005.webm`, and Credits is chosen on the main menu.
Because the port's path is shorter than A0's, its box is narrower. So the
case finds the frame's left and right edges on row 250 and checks the
frame, title, close box, body, band and OK edges at A0's offsets from
them, and at A0's rows. The same offsets on A0 `53` give the expected
colours. Enter then shows the main menu again.

The first run showed the close box outside a narrower frame. It was
placed from the frame's left at A0's offset. It now stands 5 pixels in
from the frame's right (`MovieErrorBox::close_box`, tested).

Run: `.artifacts/interface-parity/cutscenes-2026-11-11T09-42-54-480Z`,
pass.

hyp: a box narrower than the caption grows to hold the caption, the close
box and 4 pixels (`layout`). With the port's short path, the caption ends
4 pixels before the close box.

## Batch b187

- 29 of 29 interface gates on Linux. `cutscenes`
  (`cutscenes-2026-11-11T10-34-05-712Z`) ran the new `missing-movie-box`
  case among them, and it passed.
- Workspace tests: 3564 passed. Fixture tests: 150. Provenance is ok.
- Tactical matrix `2026-11-11T10-43-17-863Z-109657`: 148 of 148.

## Batch b189

- 29 of 29 interface gates on Linux, among them `build-selection`
  (`build-selection-2026-11-11T12-39-42-512Z-572262`) with the new
  case.
- Workspace tests: 3572 passed, 0 failed. Fixture tests: 151. Provenance
  is ok.
- Tactical matrix `2026-11-11T13-23-46-184Z-633999`: 148 of 148.
