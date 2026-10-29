---
title: "The agent's advice list"
description: "Agent Advice (0x11e): the TEXTSTRA advice list, its kinds and keys, the windows whose first opening posts advice, the 300-sub-tick period, the advice message class, and the sub-tick clock it reads"
category: ghidra
created: 2026-10-29
updated: 2026-10-29
tags: [agent, advisor, advice, message-index, textstra, sub-tick]
---

# The agent's advice list

Read from the typed corpus (`~/Games/rebexe-decompile/decompiled-typed`)
and the GOG `REBEXE.EXE` (disassembly at `0x439904..0x43993c`), and
compared with the A0 VM (2026-10-29). Agent Advice is menu item `0x11e`;
bit `0x8000` of `DAT_006b28b0` set turns it off (`agent-menu.md`,
`agent-advice.md`).

## The list (`FUN_0048b460`)

The agent keeps a list at `+0x154`. `FUN_0048b460(side, list)` fills it
from the string module (`FUN_005fefd0(2)`, TEXTSTRA.DLL):

- `RT_RCDATA` `0x6000` (side 1) or `0x6800` (other) holds the count, a
  `u16`: `0x1f` on both sides.
- Advice `n` (1-based) is record `base + 3n`, four bytes: the kind in byte
  0's low nibble, byte 1 unused, and a `u16` key at `+2`.
- Each record makes a node (`FUN_005f5f10`) with `+0x1c = n << 4 | kind`
  and key `+0x18` (`FUN_005f5600`), inserted by `FUN_005f5440` /
  `FUN_005f4f10` into a tree that keeps key order (`+0x10` is the next
  node). hyp: the comparator (vtable `+8`) orders keys ascending; the A0
  list agrees (below).
- A record that does not load is skipped. The function returns 1 when the
  count is above 0.

The Alliance's 31 (the Empire's have the same kinds and keys):

| n | Kind | Key | Title |
|---|---|---|---|
| 1..9 | 7 | 10..90 | Victory Conditions, Mon Mothma..., The Galactic Information Display, Positioning Fleets, The Battle Alert, Manufacturing New Items, Adjusting Time, Furthering Our Cause, Advice Available |
| 10..15 | 1 | 140..190 | Maintenance Points, Characters and Special Forces, Finders, Galactic Encyclopedia, Galaxy Overview, Messages |
| 16, 17 | 6 | 100, 230 | What We Know About the Empire, Exploration and Colonization |
| 18..22 | 3 | 120, 220, 260, 270, 280 | Fleet Window, Creating Fleets, Blockades, Bombardments, Planetary Assault |
| 23..26 | 2 | 110, 210, 240, 250 | The Manufacturing and Production Window, Mines and Refineries, Agent Management, Giving Your Agent Build Orders |
| 27..31 | 5 | 130, 200, 290, 300, 310 | The System Defenses Window, Selecting Characters for Missions, Defensive Facilities, Garrisons, Uprisings |

## Posting

`FUN_0048a590(node)` builds the message `FUN_0048b2e0` (class
`0x0065b188`) and delivers it (`FUN_0048a060`):

- the reader is the agent's side (`+0x20`);
- category `FUN_0048b450`: `0x200`, Advice;
- `+0x28` = `0x2a`, which the reader's advisor hears (`advisor-briefing.md`,
  "The events");
- title `+0x44` and text `+0x50` from TEXTSTRA `RT_RCDATA`
  `0x6001 + 3n` and `0x6002 + 3n` (Alliance) or `0x6801 + 3n` and
  `0x6802 + 3n`;
- background `+0x2e` = `0x42f` / `0x430`, media `+0x30` = `0x461` /
  `0x462`. hyp: `+0x32` stays 0, so `FUN_00469de0` starts no media;
- the class's kind (vtable `+0x10`, `0x48ad30`) is 8, so it reaches the
  advisor.

The posted node is then removed (`FUN_005f54d0`) and destroyed.

## When

- **`FUN_00439f20`** (the list's build): only while `+0x150` bit
  `0x10000000` is clear. It calls `FUN_0048b460` and sets the bit when that
  returns 1, then posts every node of kind 7 (or every node when
  `DAT_006b28d4`, which nothing writes, is set).
- **The briefing's end:** the briefing tables' action `0xd`
  (`FUN_004c30c0` / `FUN_004c0fc0`) closes window `0xd`, the Message Index
  (`FUN_0042d760` → `FUN_00604460(0xd)`). With advice on it calls
  `FUN_00439f20` and `FUN_0041d770(1, 0x82)`, which posts `0x468` kind
  `0xd`: the Message Index opens on Advice.
- **A window's first opening (`FUN_0043a0b0(flag)`)**, with advice on and
  the agent active (`+0x10`): it builds the list if needed, and if the
  flag is new it sets it in `+0x150` and posts the first node of the
  flag's kind, storing the time in `+0x148`. The flags come through
  `FUN_0041d2d0` → `FUN_004369d0` → `FUN_00487fd0` from the window
  creators, only for a window created, not raised:

  | Opener | Window | Flag | Kind |
  |---|---|---|---|
  | `FUN_00429ce0` | a sector window | 1 | 6 |
  | `FUN_0045aac0` kind 4 | System (type 9, Manufacturing) | 2 | 2 |
  | `FUN_0045aac0` kind `0x10` | Fleet (type 4) | 4 | 3 |
  | `FUN_0045aac0` kind 8 | System Defenses (type 10) | 8 | 5 |
  | `FUN_0045aac0` kind `0x40` | Missions (type 11) | `0x10` | 4 (none) |

- **The period (`FUN_00439bc0`)**, from the panel's `Class_0065ad70_vt38`,
  with advice on and the agent active: once `+0x148 + 300 <=
  DAT_006b28cc`, `FUN_00439fb0` builds the list if needed (reading its
  head before, so that call posts only the kind-7 nodes), posts the first
  node whose kind 2, 3, 4, 5 or 6 has its window flag, or of any other
  kind, and stores the time in `+0x148`.

## The clock: sub-ticks, not days

`+0x148`, and `DAT_006b28cc` it is compared with, come from
`FUN_004fcee0`, the scheduler's `+0xc`. `FUN_0051dea0` raises `+0xc` once
a sub-tick and the day `+0x10` (`FUN_004fcf00`) once the day's sub-ticks
reach the larger side's `+0xc4`, the speed's rate: 600 Very Slow, 60
Slow, 12 Medium, 4 Fast (`timer-scheduler.md`, `ai-mission-planning.md`).
So the period is 300 sub-ticks: half a day at Very Slow, 25 days at
Medium, 75 at Fast.

The advisor's own times (`DAT_006b28cc` in `FUN_004c2dd0`, `FUN_004c0d00`,
`FUN_004c0a60`, `FUN_004c2b20`) are the same sub-ticks; the port's
advisor schedule counts them as days, which is wrong and open.

## Saved

`FUN_004397a0` ends with `+0x148`, `+0x14c` (`FUN_005f4db0`), `+0x150`
(`FUN_005f49a0`) and the list `+0x154` (`FUN_005f5da0`). `FUN_00439320`
sets `+0x148` to 0 and `+0x14c` to 8.

## A0 (2026-10-29, Windows 10 VM, Empire, days 175..178)

The captures stay outside Git (`.artifacts/a0-vm/agent-advice/`):

| Capture | sha256 | What it shows |
|---|---|---|
| `00-resume.png` | `fce76256…` | day 175, the Advice rail control lit (the red droid, right rail) |
| `01-advice-index.png` | `40b45302…` | the index on Advice, newest first: Planetary Assaults (key 280), Bombardments (270), Blockades (260), Exploration and Colonization (230), Creating Fleets (220), The Message Window (190), Galaxy Overview (180), The Galactic Encyclopedia (170), Finders (160) |
| `02-advice-message.png` | `09f0f692…` | Planetary Assaults opened: the IMP-22 picture (`0x430`), the title, the text |

The keys rise from bottom to top, the Manufacturing (kind 2) and System
Defenses (kind 5) advice are missing between them, and the kind-1 advice
come in by the period: so the list keeps key order and skips a kind whose
window was never opened. More advice came in 178 days than a 300-day
period allows, which agrees with sub-ticks.

## Ported 2026-10-29

`rebellion_core::agent_advice`, `GameClock::sub_tick`, save v109
(`evidence/2026-10-29-agent-advice/`).
