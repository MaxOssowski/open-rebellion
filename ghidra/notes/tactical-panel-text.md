---
title: "Tactical right-hand panel: texts, fonts and the fighter count line"
description: "Where the tactical panels place their texts, which font they use, and how the fighter panel builds its 'X-wings: 1 12/12' line (FUN_005e1fd0, FUN_005f1050, FUN_005c6530, FUN_005ba330)"
category: "ghidra"
created: 2026-10-24
updated: 2026-10-24
tags: [tactical, panel, font, fighters, a0]
---

# Tactical panel texts

Read on 2026-10-24 with Ghidra 12.1.4 (through REA) and `objdump`, from the
audited `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`). Positions were checked
against the lossless A0 captures (`.artifacts/interface-parity/a0/`, ignored).

## Panel origin

Every 149x236 right-hand panel (TACTICAL 1302-1310) sits at (482, 28) in the
captures, and child positions are panel-local. For example, `FUN_005f0f40`
places the assign buttons 1113/1114 at (0x42, 0xca), which is (548, 230) on
screen.

## Fonts

The labels are `FUN_00601880(text, x, y, w, h, align, flags)` controls.
`FUN_00601c60(n)` picks font `n` of the table that `FUN_0060eed0` builds,
from the face "Arial" (`FUN_0060ee20` copies `0x6ac7b8`). `FUN_00601c90`
sets the colour; `0x2ffffff` is white.

- The fighter panel (`FUN_005e1fd0`) uses font 8, a 12 px cell at weight
  400:
  - label `+0xa4` at (26, 9), 97 wide, align 1: the group name;
  - label `+0xa8` at (20, 20), 102 wide: the orders;
  - label `+0xac` at (20, 31): the tactics;
  - labels `+0xb0..+0xbc` at (-100, -100): the count lines, moved later.
- The withdrawal confirmation (`FUN_005f1050`) uses font 10, a 14 px cell:
  TEXTTACT 56772 at (20, 28) and 56771 at (14, 92).
- In the captures the first capital row lies 2 rows (font 8) or 3 rows
  (font 10) below the label's y. The capitals are 7 and 8 rows tall. The
  widths fit Liberation Sans (Arial's metrics) at 9.36 and 10.9 px, about
  0.78 of the cell. That ratio is measured, not read.

## Fighter portrait and count line

- `FUN_005e2320` creates the portrait surface, 102x107 at (0x17, 0x3b),
  which is (505, 87) on screen.
- The code around `0x5e1c80`-`0x5e1f37` (no function boundary in Ghidra)
  walks the group's fighter types `0x1d..0x24`, asking the group's vtable
  `+0x3c` for each type's count. The first type with a count picks:
  - its TEXTTACT label `0xddc5 + k` (56773 "X-wings:", "Y-wings:",
    "A-wings:", "B-wings:", "Fighters:", "Intrcpts:", "Bombers:",
    "Defenders:");
  - its portrait `0x7ee + k` (2030 + k).
- It then moves label `+0xb0` to (0x17, 0x92), which is (505, 174), and
  streams: label, " ", the type's count, " ", `FUN_005c6530`, "/" (string at
  `0x6aa5cc`), `FUN_005c6570`.
- `FUN_005c6530` sums `FUN_005ba330` over the group's squadrons. That is
  `trunc(current hull (+0x3cc) / 8)`, at least 1, through `FUN_005b1770`.
- `FUN_005c6570` sums `FUN_005ba380`: `trunc(maximum hull (+0x3c8) / 8)`,
  at least 1. The divisor is `0x66c6b8`, 8.0, so one fighter is eight hull
  points.
- A0 TAC-02-C002 reads "X-wings: 1 12/12": one X-wing squadron at full
  strength.
