---
title: "The opening briefing: what starts it, ALBRIEF/EMBRIEF, and how a clip reaches the galaxy view"
description: "Read 2026-10-23 and 2026-10-28 for P34. The clip definition record is settled (FUN_00404440). The advisor panel loads the briefing DLL at the start of a game. The advisor messages come from GDATA C3POACT.SPT / IMP22ACT.SPT; the briefing is message 0x29 / 0x4a, an action (FUN_004c0fc0 code) then a voice clip, in turn. Each side's advisor constructor schedules it (FUN_004c27f0 / FUN_004c0710) and the idle clip's end starts it (FUN_00439ef0); the Alliance runs its own action table FUN_004c30c0; FUN_00425d00 sets the captions by viewer."
---

# The opening briefing

Read 2026-10-23 (P34, tracker item "Droidy-doradcy"). It continues
`gid-markers.md`, "The advisor's show me actions".

## What starts it

- The advisor panel's setup `FUN_0042adb0` asks `FUN_004fcee0()`, the
  game object's `+0xc` (`FUN_0051ce00`). Under 2 it sets the panel's
  `+0xc0` to 8 and passes 1 as the second argument of
  `FUN_0042d650(side, briefing)`.
- `FUN_0042d650` loads `alsprite.dll` / `emsprite.dll` into DLL slot 9
  (`FUN_00603810(.., 9)`), and with `briefing` set `albrief.dll` /
  `embrief.dll` into slot `0xd`; without it, it unloads slot `0xd`
  (`FUN_00604460(0xd)`). `FUN_0041daf0` calls it too.
- The panel then makes the two clip players (`FUN_004727e0`, vtable
  `0x0065a358`) with ids `0xdead` and `0xbeef` and starts their idle
  clips `0x90193` and `0x90192` (`FUN_00472b20(0x12, ..)`). A clip id is
  `slot << 16 | RCDATA id`: `0x90193` is record `0x193` of the sprite DLL.

hyp: `+0xc` is the game's day, so the briefing plays on a new game's
first day. The GOG install ships the two briefing DLLs renamed
`ALBRIEF.DLL.OLD` / `EMBRIEF.DLL.OLD`; with the files missing,
`FUN_00603810` cannot load slot `0xd`. What the game then does is not
read.

## The briefing DLLs

| DLL | type 302 frames | RCDATA | BMP | WAVE |
|---|---|---|---|---|
| ALBRIEF | 2684 | 366 | 20 | 17: 1155, 1157-1161, 1163-1165, 1168, 1220-1226 |
| EMBRIEF | 2738 | 471 | 18 | 22: 1147, 1149-1153, 1155-1157, 1160, 1220-1226, 22002-22003, 23001-23003 |

RCDATA records, all little-endian 16-bit words:

- **Clip definitions**, 10 bytes: `[0, wave, slot, count, first]`. In the
  briefing range `slot` is `0xd` and the record id is the WAVE id plus
  3000: ALBRIEF `0x103b` = `[0, 0x483, 0xd, 0x89, 0x835]` is WAVE 1155,
  `0x89` frames from type-302 frame `0x835` (hyp: count and first frame;
  the 302 ids run `0x7d2..0x4c6a`). ALBRIEF has `0x103b..0x1048` and
  `0x107c..0x1082`; EMBRIEF `0x1033..0x1040` and `0x107c..0x1082`.
  EMBRIEF also has slot-9 definitions (`0x320`, `0x385`, `0x1396`,
  `0x157d..0x15c2`, all 15 or 16 frames from `0xbb9` / `0x7d1`).
- **Frame scripts**, 8 bytes: `[op, arg, 0, 0]`, one instruction per
  record, the record id its address. Ops seen: `4` (a frame, `arg` the
  type-302 id; 301 in ALBRIEF), `0x14` (also a frame), `0x11` (`arg` a
  record: `0x2710 -> 0x18f`, `0x2904 -> 0x18e`), `0x13` (`0x384..0x38a`),
  `3`, `0x15` (`0x2d37 -> 0x29a`).
- Short records: `0x18e` = `[0xd03]`, `0x18f` = `[0x7d1]`, `0x192` =
  `[0, 0xd03]`, `0x193` = `[0, 0x7d1]`, `0x29a` = `[0xd03]`.

No record holds a map action (`kind`, `code`); the action is given when
the clip is queued.

### The clip classes and the definition record (read 2026-10-28)

A queued record's word 0 is an op. `FUN_00403ed0(op)` clones the prototype
registered for it (`FUN_00403eb0` stores each by its slot `+0x2c`):

| Op | Prototype | Class (vtable) |
|---|---|---|
| 4 | `0x6b0628` | `0x00658520`: a clip definition |
| 5 | `0x6b0698` | `0x00658550` |
| `0x11` | `0x6b1328` | `0x00658848`: a frame list (the idle clips) |
| `0x12` | `0x6b1390` | `0x00658878` |
| `0x13` | `0x6b13f0` | `0x006588a8` |
| `0x14` | `0x6b1258` | `0x006587e8` |
| `0x15` | `0x6b12c0` | `0x00658818` |

The player's slot 8 (`FUN_00472930`) gives the clip its record
(`slot << 16 | id`) and its action, kind and code (slot `0x14`).

Op 4's loader `FUN_00404440` (vt `+0x1c`) reads the definition record
from the record's DLL slot, as 16-bit words:

| Word | Meaning |
|---|---|
| 0 | a sound in the same slot, started with the clip when not 0 (`FUN_00403f00`) |
| 1, 2 | a WAVE id and the slot it is in, started when word 1 is not 0 (`FUN_00403f70`) |
| 3 | the frame count (`+0x4c`) |
| 4 on | per layer (`+0x48`, from the player), the first frame |

The first frame is a standard bitmap (`FUN_005fbd20(module, id, 2)`).
Frames 1 to count - 1 are the type-302 deltas `first + 1 .. first +
count - 1` (`FUN_0041c590`), each added to the frame list
(`FUN_005f5600`, `FUN_005f5440`).

Checked against both briefing DLLs: every definition the briefing plays
has its first frame as a bitmap, all its type-302 deltas, and its WAVE in
slot `0xd`. For example, ALBRIEF `0x103b` is `[0, 1155, 0xd, 137,
2101]`: WAVE 1155, BMP 2101, then type-302 2102..2237. The definitions
the briefing does not play are ALBRIEF `0x103c` (WAVE 1156) and `0x1042`
(WAVE 1162); their WAVEs are absent and 4 and 1 deltas are missing.

`stage-ui-assets` stages each advisor DLL's RCDATA (`RCDATA/{id}.bin`)
and WAVE (`WAVE/{id}.wav`), and the two briefing DLLs, under their GOG
names `*.DLL.OLD` too, into `data/base/ui/{al,em}brief-dll/`.

## How a clip's action reaches the galaxy view

- A clip is `Class_00658848`; `FUN_00412280` loads its frames. The
  player's slot 8 (`FUN_00472930(clip id, kind, code, ..)`) makes the clip
  (`FUN_00403ed0`) and gives it its action through slot `0x14`
  (`FUN_004124b0`, fields `+0x54` kind and `+0x58` code).
- The panel's virtual `FUN_00442070(.., player id, ..)` finds a player
  (`FUN_005f5500`) and calls its slot 8.
- When a clip ends, its slot `0x20` (`FUN_00412400`) runs the action:
  `FUN_0041d140` -> `FUN_00436620` (only when its `+0xc` is set) ->
  `FUN_004878d0` -> `FUN_00439eb0(kind, code)`, which stores the pending
  action `DAT_006b28c4` = kind and its code.
- The advisor's renderer `FUN_004c0a60` (and `FUN_004c2b20`) runs a
  pending action through `FUN_004c0f60` / `FUN_004c3060`; kind 2 is
  `FUN_004c0fc0(code)`, the table in `gid-markers.md` (displays `0x11`,
  `0x13..0x17`, `0x20`, `0x91..0x93`, Coruscant, Yavin, the Emperor and
  Vader).

## The advisor's messages: C3POACT.SPT and IMP22ACT.SPT (read 2026-10-23)

- `FUN_004c2c70` (Alliance) and `FUN_004c0ba0` (Empire) open
  `GDATA\C3POACT.SPT` and `GDATA\IMP22ACT.SPT` (REBEXE RCDATA strings
  `0x618` "GDATA\\", `0x61b`, `0x61c`; `FUN_005f49e0`) and read them with
  `FUN_0049c6f0`: a count, then each message (`FUN_004c5230`, vtable
  `0x0065c5b0`, read by `FUN_004c52d0`). The save carries them too
  (`FUN_00439550`).
- File layout, little-endian: `u32` count; per message `u32` id
  (`+0x18`), `u32` (`+0x1c`, 1 in every message), `u32` entry count
  (`+0x20`), `u32` (`+0x24`, 0 in every message); per entry `u16` record,
  `u32` flags, `u32` a, `u32` b (14 bytes). C3POACT holds 87 messages
  (3510 bytes), IMP22ACT 85 (3422 bytes); both parse to their exact
  length.
- `FUN_004c0a60` (the advisor's update) walks the panel's schedule
  (`+0x164` count, `+0x168` the day each message is due, `+0x16c` the
  next free day). A due message (`FUN_005f5500(id)`) plays each entry
  whose flag bit 0 is clear (or when the advice flags `DAT_006b28b0`
  have `0x1000` without `0x4000`) through `FUN_0041d660(record, a, b,
  flags >> 1 & 1)` -> `FUN_0042b1d0`.
- `FUN_0042b1d0(record, a, b, briefing)` reads the RCDATA record from DLL
  slot 9, or slot `0xd` (the briefing DLL) when `briefing`; a or b of 0
  take the record's words 2 and 3. It calls the `0xdead` player's slot
  `0x38` with the record's word 0 (the op), `slot << 16 | word 1`, a and
  b. (`FUN_0042b290` does the same for `0xbeef` from slot 9.)
- Records: op 4 plays a clip definition (`[4, def]`, the 10-byte record
  with its WAVE and frames); op `0x11` is the idle clip `[0x11, 0x18f]`
  (ALSPRITE / EMSPRITE `0x2710` / `0x2af8`), which the messages use to
  carry an action: its a and b are the kind and code that
  `FUN_00439eb0` stores when it ends.

### The briefing

The briefing is message `0x29` in C3POACT and `0x4a` in IMP22ACT: an
action, then a voice clip from the briefing DLL (flag 2), in turn.
Each action is kind 2, so `FUN_004c0fc0(code)`:

| # | Action (code) | Alliance clip (ALBRIEF) | Empire clip (EMBRIEF) |
|---|---|---|---|
| 1 | `0xc`: `FUN_0041d9d0` | `0x27ab` -> def `0x103b`, WAVE 1155 | `0x2b8b` -> `0x1033`, 1147 |
| 2 | 1: display `0x11` (Popular Support) | `0x27ec` -> `0x107c`, 1220 | `0x2bd4` -> `0x107c`, 1220 |
| 3 | `0xe`: display `0x13` ("Systems Loyal to the Empire") | `0x27ed`, 1221 | `0x2bd5`, 1221 |
| 4 | `0xf`: display `0x15` ("Systems Under Military Control") | `0x27ee`, 1222 | `0x2bd6`, 1222 |
| 5 | `0x10`: display `0x14` ("Unexplored Systems") | `0x27ef`, 1223 | `0x2bd7`, 1223 |
| 6 | `0x11`: display `0x17` | `0x27f0`, 1224 | `0x2bd8`, 1224 |
| 7 | `0x12`: Coruscant, display `0x92`, caption `0x1624` | `0x27ad`, 1157 | `0x2b8d`, 1149 |
| 8 | 3: Yavin, display `0x93`, caption `0x1622` | `0x27ae`, 1158 | `0x2b8e`, 1150 |
| 9 | 4: display `0x17` | `0x27af`, 1159 | `0x2b8f`, 1151 |
| 10 | 5: display `0x20` | `0x27b0`, 1160 | `0x2b90`, 1152 |
| 11 | 6: display `0x91` | `0x27b1`, 1161 | `0x2b91`, 1153 |
| 12 | 7: the Emperor's system, `0x92`, caption `0x1627` | `0x27f1`, 1225 | `0x2bd9`, 1225 |
| 13 | `0x13`: Vader's system, `0x92`, caption `0x1628` | `0x27f2`, 1226 | `0x2bda`, 1226 |
| 14 | `0x14`: Yavin, `0x93`, caption `0x1622` | `0x27b3`, 1163 | `0x2b93`, 1155 |
| 15 | 9: Display Off `0x80` | `0x27b4`, 1164 | `0x2b94`, 1156 |
| 16 | `0xa`: not in `FUN_004c0fc0`'s table | `0x27b8`, 1168 | `0x2b98`, 1160 |
| 17 | `0xb`: display `0x11`, then `FUN_0041dbe0` | | |
| 18 | `0xd`: `FUN_0042d760`; with advice on, the Message Index; `FUN_0041da80` | | |

Correction (read 2026-10-28): the action column is the Empire's
(`FUN_004c0fc0`, run by `FUN_004c0f60` of the Empire's class
`0x0065c4a0`). The Alliance's class `0x0065c4c8` runs its own table,
`FUN_004c30c0` (through `FUN_004c3060`), with the same codes:

| Code | Alliance (`FUN_004c30c0`) |
|---|---|
| 1, `0xb` | display `0x11` (`0xb` then as the Empire's) |
| 2, `0xe` | display `0x13` |
| 3 | select the system of the Alliance headquarters `0x20000005` (`FUN_004f2d10`), display `0x93`, caption `0x1623`; without it, Display Off |
| 4, 8, `0x14` | select Coruscant (`0x109`), display `0x92`, caption `0x1624` |
| 5 | display `0x20` |
| 6 | display `0x91` |
| 7 | select the system of character `0x30000240` (Mon Mothma), display `0x93`, caption `0x1625` |
| 9 | Display Off `0x80` |
| `0xc`, `0xd` | as the Empire's |
| `0xf` | display `0x15` |
| `0x10` | display `0x16` |
| `0x11` | display `0x17` |
| `0x12` | select Yavin (`0x121`), display `0x93`, caption `0x1622` |
| `0x13` | select the system of character `0x32000242` (Luke Skywalker), display `0x93`, caption `0x1626` |

So the Alliance's briefing shows Coruscant at row 9 and Yavin at row 7,
and its own leaders at rows 12 and 13.

The captions of the displays `0x13..0x16` (`FUN_00425d00`, read
2026-10-28) follow the viewer:
- `0x13` and `0x15`: `0x161f` "Systems Loyal to the Alliance" when
  (viewer is the Alliance) equals (display is `0x13`), else `0x1620`
  "Systems Loyal to the Empire";
- `0x14` and `0x16`: `0x1621` "Systems Under Military Control" unless
  (display is `0x14`) differs from (viewer is the Empire); otherwise no
  caption.

This settles the hyp below the other way. In the Alliance's briefing,
row 4 (`0xf`, display `0x15`) reads "Systems Loyal to the Empire" and
row 5 (`0x10`, display `0x16`) "Systems Under Military Control", as
the captures 0129 and 0139 do. In the Empire's, row 4 (`0x15`) reads
"Systems Loyal to the Alliance" and row 5 (`0x14`) "Systems Under
Military Control".

Message `0x28` / `0x49` is a short form: action `0xb`, one clip
(`0x27b5` -> `0x1045`; `0x2b95`), action `0xd`. Messages `0x56`, `0x57`
(Alliance) and `0x54`, `0x55` (Empire) end with action kind 3, code 1.

The reference captures 0119-0149 show rows 2 to 5 in that order.

## The briefing's displays in `FUN_0042b330` (read 2026-10-23)

`+0x9c` is the viewing side, the holder is the system's `+0x24` bits
6..7, and the support side is `+0x84` bits 2..3 (`system-control.md`:
support of at least GNPRTB 7760 for one side, else 3). The enemy is 2
for viewer 1, 1 for viewer 2, 3 for viewer 3. `FUN_00426e70(value,
side, t1..t5)` picks the mark as for the other displays.

| Display | Value | Side | Thresholds |
|---|---|---|---|
| `0x13` | `0x52` when holder = viewer and support side = viewer, else 0 | holder | `0x51, 0x50, 0x3c, 0x3b, 0x32` |
| `0x14` | `0x52` when holder = viewer and support side != viewer | holder | same |
| `0x15` | `0x52` when holder = enemy and support side = enemy | holder | same |
| `0x16` | `0x52` when holder = enemy and support side != enemy | holder | same |
| `0x17` | `0x52` for an unexplored system (`+0x88` bit 1 clear), else 0 | 3 (no side) | same |
| `0x20` | as Idle Fleets `0x21` (the same case) | | |
| `0x91` | the holder's regiments (`FUN_005044f0`, families `0x10..0x13`) + armed defenses (`FUN_00526e90`, `0x22..0x27`) + squadrons (`FUN_00503550`) | holder | 6, 5, 3, 2, 1 |
| `0x92` | 7 for the system `FUN_005f5500` finds from the view's id (hyp: the one the action selected), else the unexplored mark (flag `0x80`) | 2 (Empire) | 6, 5, 3, 2, 1 |
| `0x93` | the same | 1 (Alliance) | 6, 5, 3, 2, 1 |

The gate at the top is inverted for `0x17`: an explored system gets
flag `0x80` (the unexplored mark, STRATEGY 10158), an unexplored one
goes through the switch. `0x20`, `0x92` and `0x93` count a destroyed
system as the fleet displays do.

So with the Alliance viewing, the briefing shows its own loyal systems
(`0x13`), the Empire's loyal ones (`0x15`), its own systems under
military control (`0x14`), the unexplored ones (`0x17`). (Superseded
2026-10-28: those are the Empire's codes; the Alliance's table and the
captions are under "The briefing" above.)

## Scheduling (read 2026-10-23 and 2026-10-28)

- The schedule `+0x168` is indexed by message id: `table[id]` is the last
  day the message may still play (`FUN_004c0a60` drops a value under the
  day `DAT_006b28cc`, from `FUN_004fcee0`), and `+0x16c[id]` its next
  free day (set to day + `0x3c` when it plays).
- The Empire's class is at `0x4c0xxx` (loader `FUN_004c0ba0`, update
  `FUN_004c0a60`, `FUN_004c0d00`), the Alliance's at `0x4c2xxx`
  (`FUN_004c2c70`, `FUN_004c2b20`, `FUN_004c2dd0`).
- The advice flag `0x2000` (`DAT_006b28b0`) schedules the short briefing
  for 40000 days and calls `FUN_0041da30`: message `0x28` in the
  Alliance's `FUN_004c2dd0`, `0x49` in the Empire's `FUN_004c0d00`.
- The writer of flag `0x2000` (read 2026-10-23): the briefing's first
  action, code `0xc`, is `FUN_0041d9d0`, which installs a keyboard hook
  (`SetWindowsHookExA(2, LAB_0041d8f0)`) and a mouse hook (`7`,
  `FUN_0041d950`). The mouse hook swallows the left and right buttons
  (`0x201..0x205`) and, on a press, sets `0x2000` (`FUN_0043a200`). So a
  click during the briefing ends it: the side's update schedules the short
  form (`0x28` / `0x49`: action `0xb`, one clip, action `0xd`) and calls
  `FUN_0041da30` (hyp: it removes the hooks). The keyboard hook's body is
  not read.
- `FUN_004c22f0(event)` maps advisor events to messages; event `0x1f`
  schedules `0x29`, the Alliance's full briefing, for 40000 days.
  `FUN_004c44b0` maps event `0x1f` to `0x1b` (offset `0x6c`); which
  side's map each is, and what sends event `0x1f`, are not read.

- What schedules the full briefing (read 2026-10-28): each side's
  advisor constructor sets it, not an event. The Alliance's
  `FUN_004c27f0` (class `0x0065c4c8`, `0x58` messages) and the Empire's
  `FUN_004c0710` (class `0x0065c4a0`, `0x56` messages) zero both tables,
  set the advice flag `0x1000`, and store 40000 as the last day of
  message `0x29` (`+0xa4`) or `0x4a` (`+0x128`).
- What starts it: `FUN_004c0a60` runs only while `DAT_006b28bc` is set.
  `FUN_00439ef0` sets it; it is reached through `FUN_004360d0` from
  `FUN_0041cea0`, which the idle clip (op `0x12`, `Class_00658878`)
  calls when its 15 ticks end. So the briefing starts after the first
  idle loop of the droid.
- The update walks the ids from 1 and plays the first due message whose
  next free day is under today (`+0x16c[id] = today + 0x3c`); one
  message per update.
- The keyboard hook `LAB_0041d8f0` (read 2026-10-28) sets `0x2000` on
  the release of Escape while the players are not yet flushed.
  `FUN_0041da30` flushes them once (`FUN_0042d620`, guarded by
  `DAT_006b14b4`), so the short form's clip, queued after the flush,
  plays.

## The reactions (read 2026-10-29)

With nothing due, `FUN_004c0a60` calls `FUN_004c2dd0` (Alliance) /
`FUN_004c0d00` (Empire); with the agent active (`+0x10`, 1 from
`FUN_00439320`) they answer one bit of `DAT_006b28b0` a call, writing
`+0x168[id] = today + days`:

| Bit | Alliance | Empire |
|---|---|---|
| `0x4000000` | `0x21` for 40000, the agent inactive, `FUN_0043a230` | `0x1a` for 10, inactive |
| `0x2000` (skip) | `0x28` for 40000, `FUN_0041da30` | `0x49` for 40000 |
| `0x1` | `0x23`, 10 | `0x1c`, 10 |
| `0x10` | `0x24`, 10 | `0x1d`, 10 |
| `0x20` | `0x26`, 10 | `0x1f`, 10 |
| `0x100` | `0x25`, 50 | `0x1e`, 50 |
| `0x200` | `0x27`, 50 | `0x20`, 50 |
| `0x1000000` | `0x4f`, 5 | none |
| `0x2000000` | `0x4a + DAT_006b28b8` in `0x4c..0x4e`, 5; past `0x4e` the count restarts | `0x49 + n` in `0x4b..0x4d`, 5 |
| `0x80000000` | `FUN_005f5700(12)` below 6: `0x50 + n`, 40000; `DAT_006b28bc = 1` | `0x4e + n`, 5 |

With the agent inactive only `0x8000000` is answered (`0x22` / `0x1b`,
the agent active again) and the bits are masked to `0xc000`.
`0x2000000` comes from opening the Agent menu (`FUN_0041d160(.., 1)` →
`FUN_00436640` → `FUN_00487c10` → `FUN_00439ed0`, which also counts
`DAT_006b28b8`); `0x80000000` from the idle clip's end once
`DAT_006b28d0 + 0x32 < today` (`FUN_00439ef0`). Each reaction message
holds one sprite-DLL record. Ported 2026-10-29 but `0x1`, `0x8000000` and
`0x4000000`.

A refused order reaches the same table through slot `+0xc`
(`FUN_004c2940`, `FUN_004c0870`), ten days ahead: messages `0xf`..`0x14`
(Alliance) or `0x10`..`0x15` (Empire), in `mission-dialog.md`, "Refusal".
Ported 2026-10-29.

## Open

- Action code `0xa`, kind 3, `FUN_0041dbe0` (code `0xb`) and
  `FUN_0042d760` (code `0xd`). Code `0xc` installs the hooks
  (`FUN_0041d9d0`) and `0xd` removes them (`FUN_0041da80`).
- Where the caption `0x1629` "Unexplored Systems" is drawn (the
  others are `FUN_00425d00`'s, above).
- Event `0x1f` (`FUN_004c22f0`, `FUN_004c44b0`): what sends it.
- How `0x92` / `0x93` find their system (`FUN_004025b0(local_f0)`).
- The frame-script ops other than 4 and `0x11`, and the short records.
  The definition's words are settled above.
