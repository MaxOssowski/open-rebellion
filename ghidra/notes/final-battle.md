# The Final Battle and Vader's pickup

Read 2026-10-18 for F-038. The port's Final Battle is its own story chain
(`story_events.rs`: `0x393` Vader Dispatched, `0x394` Vader En Route,
`0x395` Vader Reports to Emperor, `0x396` Final Battle Imminent, `0x220`
The Final Battle, on Dagobah's end, tick thresholds and a co-location
test). This note records the original, which the mission manager runs
through five flags and their hooks.

## The flags

The mission manager's `+0x58` holds five story bits (`setters.tsv`). Each
setter's change runs a hook slot of the manager (vtable `0x00661c08`) and
notifies the views:

| Bit | Flag | Setter | Hook | Notifier |
|---|---|---|---|---|
| `0x200` | DarthPickup | `FUN_00543d90` | `+0x1d8` `FUN_0054abc0` | `0x208` `FUN_0054b900` |
| `0x400` | DarthToLukeFinalBattle | `FUN_00543e00` | `+0x1dc` `FUN_0054ac70` | `0x20c` `FUN_0054b940` |
| `0x800` | DarthToEmperorFinalBattle | `FUN_00543e70` | `+0x1e0` `FUN_0054adf0` | `0x210` `FUN_0054b980` |
| `0x1000` | FinalBattleReady | `FUN_00543ee0` | `+0x1e4` `FUN_00524fb0` (returns 1) | `0x214` `FUN_0054b9c0` |
| `0x2000` | FinalBattle | `FUN_00543f50` | `+0x1e8` `FUN_0054b040` | `0x218` `FUN_0054ba00` |

The checks are among the manager's story checks (`FUN_00549910`), run in
this order: `FUN_005442f0`, `FUN_00544da0` (FinalBattleReady),
`FUN_00544030` (Dagobah), `FUN_00544130`, `FUN_005443f0`, `FUN_005444e0`,
`FUN_005445d0` (DarthPickup), `FUN_00544a20` (DarthToLuke),
`FUN_00544be0` (DarthToEmperor), `FUN_00544fc0` (FinalBattle). The
manager's handlers run them on many events (`dagobah.md`), among them
event `0x313` LukeFinalBattleReadyChanged (`FUN_00546e30`).

The characters are found by record, each with its location key
(`FUN_004f6b50`, the container `+0x1c`):
- Luke `0x32000242` (`FUN_005434b0`);
- Leia `0x31000241` (`FUN_00543550`);
- Han `0x33000243` (`FUN_005435f0`);
- Chewbacca `0x38000343` (`FUN_00543690`, a minor character,
  `FUN_005066a0`);
- Vader `0x35000281` (`FUN_00543760`);
- the Emperor `0x34000280` (`FUN_00543800`).

"Free" is `FUN_005439e0` -> `FUN_005439b0`: not captured (`+0xac` bit 0),
usable (`+0x50` bit 0), not autorouting (`+0x50` bit `0x1000`), and not
adrift (`+0x78` bit `0x40`) (`dagobah.md`).

## Luke's own FinalBattleReady (`+0xb0` bit 2)

`FUN_0056f7a0` sets it (`Luke_SetFinalBattleReady`, `FUN_0056f6c0`) to
`KnowsHeritage` (`+0xb0` bit 0) and Force `+0x8c` >= GNPRTB 3078 (60).
Luke's class runs it when his Force changes (slot `+0x31c`,
`FUN_0056fb80` over `FUN_004f16a0`) and when `KnowsHeritage` changes
(slot `+0x3ec`, `FUN_0056fbb0`). Its notification `LukeFinalBattleReady`
raises event `0x313`, whose handler runs the story checks.

## FinalBattleReady (`FUN_00544da0`)

While FinalBattle (`0x2000`) is clear, the flag is set to 1 only when all
of the following hold, and to 0 otherwise:
- Luke, Vader and the Emperor exist and have locations;
- Vader and the Emperor are free;
- Luke is captured (`+0xac` bit 0), usable (`+0x50` bit 0), neither on a
  mission nor adrift (`+0x78 & 0xc0` clear), and not autorouting;
- Luke is not held in Jabba's Palace (location `0x98000481`);
- Luke's own FinalBattleReady (`+0xb0` bit 2) is set.

So the chain starts once a captured Luke knows his heritage and has Force
60 or more.

## DarthToLukeFinalBattle (`FUN_00544a20`, hook `FUN_0054ac70`)

With FinalBattleReady set, and Luke, Vader and the Emperor located: when
Vader's location differs from Luke's, the flag pulses 1 then 0. The hook
sends Vader to Luke's location: `FUN_00547340(Vader's side, Luke's
location, {Vader})`, which creates a Move order (`0x41000001`,
`FUN_00547ba0`).

## DarthToEmperorFinalBattle (`FUN_00544be0`, hook `FUN_0054adf0`)

With FinalBattleReady set: when Vader is at Luke's location and the
Emperor elsewhere, the flag pulses. The hook acts only when Vader and Luke
are both on the Empire's side (`+0x24 & 0xc0 == 0x80`; Luke as the
Empire's prisoner) and neither is on a mission (`+0x78` bit `0x80`). It
then moves both to the Emperor's location (`FUN_00547340`, a Move
order).

## FinalBattle (`FUN_00544fc0`, hook `FUN_0054b040`)

With FinalBattleReady set: when Luke, Vader and the Emperor share a
location, FinalBattle is set to 1 and FinalBattleReady is re-run, which
clears it since FinalBattle is now set. The hook fights the battle once
the three still share a location:

- At a system (`0x90..0x97`), both sides observe it at level 6
  (`FUN_0050d5a0(6, 1)`, `(6, 2)`).
- When Luke's Force `+0x8c` < GNPRTB 6151 (100), the Empire wins
  (result 2):
  - Luke's `CanEscape` (`+0xac` bit `0x100`) is cleared (`FUN_004eeb10(0)`);
  - Luke is injured by rand(0..=GNPRTB 6153 (199)) + 6152 (1)
    (`FUN_00549a20`, `FUN_0055c650`, slot `+0x2f0`).
- Otherwise Luke wins (result 1):
  - Luke's slot `+0x218` (`FUN_004ef5b0`) frees him: `FUN_0051ebb0`, then
    status `+0x98` = 2 (`FUN_004ee3e0`);
  - Vader and the Emperor are captured with Luke as captor (slot
    `+0x20c`, `FUN_004ef480`);
  - their `CanEscape` is cleared.
- Then, when the winner (Luke after a win, Vader after a loss) stands in
  a container of another side (`+0x24` of the location against the
  character's, disassembly `0x54b589`..), the three (a set of three keys
  built at `0x54b5b6`; hyp: Luke, Vader and the Emperor) move to the
  winner's autoroute destination (`FUN_00543040`) by `FUN_00547480`,
  another Move order (`0x41000001`). The hook's return also checks that
  the three now share a side (`0x54b543`..`0x54b567`).
- `FUN_004f9510` is called around the battle on both sides (not read).

The class `0x65b408` sets its bit `0x10` on `0x220`
(`MissionMgrFinalBattleNotif`) and builds The Final Battle
(`FUN_0048e3c0`) on notification 2 for each side:
- title `0x71e0` "The Final Battle";
- body `0x71e1` (Luke free: "Luke confronted and overcame the Emperor.
  Both Emperor Palpatine and Darth Vader were lost.") or `0x71e2` (Luke
  captured: "The Emperor confronted Luke and captured him.");
- background `0x428` when Luke's `DagobahCompleted` (`+0xb0` bit 8) is
  set, else `0x427`.

## DarthPickup (`FUN_005445d0`, hook `FUN_0054abc0`)

The flag pulses when all of the following hold:
- Luke, Leia, Han, Chewbacca and Vader exist and have locations;
- Luke, Leia and Han are captured;
- Chewbacca is captured or `+0x50` bit 8 is set;
- some character in Jabba's Palace (the galaxy's `+0xdc`,
  `FUN_00506ed0`, walked by `FUN_004f2640(palace, 1, 2)`) is captured;
- Vader is free and not on a mission (`+0x78` bit `0x80`).

When Vader is on a mission instead, the check does nothing if it is a
Pickup (family `0x73`). Otherwise it asks him to resign
(`FUN_00534640`), provided he may resign (`+0x78` bit `0x10`) and holds a
mission key.

The hook creates the Pickup mission (`FUN_005479c0`): record
`0x73000082` (TEXTSTRA 11394), target Jabba's Palace `0x98000481`,
member Vader.
- MISSNSD: Empire only, hidden, 0 + rand(0..=0) days, detection on, no
  resigning; the target is the Empire's own.
- Class `0x00665058`: chance `+0x274` 100 (`FUN_00575060`). The hit
  `+0x27c` `FUN_0056da80`:
  - sets the character manager's PickupInProgress (`+0x58` bit 2,
    `FUN_0055f6c0`);
  - walks the palace's characters of the mission's side. Each one that
    is captured and not on a mission joins the mission (slot `+0x1cc`,
    `FUN_00534390`), and the result is 3;
  - clears PickupInProgress.

## Against the port

- The port has none of the five flags, Luke's FinalBattleReady, the
  Pickup mission, or the battle's outcome. Its `0x393`..`0x396` and `0x220`
  story events fire on Dagobah's end, a tick threshold and the three
  characters being together. They take no outcome; the
  `SetHeritageKnown` action went in save v75.
- The port has Move orders (`0x41000001`, F-040), abodes (Jabba's Palace
  `0x98000481`), autorouting (`FUN_00543040`), injuries, captures, and
  `KnowsHeritage` from the confrontation (`encounter.md`).
- DarthPickup needs Luke, Leia and Han all captured, and a captive in
  Jabba's Palace. The port's palace captures are its own story events
  (`0x380`..`0x39a`), and it has no Palace mission (`0x64`).

## Plan

1. FinalBattleReady path: Luke's flag, the four checks, the Move orders,
   the battle and its message; remove the port's `0x393`..`0x396` and
   `0x220` chain.
2. DarthPickup and the Pickup mission, after Jabba's palace (the Palace
   mission `0x64`, HanCapturedAtPalace and the Bounty Hunters) is read.

## Open

- `FUN_004f9510` (both sides, around the battle).
- The checks `FUN_005442f0`, `FUN_00544130`, `FUN_005443f0`,
  `FUN_005444e0`.
- Status `+0x98` value 2 (Luke's slot `+0x218`); 0 is captured and 1 free
  (`decoy-roll.md`).
- `FUN_00547480`'s extra argument from `FUN_005834d0`.
- Character `+0x50` bit 8 (Chewbacca's alternative to captured).
