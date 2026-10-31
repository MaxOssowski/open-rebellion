# Unified Game Options: source contract

Scope: PRE-03 / RE-OPT-01. Recovery against the owned English executable
SHA-256 `b3fe3997cab9a6e96403d638875dcba25484e4d8601751afec748471ac0ed6ab`.
This is static evidence, not original-executable visual acceptance.

## Entry and construction

The main-menu CD-ROM command is **0x68**, not RE-OPT-01's current 0x67
retrieval hint (0x67 is multiplayer). `FUN_00406000`'s jump table at
`004064e0` maps 0x68 to `004061d0`, which requests application state 11
through `FUN_00401040`. Cockpit/F1 uses command 0x133.

`FUN_00406950` constructs the shared window and snapshots options flags.
`FUN_00406b80` handles creation by calling `004070e0` (background),
`004084b0` (text), and `00407180` (controls). The background loader at
`00407119` requests resource **COMMON 20002**, 640 by 480 pixels.

## Controls

Coordinates below are `FUN_00407180`'s literal arguments (corrected
2026-10-31 from the upstream P58-B12 placement, which was off by one to
three pixels). A0 `tooltip/200-options.png` shows the Save, Load, Restart,
Return, Exit, Play Music and display-switch bitmaps at exactly these
positions. Bitmap triples mean normal, pressed or set (state 4), disabled
(state 2). The six save rows start at y=81 and increment 42.

| Control | Rectangle | Command | COMMON bitmaps |
|---|---|---|---|
| Save row 1–6 | (34,y,42,20) | 0x6f–0x74 | 10046/10047/10048 |
| Save name 1–6 | (119,y,160,20) | 0x83–0x88 | native edit control |
| Side crest 1–6 | (85,y+2), 26x19 | — | 10056 Alliance, 10055 Empire, 10057 multiplayer |
| Load row 1–6 | (287,y,41,20) | 0x79–0x7e | 10049/10050/10051 |
| Effects volume | origin (392,190), 201x54 | 0x8e | 10053, thumb 10054 |
| Music volume | origin (392,130), 201x54 | 0x8d | 10052, thumb 10054 |
| Restart | (76,381,42,42) | 0x91 | 10035/10036/10037 |
| Return | (162,382,42,42) | 0x90 | by window mode, below |
| Exit | (248,381,42,42) | 0x8f | 10038/10039 |
| Play music | (352,76,19,35) | 0x99 | 10040/10041/10042 |
| Starfield | (357,311,35,22) | 0xa1 | 10043/10044/10045 |
| Planet | (357,337,35,22) | 0xa2 | same |
| Pyrotechnics | (357,365,35,22) | 0xa3 | same |
| High detail | (357,392,35,22) | 0xa4 | same |
| Holocube | (357,419,35,22) | 0xa6 | same |

A switch that is on shows its pressed art: `FUN_00603150(4, 0x2739)` for
music and `FUN_00603150(4, 0x273c)` for the display switches, so an "on"
display switch shows 10044 (A0: all five) and an "off" one 10043; 10045 is
the disabled art in a battle (`FUN_00603150(2, 0x273d)`).

Return's art (`FUN_00407180`, switch on the window's mode `+0x9c`):

| Mode | Art | Note |
|---|---|---|
| 8, player not the Alliance (`FUN_00401060() != 1`) | 10023/10024/10025 | A0: an Empire game from the galaxy |
| 8, Alliance; and the default | 10020/10021/10022 | |
| 9 | 10026/10027/10028 | the holocube; hyp: the battle |
| 4, 0xc | 10029/10030/10031 | multiplayer (modes 4/5 tip TEXTCOMM 0x1019 "Disconnect from Opponent") |

All these buttons are CoolStrobeButtons and blit keyed on their first DIB
pixel's colour (`FUN_00602d30`): white index 11 for Restart and Exit, blue
for Return's corner. The slider tracks and thumb (`FUN_00604a50` ->
`FUN_005fd0f0`) and the crests key the same way (magenta, blue corner).

Text, slider and version details are in `game-options-text.md`.

`FUN_004084b0` maps labels to TEXTCOMM 0x1025 (music), 0x101f–0x1022
(first four display switches), 0x1024 (Holocube), and 0x1031/0x1032 (On/Off).
`FUN_004082b0` maps the switches to masks 0x100, 0x10000000, 0x01000000,
0x00100000, 0x00010000, and 0x02000000 respectively.

## Context and mutation boundaries

`FUN_00407180` disables save/edit controls when the current state is zero or
20–23; load is also disabled for states 20–23 and for absent/incompatible
save metadata. All five tactical display switches are disabled in states
20–23. The manual's printed pages 75–77 independently confirm six slots,
overwrite/current-game confirmations, and that display options cannot change
during a tactical battle. All five display options default on in the original.

The manual describes Restart as abandoning the campaign and returning to the
Shuttle, not replaying a battle. Return is unavailable from the Shuttle.
These are navigation operations; opening/closing options must not mutate a
battle roster, apply results, advance simulation, or reset a paused state.

Native confirmation construction is `FUN_00416c50` / `FUN_00416b90`:
REBDLOG bitmap 10623, 412 by 176. `00416e90` constructs yes/no at (138,135)
and (229,135), with bitmap pairs 10624/10625 and 10626/10627; both are 57 by
28. Upstream stages all 23 numeric REBDLOG bitmaps; the unused named corner is excluded. This contribution requires 10623–10627 at packaging time and provides a visible fallback for older packs.

## Persistence

The options outlive the run in `HKLM\SOFTWARE\LucasArts Entertainment
Company\...` (`FUN_005f4250` writes, `FUN_005f41b0` reads a DWORD, 0 when
absent):

| Value | Written by | Read by | Meaning |
|---|---|---|---|
| `MusicVolume` | `FUN_00417670` | `FUN_004173c0` | music, negated hundredths of a dB |
| `Volume` | `FUN_00416360` | `FUN_00413000` | effects, negated hundredths of a dB |
| `MusicSwitch` | `FUN_004176e0` | `FUN_004173c0` | 1 when the music is off |
| `DisplayOpts` | `FUN_004163a0` | `FUN_00413000` | the window's flag word `+0xa0` |

The slider's percent v becomes `floor(2000 * log10(v / 100))` hundredths of
a dB (`FUN_00406b80`), so the DirectSound amplitude `10^(dB / 20)` is v / 100
less at most 0.12 per cent: the port's linear gain v / 100 matches it. The
port keeps the same four values (`rebellion_data::settings`) in
`saves/settings.json` and in the browser's `localStorage`
(`rebellion_settings_v1`). port: an absent store keeps the port's defaults
(all display options on, as the manual says, music 0.8) rather than the
registry's zeros.

## Compatibility and evidence gaps

- The authentic window has six rows. The compatibility panel retains all ten
  existing slots through F8 / Ctrl+L (load) and campaign F9 / Ctrl+S
  (save/delete), sharing confirmation guards. No storage migration is needed.
- No native delete button is present in this constructor or the inspected
  manual pages. Do not invent one; existing legacy deletion remains separate
  until its native gesture is recovered.
- Upstream display flags, including Holocube, are preserved. Their full original
  visual behavior remains outside this save/confirmation checkpoint.
- The slider transfer function, the text objects and persistence are
  recovered (`game-options-text.md`, above); the battle context in A0 and
  lossless A0 interaction captures remain to verify.
- The Rust confirmation guard tests are implementation evidence only; they
  do not establish original pixel, audio, or full PRE-03 acceptance.
