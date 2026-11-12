# Tactical states against A0: retreat and withdrawal (2026-11-12)

This adds to EVT-02-C005 and TAC-07-C004. It also lists what A0 shows for
TAC-04-C014, TAC-07-C014, TAC-01-C013 and TAC-01-C008, and which captures
or browser cases those cells still need. The ledger JSON is not changed.

## A0 used

All captures are in the main checkout's `.artifacts/` (git-ignored). No new
capture was taken for this work.

| Capture | SHA-256 | What it shows |
| --- | --- | --- |
| `a0-vm/message-rail/retreat-alert-commenor` | `8b1240e5e0c5e0ab2259142fe4d19e357004945f39c272fdae145211b8be5d08` | The Alliance's Battle Alert at Commenor, before Retreat |
| `a0-vm/message-rail/retreat-result-commenor` | `5a2fb990671a5ac152c6c4a1dd4f2e7bfabf67fedfad1af5f5508f6d932bd3f6` | After Retreat: "The Alliance fleet is defeated." / "Commenor is now under blockade by Imperial forces. The Alliance fleet has withdrawn to Bortras." over scene 10757 |
| `a0-vm/battle-alert-empire/04-result` | `9da38d26599180ddb06e793673bfa3224b96e845f08d5f6c9876c0a0e0174631` | The Empire's own withdrawal (after Simulate Results): "The Imperial fleet is defeated." / "Tralus is now under blockade by Alliance forces. The Imperial fleet has withdrawn to Coruscant." over scene 10758 |
| `interface-parity/a0/TAC-07-C004/original` | `5f9c6dada1dcf5a5ea46ebe299b874806c9dc6061e3d71c0ffa97b0dfc553966` | The Alliance's Withdraw From Battle at Coruscant (tactical): "The Alliance attack on Coruscant has failed. The Alliance fleet has withdrawn to Averam." over scene 10757 |
| `interface-parity/a0/TAC-04-C014/original` | `8f2c212e346124ac51f16c8893e82886c36a91f2cd3363d4943c2bc2d683ab0a` | F1 chase, Battle at Coruscant, paused |
| `interface-parity/a0/TAC-07-C014/original` | `344669c2f1b8cdc970b3499b8c0d00158a7780900b62f698dc2d9e907cd1c03e` | The Sluis sector window after the Battle at Mon Calamari |
| `interface-parity/a0/TAC-01-C013/original` | `cd4eda7b3b4f45406eef7a8f9f2bbf95606fa39d8a04a85e23edd7a83bc77b21` | The Battle Summary over the strategy screen after the last Alliance ship died |
| `interface-parity/a0/TAC-01-C008/original` | `d0e829ffa18b40ce1ee72d4b9fcd70fefb4255507ae52d2892a722122f6879be` | Umgul's planet at the top right of the holocube |

## The port's fault: a simulated withdrawal read as destroyed

Retreat and Simulate Results run the abstract battle. A side that leaves
there keeps its hulls whole (`FUN_005af910`, state 9). But
`present_auto_resolve_result` never marked those hulls as gone. So the
report found no withdrawn side. It read "The Alliance fleet has been
completely destroyed." (`0x700b`) and showed the enemy's victory scene,
though the fleet had left.

In the original, the report's withdrawn side (`+0x54`) is the system's
`+0x78` bits 2..3 (`FUN_0049b060`). It is set once, when a fleet or
squadron whose side is not the battle's winner (`FUN_00509ad0`) leaves
the system (`FUN_0050c6c0` -> `FUN_0050a0b0`). That test also reads the
system's `+0x88` bit `0x1000` and the unit's `+0x50` bits `0x10` and
`0x40`; their meaning is not traced.
`FUN_00516ef0` sends the loser's fleets away at the battle's end. The
abstract battle ends a side only when it has no active unit
(`FUN_005ae9c0`, words 1 and 3). So a loser's hull that is still whole
escaped.

The fix: `present_auto_resolve_result` marks the loser's live hulls as
withdrawn. A draw marks none.

- New test `a_simulated_retreat_whose_fleet_escaped_reports_its_withdrawal`
  (`tactical_view.rs`). It fails without the fix (the report's withdrawn
  side is `None`). It checks `0x7003`, `0x7008`, `0x700d` word for word
  against the Commenor capture, and scene 10757.
- New test `a_simulated_battle_whose_loser_died_reports_no_withdrawal`:
  a dead loser still reads as destroyed (A0 TAC-01-C013).

The 2026-11-05 note on EVT-02-C005 said the fixture's Imperial fleet died
before it could leave. The fault above gives the same text either way, so
that reading is not proven.

## The browser case

New scenario `production-battle-alert-retreat` in `run.mjs --battle`
(tactical fixture 39, codes 65831 and 66087, appended at the end of the
catalog). The viewer holds the system and the enemy arrived last. The
viewer has one hull with a hyperdrive and no fighters. The enemy has four
hulls. A quarter of the enemy's strength is under the leave threshold
0.33 (`FUN_005d19a0`), so the fleet leaves at once.

The probe presses Retreat (`0x67`) and requires:

- Retreat enabled: art 10974 (Alliance) or 10971 (Empire), every pixel;
- the log `[battle_alert] command=retreat … status=simulated`, and no
  tactical battle;
- the report's words, logged by the app (`[tactical-results] summary
  lines=…`): "The {viewer} fleet is defeated." and "{system} is now under
  blockade by {enemy} forces. The {viewer} fleet has withdrawn to
  {another system}.";
- at 640 x 480, the withdrawal scene (10757 or 10758) exact against its
  BMP, outside the text boxes;
- at 640 x 480, the scene's pixels against A0, outside the text boxes
  and A0's pointer: 0 differ.

| Side | Port text | A0 compared | Pixels checked | Differ |
| --- | --- | --- | --- | --- |
| Alliance | "Akrit'tar is now under blockade by Imperial forces. The Alliance fleet has withdrawn to Ithor." | `retreat-result-commenor` | 66,913 | 0 |
| Alliance | same | `TAC-07-C004/original` | 67,113 | 0 |
| Empire | "F'tral is now under blockade by Alliance forces. The Imperial fleet has withdrawn to Coruscant." | `battle-alert-empire/04-result` | 67,280 | 0 |

The names differ from A0's, as the fixture's systems differ. The text's
glyphs are the accepted deviation (Liberation Sans for GDI's Arial).

## The tactical withdrawal played out (TAC-07-C004)

A0 TAC-07-C004 is the Alliance's Withdraw From Battle at the Empire's
Coruscant: Battle Options, Withdraw, confirm, close, unpause, and about
44 s later the battle ends on its own.

New scenario `production-battle-withdrawal-report` (tactical fixture 40,
codes 65832 and 66088). One hull with a hyperdrive a side, no fighters, at
the enemy's system, which the viewer reached last. The probe clicks Battle
Options (618, 319), Withdraw (521, 64), the confirmation's 1113
(561.5, 242.5), the panel's close (593.5, 228.5) and the pause control
(575, 319). The app logs `withdrawal_started`. The hull jumps out, the
battle ends when the viewer's side has no unit left (`check_battle_end`),
and the Battle Summary opens. The probe requires:

- the report's words: "The {viewer} fleet is defeated." and "The {viewer}
  attack on {system} has failed. The {viewer} fleet has withdrawn to
  {another system}." (`0x7003`, `0x7009`, `0x700d`, A0 TAC-07-C004's
  sentences);
- at 640 x 480, the withdrawal scene exact against its BMP;
- for the Alliance at 640 x 480, the scene against A0 TAC-07-C004 outside
  the text boxes: 67,113 pixels, 0 differ.

Port text: "The Alliance attack on Akrit'tar has failed. The Alliance fleet
has withdrawn to Ithor." and "The Imperial attack on F'tral has failed. The
Imperial fleet has withdrawn to Coruscant." The Alliance's lines wrap after
"The" as A0's do.

The app now logs the report's lines once per battle, when the summary
first has its result, simulated or played (`TacticalState::take_summary_log`).

## What each cell still needs

**EVT-02-C005 (retreat).** The route, the report's words and the scene now
match A0 on both sides. Proposed: pass.

**TAC-07-C004 (retreat).** The tactical Withdraw From Battle now runs in
the browser until the battle ends. The report's sentences match A0's, and
the scene matches A0 TAC-07-C004 with 0 pixels differing. Proposed: pass.
Not compared: the time the jump takes (A0 about 44 s from unpause), and
the strategy screen under the window (the fixture opens the battle
directly, so the shell bitmap shows there).

**TAC-04-C014 (chase camera).** A0 is the Battle at Coruscant (an Escort
Carrier with X-wing and Y-wing squadrons against the home fleet), paused,
after F1: the nav pyramids fill the foreground. No port battle has that
composition, so the 3D view cannot be compared. Needed: either a fixture
that rebuilds A0's battle (its input trace is in
`.artifacts/interface-parity/a0/manifest.json`), or a new A0 capture of F1
in a battle the port can rebuild, such as A0 TAC-01-C003's (one Medium
Transport against one Victory Destroyer at Mon Calamari, fixture 37):
press F1 on the paused battle and capture with the cursor off the view.

**TAC-07-C014 (strategic state persistence).** A0 shows the Sluis sector
window after the Battle at Mon Calamari. Its manifest notes Mon Calamari
as Imperial, an Imperial fleet icon and a battle mark, and no Alliance
fleet. No browser
case opens a sector window after a battle. Needed: a browser case that
ends a fixture battle and clicks the results' system button, then
compares the sector window. No new A0 is needed.

**TAC-01-C013 (return to strategy).** As in
`2026-11-11-hq-and-tactical-states`: the words match in a unit test; no
browser case ends a battle by the player's last ship over the live
strategy screen. No new A0 is needed.

**TAC-01-C008 (planet backdrop).** A0 is the Empire's Battle at Umgul
after Rotate Left 16 times. The planet's framing depends on the camera
path. Needed: a browser case that presses Rotate Left 16 times on a
fixture at Umgul, or a new A0 capture of the first paused frame (no
rotation) at a system the port can rebuild (Mon Calamari, fixture 37).

## Gate runs (Linux, Arch Chromium)

Run ids are UTC.

- `2026-11-11T23-54-57-436Z-3116789`: `run.mjs --battle --all`, all 160
  executions pass (40 scenarios, both sides, both viewports). It holds
  both new scenarios.
- `2026-11-11T23-47-41-118Z-3019205`: `--scenario=production-battle-withdrawal-report`, 4 of 4.
- `2026-11-11T23-42-23-298Z-3014002`: `--scenario=production-battle-alert-retreat`, 4 of 4.
- An earlier full run, `2026-11-11T23-27-41-350Z-2979025`, failed 6 of 156
  on the score (MDATA.307/308/313/314 `loaded=false`): the worktree had no
  staged music (`data/sounds`) or videos (`web/data/video`). That was the
  environment; with both staged the runs above pass.

In `2026-11-11T23-41-59-134Z-3013151`, the alert-entry scenario's Empire
letterboxed Retreat read "The Imperial fleet is victorious." at F'tral:
with one hull and five squadrons a side, the Alliance's fleet did not fall
under the leave threshold. Its text is not asserted.

## Checks

- `cargo test -p rebellion-render -p rebellion-app`: pass.
- `cargo run -q -p provenance-scan -- check`: 0 uncited items.
- `cargo mutants --in-place -p rebellion-render --file
  crates/rebellion-render/src/tactical_view.rs --re
  'TacticalState::(present_auto_resolve_result|battle_summary_lines|take_summary_log)'`
  (two runs): every mutant of the new lines is caught. Three older lines of
  `present_auto_resolve_result` survive: the fleet and system guard
  (`||` to `&&`, twice; every test session uses the default keys) and the
  fighter filter (`&&` to `||`; the tests give each side one fighter
  entry). The tool also listed field-deletion mutants of drawing
  functions outside the regex; they are not part of this change.
