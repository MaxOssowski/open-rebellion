# Personnel end states and the Status window's en route mark (2026-11-11)

This covers OBJ-10-C004 ("en route"), and records what is still missing for
OBJ-03-C011 ("dead"), OBJ-12-C003 ("complete"), OBJ-12-C009 ("captured")
and OBJ-12-C010 ("killed").

## OBJ-10-C004: an en route special force's Status

### A0

`.artifacts/a0-vm/sfstatus/03-sf-status.png` (SHA-256
`485b385232e491cee91b81da3ef4db1b0816bcbe8a5195a1622d71513688811e`; the
Alliance's DS_flip_allian save, day 180, see
`2026-11-02-special-force-status-a0/`). The Infiltrators travel to Yavin.
Yavin's Defenses window lists them, and their Status reads:

- the title "Spec Forces Status", the window at (108, 79);
- eight rows: Attached, Status "Enroute", ETA Destination "Day 249",
  Maintenance Cost and the four ratings;
- the picture: the Infiltrators' mini (GOKRES 1345) over the en route
  mark, grey panels with blue streaks (STRATEGY 11520).

### The source

- `FUN_00445780` fills the rows. The ETA row shows only while the unit
  travels.
- `FUN_0042c3b0` draws STRATEGY 11520 under a travelling special force's
  picture. Classes `0x3c000003` and `0x3c000005` take the GOKRES mark
  instead (`2026-10-22-status-picture-marks/`).

### The port

No port change was needed. The new fixture `SpecialForceEnroute` (code
104) is the Recon fixture with a team on its way to the primary system:
Infiltrators (`0x3c000002`) for the Alliance, Imperial Commandos
(`0x3c000007`) for the Empire. Neither class takes the GOKRES mark. The
team is a member transit, so the destination's Defenses window lists it
first on its personnel page, as A0 does.

The fixture now also reports the Status window's marks for this
scenario (`emit_status_rows`).

### Browser (Linux Chromium, `status-window.mjs`)

New case `special-force-en-route`, both sides:

- the menu is 104 by 122 with A0's six rows;
- the rows' labels are A0's eight; Status is "Enroute", ETA Destination
  "Day 249";
- the marks are exactly STRATEGY 11520;
- the window's background and buttons are exact, as in the other cases.

Alliance, against A0 `sfstatus/03`:

- the picture frame's interior (130 by 98 pixels, 12,740 in all):
  **0 pixels differ**;
- the title and all eight labels: each ink box within 1 pixel of A0's.

The values are not compared with A0. The fixture's unit stands at
another system and has other ratings.

Run `status-window-2026-11-11T14-16-17-925Z-707410`: pass, 22 of 22 cases
(11 per side). Test WASM
`7985f94b9deb34377ebbd024f55f60a92235b101d47b75548620391fc1ba83fe`.
After the run, three statements in the Rust were reformatted only.

### Tests

- `the_special_force_enroute_scenario_sends_a_team_to_the_primary_system`
  (the fixture's transit and class, for both sides).
- `the_targeting_fleet_move_and_fleet_window_scenarios_report_the_object_menu`
  now lists the new scenario.
- Without the scenario, fixture code 104 does not exist, and the gate's new
  case cannot open.
- `cargo mutants` on the new fixture lines: 10 caught, 0 missed. Two other
  mutants in the run were missed (`FleetLoadWatch::next`, lines 6054 and
  6057). Those lines were not changed here. The `emit_status_rows` arm runs
  only in the WASM build, so the browser gate tests it.
- `provenance-scan check`: 0 uncited.

## OBJ-10-C004: an en route regiment, and one in training

### A0

`.artifacts/a0-vm/2026-11-11-regiment-en-route/` (an Alliance game in the
Windows 10 VM; `NOTES.txt`, `SHA256SUMS`). On day 42 one of Yavin's
Alliance Army Regiments was sent by Move to Averam. Another was in
training at Averam.

| File | SHA-256 | Shows |
|---|---|---|
| `00-averam-troops-page.png` | `4e1333c0…` | Averam's Trooper Regiments page: the travelling regiment first, with the striped mark under its mini; then the one in training, on the red grid |
| `01-training-regiment-menu.png` | `4304c0fb…` | the training regiment's menu: Move, Confirmed Move, Encyclopedia and Status lit; Scrap grey |
| `02-training-regiment-status.png` | `ca97df9a…` | its Status: Attached Averam, Status Training, no ETA row; the picture over the red grid |
| `03-first-regiment-menu.png` | `52bdbe28…` | the travelling regiment's menu: Move, Confirmed Move and Scrap grey |
| `04-first-regiment-status.png` | `c65ca344…` | its Status: Attached Averam, Status Enroute, ETA Destination Day 135; the picture over the striped mark |

### The source

- `FUN_00445280` fills the rows. The Status word is "Enroute" (bit 4),
  "Awaiting Orders" (bit 2) or "Training". The ETA row shows only en route.
- `FUN_0042c3b0` draws the regiment's backdrop, STRATEGY 11514. Over it,
  en route, it draws 11516. Classes `0x10000002` and `0x10000008` take
  their own GOKRES mark instead: the portrait plus `0x1000`. An unfinished
  regiment takes 11571 for side 1, 11573 for side 2.
- A0 `04`'s picture is exactly 11514, GOKRES 5185 and the portrait 1089.
  A0 `02`'s is exactly 11514, 11571 and 1089.

### Port fixes

1. **The Defenses window's Trooper Regiments page left out a travelling
   regiment.** A0 `00` lists it at its destination. The page now adds the
   regiments moving to that planet's surface after the planet's own
   (`built_rows`). Their mini carries the en route mark (`row_marks`).
   `port:` A0 shows no case with both, so the order between the planet's
   own regiments and travelling ones is a guess.
   Test:
   `a_regiment_travelling_to_the_planet_lists_on_its_page_with_the_en_route_mark`.
   It fails without the change.
2. **The regiments' own GOKRES en route marks (5184..5252) were not
   keyed.** Their one key pixel showed blue in the Status picture; A0
   `04` shows the frame there. `bmp_cache` now keys them. Test:
   `a_regiments_own_en_route_mark_is_keyed`. It fails without the change.

`DefensesProduction` now carries the travelling regiments
(`TroopTransportState::transits`). The save format is unchanged.

### Fixture

`RegimentEnroute` (code 105) opens the primary system's Defenses window
on its Trooper Regiments page. It holds:

- an Alliance Army Regiment (Imperial Army for the Empire) sent there by
  Move from the third system;
- one of the same class in training there.

The page lists the travelling regiment first and the trainee second, as
in A0 `00`. The fixture now also reports the Status window's marks for a
regiment and for an unfinished unit. Test:
`the_regiment_enroute_scenario_moves_a_regiment_to_the_primary_system`.

### Browser (Linux Chromium, `status-window.mjs`)

**`regiment-en-route` (both sides):**

- the menu is 104 by 102, with A0 `03`'s rows and states: Move,
  Confirmed Move and Scrap grey; Encyclopedia and Status lit;
- the labels are A0's eight, and Status reads "Enroute";
- the marks are STRATEGY 11514, then GOKRES 5185 (Empire: 5250).

**`regiment-training` (both sides):**

- the labels are A0's seven, and Status reads "Training";
- the marks are STRATEGY 11514 and 11571 (Empire: 11573).

For both cases, on the Alliance side, against A0 `04` and `02`:

- the picture frame's interior differs from A0 in **0 of 12,740
  pixels**;
- the title and every label are within 1 pixel of A0's.

Run `status-window-2026-11-11T14-40-50-657Z-906974`: pass, 26 of 26 cases.
Test WASM
`a1fff5d6d3cb0364130f08803258f134a3ab31656d8cba1c4d829d152b53830e`.
After the run, some Rust lines were reformatted; nothing else changed.

### Still different from A0

- **The trainee's menu.** A0 `01` offers Move and Confirmed Move for a
  regiment in training. The port greys them; `build-selection.mjs` already
  records this for an unfinished squadron. Porting it means tracing what a
  Move does to a unit not yet built. The case therefore does not compare
  the menu's states.
- **The list cell's pixels.** A0 `00`'s list cell is not compared pixel
  for pixel. Only the unit test covers its marks.

### Mutants

The new lines of `defenses_window.rs` and `bmp_cache.rs`: 2 caught, 0
missed. `place_regiment_enroute`: 9 caught, 0 missed. One of them, the
rule that keeps the other side's regiments, was missed at first. The
fixture test now places a regiment of each side on the primary system.
The run again missed the two old `FleetLoadWatch::next` mutants, on lines
this change did not touch. The fixture's new `emit_status_rows` arms run only in the WASM
build, so the browser gate covers them.

## Cells with no A0 yet

### OBJ-03-C011: the Personnel Finder's dead row

There is no A0 capture of a dead character's row. The port follows
`FUN_00465bb0`: "Name - Killed" (`0x8798`), or "Retired" (`0x8799`) for
reasons `0x14..0x16`. Today only a unit test covers it.

The killed characters in A0 (`alliance-game2/jan-dodonna-killed`,
`empire-game3/jerjerrod-killed`) have no Personnel Finder capture after
their deaths. `health-2026-11-09/06` (day 839) shows only Captured rows.

**Capture needed:** in a game where an own character has died, press F5,
choose the own side's tab and the Characters view, and capture the dead
character's row. Then right-click it and capture its menu.

### OBJ-12-C009: a mission member captured

The message is already compared with A0 (EVT cells:
`alliance-game2/han-solo-captured`, `empire-game3/ozzel-captured`,
`messages-day633/capture-palpatine`). But those captures did not come from
a mission.

`health-2026-11-09/00-list-day244` (SHA-256
`f501674c45214170dceaaa0cdff684db07d0b713418e7bee81f85060c790aa6e`) does.
Chewbacca and Leia Organa went on Incite Uprising to Bortras. The list
shows "Chewbacca Captured", then "Incite Uprising Mission Foiled",
"Leia Organa Injured" and "Leia Organa Mission Foiled". None of these
messages was opened, so there is no text or picture to compare.

The Mission Status window (`FUN_00445c60`) has no state row. It shows
only Target, ETA Destination, Team Size and Decoys. Whether a captured
member still counts in Team Size is not traced.

**Captures needed (Alliance game):** send two characters on Incite
Uprising to a well-defended Imperial system. Then capture:

1. the mission's Mission Status before the capture;
2. the "<name> Captured" message, opened;
3. the same Mission Status after the capture, if the mission still runs;
4. the Message Index with the capture and the Mission Foiled rows.

### OBJ-12-C010: a mission member killed

The Killed message matches A0 (`jan-dodonna-killed`, `jerjerrod-killed`).
Neither death is known to have happened on a mission.

**Capture needed:** any "<name> Killed" message for a character who died
while on a mission, with that mission's window before and after. This
will happen only by chance.

### OBJ-12-C003: complete

Mission Status has no "complete" state (`FUN_00445c60`, see above). A
completed mission posts its report. Success and failure reports already
match A0 (OBJ-12-C004 and C005). The lead should decide whether the
existing report evidence settles this cell or whether it should be
excluded. No new capture is needed for the window itself.

The scenarios were renumbered at the merge onto main, after the other
agents' scenarios 100..102: SpecialForceEnroute is 103 (code 104) and
RegimentEnroute is 104 (code 105). The run ids above used codes 101 and
102 in the worktree.
