# Force detection and the encounter (Luke and Vader)

Read 2026-10-18 for F-038. The port's Luke-Vader confrontation and
heritage reveal are story events of its own; this note records the
original, which runs both inside a mission's detection.

## Force detection (`FUN_00589e40`, the detection manager's character hook)

- The detection manager (`decoy-roll.md`) keeps two sets of the mission
  side's Force users, `+0x50` and `+0x58` (`FUN_0058a5f0`: the team and
  the rest of the members, `FUN_00587fc0`, `FUN_00587fe0`), and two of the
  defenders', `+0x60` and `+0x68` (`FUN_0058a6c0`, the system's and the
  fleets' characters).
- Each defender (`FUN_0058a9e0`: Luke `0x32000242` first, then Vader
  `0x35000281`, then the rest of `+0x68`, then `+0x60`) meets each member
  (`FUN_0058a2c0` -> `FUN_0058ab00`: Luke, Vader, then `+0x50`, then
  `+0x58`).
- `FUN_0058a3f0`: when both have a Force level, the chance is
  `force(defender) + force(member) + GNPRTB 3587 (-100)` percent
  (`FUN_0055e4d0`, rolled by `FUN_0053e2f0`; logged "force_detect").
- On a hit:
  - unless the member is a decoy (`+0x78` bit 0), the mission is detected
    (manager `+0x4c` = 1);
  - `FUN_0055ff60(member, defender)` starts an encounter (below);
  - `FUN_0058a530` takes both out of their sets.

## The encounter is set (`FUN_0055ff60`)

When neither already has one:
- at a system (`0x90..0x97`), both sides get observation level 6 there
  (`FUN_0050d5a0(6, 1)` and `(6, 2)`);
- each one's `Encounter` key (`+0x9c`, `FUN_004ee500`,
  `CharacterEncounterKeyNotif`, notification `0x360`) names the other;
- both get the same encounter day (`FUN_004ef1c0`: record `+0x54` ->
  `+0x34`).

Notification `0x360` builds the story messages "Force user Detects
Enemy" (`FUN_0048e1d0`), "Character Confronts character" (`FUN_0048df20`)
and "Luke Discovers His Heritage" (`FUN_0048dc50`, with `0x211`); see
`story-message-classes.md`.

## The confrontation (`FUN_00560200`)

The detected outcome sends a member holding an `Encounter` key to
`FUN_00588d30` -> `FUN_00560200(member, partner)`, instead of a random
defender. It runs when each names the other and both share the encounter
day (`FUN_004ed300`):

1. **The duel.**
   - The member rolls RLEVADTB against the partner's effective combat
     (`FUN_005349e0`): evade or be captured, with the injury roll.
   - A member not injured by that takes `FUN_0053ea70(partner combat,
     member combat)`: a chance of max(GNPRTB 2571 (1), member - partner)
     percent, then an injury of rand(0..=chance) + rand(0..=2573 (29)) +
     2572 (1), added through slot `+0x2f0`.
   - The partner takes the same roll the other way.
   - The side that injured the other raises its base combat `+0x62` by
     GNPRTB 2578 (1) (`FUN_005340a0`).
2. **The end.** Both `Encounter` keys clear and both encounter days go to
   0.
3. **Luke and Vader.** With Luke (`0x32000242`) and Vader (`0x35000281`):
   - unless Luke knows his heritage, he learns it now: `KnowsHeritage`
     (`FUN_0056f650`, Luke `+0xb0` bit 0);
   - and, if he is free (`+0xac` bit 0 clear) and his Force is below
     GNPRTB 3077 (80), he is injured by rand(0..=3080 (99)) + 3079 (1)
     (`FUN_00561b60`, `FUN_0055e810`);
   - then, if free, his `ForceExperience` gains max(GNPRTB 3084 (1),
     (Vader's Force - Luke's) x 3072 (25) / 100).
4. **Luke and the Emperor** (`0x34000280`): if free, Luke gains max(GNPRTB
   3085 (1), (Emperor - Luke) x 3073 (20) / 100).
5. **Leia** (`0x31000241`), free and Force aware, against Vader: max(3086
   (1), (Vader - Leia) x 3074 (20) / 100); against the Emperor: max(3087
   (1), (Emperor - Leia) x 3075 (20) / 100).

So Luke learns his heritage in his first confrontation with Vader, losing
up to 100 health (the hand) while still below Force 80, and every
confrontation teaches the weaker Jedi some Force.

## Against the port

- `mission_detection.rs` runs the decoys, the FOILTB detection, betrayal
  and the exposure, but no Force detection, no `Encounter` and no
  confrontation.
- Luke's `KnowsHeritage` is the port's `heritage_known`, which only an
  invented story event (`SetHeritageKnown` at "Final Battle Imminent")
  sets. The invented chain also has its own Luke-Vader confrontation.

## Open

- `FUN_004f9510` (called twice per side at both ends): not read.
- The three `0x360` builders and when each applies.
