# Assault and bombardment presentation against A0

Date: 2026-11-11. Adds to TAC-08-C001, C003, C006, C007, C008 and
EVT-03-C001, C011. The other cells in this batch have no A0 capture yet;
see "A0 still needed".

## A0

All captures are in `.artifacts/a0-vm/` (git-ignored). No new capture was
taken for this work.

| Capture | SHA-256 | What it shows |
|---|---|---|
| `assault-clean/01-summary-yavin` | `e02f54bed5766efc28b8ae7797019ff801b148970255b75d5c01acdb8b304346` | The Alliance reads the Empire's "Assault on Yavin": "Imperial troops have taken control of the Alliance system Yavin", picture 0x2b99 |
| `assault-clean/02-attackers` | `3edfc71e707b6d11ebdf2d240fe43335160e318b50489dccdb180f4b5c422bef` | Imperial Forces, Trooper Regiments: Stormtrooper Regiments, a 38-pixel scroll thumb; Destroyed "No Casualties" |
| `assault-clean/03-system-forces` | `5880eddb99e39a6c9dd4c5cc0c6d892c7eac11be82503ae851834ce8b255cba6` | System Forces: Operational "None"; Destroyed one Alliance Army Regiment |
| `assault-clean/04-display` | `32f96b504393f05e8084abd3c6c7b9eab750a27cd19045142ccb74211fc83c6d` | The Display page |
| `assault-clean/07-system-button-closes` | `76f0bca0c7504d9350b48dc2ae2be9b85ec0f2ec5e908e3081c6bd002655ef14` | The system button closes the summary |
| `menus/13-own-bombard-sub` | `bb56a0004ab349ed4cf54919e690f2c66a42576d19a220660214168c46e4753e` | Fleet 1's menu over its own Kimanan (day 316): Planetary Assault grey; the submenu's four rows, Destroy System grey |
| `menus3/29-assault-2` | `c9cd91b243e74fdac4ee18826250abda29fd40f6665da6f29a368d06056bedde` | The Empire's own Planetary Assault on Chandrila: the summary opens at once, the Empire's frame at (125, 52) |
| `menus/15-own-bombard-result` | `f2a463469df10ea05b6961f9667b52e0d01402bfab2c88916dffcf99dd369bb4` | Target Military Facilities carried out at once: "Orbital bombardment of Kimanan", picture 0x2b9a, the rail with two Alliance buttons |

`assault-clean` is an unedited Alliance game (day 720). `menus` is the
same Alliance game the `a0-kimanan` case uses (day 316). `menus3` is the
Empire save of 2026-10-24 (one ship retyped to the Death Star; menus and
orders unchanged).

## Port changes

**The Display page.** `FUN_0044c630` draws the title again on page 5, then
RCDATA `0x7013` in font 6, 350 wide, word-broken (`0x110`), in the
attacker's colour, measured at (25, 225) and moved to (29, 177 less its
height). The port drew no title and one 11-pixel line at (25, 140). A0
`04-display` shows the title and two lines ending just above the
buttons. hyp: the height term is the measured top less its bottom.
`display_goes_to_the_system_and_closes` checks the title and both lines'
pens. It fails without the change.

**A row's name font.** `FUN_0044edb0` sets a row's name item to font 10
(`+0x38 = 10`), centred (`+0x50 = 1`), its top at the picture's bottom
(`+0x34`). The port drew it in an untraced 9-pixel font, so A0's
"Alliance Army Regiment" was 112 pixels wide and the port's 91.
`assault_summary.rs` now draws the name with `paint_gdi_text` in font 10.
`the_rail_shows_each_sides_forces_with_its_columns` checks the font and
the pen. It fails without the change (it was the old 9-pixel
expectation, which `FUN_0044edb0` contradicts).

**Fixtures (test-only).**
- New scenario `EnemyAssaultTaken` (appended, code 106). Yavin is the
  Alliance's with one Alliance Army Regiment. An Imperial fleet with nine
  Stormtrooper Regiments takes it through the real `BombardEnv::assault`
  (`FUN_0058c120`). The fixture takes the first seed whose draws give A0's
  outcome: the defender destroyed, no attacker lost, the system taken.
  The app files the report on the Conflict rail with
  `present_assault_reports` (kind 3) and opens it as the message index's
  Display does (`FUN_00469de0`).
  `the_enemy_assault_scenario_takes_yavin_from_its_one_regiment_as_a0_shows`
  checks the report.
- `FleetWindowA0` (Kimanan) now reports its object menu and its open
  Assault Summary. The object-menu report names the submenu's Target
  Military and Target Civilian rows.

## Browser (Linux Chromium)

`fleet-window.mjs`, two new Alliance cases:

**`enemy-assault-taken`** (against `assault-clean`):
- Summary: the exact title "Assault on Yavin" and body "Imperial troops
  have taken control of the Alliance system Yavin"; Yavin's holder is the
  Empire afterwards. The picture, the rail and the frame's top corners
  are 0 pixels from A0. The title's and the body's ink are at A0's place
  (within 2 px).
- Imperial Forces: nine Stormtrooper Regiments under Operational, none
  Destroyed. The category bar, the scroll bar (A0's 38-pixel thumb) and
  the first two rows' pictures are 0 pixels from A0. The subtitle,
  category label, column heads, the rows' names and "No Casualties" are
  at A0's place.
- System Forces: Operational empty ("None"), Destroyed one Alliance Army
  Regiment. The bar, the empty scroll bar and the destroyed regiment's
  picture are 0 pixels from A0; the labels and the name are at A0's
  place.
- Display: everything above the buttons' labels, and the rail, 0 pixels
  from A0 `04-display`. The system button closes the summary (A0 `07`).

**`assault-taken` (Empire), extended** (against `menus3/29-assault-2`,
the Empire's own view): the picture (less A0's cursor), the frame's top,
and the rail's Close and four page buttons are 0 pixels from A0. The rail
metal between the buttons (STRATEGY 10821) is one palette step off in
A0's Empire game, as are the buttons' keyed left corners where it shows
through; that is counted (`own_a0_rail_metal`), not asserted. It is the
accepted palette-step deviation. The port's pixels are the bitmap's.

**`a0-kimanan-bombard`** (against `menus/13`, `menus/15`):
- Fleet 1's menu rows and states are A0's: Move, Confirmed Move,
  Planetary Bombardment, Planetary Assault (grey), Rename, Encyclopedia,
  Status, Scrap.
- The submenu: Target Military Facilities, Target Civilian Facilities,
  General Bombardment, Destroy System (grey).
- Target Military Facilities runs at once and opens the summary: the
  exact title and body, picture 0x2b9a. The picture (less A0's cursor),
  the rail and the corners are 0 pixels from A0; the red title and body
  at A0's place.

## Runs

- `fleet-window.mjs --no-build` in full (Linux Chromium, both sides):
  88 of 88 passed, run `fleet-window-2026-11-11T16-59-47-032Z-1495672`
  (test WASM sha256 `17cd88d4da4221c1e440a18ce46ebe2e4eafade010a829d01bfd7c382ecd02dd`).
- Checks recorded there: `enemy-assault-taken` Display `above_labels` 0,
  rail 0; `a0-kimanan-bombard` picture 0, rail 0, corners 0; Empire
  `assault-taken` `own_a0` picture 0, rail buttons 0, frame top 0, rail
  metal 5650 (palette step, not asserted).

## Tests and mutants

- `cargo test -p rebellion-render` and `-p rebellion-app`: pass.
- `provenance-scan check`: 0 uncited items.
- `assault_summary.rs` (in-diff): 5 mutants, 5 caught after the name's
  rectangle lost its unused height (the 2 earlier survivors changed only
  that height, which a top-aligned line ignores).
- `interface_test_fixture.rs --re 'enemy_assault|reports_object_menu'`:
  22 mutants, 16 caught, 1 unviable, 5 missed. The missed ones turn `&&`
  into `||` in the fixture's filters (the Imperial Star Destroyer class,
  the Empire's fleets at Yavin, the seed's outcome test). The synthetic
  test world has one such class, one fleet, and its first seed gives A0's
  outcome, so they are equivalent there. The browser run checks the real
  outcome.
- `EnemyAssaultWatch::observe` is the WASM bridge; the browser gate
  covers it.

## Proposed status

| Cell | Proposal | Why |
|---|---|---|
| TAC-08-C001 assault action | pass | A0 `menus3/27..29`: the assault runs at once and opens the summary; the Empire's `assault-taken` does the same and matches A0's frame, rail buttons and picture. The menu in A0 is the Fleet window's; the port's case uses the sector icon's menu. |
| TAC-08-C003 force details | pass | The Empire's force rows and the destroyed defender are now compared with A0 `02`, `03` in the Alliance's view; names in A0's font. |
| TAC-08-C006 casualties | pass | System Forces' destroyed Alliance Army Regiment and Imperial Forces' "No Casualties" match A0 `02`, `03`. |
| TAC-08-C007 occupation | pass | The real assault takes Yavin (holder Controlled(Empire)) and the summary reads A0's text exactly. |
| TAC-08-C008 return | pass | Display matches A0 `04` and the system button closes the summary as A0 `07`; Escape closes it in the Kimanan case. |
| EVT-03-C001 bombard military | pass | A0 `menus/13`, `15`: the submenu, the strike at once and its summary match. Only a strike on the side's own system with no losses is compared. |
| EVT-03-C011 destroyed | pass | The Destroyed column against A0 `03`. |
| TAC-08-C005, EVT-03-C006, EVT-03-C009 | stay partial | No A0 of a failed assault. |
| EVT-03-C002 bombard civilian | stays partial | No A0 of Target Civilian Facilities carried out. |

## A0 still needed

All from the Alliance game in the VM (day ~212). Take each with the window
fully on screen, the cursor off the window.

1. **Failed assault (TAC-08-C005, EVT-03-C006, EVT-03-C009).** An
   Alliance fleet carrying one Alliance Army Regiment, in orbit over an
   Imperial system held by two or more Imperial regiments (ideally no
   defense facility). Right-click the fleet's entry in the system's Fleet
   window, capture the menu (Planetary Assault enabled), choose Planetary
   Assault. Capture: the Summary page (expect "...repulsed..." text,
   0x7092), Alliance Forces (Trooper Regiments; the lost regiment under
   Destroyed if it dies), System Forces (Trooper Regiments), and Display.
   If the regiment survives the duels it may stay Operational; capture
   whatever comes.
2. **Target Civilian Facilities (EVT-03-C002).** An Alliance fleet with a
   bombarding ship (e.g. a Corellian Corvette, as Kimanan's Fleet 1) over
   an Imperial system that has mines, refineries or yards. Menu, Planetary
   Bombardment, capture the submenu, choose Target Civilian Facilities.
   Capture the Summary, then System Forces on Manufacturing and on
   Defense Facilities, and Capital Ships on the Alliance Forces page.
3. **Target Military Facilities on an enemy system (EVT-03-C001, the
   losses case).** As 2 over an Imperial system with regiments or
   batteries; choose Target Military Facilities. Capture the Summary
   (picture 0x2b9b when the system lost units) and System Forces on
   Trooper Regiments and Defense Facilities.
4. **Escape on an Assault Summary (TAC-08-C008).** Open any Assault
   Summary from the Conflict list, press Escape, capture the screen after.
