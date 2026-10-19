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

## Plan (tracker item "Zdrada")

1. **This note.**
2. **Remove `betrayal.rs`'s defection.** The day step and `BetrayalState`
   go, a save format change (v88). The mission phase stays. Done
   2026-10-19: `betrayal.rs`, `BetrayalState`, the app's and the
   simulation's step 10, `apply_betrayal_events`,
   `GameEffect::CharacterBetrayed`, and the telemetry `betrayal`,
   `betrayal_check` and `side_change`.
3. **Read when each loyalty handler runs**, and the counters
   `galaxy[+0x54]+0x60..0x6c`, `FUN_00509a20`, `FUN_005099c0` and
   `FUN_005099f0`.
4. **Port the loyalty changes**, one handler a stage.
