# Disabled controls, Battle Summary modality and dead or captured characters (2026-11-12)

This covers five partial cells:

- CMD-10-C010 ("disabled ignores input");
- CMD-04-C018 ("Battle Summary window is modal");
- OBJ-03-C011 ("dead");
- OBJ-12-C009 ("captured");
- OBJ-12-C010 ("killed").

None of them has an A0 capture of its state yet. None can pass today.
This README records:

- the A0 search;
- the source traces;
- one new browser check (the dead row);
- the exact captures still needed.

## The A0 search

I searched `.artifacts/a0-vm/` (129 entries) by file name and by
reading the evidence READMEs. I also looked at the likely frames.

- **Loading.** `load-from-menu/01-after-load` and
  `load-in-game/00-load-question` show the cockpit before or after a load,
  not during it. The `*-wait-*` files (`build-orders/10-wait-1..6`,
  `tooltip/311..318-wait`, `menus3/22-chandrila-wait`) wait for game days,
  not for a load. No capture shows the cockpit while the game is loading.
- **Battle Summary.** These show the window over the command center:
  - `battle-clean/01`, `11`, `23` and `71`;
  - `empire-clean/11`, `41`, `51` and `60`;
  - `battle-display/01`;
  - `2026-10-24-coruscant-withdrawal/f23..f40`.

  In `f23..f40` only C-3PO's animation changes (box 541,349 to 608,453).
  No capture records a click outside the window while it is open.
- **Dead characters.** `alliance-game2/jan-dodonna-killed` and
  `empire-game3/jerjerrod-killed` are event messages. Neither game has a
  Personnel Finder capture after the death. The Finder captures
  `personnel-captured/finder-day628`, `dm`, `ff`, `r2`, `r3`, `r5`, `sf`
  and `health-2026-11-09/06`, `07` show no "Killed" row.
- **Captured on a mission.** `2026-11-11-captured-member/00..05` (an
  Alliance game, days 60 to 66; see `SHA256SUMS` there) is the first half
  of the capture plan in `../2026-11-11-personnel-end-states/`:
  - Chewbacca alone on Incite Uprising to Coruscant;
  - `03`: the Missions window "Mission at Coruscant";
  - `05`: its Mission Status, which reads Target Coruscant, ETA
    Destination Day 150, Team Size 1, Decoys 0.

  There is no capture of the outcome.

A Steam guide screenshot (`docs/qa/2026-09-10-interface-parity-audit/
reference-captures/steam-guide/guide-023.png`) shows the original
Personnel Finder with "Ackbar - Killed", "Adar Tallon - Killed",
"Carlist Rieekan - Killed" and "Chewbacca - Killed". That screenshot is
not A0 (the ledger treats guide screenshots as supporting only). It
confirms the row text and that dead characters stay in the list.

## CMD-10-C010: disabled controls

### The source

- `FUN_00422ce0` (the galaxy view's WM_CREATE) creates the controls in
  `FUN_00427270`. It then calls `FUN_0042d570` at once.
- `FUN_0042d570` sets the wait cursor (`LoadCursorA(0, 0x7f02)`, IDC_WAIT).
  It disables the seven controls (`+0x134..+0x150`) and the control at
  `+0x248`, each through vtable `+0x10`. That slot is `FUN_006028a0`:
  `FUN_006030c0(2)` sets bit 2 of `+0xc4`.
- `FUN_0042d4d0` restores the cursor (`+0x464`) and enables them through
  vtable `+0xc`. Only `FUN_00422620` calls it.
- **The disabled art is the normal art.** `FUN_00427270` builds each
  control with `FUN_00602150` (class "CoolStrobeButton"). That
  constructor clears `+0x9c` (`param_1[0x27] = 0`). `FUN_00427270` never
  calls `FUN_00603150(2, ..)` for these controls. It does so only for
  the rail buttons (`FUN_00603870`). `FUN_00602d30` paints state 2 from
  `+0x9c`, else from `+0x94`, the normal art. So a disabled control
  looks exactly like one at rest. The only visible sign is the hourglass
  cursor.

### The port

The port shows its loading screen until the game is ready. It draws the
cockpit only after that. So no control is ever shown or pressed while
disabled. Visually this matches, because the disabled art is the rest
art (CMD-10-C001 and C003 already compare the rest art with A0). The wait
cursor is not drawn.

No port change was made. No browser case was added, because there is
no A0 frame to compare with.

### Capture needed

A start or a load that A0 catches between `FUN_0042d570` and
`FUN_0042d4d0`:

1. Start a new game, or load a save from the main menu.
2. From the moment the load starts, take QMP screendumps as fast as
   possible (one every 50 to 100 ms). Stop when the day counter shows.
3. Keep the frames that show the command center with the hourglass
   cursor.
4. If such a frame exists, repeat the load. While the hourglass shows,
   click the Fleet Finder control (about 160, 413) and press F3. Capture
   whether a Finder opens after the load ends.

If no frame shows the cockpit with the hourglass, the disabled state is
never visible. The lead could then propose to exclude the cell, as
other absent states were (user decision 2026-11-10).

## CMD-04-C018: Battle Summary modality

### The source

`FUN_0042a640` (a galaxy view method) opens the Battle Summary:

- it posts `0x409` to the view;
- it closes window id 7 (the Battle Alert, `FUN_00604500(7)`);
- it builds the window `FUN_0046f140` (id 8);
- it creates it (`FUN_005ffce0`);
- it shows it (vtable `+0x2c(5)`);
- it registers it with `FUN_005f4f10`;
- it gives it the focus.

`FUN_00429f30` opens the Encyclopedia with the same calls: `0x409`,
`FUN_005ffce0`, `FUN_005f4f10` and `+0x2c(5)`. A0 `modal2/12..16` shows
the Encyclopedia is modal (CMD-04-C019). So the Battle Summary is
probably modal too. This is a `hyp:` reading. Where the galaxy view
checks the registered window was not traced.

### The port

The port shows the Battle Summary in the tactical mode. The galaxy is not
on screen, so it takes no input. Whether the summary should open over
the strategy screen is an open question for the user (TAC-01-C013). No
change and no browser case were made.

### Capture needed

In any game, when a space battle's Battle Summary opens over the command
center (after Simulate Results or after a tactical battle):

1. capture it untouched;
2. click the GID control (537, 446) and capture;
3. right-click the agent droid and capture;
4. click a rail button and capture;
5. double-click a planet in an open sector window and capture;
6. click a cockpit control (the Fleet Finder) and capture;
7. press F3 and capture;
8. close the summary and capture, to see whether any input took effect
   later.

This is the same protocol as `../2026-10-30-modal-windows/`.

## OBJ-03-C011: the Personnel Finder's dead row

### The source

`FUN_00465bb0`: a destroyed character (`+0x50` bit 3) reads the name,
" - " and "Killed" (`0x8798`). With destruction reason `0x14..0x16` it
reads "Retired" (`0x8799`). It names no place and no state.

### The port, and a new browser check

The port already writes this row (`character_text` in
`crates/rebellion-render/src/personnel_finder.rs`). The unit test is
`a_killed_character_is_listed_as_killed`.

New in this change:

- **Fixture.** The PersonnelStates fixture (code 93) now kills the
  side's sixth recruited character, if it has one, with
  `Character::mark_killed`. Its abode and ship are cleared, and no fleet
  lists it. Only the Alliance has a sixth character.
- **Unit test.**
  `the_personnel_states_scenario_lists_its_sixth_character_as_killed`
  builds six Alliance characters. It checks that only the sixth is dead,
  that it stands nowhere, and that the open Finder lists "Six - Killed".
  It fails without the fixture change (`left: []`).
- **Browser.** `personnel-states.mjs` now finds exactly one row that
  matches `^Name - Killed$` for the Alliance, and none for the Empire.

Run `personnel-states-2026-11-12T06-48-16-828Z` (Linux Chromium): pass,
both sides. Test WASM
`2215372147f7983a1a408b6e6cbbc4a4ac7add8800f99c6fe57fd59a7211a043`.
The Alliance's Characters view lists "Chewbacca - Killed" first
(`alliance/characters.png`, SHA-256
`0fc110f2839fc4e8c25ce3ba6b3d453ffa3507eb2ddec1d6d6942c3ae3e95806`).
The other rows are unchanged, and the later cases (row menu, other-side
tabs, Encyclopedia) still pass. After the run, only the unit test
changed: it now seats the sixth character in a fleet first.

`cargo mutants` on the new fixture lines: the one mutant (`!=` to `==`
in the fleet `retain`, line 1199) was missed at first. It is caught
since the test seats the character in a fleet. The run again missed the
two old `FleetLoadWatch::next` mutants (lines 6469 and 6472), which this
change did not touch. `provenance-scan check`: 0 uncited.

The row is not compared with A0, because no A0 shows it. The cell stays
partial.

### Capture needed

In a game where one of the player's characters has died (for example,
the Alliance game of `alliance-game2/`, after Jan Dodonna's death):

1. press F5;
2. choose the player's tab and the Characters view;
3. scroll to the dead character and capture the row;
4. right-click the row and capture its menu.

Then the gate can compare the row's text and ink box with A0.

## OBJ-12-C009: a mission member captured

No change. The plan in `../2026-11-11-personnel-end-states/` still holds.
Its first step is done (`2026-11-11-captured-member/05`, see above).
Chewbacca's mission reached Coruscant on day 150 of that game. If that
game is still the one in the VM (at about day 217), its Message Index
may already hold the outcome.

**Captures still needed:**

1. the Message Index on All, with Chewbacca's rows after day 150 (for
   example "Chewbacca Captured" or "... Mission Foiled");
2. each of those messages, opened;
3. F5, the Alliance tab: Chewbacca's row (for example "( Captured )");
4. the Missions window at Coruscant, if the mission still lists.

## OBJ-12-C010: a mission member killed

No change. The Killed message matches A0 (`jan-dodonna-killed`,
`jerjerrod-killed`), but neither death is known to come from a mission.

**Capture needed:** a "<name> Killed" message for a character who died
on a mission. Capture that mission's Missions window and Mission Status
before and after the death. This needs luck. Sending one weak character
on Assassination or Abduction to a well-defended Imperial system gives
the best odds.

## The VM

The VM was paused (QMP `query-status`: paused) when I looked. The game
showed day 217 with the Sesswenna and Sumitra windows open. I sent
input before I saw that it was paused:

- F5;
- a click at (271, 413), the Personnel Finder control;
- pointer moves to (300, 300).

The VM was not resumed. If these inputs are queued, the Personnel Finder
may open when the VM is resumed. `~/VMs/rebellion-win10/cursor.json`
now says (300, 300). Run `g.py home` before the next walk.
