# Failed assault, civilian bombardment and the render rate against A0 (2026-11-14)

For EVT-03-C002, EVT-03-C006, EVT-03-C009, TAC-08-C005 and TAC-03-C004.
It follows `2026-11-11-assault-and-bombardment` ("A0 still needed" 1, 2)
and `2026-11-13-fireball` ("Still open"). The ledger JSON is not changed.

## A0

Original REBEXE (`b3fe3997…`), offline Windows 10 QEMU VM, 2026-11-14
03:30..05:00. A new Alliance game (the day-711 game of 2026-11-14 was left
through Game Options first). Captures are in `.artifacts/a0-vm/` (git-ignored):
`2026-11-14-bombard-assault` (44 PNGs) and `2026-11-14-fireball-rate`, each
with a README and SHA256SUMS. The ones cited here:

| Capture | SHA-256 | What it shows |
|---|---|---|
| `bombard-assault/09` | `a50cd7f29761d491859accbbbe6f38fab2cd8656f010c97e6aa638f145a071f5` | The Alliance's own Planetary Assault on Imperial Commenor (one Alliance Fleet Regiment against five Imperial Army Regiments, no shield, no battery) opens its summary at once: "Assault on Commenor", "Imperial Troops have defended Commenor from an Alliance assault." (0x7092), picture 0x2b98 |
| `bombard-assault/10` | `6bfe89813a582883b997788dc0017c6cf285c9397d4aee7bc6c56b90049d3455` | Alliance Forces, Trooper Regiments: Operational "None", Destroyed one Alliance Fleet Regiment |
| `bombard-assault/11` | `7be28c58a15ac6011c63dc306caa17bda6382a3a1338c9815193fb842b79fb27` | System Forces: the Imperial Army Regiments Operational, "No Casualties" |
| `bombard-assault/12` | `23645a556a424a2742db67e56e679cfb75d8068deea0887d77114099ed1c0a29` | Display |
| `bombard-assault/17` | `9ff7a362d325fb5a2437c5605029edc1fc8b398478a075dc5f40d11897f156da` | Target Civilian Facilities on Commenor, nothing destroyed: "Orbital bombardment of Commenor", "Alliance ships have conducted an orbital strike on the Imperial system of Commenor" (0x7099), picture 0x2b9a |
| `bombard-assault/24` | `8e9de5789e224b193690c1b464876b528ba10e6f5d4a233acc695bee1f70f8f8` | the second Target Civilian Facilities: the same texts, picture 0x2b9b |
| `bombard-assault/26` | `0423219ec2fa0623b0e99abc4a58b555b5fcbe2da7cb1eaac48a58e4e25d526a` | its System Forces, Manufacturing: one Refinery destroyed |
| `bombard-assault/15` | `8795d52fad0314c6c6240f0efa9ac79fe88578fa41cc0f2f7cd88fa0ca384192` | the Corellian sector before (day 172): Xyquine, Drall, Selonia, Talus, Corfai neutral |
| `bombard-assault/40` | `9745c6605e3dd48bc5bb10e75aad6835735817df438e997562e3f0cd7477e9b2` | the sector after (day 266): Xyquine, Drall, Corellia, Selonia, Talus, Corfai, Tralus Imperial; Popular Support Messages 29..36 "X Joins Enemy" from day 218 |
| `bombard-assault/41` | `b87f66bf175ee4b9c410c31e6c6faebeb2f4376f10c4bc166964462426aefa9e` | All Messages: the own Assault on Commenor and both Orbital bombardments of Commenor are filed (grey, read) |
| `bombard-assault/42` | `bcc2a88b5b64a6f2aad9840715e7386d9d3989f3e93fc3529b1df3a01716419c` | Conflict Messages: the same three |
| `bombard-assault/43` | `e6d8ba94f05e4e0573e619af473baa0a065c1bca944d703560c4157212a9d19f` | Display Message on the filed assault reopens its summary |

The pictures were matched against STRATEGY 11160..11164 by sampling every
seventh pixel at the window's picture place: 09 = 0x2b98 (2435 of 2610), 17 =
0x2b9a (2428), 24 = 0x2b9b (2428); every other id under 250.

## What changed in the port

**The sector's support after civilian damage (simulation).** `FUN_0058e320`
follows civilian damage on a system it did not destroy with
`FUN_0055d900(side, 1)` and `(side, 0)` and `FUN_00518ac0` -> `FUN_00518b20`
-> `FUN_00518be0`: every populated system of the bombarded system's sector
gets an event 0x373 of SDPRTB `0x140a` (family `0x90..0x92`; Alliance -4,
Empire -3) or `0x1408` (`0x92..0x94`; -2, -1), below the limits `0x140b` /
`0x1409` (101). The amount is the Alliance's (`Event373_Fire` ->
`FUN_0050c9f0` side 1), negated for an Imperial attacker. The port applied
only G`0x1e19` at the bombarded system. A0 15/24/40: the strike that
destroyed a refinery was followed by seven Corellian systems joining the
Empire within twelve days. `bombardment::sector_support_event`;
`ghidra/notes/bombardment-order.md` "The sector event". port: applied at
once, not queued; the exploration status stands for the SYSTEMSD family.
hyp: `+0x88` bit 3 is a destroyed planet.
- `civilian_damage_moves_the_support_of_every_system_in_the_sector` and
  `the_sector_event_skips_a_system_where_the_attacker_holds_the_limit`;
  the first failed without the change (core system 50/50, expected 53/47).
- `military_targeting_risks_the_first_civilian_facility` expected 30/70 at
  the bombarded system, which left the sector event out; it is 27/73 now.

**The own report is filed (interface).** `FUN_0048a060` kind 7 (the
attacker is the viewer, `FUN_00497530`) calls `FUN_005f5440`, which files
the report, then `FUN_0041d7d0`, which opens it; the rail mask, its light,
the advisor and `FUN_0048a2a0` are kind 3's. The port opened the summary
and filed nothing. A0 41..43 list the own assault and both bombardments in
All and Conflict Messages, read, and Display reopens them.
`present_assault_reports` now files the report (read, so no rail lights)
and opens it with the message id.
`an_own_assault_opens_at_once_and_is_filed_in_the_conflict_messages`
failed without the change (no message).
Open: A0 09/12 show the Conflict rail dark with the assault's summary up,
17/24 lit with the bombardments'. The port lights nothing for a kind 7
report; when the original's rebuild (`FUN_0048a2a0`, on the next kind 3
message) relights an unread own report is not traced.

**Fixture and gate (test-only).** Scenario `OwnAssaultRepulsed` (121, code
122): Commenor Imperial with five Imperial Army Regiments and no defense;
the Alliance's Fleet 1, a Medium Transport with one Alliance Fleet
Regiment. The fixture runs the real assault (`BombardEnv::assault`,
`FUN_0058c120`) with the first seed that gives A0's outcome (the attacker's
regiment lost, no defender, the system kept) and presents the report as
the app does. `the_own_assault_scenario_is_repulsed_from_commenor_as_a0_shows`.
The fleet-load observation's `last_assault_message` now skips filed
reports (it reads the port's log line).

## Browser (Linux, Arch Chromium 155, not the pinned build)

`fleet-window.mjs`, new Alliance case `own-assault-repulsed`, against A0
09..12 (window at (62, 49)):
- Summary: the exact title "Assault on Commenor" and body "Imperial Troops
  have defended Commenor from an Alliance assault."; Commenor stays
  `Controlled(Empire)`. Picture 11160, rail and frame corners 0 pixels from
  A0. The red title's ink (A0 110..313, port 112..312) and the body's
  (A0 26..354 x 228..257, port 26..362 x 228..258) at A0's place.
- Alliance Forces: Operational empty, Destroyed one "Alliance Fleet
  Regiment"; category bar, scroll bar and the destroyed row's picture 0
  pixels from A0; labels at A0's place.
- System Forces: five "Imperial Army Regiment" Operational, none
  Destroyed; bar, scroll bar (A0's thumb over five rows) and the first two
  rows' pictures 0 pixels from A0; labels at A0's place.
- Display: everything above the buttons' labels and the rail 0 pixels
  from A0 12; the title and the 0x7013 line at A0's place. The system
  button closes the summary.

Runs: `fleet-window-2026-11-14T03-34-51-021Z-1605582` (the new case
alone) and `fleet-window-2026-11-14T03-38-03-915Z-1613917` (11 assault and
bombardment cases of both sides, all pass; test WASM sha256
`8e003ed5f6f79f640698e21384a409022b8f69665ce99b2e43de41ffae5eaa70`).
Then `fleet-window.mjs --no-build` in full: 105 of 105 passed, run
`fleet-window-2026-11-14T03-39-26-659Z-1619488` (the same WASM).

Checks: `cargo test -p rebellion-core`, `-p rebellion-app` (with and without
`interface-test-fixtures`) pass; `provenance-scan check` 0 uncited items;
rustfmt (edition 2021) clean on the touched files; no clippy finding in the
touched lines. Mutants (`--in-place`): `bombardment.rs`
`--re sector_support_event`, 18 of 18 caught (the first run missed `<` to
`<=` on the limit and `&&` to `||` before the limit test was added);
`main.rs --re present_assault_reports`, 5 caught, 1 unviable, and 5 missed
field deletions in `window_conf` and `draw_fullscreen_texture`, which `--re`
does not filter and this change does not touch.

`fleet-window.mjs`, new Alliance case `bombardment-civilian` (the
`FleetBombardment` fixture: the player's fleet over an Imperial system with
two Imperial regiments, no civilian facility): the submenu's Target
Civilian Facilities row (1) carries the strike out at once; the summary
reads "Orbital bombardment of Akrit'tar" and "Alliance ships have
conducted an orbital strike on the Imperial system of Akrit'tar" (A0 17's
0x7099 wording with the system's name), picture 0x2b9a; the picture, the
rail and the frame corners are 0 pixels from A0 17; the red title and
body begin at A0's rows and column. Run
`fleet-window-2026-11-14T04-06-30-437Z-1687499` (pass, the same WASM).
The 0x2b9b picture (a lost unit, A0 24) is unit-tested
(`AssaultReport::picture`), not driven in the browser.

## TAC-03-C004: the render rate

`2026-11-14-fireball-rate` holds two capture series (QMP screendumps over
one connection, about 100 a second, a capture kept only when the battle
view changed):
- Battle at Commenor (150 s, camera still, ships moving): at most 50 view
  changes in 1.0 s, 98 in 2.0 s, 235 in 5.0 s; mostly 40..49 per second.
  The VM renders at about `FUN_005c1cf0`'s 50-per-second cap: a render
  period of 20..25 ms, as `ORIGINAL_TACTICAL_RENDER_SECONDS` assumes.
- Battle at Yaga (old game): Alliance Escort Carrier 3 dies far from the
  camera (yaga-00796..00808). The panel turns to the emblem at 39.10 s
  (the hull still drawn, one fire-palette pixel), the hull is gone and an
  11 x 6 fireball shows by 39.20 s (best fit TACTICAL 3368, weak), and no
  fire is left by 39.26 s. From the panel change the fire lasted at most
  0.17 s.

At a 20..25 ms period the port's model (`FUN_005d41a0` once per render:
swaps at 0, 5, 10, 15; the hull goes at call 12; the effect at 16) runs
0.32..0.4 s, the hull leaving 0.24 s in. The Yaga series is shorter, which
fits two `FUN_005d41a0` calls per render (8 renders, the hull at render 6,
frames 3360 then 3370) or an effect that began before the panel changed.
The series does not settle which; the port is not changed. Still open: one
more death near the camera, with the selection on the dying ship, timed
from its last hit.
