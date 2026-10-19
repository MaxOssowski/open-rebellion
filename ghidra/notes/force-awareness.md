# Force potential, awareness and discovery (timer `0x389`)

Read 2026-10-17 for F-038. The port's `jedi.rs` ran its own Force tiers
with flat probabilities; this note records what the original keeps and
does. `jedi.rs` is gone since 2026-10-19 (`jedi-rs-replacement.md`).

## Fields and flags

| Where | Name | Meaning |
|---|---|---|
| character `+0x8c` | Force level | short; Luke starts at 50, Leia at 10, the Emperor 150, Vader 120 (MJCHARSD `jedi_level`) |
| character `+0xac` bit `0x10` | `ForceAware` | the character knows its Force (`FUN_004ee950`) |
| character `+0xac` bit `0x20` | `ForcePotential` | the character has the Force (`FUN_004ee9c0`) |
| character `+0xac` bit `0x80` | `DiscoveringForceUser` | it can find the Force in others (`FUN_004eeaa0`) |
| character `+0xa4` | discoverer | the key of the one who found its Force (`FUN_004ee680`) |
| class `+0x98` | `jedi_probability` | MJCHARSD file offset `0x74` (class = file + `0x24`, as the refined cost `+0x48` is file `0x24`) |
| class `+0x9c` | `is_known_jedi` | file `0x78` |
| class `+0xb8` | `is_jedi_trainer` | file `0x94` |

MJCHARSD: 577 (`0x241`, family `0x31`) is Leia, 578 (`0x242`, family
`0x32`) Luke (trainer, Force 50), 640 (`0x280`, family `0x34`) the Emperor
(Force 150), 641 (`0x281`, family `0x35`) Vader (trainer, Force 120). The
names come from TEXTSTRA; an earlier version of this note swapped the
last two.

## At creation

Slot `+0x114` (`FUN_004f11f0`): `ForceAware` = class `is_known_jedi`;
`ForcePotential` = `FUN_0053e2f0(jedi_probability)`, a percent roll; then
the effective skills are recomputed (`FUN_004eee60`..`FUN_004eede0`).

## The Force base and level

- `FUN_004eee60` (run at creation and by both flags' hooks, slots
  `+0x358` and `+0x35c`, `FUN_004f1b30`): with no Force base `+0x8e` and
  both `ForcePotential` and `ForceAware`, the base becomes the class
  `+0xa0` plus `FUN_0053e290(class +0xa4)`, a draw from `0..=` the
  variance. `FUN_004ee150` sets it, clamped to `0..=0x7fff`.
- The base's hook (slot `+0x320`, `FUN_004f1700`) sets the Force level
  `+0x8c` to `+0x8e + +0x90` (`FUN_004eee40`, `FUN_004ee0d0`). `+0x90` is
  not read yet.
- The level's hook (slot `+0x31c`, `FUN_004f16a0`) recomputes the
  effective diplomacy (`FUN_004eecb0`, `+0x58` plus its Force share), the
  effective espionage (`FUN_004eecf0`, `+0x5a`), the effective combat
  (`FUN_004eed90`), `FastHeal` (`FUN_004eeff0`) and
  `DiscoveringForceUser` (`FUN_004ef070`). The share is `FUN_0053e940`:
  skill + skill × Force / 100.
- The Status window's Force Ranking reads `+0x8c` (`FUN_004486f0`).

So an unaware character has Force 0, whatever its class template.

## Force growth (read 2026-10-18)

The notifier names settle the three shorts: `+0x8c` is `Force`
(`CharacterForceNotif`, slot `+0x394`, sync `FUN_004f0220`), `+0x8e` is
`ForceExperience` (`+0x398`, `FUN_004f02b0`, setter `FUN_004ee150`) and
`+0x90` is `ForceTraining` (`+0x39c`, `FUN_004f0340`, setter
`FUN_004ee1d0`). Force = ForceExperience + ForceTraining (`FUN_004eee40`).
The port's `force_base` is ForceExperience.

What writes them:

- **ForceTraining** has one writer, the Jedi Training mission (`0x58`,
  vtable `0x006667d8`, slot `+0x27c` `FUN_005712b0`):
  - the teacher (mission `+0xa8`, `MissionJediTrainingTeacherKeyNotif`) is
    the first member that trains Jedi at Force GNPRTB 2581 (100) or more
    (`FUN_00570e40`, `FUN_004ed310`); the side must be 1 or 2;
  - each other member's chance is teacher Force - pupil Force percent
    (`+0x274` `FUN_005710b0` -> `FUN_0055c7c0`, rolled with
    `FUN_0053e2f0`);
  - on a hit the pupil's Force becomes Force + rand(0..=(teacher - pupil)
    x GNPRTB 6173 (50) / 100) (`FUN_0055ca20`), the gain going into
    ForceTraining;
  - rules (`FUN_00570690`, `FUN_00570940` running, `FUN_00570bc0` at
    creation on the class records): no teacher refuses `0x40`/`0x90`;
    a pupil at Force 0 (class template base + variance 0 at creation)
    refuses `0x40`/5; a pupil not below the teacher's Force refuses
    `0x40`/`0xd`; fewer than two members refuses `0x40`/`0x94`;
  - MISSNSD `0x58`: own-side system, 60 + rand(0..=30) days, no repeat,
    detection on, resign allowed, characters of both sides.
- **ForceExperience**:
  - the creation roll (`FUN_004eee60`, above);
  - Luke and Leia evading capture: their slot `+0x208` (`FUN_0056f8b0`,
    `FUN_0056ffa0`) runs the base evade (`FUN_004ef450`, the resign
    request) and adds GNPRTB 3081 (Luke, 1) or 3082 (Leia, 1). The evade
    is the RLEVADTB roll `FUN_005349e0` of mission detection, the bounty
    hunters (`FUN_00575de0`) and the encounter (`FUN_00560200`);
  - the Dagobah mission's end (`0x71`, slot `+0x1f4` `FUN_005751d0`, step
    `0xb` after phase 8): Luke's Force becomes Force + Force x GNPRTB 6174
    (60) / 100 on a success (`+0x60` = 3, `FUN_0055ca60`), else Force +
    Force x (days since the first training day / GNPRTB 6175 (2)) / 100
    (`FUN_0055ca90`); the difference goes into ForceExperience and Luke's
    `DagobahCompleted` is set;
  - the Luke-Vader encounter (`FUN_00560200`), with GNPRTB 3084..3087: not
    read in detail.

So in the original Luke grows by evading capture, by Dagobah, by the
encounter, and by Jedi Training once someone of his side reaches Force
100. Ported: the mission detection's evade gain (2026-10-18,
`MissionEffect::ForceExperienceGained`,
`evidence/2026-10-18-evade-force/`) and Jedi Training (2026-10-18,
`MissionKind::JediTraining`, `Character::force_training`, save v71,
`evidence/2026-10-18-jedi-training/`). The AI's Jedi Training planner is
not read, so only the player sends it.

## Discovering others (timer `0x389`)

- `DiscoveringForceUser` (`FUN_004ef070`) holds for a trainer
  (`is_jedi_trainer`, `FUN_004ed220`) with Force level >= GNPRTB 2580 (80)
  (`FUN_0053eb70`) that is usable, not on a mission and not captured. It
  is recomputed when those change (slots `+0x110`, `+0x268`, `+0x31c`,
  `+0x34c`) and at creation.
- Its hook (slot `+0x364`, `FUN_004f1b70`) arms timer `0x389` with it, on
  record `+0x54`+`0x20`: GNPRTB 2582 (1) + rand(0..=2583 (0)) days
  (`FUN_004ef750`, disassembly `0x4ef7aa..0x4ef7c8`: `ecx = [esi+0x54] +
  0x20`). So it fires every day. An earlier version of this note gave
  2584/2585; those fill record `+0x28`, the captive escape timer `0x38a`.
- Each fire (`FUN_00560a30`) walks the characters of the discoverer's side
  in its container (`FUN_004f2640(container, 1, side)`). For each one not
  captured, not on a mission, not itself, with `ForcePotential` and without
  `ForceAware`:
  - Leia (`0x31000241`) only when the discoverer is Luke (`0x32000242`)
    with its `+0xb0` bit 0, `KnowsHeritage` (Luke's setter `FUN_0056f650`,
    called from `FUN_00560200`), and then `FUN_0056fe90(1)` sets Leia's
    `KnowsHeritage` (`+0xb0` bit 0, slot `+0x3ec`);
  - anyone else at once;
  - then `ForceAware` is set and `+0xa4` stores the discoverer.

So only Vader, and Luke once his Force reaches 80, discover, each day, the
Force of the free characters of their side beside them.

- Setting the discoverer `+0xa4` syncs to the side views (`FUN_004f0940`),
  which send notification `0x362` (slot `+0x3bc`, `FUN_004f20e0`,
  `CharacterForceUserDiscoveredKeyNotif`). Its builder `FUN_0048f1e0` makes
  "Future Jedi Discovered" (TEXTSTRA `0x7178` with `0x7179` or `0x717a` by
  `FUN_004ed310`, GNPRTB 2581), or "Leia Uses Force" (`0x71d8`/`0x71d9`)
  for Leia, shown to the character's side.

## Force detection between sides (`FUN_0058a3f0`, "force_detect")

Called from `FUN_0058a2c0`. With two characters that both have a Force
level, the chance is `force1 + force2 + GNPRTB 3587 (-100)` percent
(`FUN_0055e4d0`, rolled with `FUN_0053e2f0`). On success, unless the
first is a decoy (`+0x78` bit 0), the mission's `+0x4c` is set, and
`FUN_0055ff60` and `FUN_0058a530` run (a system-wide notice to both sides
through `FUN_0050d5a0(6, side)`, and a record of the pair). Not read
further.

## Injury death, corrected

`character-injury.md` said the subclass `0x00665be0` dies of any injury.
Its slot `+0x2e0` (`FUN_0056f160`) calls `FUN_0053eb30`, which is
`FUN_0053e2f0(GNPRTB 2577)`: a 25 percent roll. That subclass dies of an
injury with 25 percent.

## Ported (stage 1, 2026-10-17)

`Character::force_potential`, `force_aware`, `force_base` (save v69),
`force_base_due`, `roll_force_base` and `force_level` (`+0x90` kept 0).
Seeding rolls the potential for every character and the base when due;
the effective combat, `FastHeal` and the Force Ranking read the level.
Potential no longer makes a character aware. Evidence:
`docs/qa/2026-09-08-full-functionality-audit/evidence/2026-10-17-force-potential/`.

## Ported (stage 1b, 2026-10-18)

`Character::effective_skill`: diplomacy `+0x7c` and espionage `+0x7e`
with the Force share (slots `+0x1dc`, `+0x1e0`), read by the missions
(`FUN_00573ff0`, `FUN_00573090`) and the Status window. Luke's data (50,
50, 90 at Force 50) gives Figure 3.46's 75, 75 and 135. Evidence:
`docs/qa/2026-09-08-full-functionality-audit/evidence/2026-10-18-force-share/`.

## Ported (stage 2, 2026-10-18)

`force_discovery.rs`: `discovering_force_user`, timer `0x389` per trainer
(`ForceDiscoveryState`, save v70), the walk of the trainer's container
(its fleet, else the characters at its system in no fleet), Leia's rule
on Luke's `heritage_known` (the port's `KnowsHeritage`), the awareness
hook's Force base roll, and `Character::force_discoverer` (`+0xa4`, by
record). Over seven 600-day and five 3000-day dual-AI campaigns no
discovery happens: Vader qualifies, but the Empire's latent characters
are recruited away and never stand free beside him, and Luke stays at
Force 50. The message `0x362` followed the same day
(`story_messages::force_user_discovered`,
`evidence/2026-10-18-force-discovery-message/`). Evidence:
`docs/qa/2026-09-08-full-functionality-audit/evidence/2026-10-18-force-discovery/`.

## Against the port (before 2026-10-19)

`jedi.rs` kept `force_tier`, `force_experience` and `is_discovered_jedi`,
and advanced tiers by flat chances. It is removed (save v87,
`jedi-rs-replacement.md`); the port keeps the original's fields below.

## Open

- The Luke-Vader encounter `FUN_00560200` (GNPRTB 3084..3087), and who
  sends the Dagobah mission (notification `0x221`).
- Leadership's share of `+0x92` (`FUN_004eede0`): settled 2026-10-19, the
  seat of power's bonus (`seat-of-power-leadership.md`).
- `FUN_0058a2c0` (who calls the detection) and `FUN_0055ff60`.
- When `FUN_00560200` sets Luke's `KnowsHeritage` (the heritage
  encounter); the port's story events set `heritage_known` on their own.
- The builder's `+0x28`, `+0x30` and `+0x32` fields.
- Which objects the subclasses `0x006657f0` and `0x00665be0` make.
