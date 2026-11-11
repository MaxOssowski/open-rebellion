# Droid, Encyclopedia and quadrant icons against A0

Date: 2026-11-12. Covers OBJ-01-C008 (topic text) and CMD-03-C018
(quadrant icon right-click menu). It also records what A0 is still
needed for CMD-07-C005, C007, C008, C012, OBJ-01-C013, CMD-03-C014 and
C019. This stage does not edit the ledger.

All runs are Linux Chromium 155 (`.browser-linux.json`), from this
worktree's test WASM (`wasm_sha256` f58f1dfc...).

## A0 used

The captures are in the main repo's `.artifacts/a0-vm/`. They are not
committed.

| Capture | SHA-256 |
| --- | --- |
| `encyclopedia/topic-step-2.png` | `6e1899881e47cef4...` |
| `encyclopedia/topic-step-6.png` | `0f225013da1a1f6d...` |
| `encyclopedia/topic-abduction-empire-day166.png` | `f45ec01c3d0ae9ad...` |
| `2026-11-11-icon-menu-topic/00-coruscant-defenses-menu-move-grey.png` | `d8594d7c598a462c...` |
| `2026-11-11-icon-menu-topic/07-sluisvan-defenses-menu-move-white.png` | `a43e0658e184239c...` |
| `2026-11-11-icon-menu-topic/12-umgul-defenses-menu-move-white.png` | `c689f3e2ac6ba749...` |

## OBJ-01-C008: the topic text

**Before.** `encyclopedia-placement.mjs --a0` checked only the first
three lines of the Empire's Abduction body, by ink box. No check read the
words, and no check covered a tab-stop topic.

**The change.** This is a test-only change.
- The PersonnelStates fixture's topic report now carries the title and
  body the window draws (`FUN_0045fa60`).
- `personnel-states.mjs` checks the topic on both sides.

**Abduction.** Both sides, against `topic-step-2` (Alliance) and
`topic-abduction-empire-day166` (Empire):
- The body equals A0's five lines word for word.
- Each line's ink box is within a pixel of A0's. So each line breaks at
  the same word as A0.

**A-wing.** Alliance only, against `topic-step-6`:
- The first five lines are label, tabs, value, as A0 reads them:
  - Refined Material Cost 5;
  - Maintenance Cost 4;
  - Attack Strength (Laser Cannon) 5;
  - Attack Strength (Ion Cannon) 0;
  - Shield Energy 5.
- Each label's ink box is exact.
- Each value sits at A0's tab stop, x 295. The port's digit inks 1
  pixel right and 1 pixel lower. That is within the accepted text
  deviation (Liberation Sans on GDI Arial metrics).

**Runs.** `personnel-states-2026-11-11T22-42-17-035Z` passed on both
sides. After the menu fix below, `personnel-states-2026-11-11T22-58-28-677Z`
also passed on both sides.

**Proposed status: pass.** The topic body now matches A0 in its words,
line breaks and placement for a prose topic on both sides. It also
matches for a tab-stop topic. The text bar's arrows were already exact
(2026-11-05).

## CMD-03-C018: the icon menu's highlight

**A0.** In `00`, `07` and `12` the pointer passed over Scrap and left the
menu. Scrap stays green, the Empire's highlight colour, with the pointer
off the menu.

**The original.** `FUN_004aab50`:
- `WM_MOUSEMOVE` (`0x200`) sets the item under the pointer in `+0x118`.
  It clears `+0x118` when the pointer is in the window but off that item.
- `WM_PAINT` (`0xf`) draws the `+0x118` item in the highlight colour
  (`FUN_004aba60(.., 1)`), wherever the pointer is.
- Without a mouse capture, no `WM_MOUSEMOVE` arrives once the pointer
  has left the window. So the last row stays lit.

**The port before.** `game_menu.rs` kept `+0x118` (`GameMenuState`).
But it lit the kept row only for a menu with a submenu. Once the pointer
left, the object menus' rows went white.

**The fix.** `draw_game_menu_with_submenu` also lights the kept row when
the pointer is outside the window. Inside the window, only the row under
the pointer is lit, as before.

**Unit tests** (`game_menu.rs`):
- `the_row_last_under_the_pointer_stays_lit_after_the_pointer_leaves_the_menu`
  fails without the fix and passes with it.
- `the_pointer_inside_the_menu_off_every_enabled_row_lights_nothing`
  checks the window rect at scales 1 and 2.

**Browser case.** The new `sector-quadrants.mjs` case is
`defenses-icon-menu-a0`. It right-clicks the defenses icon, moves the
pointer onto Scrap, then off the menu.
- The rows are Move and Confirmed Move grey, Encyclopedia white, Status
  grey, and Scrap enabled. In the fixture the moves are grey because the
  team holds the player's agent on a mission (`FUN_0054c110`). In A0
  `00` the cause is inferred to be deliveries.
- Scrap is lit while the pointer is on it. It stays lit after the pointer
  leaves: green for the Empire, red for the Alliance.
- On the Empire, against A0 `00` with the menu at (203, 237):
  - the size is 104 by 102;
  - the frame's two-pixel ring differs in 0 of 808 pixels;
  - each row's colour class matches A0;
  - each row's ink box is within a pixel of A0's, except right edges,
    which are within two. "Encyclopedia" inks 2 pixels wider (the
    accepted text-width deviation).
  - The port's Status grey is 127 where A0 has 128 (the accepted
    palette step).
- The fill shades a different scene, so it is not compared.

**Runs.**
- `sector-quadrants-2026-11-11T22-52-16-682Z-2493234`: 32 of 32 cases
  pass on both sides, the new case among them.
- `fleet-move-2026-11-11T22-56-37-897Z-2585423`: passes.
- Other menu gates are listed under "Gates" below.

**Proposed status: still partial, but closer.** Every icon's menu rows
are ported, and the defenses icon's menu now matches A0. But no A0 shows
the fleet, system, missions or facility icon's menu after a pointer
leaves it. The Coruscant day 12 to 17 cause is still inferred.

## Not settled, and the A0 needed

**CMD-07-C005, C007, C008 (the advisor droid).** No A0 sequence shows
the cockpit droid speaking or gesturing frame by frame. The gates check
clip ops and voices, not pixels. Needed A0:
- In the Windows 10 VM, on a running game, wait for a message that
  speaks. A system joining your side is one (code 1, `FUN_00499760`).
  Take QMP screenshots every 100 ms of the droid's box at the
  bottom-left, (0, 330) to (110, 480), until the clip ends.
- For a gesture, leave the game running unpaused with no events for 50
  sub-ticks. A quip clip (op `0x13`, C3POACT `0x50..0x55`) should play.
  Capture it the same way.
- With those frames, a gate can match each port frame to an A0 frame, as
  `briefing.mjs` does for the briefing droid.

**CMD-07-C012 (missing frame).** No shipped cockpit or briefing clip
that plays misses a frame (`2026-11-09-advisor-missing-frame`). A0 can
show this state only from an edited DLL. The 2026-11-09 check covered
the briefing DLLs. This stage checked the cockpit sprite DLLs, staged in
`data/base/ui/{al,em}sprite-dll/`:
- A definition is a 10-byte RCDATA record `[sound, wave, slot, count,
  first]` (`FUN_00404440`).
- ALSPRITE has 285 such records. 282 have their first BMP and every
  type-302 delta `first + 1 .. first + count - 1`.
- EMSPRITE has 286. 284 are complete in the same way.
- The others are not definitions with missing frames:
  - 232 and 234 in both DLLs read `(4, 1354, 1355, 1356, 1357)`, a
    frame list;
  - ALSPRITE 4155 reads `(0, 1155, 9, 137, 2101)`, and ALSPRITE entry
    10155 (`0x27ab`) reads `(4, 4155, 0, 0)`. They copy ALBRIEF's first
    briefing clip. The briefing queues `0x27ab` from ALBRIEF (flag 2;
    `advisor-briefing.md`, "The briefing"), where the frames are.
    ALSPRITE holds none of them: no BMP 2101, no TYPE302 2102, no WAVE
    1155. Whether anything queues ALSPRITE's copy is not traced. If
    something did, it would be the missing first bitmap case, which
    2026-11-09 left open.

So no cockpit clip misses a frame. Proposal: exclude the cell as a state
absent from the original (user decision 2026-11-10), with the trace, the
unit test and this scan.

**OBJ-01-C013 (missing entry).** The ten source-empty records are the
hidden missions that `FUN_00422620` leaves out of the list. Still open:
whether an object menu's Encyclopedia can name one. Needed A0: in the
VM, send a character on a Move (`0x241`) order. While it travels,
right-click the character in its fleet or the Personnel Finder. Then
choose Encyclopedia and capture what opens. Pick "Move" in the category
list if the menu row is grey.

**CMD-03-C014 (visibility predicate).** Still open: the hypothesis that
the enemy's units count only as the player knows them. Needed A0: an
enemy-held system with a known garrison but no scouting of it, shown in
a sector window. Then the same system after a Reconnaissance mission.
Compare its defenses icon in both captures.

**CMD-03-C019 (icon drag).** No A0 shows a quadrant icon dragged.
`headquarters-move/01`, `02` show the HQ overlay's drag cursor only.
Needed A0, each a press on the icon, a drag onto another system of the
side, and a capture before and after the release:
- a fleet icon, with the Status ETA after it;
- a defenses icon, with and without Ctrl, to show the Confirmed Move
  window;
- a missions icon, to show the refusal;
- a facility icon, to show the managers' Destination.

## Gates

Linux, `--no-build`, the WASM above. Run ids are in `gates.txt`.
- These pass: `personnel-states`, `sector-quadrants` (32 of 32),
  `fleet-move`, `personnel-orders`, `build-selection`, `fleet-window`,
  `status-window` and `mission-dialog`.
- `agent-objectives` failed for an environment reason. It serves the
  production `web/`, whose committed WASM lacks this change, and this
  worktree has no intro videos, so the main menu never came up. The
  batch should run it from the main checkout.

## Mutants

`cargo mutants --in-place -p rebellion-render --file
crates/rebellion-render/src/game_menu.rs --re draw_game_menu_with_submenu`
gave 31 mutants: 27 caught, 1 unviable and 3 missed. All mutants on the
changed lines were caught. The three survivors are in older code of the
same function:
- `584`, `&&` to `||`: it only picks between `request_repaint` and
  `request_repaint_after`, which headless tests do not see.
- `596`, `top + height` to `-` or `*`: the parent row's bottom goes to
  `submenu_origin`. It only matters when a submenu flips above its
  parent near the canvas bottom. `submenu_origin`'s own test covers the
  flip, but no test drives a flipping submenu through this function.

The fixture change (`emit_encyclopedia`) is wasm32-only, so mutants
cannot run it natively. The gate checks it: without the change the probe
has no `body`, and the new assertions fail.
