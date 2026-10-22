---
title: "The opening briefing: what starts it, ALBRIEF/EMBRIEF, and how a clip reaches the galaxy view"
description: "Read 2026-10-23 for P34. The advisor panel loads the briefing DLL at the start of a game. The advisor messages come from GDATA C3POACT.SPT / IMP22ACT.SPT; the briefing is message 0x29 / 0x4a, an action (FUN_004c0fc0 code) then a voice clip, in turn. Who schedules it is still open."
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

Message `0x28` / `0x49` is a short form: action `0xb`, one clip
(`0x27b5` -> `0x1045`; `0x2b95`), action `0xd`. Messages `0x56`, `0x57`
(Alliance) and `0x54`, `0x55` (Empire) end with action kind 3, code 1.

The reference captures 0119-0149 show rows 2 to 5 in that order.

## Open

- Which code puts message `0x29` / `0x4a` (or `0x28` / `0x49`) on the
  schedule at a new game, and what the panel's `+0xc0` value 8 drives.
- Action code `0xa`, kind 3, and the functions behind codes `0xb`, `0xc`
  and `0xd` (`FUN_0041dbe0`, `FUN_0041d9d0`, `FUN_0042d760`,
  `FUN_0041da80`).
- The marks of displays `0x13..0x17`, `0x20` and `0x91..0x93`
  (`gid-markers.md`: `FUN_004522f0` has no legend layout for them).
- The frame-script ops other than 4 and `0x11`, the short records, and
  the clip definitions' last two words (hyp: frame count and first
  frame).
