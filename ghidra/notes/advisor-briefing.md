---
title: "The opening briefing: what starts it, ALBRIEF/EMBRIEF, and how a clip reaches the galaxy view"
description: "Read 2026-10-23 for P34. The advisor panel loads the briefing DLL at the start of a game; the DLLs hold clip definitions (voice WAV + frame range) and frame scripts; the clip's map action goes through FUN_00436620 -> FUN_00439eb0 -> FUN_004c0fc0. Who queues the briefing clips is still open."
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

## Open

- Who calls the panel's `FUN_00442070` with the briefing clips
  (`0xd << 16 | 0x103b..`) and which `kind`/`code` each gets. The clip
  ids are not constants in the code (no `0x103b` or `0xd103b` immediate),
  and no table in `.rdata`/`.data` holds them as 16- or 32-bit values;
  they are likely computed (WAVE id + 3000).
- The frame-script ops and the short records' meaning.
- What the panel's `+0xc0` value 8 drives.
- The reference captures 0119-0149 show the briefing's displays in
  order: Popular Support, "Systems Loyal to the Empire", "Systems Under
  Military Control", "Unexplored Systems".
