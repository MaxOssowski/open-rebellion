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

- Ported 2026-10-18 (plan 1 and 2 together, save v77):
  - `jabba.rs`: BountyHuntersActive from the start, timer `0x393` with
    GNPRTB 6148 + rand(6149) days, firing once; the chance 6150; the
    Bounty Hunters unit in the palace's special forces; the Empire's view
    of Han's system; the Bounty mission (`MissionKind::Bounty`, record
    `0x65000083`);
  - the Bounty hit in `missions.rs` phase 10: Han only, RLEVADTB against
    combat 0 (`mission_detection::rlevadtb_evades`), the injury, then
    BountyAttack and, when captured, `escape_barred` and
    `Character::bounty_captured`;
  - the captured Han autoroutes to the palace (`autoroute_to_abode`, a
    1-day abode trip) and enters it;
  - the app posts notification `0x212` (`story_messages::bounty_messages`).
- port:
  - the Bounty Hunters unit takes its class's base skills;
  - the mission leaves from the palace on a 1-day abode trip, as the
    Dagobah mission does;
  - only the Bounty mission's captive autoroutes; other captives still
    stay where they are captured (`FUN_00543360` not read).
- The port's invented chain (`0x380`..`0x39b`, `0x212`, `0x200`, `0x231`)
  is gone, since its trigger was the port's own `0x212`.
- Ported 2026-10-18 (plan 3):
  - `jabba::han_captured_at_palace` (`FUN_005442f0`), derived each day;
    its hook clears BountyHuntersActive and timer `0x393`;
  - the Luke, Leia and Chewbacca checks send each free rescuer on no
    mission on a Palace mission (`MissionKind::Palace`, record
    `0x64000044`), aimed at the palace;
  - the chance espionage / 3 + combat / 3 (`FUN_0056e620`);
  - the hit (`FUN_0056e650`): the skill raises, then Han and every other
    prisoner of the palace freed. The freed of another side than the
    palace's autoroute out with the rescuer (`FUN_004ef260`);
  - the failure (`FUN_0056e870`): each member captured for the palace's
    side, `CanEscape` cleared. The rescuer stays in the palace. Correction
    (2026-10-18): `FUN_004ef260` compares the raw side `+0x24`, which is
    the character's own (`FUN_004eec60` only derives the captor's side).
    So a held rescuer is autorouted too (slot `+0x218`, status 2). hyp: a
    prisoner's autorouting leads to its captor, the palace, where it
    already is. The port keeps it in place by reading the captor's side;
  - the mission ends (code 6) once Han is no longer held there;
  - the Dagobah check waits while Han is held (`FUN_00544030`);
  - Jabba Captures (`FUN_00490340` case 5, `story_messages::
    jabba_captures_message`) for a character captured in the palace.
- port (plan 3):
  - "usable" (`+0x50` bit 0) has no port field;
  - the Alliance's observation of Han's location on HanCapturedAtPalace
    is not ported, as an abode has no system;
  - Luke's branch on the Dagobah mission in phase 8 (slot `+0x1d8`) is
    not read;
  - no one detects a mission at an abode, as the port models no defenders
    there;
  - a freed prisoner keeps its own side; a neutral one goes to the
    rescuer's;
  - Jabba Captures goes to both sides.
- Not ported: DarthPickup, the Pickup mission and Jabba Delivers
  Prisoners (plan 4).

## Han's odds against the hunters (checked 2026-10-18)

- The hit calls `FUN_005349e0` on Han (`ECX` is the target at
  `0x575eaf`) with captor combat 0. `FUN_0055bfa0` looks up RLEVADTB
  (table 13) at Han's effective combat less 0.
- MJCHARSD gives Han combat 100 with no variance. RLEVADTB's rows run
  from (-20, 15) to (101, 99), and the row at 91 holds 98. So Han evades
  98 times in 100.
- MISSNSD `0x65000083` does not repeat, and timer `0x393` fires once. So
  the hunters strike once a game and capture Han about 1 game in 50, when
  they fire at all (GNPRTB 6150 is 100).
- The port's campaigns agree: in 24 seeds the hunters struck 16 times and
  Han evaded each time. The rest of the palace story follows only after a
  capture, so it is rare in play too.
- Pinned by `rebellion-data/tests/mission_records.rs`
  (`han_evades_the_bounty_hunters_ninety_eight_times_in_a_hundred`, and
  the story missions' records).

## The Pickup (plan 4), read 2026-10-18

The DarthPickup check and the Pickup record are in `final-battle.md`. In
addition:

- The hook `FUN_0054abc0` creates the mission (`FUN_005479c0`) with
  target the palace `0x98000481`, record `0x73000082` and member Vader
  `0x35000281`.
- The hit (`FUN_0056da80`), with the target container (`FUN_00521070`)
  and the galaxy (`thunk_FUN_00506e80`) found:
  - sets PickupInProgress (`FUN_0055f6c0(1)`, character manager `+0x58`
    bit 2). Its notifications go to both sides (`FUN_00539fd0(1)`, `(2)`)
    and then slot `+0x1dc`;
  - walks the palace's characters of the mission's side
    (`FUN_004f2640(palace, 1, side)`, the side bits of `+0x24`; a
    prisoner's side is its captor's). Each one that is captured (`+0xac`
    bit 0) and on no mission (`+0x78` bit 7 clear):
    - joins the mission (slot `+0x1cc(key, 0)`);
    - takes the mission's `+0x70` as its own `+0x70` (`FUN_00535760`,
      `FUN_00534390`, which notifies both sides and calls slot `+0x244`).
      The disassembly settles whose: `FUN_00535760` is called with `ECX`
      the hit's member (`param_2`, at `0x56dbba`), Vader. A role's `+0x70`
      is its home object (`FUN_00536490` sets it to the role's location),
      and slot `+0x244` (`FUN_00536300`) sets `+0x74` to that object's
      system;
    - and the result is 3 (`FUN_00521880(3)`);
  - clears PickupInProgress (`FUN_0055f6c0(0)`).
- Jabba Delivers Prisoners (`FUN_0048b6e0`): notification `0x231` with
  its argument set, once per message object (`+100`):
  - background `0x42a` and media `0x479`, or `0x42b` and `0x47a` when the
    side record's (`FUN_004f3e50(side)`) `+0xb0` bit 8 is set;
  - speaker `0x1b`, subject Vader `0x35000281`;
  - TEXTSTRA `0x7200` "Jabba Delivers Prisoners" and `0x7201` "Jabba has
    deliverd Luke, Han, Chewbacca and Leia to Darth Vader. They are now
    prisoners of the Empire." (sic), with no arguments.

- The Pickup class's phase slot `+0x1f4` (`FUN_00594460`) only calls the
  shared `FUN_00592f50`.
- When the mission at the palace ends, `FUN_004ef260` runs for each
  member. A prisoner's own side `+0x24` is not the palace's, so its slot
  `+0x218` (`FUN_004ef5b0`, status 2) runs and Autorouting is set. The
  autorouting's candidates (`FUN_005545e0`) take the `+0x70` object
  first after the container (`FUN_00554ab0`), which is Vader's home. So
  the prisoners leave for Vader's home, still captive. Vader is of the
  palace's side and stays. DarthToLuke then sends him after Luke
  (`final-battle.md`).
- `DarthPickup` (`FUN_005445d0`) also accepts Chewbacca destroyed
  (`+0x50` bit 3) in place of captured. Its palace walk
  `FUN_004f2640(palace, 1, 2)` reads the Empire's side, which matches
  prisoners only through the captor's side.
- Open: the side record's `+0xb0` bit 8, which picks the `0x231`
  background.

Ported 2026-10-18 (plan 4):

- `jabba.rs`: the DarthPickup check and `JabbaStep::Pickup`, which
  dispatches the Pickup mission (`MissionKind::Pickup`, record
  `0x73000082`) aimed at the palace.
- `missions.rs`:
  - the hit: `PickupInProgress`, then `PrisonerPickedUp` for each
    prisoner held for the mission's side and on no mission (result 3);
  - at the end, each picked-up prisoner autoroutes, still captive, to
    the system the mission left from.
- `story_messages::jabba_delivers_message` (0x231), posted by the app on
  `PickupInProgress`.
- port:
  - Vader's home (`+0x70`) is the system he left from;
  - Vader's resign request when he is on another mission is not ported;
  - the message goes to both sides on `0x42a`.

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
