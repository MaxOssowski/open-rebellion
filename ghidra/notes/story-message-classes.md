---
title: "Story and special-event message classes"
description: "Every story and special-event message the original can post: its notification, builder, texts, picture and game-side sender"
category: "ghidra"
created: 2026-10-13
updated: 2026-10-13
tags: [story-events, special-events, messages, notifications, textstra]
---

# Story and special-event message classes

Extends the "Story message classes" table in `message-index-rows.md`, which
listed five builders. This note lists all of them. The pictures of the
ordinary reports are in `message-pictures.md`.

## Method

1. Every message title and body is a TEXTSTRA `RT_RCDATA` record
   (`textstra-messages.json`, 510 records). The story and special-event
   texts are `0x7150`-`0x7219`.
2. Each id was located in `REBEXE.EXE` (GOG build, md5
   `b8869448d072582d1a59d2e071ea4ee7`) as a 32-bit immediate. All sit in
   `0x48ab00`-`0x49a000`.
3. Most of those functions are reached only through vtables, so Ghidra's
   auto-analysis had not created them: 104 vtable targets in
   `0x48a000`-`0x49b000` had no function. They were created and decompiled
   read-only; the output is outside the repository
   (`~/Games/rebexe-decompile/vtable-targets/`).

## Class layout

The message classes' vtables run from `0x65b0b0` to `0x65bc10`, 15 or 17
slots each. A class handles a notification in one of four slots, `+0x14`,
`+0x18`, `+0x1c` or `+0x20`; which slot is by the notification's argument
shape (hyp:). A builder writes these fields of the message:

| Field | Meaning |
|---|---|
| `+0x20` | the side the message is built for: 1 Alliance, 2 Empire |
| `+0x40` | 1 when this side is shown the message |
| `+0x2e` | background picture (STRATEGY) |
| `+0x2c` | foreground picture, a character portrait (`FUN_004c5000`) |
| `+0x30` | media started from module 7 (`FUN_00469de0`); hyp: the sound |
| `+0x32` | media started from module 9, set through `FUN_004c4990(n)`; hyp: voice |
| `+0x28` | `0x15` generic, `0x16` Luke, `0x17` Mon Mothma, `0x18` Leia, `0x19` Han, `0x1a` Emperor, `0x1b` Vader (`FUN_0048db30`); its use is untraced |
| `+0x44`, `+0x50` | title and body (`FUN_0060b9d0`, four parameters) |
| `+0x64` | per-class state: a fired-once flag or accumulated bits |

Character ids: Mon Mothma `0x30000240`, Leia `0x31000241`, Luke
`0x32000242`, Han `0x33000243`, Emperor `0x34000280`, Vader `0x35000281`.

## Story lines

"Port" is the state of `story_messages::story_class` on 2026-10-13.

| # | Story line | Notification | Builder (vtable) | Background | Title / body | Shown to | Port |
|---|---|---|---|---|---|---|---|
| 1 | Luke Goes to Dagobah | `0x221` | `FUN_0048d190` (`0x65b380`) | `0x421` | `0x71b8` / `0x71b9` | — | mapped |
| 2 | Luke Leaves Dagobah | `0x210`, arg set | `FUN_0048d0f0` (`0x65b380`) | `0x412`, Luke's portrait | `0x71ba` / `0x71bb` | Alliance only | missing |
| 3 | Force Growth | `0x1e1` | `FUN_0048ed80` (`0x65b4d8`) | side `0x412`/`0x413`, portrait | `0x7180` / `0x7181` | — | mapped |
| 4 | Future Jedi Discovered | `0x362` | `FUN_0048f1e0` (`0x65b520`) | side, the teacher's portrait | `0x7178` / `0x7179` or `0x717a` | — | missing |
| 5 | Leia Uses Force | `0x362`, Leia | `FUN_0048f1e0` | side, Leia's portrait | `0x71d8` / `0x71d9` | — | mapped |
| 6 | Force user Detects Enemy | `0x360`, then 2 | `FUN_0048e1d0` (`0x65b408`) | side, the detector's portrait | `0x71b0` / `0x71b1` | — | missing |
| 7 | Character Confronts character | `0x360`, then 2 | `FUN_0048df20` (`0x65b408`) | `0x41f`, `0x420`, `0x423`-`0x426` | `0x71a8` / `0x71a9` + `0x71aa`-`0x71ae` | — | missing |
| 8 | Luke Discovers His Heritage | `0x360` + `0x211`, then 2 | `FUN_0048dc50` (`0x65b408`) | `0x422` | `0x71a0` / `0x71a1` + `0x71a2`-`0x71a7` | — | missing |
| 9 | The Final Battle | `0x220`, then 2 | `FUN_0048e3c0` (`0x65b408`) | `0x427`, or `0x428` | `0x71e0` / `0x71e1` or `0x71e2` | — | missing |
| 10 | Han Solo Attacked by Bounty Hunters | `0x212`, arg 0 | `FUN_0048fa00` (`0x65b5a8`) | `0x41c` | `0x7168` / `0x7169` | Alliance | missing |
| 11 | Bounty Hunters Locate Solo | `0x212`, arg 0 | `FUN_0048fa00` | `0x41e` | `0x716a` / `0x716b` (system) | Empire | missing |
| 12 | Han Solo Captured by Bounty Hunters | `0x212`, arg 0 | `FUN_0048fa00` | `0x41d` | `0x716c` / `0x716d` | both | missing |
| 13 | Rescue Attempt | `0x200` | `FUN_0048c4c0` (`0x65b2a8`) | side, the rescuer's portrait | `0x71ea` / `0x71eb` | — | missing |
| 14 | Jabba Captures a character | `0x1e0` case 5 | `FUN_00490340` (`0x65b638`) | `0x429` | `0x7156` / `0x7157` | — | mapped |
| 15 | Jabba Delivers Prisoners | `0x231`, arg set, once | `FUN_0048b6e0` (`0x65b1d0`) | `0x42a`, or `0x42b` | `0x7200` / `0x7201` | both | missing |
| 16 | Emperor Arrives at Coruscant | `0x230`, arg set, once | `FUN_0048ce80` (`0x65b338`) | `0x42c` | `0x71c8` / `0x71c9` | Empire only | missing |
| 17 | Character Discovers Traitor | `0x361` | `FUN_0048c9c0` (`0x65b2f0`) | side, the discoverer's portrait | `0x71e8` / `0x71e9` | one side, by `FUN_004edc80` | missing |
| 18 | Killed by Imperial Assassins | `0x306` | `FUN_0048b050` (`0x65b140`) | `0x42e` | `0x7208` / `0x7209` | Alliance only | missing |
| 19 | Death Star Sabotaged | `0x305`, Death Star target | `FUN_0048d470` (`0x65b3c8`) | — | `0x71bd` / `0x71be` (system) | — | missing |
| 20 | Coruscant Captured | `0x128` | `FUN_0048f790` (`0x65b568`) | — | `0x7170` / `0x7171` | when the Alliance holds `0x90000109` | missing |
| 21 | Alliance Headquarters Destroyed | `0x128`, arg bit 0 | `FUN_0048e5a0` (`0x65b450`) | — | `0x7188` / `0x7189` | both | missing |

Notes on the rows:

- **2.** The body is in the first person ("I have finished my training with
  Yoda"); the picture is Luke over the Alliance background.
- **6-9.** One class (vtable `0x65b408`) owns all four. Its `+0x14` slot
  `FUN_0048d9a0` takes `0x360` (`CharacterEncounterKeyNotif` / `Encounter`)
  and files the two characters at `+0x68` and `+0x6c`. Its `+0x20` slot
  `FUN_0048d7f0` sets bit 0 on `0x211` (`LukeKnowsHeritageNotif`) and bit
  `0x10` on `0x220` (`MissionMgrFinalBattleNotif`), then builds on
  notification 2: the Final Battle when bit `0x10` is set; otherwise, for
  the Luke and Vader pair, the heritage message when bit 0 is set and the
  confrontation when not; for any other pair, the enemy-detected message.
- **7.** The background is by pair: `0x41f` or `0x420` when the first
  character is not Luke (second is Vader, or not); `0x423`/`0x425` for
  Luke against Vader or another, and `0x424`/`0x426` for the same when
  Luke's flag `+0xb0 & 8` is set. hyp: that flag is Luke's Jedi appearance,
  since it also picks between two media ids in every Luke case.
- **8.** Bodies: `0x71a2` Luke taken to the Emperor, `0x71a3` Luke captured,
  `0x71a4` Luke injured (`+0x94` non-zero), `0x71a5` Luke escaped; then
  `0x71a6` Vader captured or `0x71a7` Vader injured.
- **9.** `0x71e2` when Luke is captive (`+0xac & 1`), else `0x71e1`;
  background `0x428` when Luke's `+0xb0 & 8` is set.
- **10-12.** One builder, by Han's captive bit (`+0xac & 1`): captured gives
  row 12; otherwise the Alliance gets row 10 and the Empire row 11, the
  latter only when Han is at a system (class `0x90`-`0x97`).
- **15.** Background `0x42b` when Luke's `+0xb0 & 8` is set.
- **17.** The speaker and its media follow the discoverer: Leia, Luke
  (two variants), Emperor and Vader have their own (`FUN_004c4990`
  `0x127`, `0x125`/`0x126`, `0x129`, `0x128`).
- **19.** The same `0x305` reaches `FUN_0048b9a0` (Saboteurs Strike,
  `0x7083`); this class answers only when the destroyed unit's class is
  `0x18`-`0x1b` and it was at a system.
- `0x71af` ("Luke has been greatly effected by the encounter and has gone
  into hiding") is referenced nowhere in `REBEXE.EXE`. hyp: cut.

## Special events

| Event | Notification | Builder (vtable) | Background | Title / body |
|---|---|---|---|---|
| Natural Disaster | `0x154`, then 2 | `FUN_0048e940` (`0x65b490`) | `0x3eb` | `0x7190` / `0x7191`-`0x7193`, units listed by `FUN_0048e890` on `0x302` (`0x719b`) |
| New Resources / Resources Depleted | `0x155`, then 2 | `FUN_0048e940` | `0x3ea` | `0x7194` / `0x7199`, `0x719a`; `0x7196` / `0x7197`, `0x7198` |
| Character Injured / Healed | `0x106` | `FUN_0048fdf0` (`0x65b5f0`) | `0x41a` / `0x41b` + portrait | `0x7160` / `0x7161`; `0x7162` / `0x7163` |
| Character Killed | `0x106`, dead | `FUN_0048fdf0` | `0x42e`, or `0x433` | `0x7164` / `0x7365` |
| Captured / Escaped | `0x1e0` | `FUN_00490340` (`0x65b638`) | `0x418`/`0x419`, `0x405`, side | `0x7158`-`0x715f` |
| Informants Provide Information | `0x153` | `FUN_00491060` (`0x65b680`) | `0x3e8` | `0x7150` / `0x7151` |
| Saboteurs Strike | `0x305` | `FUN_0048b9a0` (`0x65b218`) | `0x3f5`/`0x3f6`, set in slot `+0x40` `FUN_0048bbf0` | `0x7083` / `0x7084`, `0x7085` |
| Maintenance Shortfall | `0x304` | `FUN_00498970` | `0x3f5`/`0x3f6`, set in slot `+0x40` `FUN_0048bbf0` | `0x7080` / `0x7081`, `0x7082` |
| Recruitment Done | `0x12c` | `FUN_0048ae40` (`0x65b0f8`) | side `0x412`/`0x413` | `0x7210` / `0x7211` |
| Research Exhausted | `0x129`, `0x12a`, `0x12b` | `FUN_0048abc0` (`0x65b0b0`) | `0x3f9` | `0x7218` / `0x7219`, `0x721a`, `0x721b` |
| Facility Idle | `0x160` | `FUN_0048be60` (`0x65b260`) | by facility | `0x71f8`-`0x71fd` |
| Smuggling | `0x145` | `FUN_00499130` | `0x3ec` | `0x7060`-`0x7067` |
| Near Uprising | `0x148` | `FUN_00498cb0` | `0x3f0`/`0x3f1` | `0x7070` / `0x7071` |
| Uprising Begins / Ends | `0x14b` | `FUN_00499460` | `0x3f2`-`0x3f4` | `0x7030`-`0x7033` |
| System Joins / Declares Neutrality | `0x100` | `FUN_00499760` | `0x3ed` | `0x7038`-`0x703d` |
| System Destroyed by the Death Star | `0x182`, then 2 | `FUN_004950d0` | — | `0x70c8` / `0x70c9`, `0x70ca` |
| Message From / Sent to the other side | — | `FUN_00498e90` | `0x410`/`0x411` | `0x71f0`, `0x71f1` |

The class at vtable `0x65b188` (`FUN_0048b2e0`, backgrounds `0x42f`/`0x430`)
is the droid's advice: its texts are `0x6001 + 3n` (Alliance) and
`0x6801 + 3n` (Empire), indexed from the RCDATA tables `0x6000` and
`0x6800` by `FUN_0048b460`. It is not a story.

## Game-side senders

| Notification | Sender | Names in the executable |
|---|---|---|
| `0x200` | `FUN_005365b0` | `RoleEnrouteActiveNotif` / `EnrouteActive` |
| `0x210` | `FUN_0056fc70` | `LukeDagobahCompletedNotif` / `DagobahCompleted` |
| `0x211` | `FUN_0056fbe0` | `LukeKnowsHeritageNotif` / `KnowsHeritage` |
| `0x212` | `FUN_00572b40` | `HanBountyAttackNotif` / `BountyAttack` |
| `0x220` | `FUN_0054ba00` | `MissionMgrFinalBattleNotif` / `FinalBattle` |
| `0x221` | `FUN_0054b7b0` | `MissionMgrLukeDagobahNotif` / `LukeDagobah` |
| `0x230` | `FUN_00562450` | `CharacterMgrSeatOfPowerNotif` / `SeatOfPower` |
| `0x231` | `FUN_005624a0` | `CharacterMgrPickupInProgressNotif` / `PickupInProgress` |
| `0x154` | `FUN_00512620` | `SystemDisasterIncidentNotif` / `DisasterIncident` |
| `0x155` | `FUN_00512670` | `SystemResourceIncidentNotif` / `ResourceIncident` |
| `0x1e0` | `FUN_004f1fd0` | — |
| `0x305` | `FUN_00511300` | — |
| `0x306` | `FUN_004fba10` | also arms timer `0x387` (`FUN_0053fa60`) |
| `0x360` | `FUN_004f2040` | `CharacterEncounterKeyNotif` / `Encounter` |
| `0x361` | `FUN_004f2090` | `CharacterTraitorDiscoveredKeyNotif` / `TraitorDiscovered` |
| `0x362` | `FUN_004f20e0` | `CharacterForceUserDiscoveredKeyNotif` / `ForceUserDiscovered` |

These senders are virtual and have no direct callers, so the conditions
that raise each one (the story's trigger) are not recovered here.

Notification names with no message class of their own, which gate the
chains above: `LukeDagobahRequired`, `LukePalace`, `HanCapturedAtPalace`,
`LeiaPalace`, `ChewbaccaPalace`, `DarthPickup`, `DarthToLukeFinalBattle`,
`DarthToEmperorFinalBattle`, `FinalBattleReady` (two senders),
`BountyHuntersActive`, `CapturedByBountyHunters`, `LeiaKnowsHeritageNotif`,
`DagobahMissionFirstTrainingDayNotif`, `ForceAware`, `ForcePotential`,
`DiscoveringForceUser`, `ForceExperience`.

## Port gaps

- `events.rs` comments name `0x305` "saboteur detected" and `0x306`
  "character killed in action". The original's `0x305` is a unit destroyed
  by sabotage and `0x306` an assassination.
- `EVT_NATURAL_DISASTER` (`0x154`) and `EVT_RESOURCE_DISCOVERY` (`0x155`)
  are marked unverified in `events.rs`; `FUN_00512620` and `FUN_00512670`
  raise them under the names above.
- Port events that share an original id and meaning and so could take
  their class next: `0x210` (row 2), `0x212` (rows 10-12), `0x230`
  (row 16), `0x231` (row 15), `0x361` (row 17).
