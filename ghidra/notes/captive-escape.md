# Captive escape (ESCAPETB)

Recovered 2026-10-14 from REBEXE.EXE (sha256 `b3fe3997…ed6ab`) with REA over
Ghidra 12.1.4, plus the saved decompile corpus
(`rebexe-decompile/decompiled`). Finding F-034 in the full-functionality
audit. The roll's inputs were written down in the 2026-10-14 overnight log;
this note closes its three open points: the cadence, slot `+0x1d4`, and the
side in `+0x24`.

## Chain

```
Captured (+0xac bit 0) changes            FUN_004ee800 -> slot +0x34c
  FUN_004f1a80(captured, ctx)
    CanEscape := captured                 FUN_004eeb10, +0xac bit 8
      arm/cancel timer 0x38a              FUN_004f1ba0 -> FUN_0053fa60
    EscapeRequest := 0                    FUN_004eeb80, +0xac bit 9
    side := captor or home                FUN_004eec60 (below)
    ...                                   FUN_004eef90, FUN_004ef070, FUN_004eeed0

timer 0x38a fires (vtable 0x00669970)     FUN_005862a0 -> slot +0x30 FUN_0057d330
  FUN_004ef150: if CanEscape,
    EscapeAttempt := 1, then 0            FUN_004eebf0, +0xac bit 10
      notify sides 1 and 2                FUN_004f1120 -> slot +0x3e8 FUN_004f23c0
        (log only: "CharacterEscapeAttemptNotif" via FUN_004f8880)
      raise event 0x317                   slot +0x370 FUN_004f1be0 -> FUN_0053f9c0(0x317)

event 0x317 fires (vtable 0x006699a8)     slot +0x20 FUN_0057d670
  FUN_00561990(captive, ...)              the roll, below
    on success: EscapeRequest := 1        FUN_004eeb80(1) -> slot +0x36c FUN_004f1bd0
      FUN_004ef0c0: if Captured, Usable (+0x50 bit 0), not on a mission
      (+0x78 bit 7) and EscapeRequest:
        slot +0x214 FUN_004ef570 -> FUN_004ee3e0(1): state +0x98 := 1
```

`FUN_004eef70` sets Captured to `(+0x98 == 0)`, so writing state 1 clears
Captured. That re-enters `FUN_004f1a80` with 0: CanEscape drops, which changes
the timer record and cancels timer `0x38a`, and the side returns home.

## Cadence

Timer `0x38a` uses record `+0x54` -> `+0x28`. `FUN_004ef750` fills it
(assembly `0x4ef786..0x4ef7a5`):

- minimum: GNPRTB 2584 (`DAT_006b90ac`, shipped 100)
- spread: GNPRTB 2585 (`DAT_006b9074`, shipped 1000)

Arming and firing follow `timer-scheduler.md`: the first attempt comes
`min + rand(0..=spread)` after capture, and each attempt reschedules with a
fresh draw. So a captive tries once every 100 to 1100 scheduler units, not
every tick. The unit is the one the port already uses for timer `0x38d` in
`uprising.rs`.

The same function fills two neighbouring records: `+0x18` minimum from
`DAT_006b9068` or `DAT_006b9064` (by `+0xac` bit 2, FastHeal), and `+0x20`
from GNPRTB 2582 (min, 1) and 2583 (spread, 0). `+0x18` is the healing timer
`0x388` and `+0x20` the Force discovery timer `0x389`
(`force-awareness.md`). Neither is part of escape.

## Roll

`FUN_00561990(captive, enabled, ctx)`:

```
loc = location of captive                                  FUN_004f6b50
side = captive +0x24 bits 6..7
guards = characters 0x30..0x3c at loc, usable (mode 1), of side,
         whose slot +0x1d4 returns 0                       FUN_004f2640
avg = sum(guard slot +0x1f0) / count(guards), 0 if none   integer division
troops = count of regiments 0x10..0x14 at loc, usable, of side   FUN_00504cc0
x = (captive +0x1e0 + captive +0x1f0) - avg - troops       FUN_0055e7e0
hit = ESCAPETB(x) roll                                     FUN_0053e310(0x2c)
```

- Slot `+0x1d4` for a character is `FUN_004edc70`: `+0xac & 1`, Captured.
  Guards are therefore the characters at the location that are not
  themselves prisoners.
- Slots `+0x1e0` and `+0x1f0` are `FUN_004edc00` and `FUN_004edc40`, the
  effective espionage (`+0x7e`) and effective combat (`+0x86`), as in
  `decoy-roll.md`.
- The side filter keeps children whose side equals the captive's
  (`FUN_005131d0`: `(child +0x24 >> 6 & 3) != filter` rejects). A captive's
  side is its captor's, see below, so the guards and troops are the captor's.
- `FUN_0053e310` wraps `FUN_0053e340`: step lookup in ESCAPETB, then a
  `0..99 < value` draw (`uprising-incident.md`).
- The handler runs only for the event's new value 1 (`param_2 != 0`); the
  pulse back to 0 does nothing.
- The guard and troop terms need CanEscape (`+0xac` bit 8) and a location;
  otherwise both are 0. A hit sets EscapeRequest regardless, and
  `FUN_004ef0c0` then requires Captured before anything moves. The return
  value (hit and CanEscape) only reports success to the caller.
- `FUN_004edc80`, passed as the first argument of `FUN_0055e7e0`, is unused
  there.

## The captive's side

`FUN_004eec60` runs on every Captured change:

```
home = FUN_004edc80(character)
if Captured and home != 3: side := (home == 1) ? 2 : 1
else:                     side := home
```

`FUN_004edc80` returns 1 when the class record has `+0x40` and not `+0x44`,
2 for the reverse, and 0 otherwise. The record is the DAT record shifted by
`0x28` (CAPSHPSD `maintenance_cost` is DAT `0x24` and class `+0x4c`), so
`+0x40`/`+0x44` are MJCHARSD/MNCHARSD `is_alliance`/`is_empire` at DAT
`0x18`/`0x1c`. It is the character's home side, not a command rank as
`personnel-finder.md` had it. A captured Alliance character carries the
Empire side, and the reverse, until it escapes or is freed.

## Port (2026-10-14)

`MissionSystem::advance_escapes` (`missions.rs`) replaces `check_escapes`,
which rolled every captive every day on loyalty. It keeps timer `0x38a` per
captive in `MissionState` (save v35), rolls ESCAPETB on `escape_input`, and
sends the escapee to `escape_destination` at the GNPRTB 1 default speed.
Port differences, tagged in the code:

- the timer arms on the first advance that sees the captive (the capture
  day when missions run first);
- the container and `+0x70` candidates are skipped, the move validator is
  not ported, and the prison itself is never a destination;
- a fleet's system is the destination, not the fleet;
- equal distances keep the arena order.

## Where an escaped character goes

`FUN_004f0580` only mirrors `+0x98` into the two side views. The change hook
is slot `+0x334`, `FUN_004f18e0`:

```
Captured := (+0x98 == 0)                  FUN_004eef70 -> chain above
FUN_004ef190(...)                         untraced (FUN_004f26d0 on a key)
Autorouting := 1                          GameObj_SetAutorouting, +0x50 bit 12
if on a mission: MissionRemoveRequest := 1
raise event 0x30a                         handler FUN_00516220, not read
```

Autorouting raises event `0x307`, whose handler `FUN_00515480` sends a
completed character (`0x30..0x3f`) to `FUN_00545df0` -> `FUN_00548da0`. That
takes the character's location (slot `+0xc`) and calls `FUN_00548e90`, which
gathers every autorouting or adrift (`+0x78` bit 6) character there, also
inside fleets, and runs `FUN_00549230` unless the location is a system in
battle (`+0x88` bit `0x1000`).

`FUN_0054c7d0` decides per character, grouping characters of the same side
and adrift state:

- the character's location is destroyed (`+0x50` bit 3) or belongs to
  another side (`+0x24` bits 6..7 differ), the character is not destroyed,
  and an object the decompile loses (`local_44`) has `+0x50` mask `0x4`
  set and `0x800` clear: pick a destination with `FUN_00543040`;
- otherwise clear Autorouting (`FUN_004f78e0(0)`): the character stays.

After an escape the character's side is home again and the captor still
holds the location, so it leaves. `FUN_00543040`:

- origin (`FUN_005431e0`): the location if it is a system (`0x90..0x97`),
  else the system key at `+0x74`, else a per-side default (the pushed keys
  are lost in the decompile);
- second key (`FUN_00543360`): `+0x70` when it is a system;
- `FUN_005545e0(character, origin, key70, 0, 1, &dest)` tries, in order,
  until one finds a destination:
  1. `FUN_00554740`: the current container (fleet `0x14..0x1b`, facility
     `0x08..0x0f` or system `0x90..0x97`; the object checked is lost in the
     decompile);
  2. `FUN_00554ab0`: the `+0x70` object, if it differs from the current
     location;
  3. `FUN_00554e80`: the nearest system of the character's side that is not
     blockaded (`+0x88` bit `0x20`);
  4. `FUN_00554f80`: the nearest system holding a fleet of the character's
     side (each such fleet, `FUN_004ffef0(system, 3, side)`);
  5. `FUN_005550e0`: the nearest blockaded system of the character's side;
  6. `FUN_005551e0`: the nearest neutral system (side bits `0xc0`);
  7. `FUN_005552e0`: the nearest system of the other side (skipped for a
     neutral character).
- A destination key of family `0x65` is replaced by `0x98000481`
  (`FUN_00543040`, end); meaning untraced.

"Nearest" is the order of the origin's row in the table at `+0x68`
(`FUN_00554150` -> `FUN_0058afd0`). `FUN_0058b060` builds one 200-entry row
per system: every system with its `FUN_0055d860` distance (integer square
root of the coordinate distance, as in `build-delivery.md`), sorted by
`qsort` with the comparator at `0x0058b140` (ascending; empty slots last).
Each candidate passes the move-order validator `FUN_005571c0`
(`FUN_005511d0`, `FUN_005513a0`, `FUN_005515d0`, then `FUN_00551630` issues
the move), and the first that validates wins. The character then travels
like any other object (`build-delivery.md`, Travel).

Port consequence: an escaped character does not reappear at home. It
travels from where it was held to the nearest unblockaded system of its own
side (in practice), with the transit time of `FUN_0055d8c0`.

## Still open

- `+0x74` on a character, and the per-side default origin. (`+0x70` is
  where the character joined its mission, `mission-lifecycle.md`, read
  2026-10-20; an escapee is on no mission, so its `+0x70` is clear.)
- `FUN_004ef190` and the handler of event `0x30a` (`FUN_00516220`).
- What `FUN_00505230` with range `1..0x12` checks before each fire
  (`FUN_0057d330`, `FUN_0057d670`).
