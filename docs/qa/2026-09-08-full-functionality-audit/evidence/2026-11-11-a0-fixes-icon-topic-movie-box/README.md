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

**Not covered.** Coruscant's icon was grey on days 12 to 17 with every
visible member free (`00`, `10`, `16`, `18`). An object en route there
is the likely cause, but none was seen. Deliveries from production
(`DeliveryState`) are not counted; whether a delivery sits in its
destination's container while it travels is untraced.

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
