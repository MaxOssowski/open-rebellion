# Character injury and healing (timer `0x388`)

Read 2026-10-17 for F-038. The port computed injuries (the uprising
incident, mission evasion and capture) and then dropped them.

## The field

Character `+0x94` is the injury, a short set by `FUN_004ee2d0` (clamped to
`0..0x7fff`; it notifies slot `+0x32c`).

- **Injury.** Slot `+0x2e4` (`FUN_004ef5f0`) rolls `FUN_0053e990` against
  the character's effective combat (slot `+0x1f0`, `+0x86`). With chance
  `max(GNPRTB 2565, 100 - combat)` it gives
  `rand(chance) + rand(GNPRTB 2567) + GNPRTB 2566` and passes it to slot
  `+0x2f0` (`FUN_004ef6d0`). That adds a positive amount to `+0x94`, then
  asks slot `+0x2e0` whether the injury kills: if so, the character is
  destroyed with reason 10.
- **Death.** For the class Character (vtable `0x0065ca70`) slot `+0x2e0` is
  `FUN_006158b0`, which returns 0: a character never dies of an injury.
  The subclass at `0x00665be0` (among the mission vtables) returns GNPRTB
  2577 (25), so any injury kills it. Which objects that subclass makes is
  not traced.

## What the injury does

The hook (slot `+0x32c`, `FUN_004f1720`):
- recomputes the effective combat `+0x86` (`FUN_004eed90` ->
  `FUN_0053e900`): combat `+0x62` + combat × Force level `+0x8c` / 100
  (`FUN_0053e190`) - injury, at least 0 (`FUN_0053e120`), at most
  `0x7fff` (`FUN_004edf30`);
- recomputes the Healing flag (`FUN_004ef030`, below);
- clears the command `+0x96` (`FUN_004eeed0` -> `FUN_004ee350(0)`) unless
  the character is uninjured, free, existing and in a fleet-type container
  (`0x50..0x7f`);
- with a new injury and `CanResignFromMission` (`+0x78` bit `0x10`), asks to
  resign from the mission (`FUN_00534640`);
- sets `Damaged` (`+0x50` bit `0x200`, `FUN_004f7800`) to injury != 0.

The Status window shows "Injured" for a non-zero `+0x94`
(`status-window.md`), and its Combat Rating is `+0x86`.

## Healing

- `CanHeal` (`+0xac` bit `0x2`, `FUN_004eef90`): the container (`+0x1c`)
  is of the character's side, and it is neither captured (`+0xac` bit 0)
  nor destroyed (`+0x50` bit 3).
- `FastHeal` (bit `0x4`, `FUN_004eeff0`): Force level `+0x8c` >= GNPRTB
  2560 (80) (`FUN_0053e970`).
- `Healing` (bit `0x40`, `FUN_004ef030`): `CanHeal`, not en route (`+0x50`
  bit `0x10`), and injured.
- The Healing hook (slot `+0x360`, `FUN_004f1b40`) arms timer `0x388` with
  the flag, on record `+0x54`+`0x18`. Its minimum is GNPRTB 2561 (1) with
  `FastHeal`, else 2562 (1) (`FUN_004ef750` at creation, `FUN_004f1af0`
  when `FastHeal` changes).
- Each fire (`FUN_004ef100`) lowers the injury by GNPRTB 2563 (2) with
  `FastHeal`, else 2564 (1), at least to 0.

## Port (2026-10-17)

- `Character::injury` (save v68), `Character::injure` (slot `+0x2f0`),
  `Character::effective_combat` (`+0x86`).
- The missions' and the uprising incident's injuries are now stored.
- `missions::member_skill` reads Combat as the effective combat; so does
  the uprising incident's injury roll and the Status window.
- `healing.rs`: `HealingState` (save v68) and `HealingSystem::advance`.
- The Status window shows "Injured".
- port: the combat and Force level are the rolled templates (base plus
  half the variance). The container is the character's fleet, else its
  system, whose side is its holder; a character with neither stands for
  "en route".

Not ported yet: the resign request, the command clearing, the `Damaged`
bit, death by injury for the `0x00665be0` subclass, and the Character
Injured / Healed messages (`0x106`).
