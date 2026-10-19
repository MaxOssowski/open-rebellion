# Betrayal and character loyalty

Read 2026-10-19 for F-028 and F-038. This is the parity tracker's item
"Zdrada" (area "Zdrada i fabuła").

## Betrayal is a mission phase

The original's only betrayal is the mission phase `FUN_00589f10`
(`decoy-roll.md`):
- a team or decoy member not decoying betrays the mission when
  `draw(0..99) < 100 - loyalty +0x8a`;
- the mission is then detected;
- the other members may learn the traitor (`0x361`, Character Discovers
  Traitor).

The port has this rule (`mission_detection::betray`).

The executable's phase names agree. Next to each other at `0x2a91d4` they
read "foil", "decoy", "not betray", "identify traitor" and
"force-detect". No function changes a character's side.

The manual, "Traitors" (p. 94), says the same:
- "If you send a traitor on a mission, he or she may betray the mission."
- Loyalty "will improve as you have victories".
- The chance of betrayal is affected by "the destruction of the Rebel HQ;
  the Empire gaining or losing control of Coruscant; and getting ships,
  fighters, or troops destroyed in battles, assaults, and bombardments".
- "Characters in command roles will not betray you".

**The port's `betrayal.rs` is invented.** It checks every 50 days a
character whose base loyalty is below 50, rolls UPRIS1TB, and switches the
character to the other side. None of this is in the original.

## Loyalty

- Effective loyalty `+0x8a` (slot `+0x1f8`) is the base loyalty `+0x66`,
  `0..=100`:
  - `FUN_004eee20` -> `FUN_004ee030`;
  - it is copied at creation (`FUN_004f11f0`) and when the base changes
    (`FUN_004f1670`).
- The base changes through `FUN_005341a0` (`Invalid BaseLoyalty value`,
  `0..=100`).

**`FUN_00561910(amount, side, ctx)`** walks every character
(`FUN_00505120` / `FUN_00562a50`) and calls
`FUN_004ef1e0(character, amount, side)`:
- `side` must be 1 (Alliance) or 2 (Empire);
- the character's class `+0xb4` must be 0. It is set for the characters
  whose loyalty never moves, presumably MJCHARSD's;
- the amount is negated unless the class side (`FUN_004edc80`) equals
  `side`;
- base += amount, clamped (`FUN_0053e0d0`), then set through
  `FUN_005341a0`.

So a gain for one side is a loss for the other's characters.

**Its four callers**, the character manager's handlers (when each runs is
not read yet):

| Function | Amount |
|---|---|
| `FUN_00560d00(_, side)` | `FUN_0053ec10(side, galaxy[+0x54]+0x68, +0x6c)` = `(b - a) / SDPRTB 2560 + b / SDPRTB 2560`, read per side |
| `FUN_00561280(_, 0)` | for the system `FUN_00505f60` finds (type `0x12`; presumably Coruscant), the holder side `galaxy[+0x54]+0x60` (`FUN_00509ac0`), flipped when the system is not destroyed and its side differs; `FUN_0053ec10` over `FUN_00509a20(1)`, `(2)` |
| `FUN_005616a0(_, old, new)` | a side change of an object: for the old side and the new, `FUN_0053ec90(side, gained, plain)` = `rand(SDPRTB 2561) + SDPRTB 2562` for that side, unless either word has bits `0x30` |
| `FUN_00561780(_, 0)` | `rand(GNPRTB 2592) + GNPRTB 2593`, for the Alliance (side 1) |

## When the loyalty handlers run (read 2026-10-19)

| Handler | Run by | On | Amount for the side (`FUN_00561910`; the other side's characters get its negative) |
|---|---|---|---|
| `FUN_00561670` -> `FUN_005616a0` | the character manager's side-change handler (`FUN_005613f0`) | an object of key index `0x109`, **Coruscant**, changes side | for the new side only (`FUN_0053ec90(new, 1)`): `rand(SDPRTB 2561) + SDPRTB 2562` = `0..=5`; none when either side word has bits `0x30`; none for the old side (`FUN_0053ec90(old, 0)` is 0) |
| `FUN_00561470` -> `FUN_00561780` | the character manager's object handler | an object of family `0x20..0x21`, the **Alliance headquarters**, with the second argument 0 (presumably its loss) | for the Alliance: `rand(GNPRTB 2592) + GNPRTB 2593` = `rand(-5)` = `0..=-5` |
| `FUN_00517270` -> `FUN_00560d00` | a battle's end at a system (`FUN_0051ddf0`, `timer-scheduler.md`; `planet-squadrons.md`) | the battle-won side (`FUN_0050a040`) | `FUN_0053ec10(won, a = rec[+0x68], b = rec[+0x6c])` on the system's record `+0x54`. For the Alliance: `(b - a) / SDPRTB 2560 + b / SDPRTB 2560`. For the Empire the same with `a` and `b` swapped. SDPRTB 2560 = 50 |
| `Event305_SystemBlockadeChanged_Handle` (`FUN_00515f40`) -> `FUN_00561280` | a system's blockade change (`0x305`), with the second argument 0 | the holder side `rec[+0x60]` (`FUN_00509ac0`), switched to the other side when the system is not destroyed and its side differs | `FUN_0053ec10(side, FUN_00509a20(1), FUN_00509a20(2))`. `FUN_00509a20(s)` = `rec[+0x44+4s] + rec[+0x50+4s]` (`FUN_005099c0`, `FUN_005099f0`), plus side 3's when `s` is not the holder |

Values (SDPRTB.DAT and GNPRTB.DAT as shipped, the same at every level and
for both sides):
- SDPRTB 2560 = 50;
- SDPRTB 2561 = 5;
- SDPRTB 2562 = 0;
- GNPRTB 2592 = -5;
- GNPRTB 2593 = 0.

`FUN_0053e290(n)` draws `0..=n`. For a negative `n` it draws `n..=0`.

The class flag `+0xb4` that exempts a character is the DAT field after
`can_be_general` (`+0xb0`). In the MJCHARSD and MNCHARSD order that is
`is_unable_to_betray`. MJCHARSD sets it for all six majors; MNCHARSD for
some (Ackbar 1, record 834 0). The manual: "Key characters have strong
loyalty that won't waver."

### The system record's loss counters (read 2026-10-19)

**Where they are counted.** `System_vtd0` (`FUN_00508c60`) is a system's
hook for an object there:
- it acts when the object is destroyed (third argument 0, object `+0x50`
  bit 3);
- the object is of family `0x10..0x1f` (a regiment, capital ship or
  fighter squadron);
- it adds the object's value to its own side's counter (`+0x24 >> 6 & 3`).

The value is slot `+0x1c0` minus `+0x1c8`:
- `+0x1c8` is `FUN_006158b0`, which returns 0;
- `+0x1c0` is `FUN_00558050` -> `FUN_00520b60`, the class record `+0x58`.
  That is the DAT field before `detection`, which the dumper names
  `uprising_defense`:
  - capital ships 3..42;
  - fighters 0;
  - regiments 3..6.

The counter chosen depends on the object's DestroyedReason (`+0x40` low
byte):

| Reason | Counter | Setter | Cleared |
|---|---|---|---|
| `0xc` (a battle) | `+0x68 / +0x6c` (`+100 + 4·side`) | `FUN_0050cee0` | at a battle's start (`System_vt21c`, `FUN_00511080`, which also posts `0x309`) |
| `0xe` (Planetary Assault, `planetary-assault.md`) | `+0x48 / +0x4c` (`+0x44 + 4·side`) | `FUN_0050ce20` | at a blockade's start (`System_vt220`, `FUN_00511300`) |
| `0xf` (bombardment, `bombardment-order.md`) | `+0x54 / +0x58` (`+0x50 + 4·side`) | `FUN_0050ce50` | at a blockade's start |

- At a blockade's start `FUN_00511300` also stores the blockading side in
  `+0x60` (`FUN_00509890` -> `FUN_0050ce80`).
- The battle's winner is `+100` (`FUN_0050ceb0`, set by `System_vt23c`,
  `FUN_00511780`).

**What the two handlers give**, with SDPRTB 2560 = 50:
- **A battle's end** (`FUN_00560d00`), for the winning side `w` with its
  own losses `L(w)` and the loser's `L(o)`:
  `(L(o) - L(w)) / 50 + L(o) / 50`.
- **A blockade's end** (`FUN_00561280`, second argument 0):
  - the side is the blockader `+0x60`;
  - when the system is not destroyed and its holder differs, the side
    switches to the other one, which is the holder;
  - the losses per side are the assault and bombardment counters
    (`FUN_00509a20`), with side 3's added for a side other than
    `+0x60`.

Still open:
- What the HQ handler's second argument 0 means. It is presumably the
  object's loss.

## Plan (tracker item "Zdrada")

1. **This note.**
2. **Remove `betrayal.rs`'s defection.** The day step and `BetrayalState`
   go, a save format change (v88). The mission phase stays. Done
   2026-10-19: `betrayal.rs`, `BetrayalState`, the app's and the
   simulation's step 10, `apply_betrayal_events`,
   `GameEffect::CharacterBetrayed`, and the telemetry `betrayal`,
   `betrayal_check` and `side_change`.
3. **Read when each loyalty handler runs** (done 2026-10-19, above).
4. **Port Coruscant's side change and the HQ's loss** (`FUN_005616a0`,
   `FUN_00561780`). Done 2026-10-19, `rebellion-core/src/loyalty.rs`:
   - `shift` is `FUN_00561910` / `FUN_004ef1e0`;
   - `watch` and `settle` compare the day's start with its end, in
     `simulation.rs` step 13e and the app's day loop;
   - a draw is taken only when its event happens;
   - telemetry `loyalty_shift`.

   port: two changes in one day count once.
   hyp: the side words' bits `0x30` are read as clear.

   In 24 dual-AI campaigns one shift happens: seed 9's Alliance loses its
   headquarters, -5. Coruscant never changes hands.
5. **Read the system record's per-side loss counters** (done 2026-10-19,
   above).
6. **Port the battle's shift** (`FUN_00560d00`). Done 2026-10-19:
   - `loyalty::battle_watch` and `settle_battle`, around every battle
     resolution:
     - the simulation's `resolve_system_space_combat`;
     - the app's AI-against-AI and auto-resolve paths;
     - the played tactical battle's `apply_results`;
   - the losses are the capital ships' class `+0x58` (`uprising_defense`),
     before against after.

   port: a regiment lost aboard is not counted; fighters are worth 0, as
   FIGHTSD ships them.

   In 24 dual-AI campaigns 15 battles shift loyalty, and 11 campaigns
   change.
7. **Port the blockade's shift** (`FUN_00561280`).
