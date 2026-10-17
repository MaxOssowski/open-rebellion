# Force potential, awareness and discovery (timer `0x389`)

Read 2026-10-17 for F-038. The port's `jedi.rs` runs its own Force tiers
with flat probabilities; this note records what the original keeps and
does, as the base for replacing it.

## Fields and flags

| Where | Name | Meaning |
|---|---|---|
| character `+0x8c` | Force level | short; Luke starts at 50, Leia at 10, Vader 150, the Emperor 120 (MJCHARSD `jedi_level`) |
| character `+0xac` bit `0x10` | `ForceAware` | the character knows its Force (`FUN_004ee950`) |
| character `+0xac` bit `0x20` | `ForcePotential` | the character has the Force (`FUN_004ee9c0`) |
| character `+0xac` bit `0x80` | `DiscoveringForceUser` | it can find the Force in others (`FUN_004eeaa0`) |
| character `+0xa4` | discoverer | the key of the one who found its Force (`FUN_004ee680`) |
| class `+0x98` | `jedi_probability` | MJCHARSD file offset `0x74` (class = file + `0x24`, as the refined cost `+0x48` is file `0x24`) |
| class `+0x9c` | `is_known_jedi` | file `0x78` |
| class `+0xb8` | `is_jedi_trainer` | file `0x94` |

MJCHARSD: 577 (`0x241`, family `0x31`) is Leia, 578 (`0x242`, family
`0x32`) Luke (trainer, Force 50), 640 Vader (Force 150), 641 the Emperor
(trainer, Force 120).

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

## Discovering others (timer `0x389`)

- `DiscoveringForceUser` (`FUN_004ef070`) holds for a trainer
  (`is_jedi_trainer`, `FUN_004ed220`) with Force level >= GNPRTB 2580 (80)
  (`FUN_0053eb70`) that is usable, not on a mission and not captured. It
  is recomputed when those change (slots `+0x110`, `+0x268`, `+0x31c`,
  `+0x34c`) and at creation.
- Its hook (slot `+0x364`, `FUN_004f1b70`) arms timer `0x389` with it, on
  record `+0x54`+`0x20`: GNPRTB 2584 (100) + rand(0..=2585 (1000)) days
  (`FUN_004ef750`, disassembly order).
- Each fire (`FUN_00560a30`) walks the characters of the discoverer's side
  in its container (`FUN_004f2640(container, 1, side)`). For each one not
  captured, not on a mission, not itself, with `ForcePotential` and without
  `ForceAware`:
  - Leia (`0x31000241`) only when the discoverer is Luke (`0x32000242`)
    with its `+0xb0` bit 0, and then `FUN_0056fe90(1)` sets Leia's `+0xb0`
    bit 0 (slot `+0x3ec`);
  - anyone else at once;
  - then `ForceAware` is set and `+0xa4` stores the discoverer.

So only Luke (from Force 80) and the Emperor discover, every 100 to 1100
days, the Force of the free characters of their side beside them.

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

## Against the port

`jedi.rs` keeps `force_tier`, `force_experience`, `is_discovered_jedi` and
`jedi_probability`, and advances tiers by flat chances. None of
`ForcePotential`, `ForceAware`, `DiscoveringForceUser` or timer `0x389`
exists. Porting this replaces part of `jedi.rs`.

## Open

- How the Force level `+0x8c` grows: what writes `+0x90` (training
  missions?).
- Leadership's share of `+0x92` (`FUN_004eede0`): what `+0x92` holds.
- `FUN_0058a2c0` (who calls the detection) and `FUN_0055ff60`.
- What Luke's `+0xb0` bit 0 means, and who sets it.
- Which objects the subclasses `0x006657f0` and `0x00665be0` make.
