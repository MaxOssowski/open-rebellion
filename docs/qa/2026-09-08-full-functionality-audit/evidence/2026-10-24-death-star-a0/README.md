# The Death Star, squadron colours and battle audio from the original (P24)

Date: 2026-10-24.

## Source

The last 12 A0 cells of the tactical catalog were captured from the original
`REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`), in the same Windows 10 22H2
QEMU/KVM guest as the earlier 94 cells (QMP `screendump`, guest display mode
640x480). The A0 store now holds 106 of 106 cells. The captures, the audio
and the edited saves stay in the ignored `.artifacts/interface-parity/a0/`
store and the VM folder. Only cell IDs and hashes are cited here.

### Editor-assisted saves

A Death Star appears only late in a campaign, so the user asked for saves
that already have one (2026-10-24). The cells are tagged
`editor-assisted-save`. The executable is unchanged.

- The original save is uncompressed. Every object record starts with its own
  file offset, then its family, serial, owner and type as 32-bit words. A
  child refers to its parent as `family << 24 | serial`.
- The Death Star is CAPSHPSD record 136, family `0x18`. Every other capital
  ship has family `0x14` (`death-star.rs`, F-025). Changing one Imperial
  capital ship's records from `(0x14, type)` to `(0x18, 136)`, together with
  its children's references, gives a Death Star that the game loads, shows
  (Death Star picture, upkeep -403) and saves again by itself.
- Empire cells: slot 4 (Empire, day 131), Imperial Star Destroyer 1 at
  Averam. The game re-saved it to slot 5, and the battle came at Mon
  Calamari, day 222 (and again on a rerun, day 220).
- Alliance cells: slot 1 (Alliance, day 2), Judicator at Coruscant.
  - Fleet 2's X-wing and Y-wing groups were retyped to A-wing and B-wing
    for the green and gold groups.
  - `FUN_005ad930` enables Attack Death Star only with fewer Imperial than
    Alliance fighter groups. Coruscant's 16 TIE squadrons make four
    Imperial groups (`FUN_005ae460` splits a colour of more than seven).
    The 11 squadrons on the planet were therefore given to the Alliance,
    leaving the Empire one group.

## Findings and changes

- **Laser control (TAC-05-C017).** 1021 matches all 381 opaque pixels at
  (611, 2), the position that `FUN_005c4ed0` gives it (`0x89, 2` in the
  gauge window). The port drew it at (613, 2).
- **Laser meter (C017, C018, C020).** `FUN_0060e150(0x22, 9, 0x5f, 6, ...,
  0x200ff00, 0x2ff0000)`: the charge is pure green (0, 255, 0) on pure blue
  (0, 0, 255), 95 x 6 at (508, 9). C020 shows 18 of 95 px about 58 s into
  the 300 s charge. The port drew 0.82 green over the gauge art.
- **Charge at entry (C017).** The first tactical frame shows a full meter
  and the ready control, and `FUN_005af4e0` stores `+0x6c = 100.0`. The
  port started at 0.
- **Committed shot (C018, C019).** The armed control is 1022. After the
  right-click target it shows 1023 at once, with an empty meter. The port
  kept 1022 while the shot was committed.
- **Death Star placement (TAC-03-C020).** `FUN_005d4d10` builds a decal
  frame at `(rand(0.4e) - 0.2e, rand(0.4e) - 0.2e, -2e)`, opposite the
  planet's `+2e`, with a child at `(0, 0.11e, -0.09e)` and a `0.5e` decal
  (`DAT_0066d244` 0.2, `DAT_0066d248` -2.0, `DAT_0066d250` 0.09,
  `DAT_0066d258` 0.11, `DAT_0066d260` 0.5). The capture shows it after the
  camera is turned away from the planet. A click on it selects nothing. The
  port had drawn 5010 as a field effect at the fleet lane and now draws the
  backdrop decal. `port:` the frame's two random offsets are taken at their
  centre.
- **Attack Death Star (C021, C022, C009).** The disabled art is 1178 and the
  enabled art 1176, at (568, 187). After the click it is 1177, and the
  mission graphic 2154 is at (496, 42). The port matches; no change.
- **Trench run (C023).** After the confirmation the view runs for about
  2 minutes (`FUN_005cfec0`'s 120 000 ms), then plays the success film
  MDATA.201 full screen and returns to the Battle Results. The port matches.
- **Green and gold squadrons (TAC-02-C004, C005).** "Green Group" with
  "A-wings: 1 12/12" and "Gold Group" with "B-wings: 1 12/12", in the same
  layout as TAC-02-C002. The port matches.
- **Music (TAC-01-C012, with an audio capture).** A 44.1 kHz recording of
  the Battle Alert, Take Command, the paused battle and the running battle:
  - The alert plays MDATA.313 (envelope correlation 0.82).
  - After Take Command there is no score. The battle is silent while paused,
    and once running it has only effects and voices: no MDATA.3xx track
    correlates above 0.76.
  - `FUN_00450540` calls `FUN_00417640` (stop score) for every Battle Alert
    choice.

  The port played MDATA.307 through the whole battle. It now stops the
  score, and a trench-run film returns to silence. `build-wasm.sh` no
  longer stages `battle.wav`.

## Gates

See the commit's gate log (`target/claude-tmp/s75-all.out`). The result is
recorded in the audit report's P24 row.
