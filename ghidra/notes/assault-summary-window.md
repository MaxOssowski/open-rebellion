# The Assault Summary window (read 2026-10-21)

The manual: "Any time a fleet attempts to take over a system by planetary
assault, the Assault Summary window comes up (Fig. 3.66). NOTE: This
window also is available as a message when your opponent assaults one of
your systems. ... click on the Imperial Forces or Alliance Forces button.
That brings up a damage summary (Fig. 3.67). ... which troopers are still
operational and which were destroyed. The other tabs summarize damage for
capital ships, fighters, manufacturing and defensive facilities, and
personnel" (p. 123). Fig. 3.66 callouts: Assaulted system; Close window;
Summary; Imperial Forces; Alliance Forces; Description and outcome of
assault; Go to system.

The same window and report also serve the orbital bombardment summary.

## The report (message class, vtable `0x0065b870`)

Constructors `FUN_00496380` / `FUN_00496460` (0xa4 bytes, six unit lists),
destructor `FUN_00496530`, clone `FUN_00497490`. Slot `+0xc`
(`FUN_005a56b0`) returns `0x0f`: the message index's battle-report type,
which opens this window (`message-index-rows.md`). Slot `+0x10`
(`FUN_00497530`) is its arrival kind: 7 when the attacker (`+0x6c`) is the
viewer's side (`+0x20`), else 3.

| Field | Meaning | Set by |
|---|---|---|
| `+0x18` | the message id (Close marks it, `FUN_00469d60`) | base |
| `+0x28` | `0x2e` (bombardment) / `0x2f` (assault) when the attacker is not the viewer | notification 2 |
| `+0x2e` | the picture: assault `0x2b98` (Alliance attacker) / `0x2b99` (Empire); bombardment `0x2b9b`, `0x2b9c` or `0x2b9a` by which lists hold units | notification 2 |
| `+0x44` | the title (TEXTSTRA RCDATA) | notification 2 |
| `+0x50` | the description and outcome (RCDATA) | notification 2 |
| `+0x5c` | the system | `0x14f`, `0x150` |
| `+0x64` | 1 for an assault (`0x150`), 0 for a bombardment (`0x14f`) | `0x150` |
| `+0x68` | the system's side (0 neutral) | `0x14f`/`0x150` |
| `+0x6c` | the attacker's side (the fleet's `+0x24` bits 6..7) | `0x182`, `0x183` |
| `+0x70` | system forces at the start | `0x14f`, `0x150` (`FUN_00496f50`) |
| `+0x78` | attacker forces at the start, the fleet's ships and their cargo | `0x182`, `0x183` (`FUN_00496770`) |
| `+0x80` | system forces destroyed | `0x302` (`FUN_00497380`) |
| `+0x88` | attacker forces destroyed | `0x302` |
| `+0x90` | attacker personnel captured | `0x182`/`0x183` (family `0x30..0x3b`) |
| `+0x98` | system personnel captured | |

`0x302` (a unit gone) files the unit under `+0x80` when its location is a
system (`DatId >> 24` in `0x90..0x97`), else under `+0x88`.

Notification 2 (`FUN_00496770`) writes the text with `FUN_0060b9d0`
(TEXTSTRA RCDATA, `|` substitutions: `:4` a side's adjective, `:1` a
name):

| Case | Title | Body |
|---|---|---|
| assault, system had a side, it is now the attacker's | `0x7090` "Assault on {system}" | `0x7091` "{attacker} troops have taken control of the {defender} system {system}" |
| assault, system had a side, not taken | `0x7090` | `0x7092` "{defender} Troops have defended {system} from an {attacker} assault." |
| assault, neutral system, taken | `0x7090` | `0x7093` "{attacker} troops have seized control of the neutral system {system}." |
| assault, neutral system, not taken | `0x7090` | `0x7094` "The neutral system {system} has repulsed an attack by {attacker} troops." |
| bombardment, system had a side | `0x7098` "Orbital bombardment of {system}" | `0x7099` "{attacker} ships have conducted an orbital strike on the {defender} system of {system}" |
| bombardment, neutral system | `0x7098` | `0x709a` "{attacker} ships have conducted an orbital strike on the non-aligned system {system}" |

"Taken" is the system's side after the assault (`+0x24` bits 6..7) equal
to the attacker's.

## Opening

`FUN_0048a060` (the arrival dispatcher) on kind 7 calls `FUN_0041d7d0` ->
`FUN_0042a410`, which posts `0x468` kind `0x1c` to the galaxy window
(`FUN_00422ce0`): `FUN_0044c410(param_1[7], param_1, kind, report)`.
Kind 3 files the report as a message; opening it from the message index
(`FUN_00469de0`, type `0x0f`) posts the same `FUN_0042a410`.

## The window (`FUN_0044c410`, vtable `0x00659c78`)

470x331 (`0x1d6`x`0x14b`), the report at `+0x174`. `FUN_0044c630` builds
it; the frame and rail are the Battle Results window's (STRATEGY
`0x285f`/`0x2860` = 10335/10336, `0x2a44`/`0x2a45` = 10820/10821), and
the Close button and rail sit where they sit there (Close `0x1a7,0x19`
Alliance / `0x1aa,0x15` Empire, bitmaps 10370/10371, 10376/10377; rail
`0x1a2,0x58` / `0x1aa,0x59`).

Rail (command, label, bitmaps by the viewer's side):

| Command | Label | Bitmap |
|---|---|---|
| `0x65` | "Summary" `0x1944` | `0x29e8` / `0x29f2` |
| `0x66` | the attacker's forces, "Alliance Forces" `0x1937` or "Imperial Forces" `0x1938` | by `+0x6c`: `0x29ea` / `0x29ec` (Empire viewer `0x29f4` / `0x29f6`) |
| `0x67` | "System Forces" `0x1943` | by `+0x68`: `0x29ea`, `0x29ec`, neutral `0x29f0` (Empire viewer `0x29f4`, `0x29f6`, `0x29fa`) |
| `0x68` | "Display" `0x1939` | `0x2a27` / `0x2918` |

Pages (`FUN_0044e3d0`):
- `0x65` Summary: the picture `+0x2e`; the title `+0x44` at (12, 13),
  400 wide, centred, in the attacker's colour (`+0x6c` 1 red `0x20000ff`,
  2 green `0x200ff00`, else yellow); the body `+0x50` at (25, 225), 350
  wide, word-wrapped, in the same colour over a black shadow at (26, 226).
- `0x66`, `0x67` forces: a category bar at (36, 59) and a list at (37,
  136), 347x166. Categories (`FUN_0044e500`, families):
  `0x97` Capital Ships `0x1928` (`0x14..0x1c`), `0x98` Fighter Squadrons
  `0x1929` (`0x1c..0x20`), `0x99` Manufacturing `0x1835` (`0x28..0x30`),
  `0x9a` Defense Facilities `0x1946` (`0x22..0x28`), `0x9b` Trooper
  Regiments `0x1930` (`0x10..0x14`), `0x9c` Personnel `0x1931`
  (`0x30..0x40`). The first shown is Trooper Regiments after an assault,
  Capital Ships after a bombardment (`+0x180`). Columns: "Operational"
  `0x1932` (38, 117) and "Destroyed" `0x1933` (204, 117), 165 wide;
  Personnel "Survivors" `0x1934`, "Captured" `0x1935` (165), "Killed"
  `0x1936` (275), 110 wide. `0x66` reads `+0x78`, `+0x90`, `+0x88`;
  `0x67` reads `+0x70`, `+0x98`, `+0x80`. An empty first or middle column
  shows "None" (`0x8526`), an empty last column "No Casualties" `0x1941`.
- `0x68` Display: RCDATA `0x7013` "Select one of the following buttons to
  close this display and go directly to..." at (25, 225) and two buttons:
  `0xb` at (29, 182) 169x96, bitmaps `0x2a0c` / `0x2a0f` or `0x2a10`,
  labelled with the system's name (or "Goto System" `0x1945`); `0xc` at
  (226, 182), `0x2a0d`/`0x2a0f` (Alliance) or `0x2a0e`/`0x2a10`, labelled
  "Fleet" `0x1942`.

Commands (`FUN_0044df90`, keys `FUN_0044e180`):
- `10` Close, or Escape: closes the window and marks the message
  (`FUN_00469d60(+0x18)`).
- `0xb` / `0xc`: if the viewer knows the system (`FUN_004f2d10`), goes to
  it (`FUN_00429ce0`); `0xc` also opens it in mode 4 (`FUN_0045c8e0`);
  then closes.
- `100` a rail button, `0x96` a category button.
- Left and right arrows (`0x25`, `0x27`) step through the visible
  category buttons.

## Ported (2026-10-22)

- `rebellion-core` `assault_report.rs`: `AssaultReport::start` takes the
  forces before the assault, `finish` files the losses and reads whether
  the attacker holds the system (`economy::system_holder`), `body`,
  `picture` and `opens_at_once` follow `FUN_00496770` and `FUN_00497530`.
  `BombardEnv::assault` keeps one report per assault (`reports`).
- `rebellion-render` `assault_summary.rs`: the window, its four pages,
  six categories, columns, Display buttons, Close, Escape and the arrows.
- `rebellion-app`: the attacker's side sees the window at once; the other
  side gets a Conflict message (title, outcome, picture) whose display
  opens the window and closes the Message Index (`FUN_00469de0`).
  Close marks the message; `0xb` opens the system's sector window, `0xc`
  also its Fleet window.

port:
- The forces are taken from the port's world: a fleet's regiments are its
  cargo; squadrons are one row per squadron of each class.
- hyp: Operational lists the units that survive; the assault captures no
  personnel, so Captured is empty.
- The window is centred in the galaxy view; a row shows its GOKRES picture
  (the name when none is loaded), the name also as a hover text; the list
  scrolls with the wheel and draws no scroll bar; the RCDATA `0x7013`
  line sits above the Display buttons.
- The port's log line ("Planetary assault at ...") stays.
- The bombardment report (`0x14f`, `0x7098..0x709a`, pictures
  `0x2b9a..0x2b9c`) is not ported yet. A system's text for kinds 1, 2 and
  4 is its name (`FUN_004f6030`), so `0x7098` reads "Orbital bombardment
  of {system}". Its picture is `0x2b9b` when the system lost units
  (`+0x80`), else `0x2b9c` when the attacker did (`+0x88`), else `0x2b9a`.

Open: the exact y of each rail button; how `+0x90`/`+0x98` are filled for
system personnel; the row widget `FUN_0044edb0`; `+0x28`'s use.
