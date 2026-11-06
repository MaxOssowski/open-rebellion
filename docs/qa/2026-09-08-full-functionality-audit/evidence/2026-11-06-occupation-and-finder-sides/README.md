# Occupation messages and the Personnel Finder's other side (2026-11-06)

This covers EVT-01-C004 (no message when regiments take a planet), OBJ-03-C003 (which of the other side's characters list), and a correction to A0 evidence from the DS_flip_allian save.

## Occupation by regiments posts no message

**Source.**
- `FUN_00510d70` (System vtable `0x208`) sets the "loyalty caused" bit `0x2000` only when neither the old nor the new control word has bits `0x30`. Regiments taking the planet set those bits.
- System Joins (`FUN_00499760`, notification `0x100`) requires `+0x88` bit `0x2000`.
- The setter's own notification `0x31b` is not mapped to any message by `FUN_00488de0`.
- So a planet taken by regiments alone posts nothing to the Message Index.

**Before.** `tactical_flow::occupy` filed "{system} occupied by {side}" on the Popular Support rail for both sides.

**Now.** That line stays in the log, off the index, like the port's other lines that have no original message.

**Test.** `one_side_s_regiments_alone_take_the_planet_and_two_sides_stand_together` asserts the line has no rail. It fails with the old filing.

**Mutants.** `occupy`: 2 caught.

## The Personnel Finder lists the other side's characters only where they are seen

**A0.** The VM's unedited "test" save (Alliance, days 7 and 8):
- the Alliance tab lists eight characters (`.artifacts/a0-vm/personnel-other-side/test-day7-alliance-tab.png`, sha256 `77ffe88a68f7c3c422d89a5fb23f5dda8256ef7d1873c51274558358d8ce55d3`);
- the Imperial tab lists none (`test-day8-imperial-tab-empty.png`, sha256 `0b552aa10f269cc8efd921d07bebaef59e33468162f172f008028fa4c32b2062`).

The Finder lists the player's side's views (`FUN_004f3d50`, `+0x50` bit 1), and at the start of a game the Alliance has no view of an Imperial character.

**Before.** The port listed every recruited character of the tab's side.

**Now.** `character_rows_travelling` lists an enemy character only where `opposing_contents_visible` holds for its system, or for its fleet's system. That is the rule the Fleet Finder already uses for enemy fleets.

**hyp.** What creates and keeps a view is not traced. The port drops the row when the view is lost, while the original may keep a stale view ("Location Unknown").

**Test.** `the_characters_view_lists_recruited_living_characters_of_the_tabs_side`:
- Vader at a neutral Yavin is not listed;
- he is listed once the Alliance holds Yavin;
- the Empire lists him either way.

The test fails without the change.

**Browser** (`personnel-states.mjs`, Linux Chromium, both sides):
- The fixture now reports the Finder's tab and each side tab's centre (`PersonnelFinderState::shown_side_tabs`).
- The gate clicks the other side's tab and finds no rows.
- On the Alliance side it compares the empty list (99..449, 185..353) and the side tabs with A0 `test-day8-imperial-tab-empty`: both are 0 differing pixels. The header's text is the accepted text deviation.

Screenshots: `.artifacts/interface-parity/personnel-states-2026-11-06T17-27-35-197Z/alliance/characters-other-side.png` (sha256 `37948ec6a3b9c465cbd5d75bf96e1265c960d3e9aab370eb47d7884bfa15c5a4`) and `.artifacts/interface-parity/personnel-states-2026-11-06T17-27-35-197Z/empire/characters-other-side.png` (sha256 `b27d00fd3695661cf3d1090e4631a4578cda8f0a8e74f039ce63f346a7de00bb`).

**Mutants.** `character_rows_travelling` and `shown_side_tabs`: 12 caught, 1 unviable.

## Correction: DS_flip_allian builds its messages for the Empire

The day-306 capture `.artifacts/a0-vm/battle-display/joins-ghorman-day306.png` (sha256 `953b9464bcd498e769c2f7ccdb5c3992b39b7ef4b20bbb717463498897ec08ef`) shows "Ghorman Joins Enemy" over "Popular dissent on Ghorman has caused that world to join the Alliance.". That is `0x7039` / `0x703c`, which `FUN_00499760` writes when the reader is not the new side. The Battle Summary at Yavin also used the winner's text, although the Alliance fleet was destroyed.

So this host-edited save shows the Alliance's cockpit but builds its messages for the Empire. Its captures settle layout and pixels, not side-dependent rules. The notes on Maintenance Shortfall (`message-pictures.md`) and the other side's Finder tab (`personnel-finder.md`) are corrected. The Maintenance Shortfall capture is the Empire's own message, as the port posts it.
