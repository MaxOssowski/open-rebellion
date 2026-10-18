# Luke's Dagobah mission

Read 2026-10-18 for F-038 and F-040. The port's Dagobah chain lives in
`story_events.rs` and is invented (Yoda, tick thresholds, Force tiers).
This note records the original.

## The trigger: timer `0x392` and LukeDagobahRequired

- The mission manager's setup (`FUN_00549d60`) arms timer `0x392` on its
  record `+0x5c`, with minimum GNPRTB 6146 (300) and spread 6147 (100), so it
  fires once, 300 to 400 days into the game. The setup only arms it when
  the manager's `+0x24` bits `0x30` are clear (game type, not traced). The
  same setup also sets up `BountyHuntersActive`'s timer `0x393` (GNPRTB
  6148/6149, 300 + rand(300)).
- The fire (`FUN_0054e620` -> `FUN_00546f80`) sets the manager's `+0x58`
  bit 0, `LukeDagobahRequired` (`FUN_00543af0`). Its hook (slot `+0x1c0`,
  `FUN_0054a800`) runs the Dagobah check (`FUN_00544030`) and disarms the
  timer.

## The Dagobah check (`FUN_00544030`)

With `LukeDagobahRequired` set and `HanCapturedAtPalace` (`+0x58` bit
`0x40`) clear:
- Luke (`0x32000242`) must exist and have a location (`FUN_005434b0`,
  `FUN_00507010`, `FUN_004f6b50`);
- he must be free (`FUN_005439e0` -> `FUN_005439b0`): not captured (`+0xac`
  bit 0), usable (`+0x50` bit 0), not autorouting (`+0x50` bit `0x1000`),
  not adrift (`+0x78` bit `0x40`), not on a mission (`+0x78` bit `0x80`);
- then `LukeDagobah` (`+0x58` bit 2) pulses 1 then 0 (`FUN_00543b60`),
  which sends notification `0x221` (Luke Goes to Dagobah, `FUN_0048d190`,
  TEXTSTRA `0x71b8`/`0x71b9`).

The check is one of the mission manager's story checks
(`FUN_00549910`), which its handlers run on many events (`FUN_00545240`,
`FUN_005457f0`, `FUN_00545540`, `FUN_00545df0`, `FUN_00545e90`,
`FUN_00546370`, `FUN_00546e30`, `FUN_00548840`). So once required, Luke
leaves at the first such event that finds him free.

## The mission

`LukeDagobah`'s hook (slot `+0x1c4`, `FUN_0054a860`) creates it
(`FUN_005479c0`): record `0x71000043`, target `0x98000441`, member Luke.

- MISSNSD `0x71000043` (TEXTSTRA 11331 "Dagobah"): Alliance only, hidden,
  100 + rand(0..=0) days, detection on, no resigning; the target is of
  "another" side.
- The target `0x98000441` is the Dagobah abode (ABODESD 1089). The galaxy's
  setup (`FUN_00519910`) creates five abodes as children of the galaxy, not
  of a system: Alliance Jail `0x98000440` (side 1), Dagobah `0x98000441`
  (side 3), Hermitage `0x98000442` (3), Empire Jail `0x98000480` (2) and
  Jabba's Palace `0x98000481` (2), stored at galaxy `+0xc0..+0xdc`. The port
  loads none of them.
- Class `DagobahMission` (vtable `0x00667f38`): chance `+0x274` 100
  (`FUN_00575060`); hit `+0x27c` sets 3 (`FUN_00572470`); `+0x280` sets 2
  when unset (`FUN_00572490`).
- Its phase slot `+0x1f4` (`FUN_005751d0`):
  - phase 8: `FirstTrainingDay` `+0xa8` = today
    (`DagobahMission_SetFirstTrainingDay`);
  - phase `0xb`, once phase 8 was reached and with a first training day:
    Luke's Force becomes Force + Force x GNPRTB 6174 (60) / 100 on a
    success (`FUN_0055ca60`), else Force + Force x ((today - first day) /
    GNPRTB 6175 (2)) / 100 (`FUN_0055ca90`). The difference goes to
    ForceExperience `+0x8e` (`FUN_004ee150`), and Luke's
    `DagobahCompleted` (`+0xb0` bit `0x8`, `FUN_0056f730`) is set, whose
    notification is `0x210` (Luke Leaves Dagobah, `FUN_0048d0f0`).

## When the requirement clears, and how Luke comes back (read 2026-10-18)

- The mission manager's phase notice `FUN_00548840` watches record
  `0x71000043`: entering phase 8 runs the story checks (`FUN_00549910`);
  entering phase `0xb` after phase 8 or later (`FUN_00520ac0() >= 8`) clears
  `LukeDagobahRequired` (`FUN_00543af0(0)`). A Dagobah mission that ends
  before its timer (Luke captured on the way, say) leaves it set, and the
  next check sends him again.
- `FUN_00544030`'s third out value is Luke's location key, which
  `FUN_004ece40` only tests for the null key (disassembly
  `0x544087..0x5440af`). No other guard stops a second trip.
- The members stay at Dagobah when the mission ends. The manager's
  handler `FUN_00546370` then calls each member's slot `+0xb0`: for a
  character not on a mission and not destroyed, `FUN_004ef260` sets
  Autorouting (`+0x50` bit `0x1000`) when its container's side differs
  from its own. Dagobah is side 3, so Luke autoroutes: event `0x307` ->
  `FUN_0054c7d0` -> `FUN_00543040` picks his destination, as for an escaped
  captive (`captive-escape.md`), and the trip from an abode takes 1 day.

## Abodes and travel to them (read 2026-10-18)

- Class: the object factory's registry (`FUN_00540540`, filled by
  `FUN_0051ebf0`) maps family `0x98` to `FUN_00577530`, which builds a
  `0x58`-byte object with vtable `0x00669260` (`GameObj_00669260`, 111
  slots, parent `GameObj`). Its slot `+0x4` returns `0x98`; families `0x90`
  and `0x92` (systems) and `0x80` (sectors) have their own entries.
- Like a system, its slot `+0xc` (`FUN_00577130`, the same body as the
  system's `FUN_00508070`) answers its own key as the place it is in, so
  `FUN_00555540` resolves an abode to itself.
- Missions treat systems and abodes alike as places: `FUN_00521070` and
  `FUN_00520f40` accept a target of families `0x90..0x97` or `0x98..0x9f`.
- Travel (`FUN_00556430` -> `FUN_00555d30` -> `FUN_00555b30`): between two
  different places the time is set to 1 first, and only when both are
  systems (`0x90..0x97`) does `FUN_0055d8c0` replace it with the distance
  time. So a trip to or from an abode takes 1 day, wherever it starts.
- An abode has no position (ABODESD holds no coordinates) and is a child
  of the galaxy, not of a system or sector.

## Luke's `+0xb0`

Bit 0 `KnowsHeritage` (`FUN_0056f650`), bit 1 `FinalBattleReady`
(`FUN_0056f6c0`), bit 3 `DagobahCompleted` (`FUN_0056f730`). Leia's bit 0
is her own `KnowsHeritage`. Han's bits are `BountyAttack` (0) and
`CapturedByBountyHunters` (1).

## Against the port

- Abodes, first stage (2026-10-18): `GameWorld::abodes` from ABODESD with
  the setup's sides, and `Character::current_abode`
  (`evidence/2026-10-18-abodes/`). Missions do not target them yet.
- `story_events.rs` starts "Dagobah" from invented conditions (Luke at
  Force tier Aware, tick 50) with Yoda, and advances Force tiers.
- Needed to port: the five abodes (load ABODESD, place them in the galaxy,
  let missions target and reach them), timer `0x392`, the Dagobah check,
  the mission with its phase-8 and phase-`0xb` slots, `DagobahCompleted`,
  and the two messages.

## Open

- The manager's `+0x24` bits `0x30` (when the timer is not armed).
- `FUN_004ece40`'s test inside the check.
- Whether and where the interface shows an abode.
