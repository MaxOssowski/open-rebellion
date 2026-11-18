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
- Timer `0x393` (vtable `0x00661f80`) fires (`FUN_0054e970`) and runs
  `FUN_00546fa0`. Correction (2026-11-17): it does not fire once. Its slot
  `+0x20` is the base timer's `FUN_005862a0`, which, while the target's
  record (BountyHuntersActive, manager `+0x64`) keeps the counter it was
  armed with, fires and then adds `FUN_00586130` (GNPRTB 6148 + rand(0..=
  6149)) to the due day `+0x40` (`timer-scheduler.md`, "Firing"). So the
  hunters come back every 300..600 days until HanCapturedAtPalace clears
  the flag. `FUN_00546fa0` itself re-arms nothing. With a chance of GNPRTB 6150 (100) percent
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
- MISSNSD `0x65000083` does not repeat, but timer `0x393` does (see
  above), so the hunters strike every 300..600 days while Han is free and
  each attack captures him 2 times in 100. (Before 2026-11-17 this note
  said the timer fired once.)
- The port's campaigns agree: in 24 seeds the hunters struck 16 times and
  Han evaded each time. The rest of the palace story follows only after a
  capture, so it is rare in play too.
- After the assassination fix (2026-10-18, `character-injury.md`), Han
  survives the AI's assassins. In 24 seeds the hunters struck 24 times
  and captured Han twice. Seed 16: Bounty day 399, Han in the palace day
  400, Chewbacca's rescue day 412. Seed 19: day 571, 572, then Luke's
  rescue day 584.
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

## What the player sees of the palace (read 2026-11-17, EVT-01-C018)

- No message, screen or window belongs to Han being held in the palace.
  `HanCapturedAtPalace` (`FUN_00543c40` -> `FUN_0054a2f0` -> slot `+0x1fc`,
  `FUN_0054b840`) only writes the debug trace `FUN_004f8880` ("TRUE" /
  "FALSE"); unlike `HanBountyAttackNotif` (`FUN_00572b40`) it posts no
  `FUN_0053fdd0` notification, so no message class answers it
  (disassembly `0x54b840`..`0x54b870`).
- The palace shows only in texts and pictures of other messages
  (TEXTSTRA RCDATA, read from the shipped TEXTSTRA.DLL):
  - Han Solo Captured by Bounty Hunters (`FUN_0048fa00`, `0x41d` = STRATEGY
    1053, Han before Jabba): "Han Solo was captured by bounty hunters and
    taken to Jabba's Palace.";
  - Rescue Attempt (`FUN_0048c4c0`): "... taken to Jabba's palace.  I will
    be departing immediately to attempt a rescue.";
  - Jabba Captures {1} (`FUN_00490340` case 5, `0x429` = STRATEGY 1065):
    "{1} was captured by Jabba while attempting to rescue Han Solo.";
  - Jabba Delivers Prisoners (`FUN_0048b6e0`, `0x42a`).
- Case 5 tests the captive's container: `ECX = [character+0x1c]`, then
  `FUN_004025b0` against `0x98000481` (disassembly `0x4905e4`..`0x49060b`).
  Han's own capture happens at his system, so it is case 3/4, and Han with
  `+0xb0` bit 2 jumps to `LAB_0049069a` (no message). His later arrival in
  the palace changes no `+0x98`, so no `0x1e0` follows.
- Port: `captured_message` returns nothing for Han with `bounty_captured`
  (2026-11-17); before, the port posted "Han Solo Captured" / "... at
  <system>" beside the bounty message. hyp: the message classes build from
  queued notifications after the hit ends, so bit 2 is already set when
  `0x1e0` is read; the check would be dead code otherwise.

## The hunters repeat (2026-11-17)

- Trace: `FUN_00549d60` sets the record's minimum GNPRTB 6148 and spread
  6149 (`FUN_00540230`, `FUN_00540270`) and then BountyHuntersActive; its
  hook `FUN_0054b740` calls `FUN_0053fa60(0x393, ...)`, which builds the
  timer through `FUN_0053fb00` and schedules it after `FUN_00586130`. The
  vtable `0x00661f80` holds `FUN_005862a0` at `+0x20` (bytes read from
  REBEXE.EXE): fire `+0x30`, then `+0x40 += FUN_00586130()`.
  `Timer393_Handle` (`FUN_00546fa0`) and `FUN_0054e970` re-arm nothing.
- Fan guides' "every 1-100 days, 30%" matches no shipped value (GNPRTB
  6148 300, 6149 300, 6150 100).
- Footage (Nuby Plays Gaming, "Let's Play Star Wars: Rebellion Part 1",
  https://www.youtube.com/watch?v=-k6Dp8jUpJc): "Han Solo Attacked by
  Bounty Hunters" opens on day 878 (t=3898) and again on day 1208
  (t=5037-5039). On day 1208 it arrives while the player builds X-wings
  (day 1207 -> 1208) and heads the Message Index as the new message. The
  gap, 330 days, lies in 300..600. Ole Cranky Gamer shows attacks in
  episodes 7 and 14 of one campaign, Pravus in parts 9, 14 and END.
- Port: `jabba.rs` reschedules `bounty_due` by GNPRTB 6148 + rand(6149)
  after each fire, whether or not the chance holds (2026-11-17). port: a
  save with BountyHuntersActive and no due day (saved after the old single
  fire) is armed again from the day it loads.

## A0 of the bounty capture and the palace (2026-11-18, EVT-01-C018)

Source: `.artifacts/a0-vm/2026-11-18-edited-saves-7/` (not committed; README
there lists every save edit). Edited saves: timer `0x393` due on day 7, Han's
combat 0, Luke's, Leia's and Chewbacca's espionage and combat 0. The edits
make the day, Han's odds and the rescues' failure artificial; the texts,
pictures, categories, order and Finder rows are the game's own. Pictures
matched by pixel against STRATEGY.DLL (400x200 at (74, 82)); texts against
TEXTSTRA RCDATA (`textstra-messages.json`).

| Frame (sha256, 12 hex) | Original | Port | Result |
|---|---|---|---|
| b-01 (6c96789f4a1a), b-02 (edf6a01596c9) | "Han Solo Attacked by Bounty Hunters" / `0x7169`, picture 1052 (`0x41c`); "Han Solo Injured" beside it | `bounty_messages` `0x7168`/`0x7169` on `0x41c`; the Bounty hit's injury roll | match |
| b-03 (4e404e563f01) | index: Captured by Bounty Hunters, Injured, three Rescue Attempt rows; no `0x1e0` "Han Solo Captured" row; the Rescue Attempt rows have no icon, the others the Mission icon 10910 (`0x2a9e`) | no `0x1e0` for Han with `bounty_captured` (2026-11-17); Rescue Attempt was on the Mission rail | fixed: Rescue Attempt has category 0 (`story-message-classes.md` row 13), `MessageRail::Unfiled` |
| b-04 (5007de37a930) | `0x716c`/`0x716d` "Han Solo was captured by bounty hunters and taken to Jabba's Palace.", picture 1053 (`0x41d`) | the same ids and background | match |
| b-05 (6e3778ef8d15) | "Han Solo Injured", picture 1050 (`0x41a`) | `pictures::injured` `0x41a`, `0x7160`/`0x7161` | match |
| b-06..b-08 (289004507a08, 2565947f27f7, db6b3d7a649a) | `0x71ea`/`0x71eb` with the rescuer, background 1042 (`0x412`) under the portrait; no title image | `rescue_attempt_message`; the title image followed the Mission rail | fixed with b-03 (`row_icon` gives none for mask 0) |
| b-09, b-10 (14023bc2188c, 144c42f9b68a) | each "Jabba Captures X" row comes with an older "X Injured" row | the failed rescuer was captured unhurt | fixed: slot `+0x20c` (`FUN_004ef480`) runs the injury roll slot `+0x2e4` (`FUN_004ef5f0`) before the capture; the port rolls `injury_roll` on the member's combat before `CharacterCaptured`. The app posts the injury (in `apply_mission_result`) before Jabba Captures, so the order matches |
| b-11, b-12 (8861720e5021, ec2681b5c72b) | "Jabba Delivers Prisoners", picture 1066 (`0x42a`), "Jabba has deliverd Luke, Han, Chewbacca and Leia to Darth Vader.  They are now prisoners of the Empire." | `jabba_delivers_message` reads `0x7200`/`0x7201` (the typo is TEXTSTRA's) on `0x42a` (port: always `0x42a`) | match |
| b-13, b-15, b-17 (68726be9377f, 85eceb6191f2, f423dd466cea) | `0x7156`/`0x7157` "{name} was captured by Jabba while attempting to rescue Han Solo.", picture 1065 (`0x429`), Mission title image | `jabba_captures_message` | match |
| b-14, b-16, b-18 | "X Injured", `0x41a` | as b-05 | match |
| b-19 (e4f53d49b065) | Personnel Finder on day 31: "Chewbacca - Jabba's Palace", "Han Solo - Jabba's Palace", "Leia Organa - Jabba's Palace", "Luke Skywalker - Jabba's Palace", no "( Captured )" | the abode rule of `character_text` gives the same text while they are in the palace | text matches; state differs, see below |

b-19, the Finder's palace rows after the Pickup (traced 2026-11-18). The
prisoners do not stay at the palace; the Alliance's Finder shows its own
stale copies of them:

- The in-game save of the same run at day 34 (`BNTY7D34`, 349f49a6506e)
  writes each object's children after it. In the master section the four
  master records (key-and-side `0x80000081`: side bits `0x80`, the captor's,
  `FUN_004eec60`) stand among Coruscant's children with Vader and the
  Emperor (0x191e2..0x195ae), and their container field (save `+0x3c`,
  `0x92000121` Yavin on day 2) holds `0x90000109`. The master palace
  (0x23783) has no child. The Empire's view section (`0x800000a1`) has
  them under Coruscant too (0x4c1a6..0x4c46e).
- The Alliance's view section (`0x80000092`) holds its copy of the palace
  (0x38a1b) followed by its copies of Leia (0x38a5b), Luke (0x38aeb), Han
  (0x38b7b) and Chewbacca (0x38c07): the Alliance still has all four in
  the palace on day 34.
- The Finder lists the side's view objects (`FUN_004f3d50`), and a
  captive carries its captor's side, so the Alliance reads a copy that
  no longer syncs; the abode rule then gives "Name - Jabba's Palace" with
  no words. b-19 is that copy, not the master's container.
- The move itself: the Pickup's hit (phase 10) and its end (phase `0xb`)
  run in one tick, and a trip with an end that is not a system takes 0
  ticks (`FUN_00555b30`, `build-delivery.md`). So nothing found holds the
  prisoners at the palace; their departure day is not seen in A0 (between
  the hit, day 22, and day 34). The port's `EscapeeTransit` (arrival the
  day after the hit, to the system Vader left from, Coruscant here) agrees
  with the day-34 state and is not changed.
- Port gap (closed 2026-11-18, below): the port's Finder read a captive
  of the player's own side live ("Luke Skywalker - Coruscant ( Captured
  )"), as `character_rows_travelling` picked the copy by home side
  (`Character::is_alliance`), and `object_views` kept no copy of an abode
  or of an own captive.

### When the old side's copy of a captive syncs (traced 2026-11-18)

- The capture runs through the character's slot `+0x20c`
  (`Character_vt20c` `FUN_004ef480`): the injury roll (`+0x2e4`), then
  `FUN_004ee3e0(0)` writes `+0x98` = 0, whose hook `Character_vt334`
  (`FUN_004f18e0`) calls `FUN_004eef70` ->
  `Character_SetCaptured(+0x98 == 0)` (`FUN_004ee800`).
- `Character_SetCaptured` sets the master's `+0xac` bit 0, then syncs copy
  1 and copy 2 (`Character_SyncCaptured`), then calls vtable `+0x34c`
  (`Character_vt34c` `FUN_004f1a80`): `Character_SetCanEscape`,
  `Character_SetEscapeRequest(0)`, `FUN_004eec60`, `FUN_004eef90` (CanHeal),
  `FUN_004ef070` (DiscoveringForceUser), `FUN_004eeed0` (clears the command
  word `+0x96` of a captive through `FUN_004ee350(0)`).
- `FUN_004eec60`: with `+0xac` bit 0 set, the owner becomes the other side
  of the home side (`FUN_004edc80` reads the DAT record's `+0x40`/`+0x44`:
  1 -> 2, else 1). `FUN_004f6f40` writes the master's `+0x24` side bits,
  syncs both copies' owner (`FUN_004f9ca0`, applied only to a live copy),
  then vtable `+0xf0`, which reaches `FUN_004fae40` -> `FUN_004f9a60`.
- So the old side's copy is live (its owner is the side) when the Captured
  bit is synced: every captive's old-side copy holds `+0xac` bit 0, and
  the copy's own owner bits follow to the captor. After `FUN_004f9a60`
  the copy is live only while the captive's `+0x50` holds the side's seen
  bit, bit 2 and not bit 4 (`per-side-object-views.md`); the seen bit is
  the container's (`FUN_004f7bf0`), cleared while `+0x50` bit `0x20`
  (EnrouteActive) is set. Setting EnrouteActive (`FUN_004f7640`) runs
  `FUN_004fb760`, which recomputes both seen bits (vtable `+0x8c`,
  `+0x90`).
- The Finder lists the row under the home side's tab (`FUN_004edc80`), so a
  captive's frozen copy stays in its own side's list.

Both A0 frames fit one rule: the row is the old side's copy, as it stood
when it last synced.

- b-19: the rescuers went to the palace as the Alliance's own (copies live,
  container the palace), were captured there (copies get Captured, then
  freeze: no Alliance fleet sees the palace), and the copies stayed when
  the Pickup took the masters to Coruscant. The abode rule of
  `FUN_00465bb0` drops the Captured word, so the rows read "Name - Jabba's
  Palace". Han's copy is in the palace too (save `BNTY7D34`).
- day628: "Jan Dodonna - Punisher ( Captured )" is a copy with a ship
  container, where the state word prints; it held Captured from the
  capture. Whether that copy was live or frozen on day 628 the frame
  does not show.

Untraced: the order of the container move and the capture in the Bounty
hit and in a mission's capture (where a captive is moved aboard the
captor's ship); `FUN_00556390` sets EnrouteActive only when
`FUN_00555410`'s trip time is 0 (`param_2 == 0`), and whether that
freezes a travelling captive's copy before vtable `+0xa8` moves it is
not settled; the palace's own seen bit (its container and vtable
`+0x4c`).

port (2026-11-18, no save change): `object_views::record_own_captives`
runs each day before the system copies are rebuilt. An own captive with
no copy yet, or whose copy stands in a system the side sees, takes a copy
where it stands: an abode by name (`CharacterCopy::ship`, no system), a
fleet's ship, a system, or a trip's destination (`FUN_00556390`); one the
side sees is left to `character_copies`, which now counts the side's own
captives. A character not held drops its copy. The Finder
(`character_rows_travelling`) reads an own captive's copy when it has
one, with the Captured word when the copy has a place; an abode copy
reads its name alone and Display opens nothing (`FUN_00429440`). hyp: a
live copy follows the move that takes a captive out of sight before it
freezes (as the four palace copies did); the copy keeps no command word.

The Empire's copies of the messages (traced 2026-11-18):

- Jabba Delivers Prisoners: `FUN_0048b6e0` has no side test; it builds for
  any message object (`+0x20` side) that receives `0x231` with its argument
  set, and the Pickup's hit notifies both sides (`FUN_00539fd0(1)`, `(2)`).
  The port's posting to both sides matches. Its background reads the
  reader side's object of Luke (`FUN_004f3e50(+0x20)`, `+0xb0` bit 3).
- Jabba Captures: `FUN_00490340` reads the reader side's object of the
  character (`FUN_004f2d10(+0x20, key)`); case 5 needs that object
  captive (`+0x98 == 0`) and in the palace. The Empire's view had the
  prisoners live, so it likely reads it too; not seen in A0, port (both
  sides) not changed.

Not shown by these frames: the Empire's copies of Jabba Captures and Jabba
Delivers Prisoners (the port posts both to both sides), the `0x42b` variant
after Dagobah, and Bounty Hunters Locate Solo (Empire).
