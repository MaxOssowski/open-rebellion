# Force detection and the encounter (Luke and Vader)

Read 2026-10-18 for F-038. The port's Luke-Vader confrontation and
heritage reveal are story events of its own; this note records the
original, which runs both inside a mission's detection.

## Force detection (`FUN_00589e40`, the detection manager's character hook)

- The detection manager (`decoy-roll.md`) keeps two sets of the mission's
  characters (`FUN_0058a5f0`), each counted by `FUN_005883b0`:
  - `+0x50`, the major characters (families `0x30..0x37`) of the team and
    the decoys (`FUN_00587fc0` = `FUN_00588000(1,0,1,1,1)`);
  - `+0x58`, the minor ones (`0x38..0x3b`, `FUN_00587fe0` =
    `(0,1,1,1,1)`).
  - `FUN_00588000`'s flags are majors, minors, team, decoys, not decoying:
    its lists are `FUN_00526350` (team majors), `FUN_005261f0` (team
    minors), `FUN_00526400` (decoy majors), `FUN_005262a0` (decoy minors).
- It keeps two sets of the defenders' Force users (`FUN_0058a6c0` ->
  `FUN_0058a7e0`): `+0x68` the major characters and `+0x60` the minor ones,
  at the system with flag `+0x20` and aboard the ships of the fleets there
  with flag `+0x24` (disassembly `0x58a898`, `0x58a96b`). Each counts
  (`FUN_00588840`) with Force `+0x8c` at least GNPRTB 3586 (60,
  `FUN_0055e590`), free (`+0xac` bit 0 clear), and of the other side.
- Each set is a keyed tree (`FUN_004f44b0` -> `FUN_005f4f10`) with an
  in-order list. A pick marks its entry (`+0x1c` `DAT_0066a7e4`); a removal
  marks it `DAT_0066a7e8` (`FUN_0058ac20`).
- `FUN_00589e40` loops over the defenders (`FUN_0058a9e0`): Luke
  `0x32000242` if in `+0x68`, then Vader `0x35000281` if in `+0x68`, else a
  random unmarked one of `+0x68` (`FUN_0058aca0`, `draw(0..count-1)`),
  else of `+0x60`. Each defender is picked once.
- For each defender, `FUN_0058a2c0` clears the members' pick marks
  (`FUN_0058a3b0` -> `FUN_0058ad80`, not the removals) and picks members
  the same way from `+0x50` and `+0x58` (`FUN_0058ab00`). It rolls each
  (`FUN_0058a3f0`) in a do-while that stops once the defender holds an
  `Encounter` key (disassembly `0x58a370..0x58a381`: `FUN_004ece60` on
  defender `+0x9c`).
- `FUN_0058a3f0`: when both have a Force level, the chance is
  `force(defender) + force(member) + GNPRTB 3587 (-100)` percent; only a
  positive chance draws (`FUN_0055e4d0`, rolled by `FUN_0053e2f0`; logged
  "force_detect").
- On a hit:
  - unless the member is a decoy (`+0x78` bit 0), the mission is detected
    (manager `+0x4c` = 1);
  - `FUN_0055ff60(member, defender)` starts an encounter (below);
  - `FUN_0058a530` takes both out of their sets.

## The encounter is set (`FUN_0055ff60`)

When neither already has one:
- when the member's container is a system (`0x90..0x97`), both sides get
  observation level 6 there (`FUN_0050d5a0(6, 1)` and `(6, 2)`);
- each one's `Encounter` key (`+0x9c`, `FUN_004ee500`,
  `CharacterEncounterKeyNotif`, notification `0x360`) names the other;
- both get the same encounter day (`FUN_004ef1c0`: record `+0x54` ->
  `+0x34`).

Notification `0x360` builds the story messages "Force user Detects
Enemy" (`FUN_0048e1d0`), "Character Confronts character" (`FUN_0048df20`)
and "Luke Discovers His Heritage" (`FUN_0048dc50`, with `0x211`); see
`story-message-classes.md`.

## The confrontation (`FUN_00560200`)

The detected outcome's walk (`FUN_00589360`) sends a member holding an
`Encounter` key to `FUN_005892a0` -> `FUN_00588d30` ->
`FUN_00560200(member, partner)` and a member without one to a random
defender (`FUN_005891e0`). The confrontation runs when each names the
other and both share a non-zero encounter day (`FUN_004ed300`); otherwise
nothing happens to the member:

1. **The duel.**
   - The member rolls RLEVADTB at its effective combat less the partner's
     (`FUN_005349e0`): evade (slot `+0x208`, with the resign request and
     the injury roll) or be captured by the partner (slot `+0x20c`).
   - The injury that roll dealt is the change of `+0x94`. A member not
     injured takes `FUN_0053ea70(member combat, partner combat)`: a chance
     of max(GNPRTB 2571 (1), partner - member) percent, then an injury of
     rand(0..=chance) + rand(0..=2573 (29)) + 2572 (1), added through slot
     `+0x2f0`.
   - The partner takes the same roll the other way: chance max(2571,
     member - partner).
   - The side that injured the other raises its base combat `+0x62` by
     GNPRTB 2578 (1) (`FUN_005340a0`); a member injured by the RLEVADTB
     roll counts.
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

- Ported 2026-10-18 in `mission_detection.rs` (`Run::force_detect`,
  `begin_encounter`, `confront`, `lessons`) with `Character::encounter`
  and `encounter_day` (save v74).
- port:
  - the run applies its effects after it ends, so it keeps the keys it
    set or cleared, and the Force its members gained, for the rest of the
    run;
  - the loader keeps a character's record index without the family byte,
    so a family of 0 reads `is_major`; the index order is the key order
    of the shipped records;
  - the observation reveals the system, as for Espionage.
- Luke's `KnowsHeritage` is the port's `heritage_known`. The invented
  story event "Final Battle Imminent" (`0x396`) still sets it, at the end
  of Dagobah in the campaigns, so the confrontation finds it known. The
  invented chain also has its own Luke-Vader confrontation.

## Open

- `FUN_004f9510` (called twice per side at both ends): not read.
- The three `0x360` builders and when each applies.
- `thunk_FUN_00506e80` (a global's `+0xc0`), which gates the encounter and
  the confrontation, is read as always present.
