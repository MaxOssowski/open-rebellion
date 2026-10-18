# Jabba's palace: the bounty hunters, the Palace mission and the pickup

Read 2026-10-18 for F-038. The port's Han and Jabba story is its own
chain (`story_events.rs`):
- `0x397` Bounty Hunters Active, `0x212` Bounty Hunter Attack, `0x398`
  Han Frozen in Carbonite and its countdown;
- `0x380` Luke Senses Han's Capture, `0x381` Leia Plans Rescue, `0x382`
  Chewbacca Joins Rescue, `0x383` Han Rescued, `0x384` Han Escapes
  Carbonite;
- `0x385`, `0x386` and `0x399`, the palace captures;
- `0x39a`, `0x231` and `0x39b`.

They run on tick thresholds and random gates. This note records the
original, which the mission manager runs with its `+0x58` bits, timer
`0x393`, two missions and Han's own flags. The Final Battle side
(DarthPickup, the Pickup mission) is in `final-battle.md`.

## The bounty hunters

- The manager's setup (`FUN_00549d60`) arms timer `0x392` (Dagobah) and
  then sets BountyHuntersActive (`+0x58` bit `0x4000`, `FUN_00543fc0`).
  Its hook (slot `+0x1ec`, `FUN_0054b740`) arms timer `0x393` on the
  manager record `+0x64` (`FUN_0053fa60(0x393, ...)`), with GNPRTB 6148
  (300) + rand(0..=6149 (300)) days (`DAT_006bb550`, `DAT_006bb548`).
  Clearing the bit cancels the timer.
- Timer `0x393` (vtable `0x00661f80`) fires once (`FUN_0054e970`) and runs
  `FUN_00546fa0`. With a chance of GNPRTB 6150 (100) percent
  (`FUN_0053e2f0`), and Han (`0x33000243`) located, not captured and
  usable, it:
  - creates a Bounty Hunters unit (SPECFCSD `0x3c000009`, TEXTSTRA 9604:
    Empire, every skill 0, loyalty 100, mission mask `0x1000`) in Jabba's
    Palace (`FUN_004f7d50` on the palace, the galaxy's `+0xdc`) for the
    palace's side;
  - gives the Empire observation level 5 of Han's location
    (`FUN_00556c70(Han, 2)`);
  - creates the Bounty mission (`FUN_005479c0`): record `0x65000083`
    (TEXTSTRA 11395), target Han, member the new unit.
- MISSNSD `0x65000083`: Empire only, hidden, 0 + rand(0..=0) days, no
  detection, no resigning, special forces with mask `0x1000` (the
  Bounty Hunters) only, an opponent's free character as target.
- Class `0x006685c0` (family `0x65`): chance `+0x274` 100
  (`FUN_00575060`). The hit `+0x27c` (`FUN_00575de0`), with the target of
  family `0x33` (Han):
  - pulses Han's BountyAttack (`+0xb0` bit 1, `FUN_00572690`; notification
    `0x212`);
  - Han rolls RLEVADTB against the bounty hunters (`FUN_005349e0`, captor
    combat 0): he evades, or he is captured with the mission as his captor;
  - when captured (result 3), Han loses `CanEscape` (`FUN_004eeb10(0)`)
    and gets CapturedByBountyHunters (`+0xb0` bit 2, `FUN_00572700`);
    otherwise the result is 2.
- A captured character's `+0x98` change sets Autorouting
  (`FUN_004f18e0`). `FUN_00543040` replaces an autoroute destination of
  family `0x65`, the Bounty mission that captured Han, with Jabba's Palace
  `0x98000481` (`captive-escape.md`). So Han travels to the palace.

## HanCapturedAtPalace and the rescues

The story checks (`FUN_00549910`, see `final-battle.md`) include:

- `FUN_005442f0` sets HanCapturedAtPalace (`+0x58` bit `0x40`) to 1 when
  Han is captured, usable, neither on a mission nor adrift (`+0x78 &
  0xc0`), not autorouting, and located in Jabba's Palace; else to 0. Its
  hook (`FUN_0054a9c0`), on 1:
  - clears BountyHuntersActive, which cancels timer `0x393`;
  - gives the Alliance observation level 5 of Han's location
    (`FUN_00556c70(Han, 1)`).
- `FUN_00544130`, with HanCapturedAtPalace set and Luke located and free
  (`FUN_005439b0`):
  - Luke on no mission: LukePalace (`+0x58` bit `0x20`) pulses;
  - Luke on the Dagobah mission (family `0x71`) in phase 8: its slot
    `+0x1d8` is called (not read; hyp: it ends the training early).
- `FUN_005443f0`: with HanCapturedAtPalace set and Leia located and free
  of a mission (`FUN_005439e0`), LeiaPalace (`+0x58` bit `0x80`) pulses.
- `FUN_005444e0`: the same for Chewbacca (`0x38000343`) and
  ChewbaccaPalace (`+0x58` bit `0x100`).
- The Dagobah check needs HanCapturedAtPalace clear (`dagobah.md`).

The hooks of LukePalace (`FUN_0054a910`), LeiaPalace (`FUN_0054aa60`) and
ChewbaccaPalace (`FUN_0054ab10`) each create a Palace mission
(`FUN_005479c0`): record `0x64000044` (TEXTSTRA 11332), target Han, member
Luke, Leia or Chewbacca.

- MISSNSD `0x64000044`: Alliance only, hidden, 5 + rand(0..=10) days,
  detection on, no resigning, an opponent's prisoner as target.
- Class `0x006652e8` (family `0x64`):
  - chance `+0x274` (`FUN_0056e620` -> `FUN_0055c910`): espionage
    (slot `+0x1e0`) / GNPRTB 6154 (3) + combat (slot `+0x1f0`) / GNPRTB
    6155 (3) (disassembly `0x56e62e`..`0x56e640`).
  - The hit `+0x27c` (`FUN_0056e650`) rolls the chance (`FUN_0053e2f0`).
    On a success, the result is 3 and:
    - the member's base combat `+0x62` rises by GNPRTB 6170 (1) and its
      base espionage `+0x5a` by GNPRTB 6169 (1) (`FUN_005340a0`,
      `FUN_00533ea0`), when its slot `+0x1d8` holds;
    - the target (Han) is released by the member (slot `+0x210`,
      `FUN_004ef510`);
    - every other captured character in the target's container (the
      palace, `FUN_00521050`) is released by the member too.
  - `+0x280` (`FUN_0056e870`), when no member succeeded (`+0x60 == 0`):
    the result is 2, and every member is captured with the mission's
    captor key (slot `+0x20c`) and loses `CanEscape`. So a failed rescuer
    stays at the palace for good. This is Jabba Captures a character
    (`0x1e0` case 5, `FUN_00490340`, `story-message-classes.md` row 14).

## Messages

- `0x212` (BountyAttack and CapturedByBountyHunters):
  - Han Solo Attacked by Bounty Hunters (Alliance);
  - Bounty Hunters Locate Solo (Empire, when Han is at a system);
  - Han Solo Captured by Bounty Hunters (both);
  - all three built by `FUN_0048fa00` (`story-message-classes.md` rows 10
    to 12).
- `0x1e0` case 5: Jabba Captures (row 14, mapped for the port's own
  event).
- `0x231` (`CharacterMgrPickupInProgressNotif`, set and cleared by the
  Pickup mission): Jabba Delivers Prisoners (`FUN_0048b6e0`, row 15).

## Against the port

- None of the flags, timer `0x393`, the Bounty Hunters unit, the Bounty or
  Palace missions, Han's flags, or the palace autorouting exist in the
  port. The port's captives stay where they are captured; the original's
  autorouting of captives (to the captor's location or, for the Bounty
  mission, the palace) is not ported.
- The port's own chain `0x380`..`0x39b` and `0x212`, `0x231` runs on tick
  thresholds and random gates instead.

## Plan

1. Captive autorouting as far as the palace needs it: a character
   captured by the Bounty mission travels to Jabba's Palace (and is held
   there).
2. BountyHuntersActive, timer `0x393`, the Bounty Hunters unit and the
   Bounty mission, Han's flags and the `0x212` messages.
3. HanCapturedAtPalace, the Luke, Leia and Chewbacca checks, the Palace
   mission (success, failure and the captures), and the Jabba Captures
   message; remove the port's `0x380`..`0x39b` chain.
4. DarthPickup and the Pickup mission (`final-battle.md`), with Jabba
   Delivers Prisoners.

## Open

- The Dagobah mission's slot `+0x1d8`, which LukePalace calls in phase 8.
- `FUN_004f7d50`'s arguments beyond the class and side.
- The mission captor key: `FUN_005349e0`'s key in the Bounty hit and
  `FUN_0056e870`'s `param_2`; read as the mission itself, from the
  autoroute replacement of family `0x65`.
- The capture's autorouting for other captors (`FUN_00543360`, `+0x70`).
