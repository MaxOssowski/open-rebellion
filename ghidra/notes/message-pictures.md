---
title: "Message pictures: what every message class shows"
description: "The background and foreground each message builder sets, where the bitmaps come from, and which the port attaches"
category: ghidra
created: 2026-10-13
updated: 2026-10-13
tags: [messages, pictures, strategy-dll, encyclopedia, notifications]
---

# Message pictures

The Message window draws a picture above a message's text
(`FUN_0046a320`, `message-index-rows.md`). This note lists what every
message builder sets. Read from the typed corpus
(`~/Games/rebexe-decompile/decompiled-typed`) and the GOG `REBEXE.EXE`;
titles are TEXTSTRA `RT_RCDATA` ids. Story lines and their triggers are in
`story-message-classes.md`.

## Where a picture comes from

A builder writes two shorts: `+0x2e`, the background, and `+0x2c`, a
foreground blitted keyed and centred on it. 42 functions in
`0x48a000`-`0x49b000` write one or both.

`FUN_0046a280` loads the background from STRATEGY (window `+0x120`). When
STRATEGY has no such bitmap it calls `FUN_0045f970`, which reads the file
name ENCYBMAP's string table gives for the same id and loads that
`EDATA.NNN`: the object's Encyclopedia artwork.

`FUN_004c5000(object)` is the object's own picture: its vtable slot
`+0x30`, masked to `0xfff`, plus `0x1000`. For a character that is a
STRATEGY portrait (6208-6273, 6720-6811), used as the foreground. For
anything else STRATEGY has nothing, so the background is the Encyclopedia
artwork: `0x1100`-`0x1105` the yards (EDATA.003-008), `0x1200`-`0x1204` and
`0x1280` the other facilities, `0x1440`.. and `0x1480`.. regiments,
`0x1540`.. and `0x1580`.. special forces, `0x1640`.. and `0x1680`..
fighters, `0x1740`.. and `0x1780`.. capital ships. The picture id is the
list mini (GOKRES) less `0x3000`.

All fixed pictures are 400 by 200: STRATEGY 1000, 1002-1006, 1008-1068,
1070-1075. The five assault and bombardment pictures, 11160-11164, are 400
by 310.

"Reader" below is the side the message is built for (`+0x20`): 1 Alliance,
2 Empire. `(reader != 1) + n` is written "n / n+1".

## Reports

| Message | Notification | Builder | Background | Foreground |
|---|---|---|---|---|
| Mission Report | `0x1f0` | `FUN_004927c0` | by mission family, below | the leading character |
| Mission Failed, Aborted | `0x1f1` | `FUN_00491500` | `0x412` / `0x413`; `0x416` / `0x417` for outcomes 3-5 when a local the builder tests is 0 (untraced) | the leading character |
| Enemy Mission Foiled | `0x1f1` | `FUN_00491500` | `0x431` / `0x432` | — |
| Personnel Arrive | `0x200` | `FUN_004955b0`, `FUN_00495bf0` | `0x412` / `0x413`; special forces alone: their own picture | the arriving character |
| Units Deployed | `0x107` | `FUN_00497940` | the new unit's own picture | — |
| Facility Lost | `0x303` | `FUN_00497690` | `0x3f7` / `0x3f8` | — |
| Ships or Fleet Arrive | `0x105` | `FUN_004981c0` cases 1, 2 | `0x3fa` / `0x3fb` | — |
| Units Arrive | `0x105` | `FUN_004981c0` case 3 | `0x3fc` / `0x3fd` | — |
| Headquarters Arrives | `0x105` | `FUN_004981c0` case 4 | `0x3fe`, Alliance only | — |
| Research Complete | — | `FUN_00499aa0` | `0x3f9` | — |
| Research Exhausted | `0x129`-`0x12b` | `FUN_0048abc0` | `0x3f9` | — |
| Squadron at Full Strength, Capital Ship Repaired | — | `FUN_00499de0` | the unit's own picture | — |
| Facility Idle | `0x160` | `FUN_0048be60` | the facility's own picture | — |
| Maintenance Shortfall | `0x304` | `FUN_00498970`, slot `+0x40` `FUN_0048bbf0` | `0x3f5` / `0x3f6` | — |
| Saboteurs Strike | `0x305` | `FUN_0048b9a0`, slot `+0x40` `FUN_0048bbf0` | `0x3f5` / `0x3f6` | — |
| Recruitment Done | `0x12c` | `FUN_0048ae40` | `0x412` / `0x413` | — |
| Informants Provide Information | `0x153` | `FUN_00491060` | `0x3e8` | — |
| Smuggling | `0x145` | `FUN_00499130` | `0x3ec` | — |
| Message From or Sent to the other side | — | `FUN_00498e90` | `0x410`, `0x411` | — |
| Droid advice | — | `FUN_0048b2e0` | `0x42f` Alliance, `0x430` Empire | — |

Mission Report backgrounds (`FUN_004927c0`), set from the mission's family
before the family's own case runs:

| Family | Background |
|---|---|
| `0x51` Diplomacy | `0x414` |
| `0x52` Espionage | `0x415` |
| `0x53` Research | `0x3f9` |
| `0x54` Reconnaissance | an object's own picture (case `0x54`; which object is untraced) |
| `0x56` Incite Uprising | `0x3ed` on outcome 3, otherwise the side's |
| `0x57` Subdue Uprising | `0x3f2`; `0x3f3` / `0x3f4` on outcome 3 |
| others | `0x412` / `0x413` |

At the end the builder looks at the leader: a character's portrait goes to
the foreground; for anything else the leader's own picture replaces the
background.

## Systems

| Message | Notification | Builder | Background |
|---|---|---|---|
| Uprising Begins | `0x14b`, argument set | `FUN_00499460` | `0x3f2` |
| Uprising Ends | `0x14b` | `FUN_00499460` | `0x3f3` Alliance system, `0x3f4` Empire system |
| Near Uprising | `0x148` | `FUN_00498cb0` | `0x3f0` Alliance system, `0x3f1` Empire system; shown to the holder |
| Joins, Joins Enemy, Declares Neutrality | `0x100` | `FUN_00499760` | `0x3ed` |
| Natural Disaster | `0x154` | `FUN_0048e940` | `0x3eb` |
| New Resources, Resources Depleted | `0x155` | `FUN_0048e940` | `0x3ea` |
| Fleet Initiates Blockade, Under Blockade | `0x14e` | `FUN_004960f0` | `0x404` Alliance system, `0x403` Empire system |
| Evacuation Losses | `0x340` | `FUN_00495e90` | `0x407` Alliance reader, `0x406` Empire reader |
| Assault | `0x150`, then 2 | `FUN_00496770` | `0x2b98` Alliance attacker, else `0x2b99` |
| Orbital bombardment | `0x14f`, then 2 | `FUN_00496770` | `0x2b9b`, `0x2b9c` or `0x2b9a`, below |

A bombardment's picture follows two lists on the message. `FUN_00497380`
files every unit the strike destroys (notification `0x302`): into `+0x80`
when the unit's location is a system (class `0x90`-`0x97`), otherwise into
`+0x88`. `+0x80` not empty gives `0x2b9b`, the scorched ground; else
`+0x88` not empty gives `0x2b9c`, the planet's guns firing on a ship; else
`0x2b9a`, the ground untouched.

## Characters

| Message | Notification | Builder | Background | Foreground |
|---|---|---|---|---|
| Injured | `0x106` | `FUN_0048fdf0` | `0x41a` | — |
| Healed | `0x106` | `FUN_0048fdf0` | `0x41b` | the character |
| Killed | `0x106` | `FUN_0048fdf0` | `0x42e`, or `0x433` when `FUN_004edc80` is not 1 | — |
| Captured | `0x1e0` | `FUN_00490340` | `0x418`, or `0x419` when `FUN_004edc80` is not 1 | — |
| Escaped | `0x1e0` | `FUN_00490340` | `0x405` | — |
| Jabba Captures | `0x1e0` case 5 | `FUN_00490340` | `0x429` | — |

`FUN_004edc80` is the character's home side (confirmed in
`captive-escape.md` from the DAT `is_alliance`/`is_empire` fields): `0x42e` is a coffin under the
Alliance flag and `0x433` one under the Empire's; `0x418` shows stormtroopers
holding Alliance prisoners and `0x419` the reverse. Its `this` is lost in
the decompilation.

The story classes' pictures are in `story-message-classes.md`: `0x41c`-
`0x42c`, the side's background with a portrait, and `0x42e`.

`FUN_0049a350` (backgrounds `0x3fa`, `0x406`-`0x40f`) builds the confirm
dialogs for scrapping, retiring, aborting and evacuating, not messages.

## The port (2026-10-13)

`rebellion-render/src/message_pictures.rs` has one function per row it
uses, and `message_index::draw_message_picture` takes Encyclopedia artwork
when STRATEGY has no bitmap (`EncyclopediaState::artwork_texture`).

| Port message | Picture |
|---|---|
| Construction complete, Unit arrived (delivery) | Units Deployed |
| Manufacturing queue idle | Facility Idle; port: the system's first yard |
| Fleet arrived | Ships or Fleet Arrive |
| Regiment arrived | Units Arrive |
| Mission result | Mission Report, for both outcomes |
| Uprising begins, ends, subdued | Uprising Begins, Ends |
| Natural disaster, New resources | the two `FUN_0048e940` pictures |
| Blockade established | Fleet Initiates Blockade |
| Troops destroyed by blockade | Evacuation Losses |
| Maintenance shortfall | `0x3f5` / `0x3f6` |
| Tech advanced | Research Complete |
| Force tier reached | the side's background with the portrait (Force Growth) |
| Character killed | Killed |
| Escaped captivity | Escaped |

The Assault and bombardment pictures are not attached: at 400 by 310 they
overflow the window's 400 by 200 picture area (seen in the browser,
2026-10-13). Those messages set `+0x28` to `0x2e` or `0x2f` for the other
side; hyp: they open in the battle windows, whose pictures (STRATEGY
10524-10580) are 400 by 306.

Port messages with no picture: space battle results, blockade lifted,
betrayal, Death Star messages, the Headquarters destroyed, a system
destroyed and uprising incidents. The original's classes for the last four
set none; the others have no recovered class.

## Open

1. Original classes the port never posts: Near Uprising, loyalty changes,
   Facility Lost, Personnel Arrive, Injured and Healed, Captured, Enemy
   Mission Foiled, Informants, Smuggling, Recruitment Done, Research
   Exhausted, repairs, and the story lines listed as missing in
   `story-message-classes.md`.
2. Mission Failed (`FUN_00491500`): which outcomes go there and not to the
   report, and the `0x416` / `0x417` condition.
3. The Mission Report cases `0x55`, `0x58`, `0x61`-`0x63` and `0x69`, read
   only for their picture writes.
4. `FUN_004edc80`'s object in the Killed and Captured branches.
5. Titles and bodies: the port's messages keep their own wording.
6. Where the Assault and bombardment messages are shown, and with what
   layout.
