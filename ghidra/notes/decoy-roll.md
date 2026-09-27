# Decoy roll (TDECOYTB / FDECOYTB)

Recovered 2026-09-26 from REBEXE.EXE with Homebrew Ghidra 12.1.3 (read-only,
no analysis). Finding F-019 in the full-functionality audit.

## Roll

`FUN_0055e410(side, a, c, b, in_fleet, &ok, &hit)`:

```c
x = (a - b) - FUN_0053e190(c, DAT_006bb710);   // c * G / 100
FUN_0053e340((in_fleet != 0) + 10, x, &ok, &hit);
```

- `FUN_0053e190(v, p)` is `FUN_0053e170(v, p, DAT_00661a88)`, which is
  `v * p / DAT_00661a88` (`FUN_0053e150` is plain integer division). The
  divisor is the percent base 100.
- `DAT_006bb710` is GNPRTB 0xe04 (3588), loaded by `FUN_0055e340` through
  `FUN_0053e390(0xe04, &DAT_006bb710)`; shipped value 35.
- Table id `10` is TDECOYTB and `11` is FDECOYTB (resource ids from
  `FUN_0058b420`, see `uprising-incident.md`). FDECOYTB is used when the
  checked object sits in a fleet.
- `FUN_0053e340` looks the table up (`FUN_0053e240` -> `FUN_0055bed0` ->
  `FUN_0058b7e0`, value at record `+0x24`) and then succeeds when
  `FUN_0053e2e0() < value`; `FUN_0053e2e0` is `FUN_0053e290(DAT_00661a88 - 1)`,
  a uniform draw in `0..=99`.

## Inputs (`FUN_00588b90`)

- `a` is slot `+0x1e0` of the decoy candidate, a random member of a pool
  (`FUN_00588700` draws `FUN_0053e290(count - 1)` over the list at `+0x30`).
  In the character vtable 0x0065ca70 slot `+0x1e0` is `FUN_004edc00`, which
  returns the short at `+0x7e`: effective espionage.
- Effective skills are recomputed by `FUN_004f16a0` (`FUN_004eecb0`,
  `FUN_004eecf0`, `FUN_004eed90`, ...). `FUN_004eecf0` sets `+0x7e` through
  `FUN_004edd30` to `base(+0x5a) + base * (+0x8c) / 100` (`FUN_0053e940`).
  The base skills are consecutive shorts from `+0x58` (diplomacy, espionage,
  ship design, troop training, facility design, combat, leadership, loyalty at
  `+0x66`); the effective copies run from `+0x7c` to `+0x8a` (leadership
  `+0x88`, loyalty `+0x8a`), and `+0x8c` is a shared percent modifier.
- `c` is slot `+0x1e0` (effective espionage) of the counterpart found by
  `FUN_00509330(system, key, &out)` (or `FUN_004fd790` on the fleet): it walks
  kind-3 objects of the system holder's side and returns the first whose
  `+0x96` short equals `key`. `key` is slot `+0x1bc` of the checked object.
- `b` is slot `+0x1c4` of the checked object.
- The roll only counts when a counterpart was found. On success the caller
  also requires `FUN_00534720(candidate, 1, ctx)` and `FUN_00558070(object, 1,
  ctx)`, then decrements `+0x34`; otherwise `FUN_005888f0` runs.

## Call chain (recovered 2026-09-27)

`FUN_00589620` has no code reference because it is a virtual slot: the pointer
at `.rdata` `0x0066a87c` is slot `+4` of the functor vtable `0x0066a878`, set
by `FUN_005895d0` (base `FUN_00587250`, which stores the manager at `+4`).

1. `FUN_00547f60(mission, ctx)` runs a mission. It checks the mission
   (`FUN_00520e40`, `FUN_00520ad0`), finds the target system (`FUN_00521070`,
   types `0x90..0x98`), builds a phase manager on the stack
   (`FUN_005897c0`), and calls `FUN_005898f0`.
2. `FUN_005898f0` calls `FUN_00589970`, which runs the phases
   `FUN_00589a40`, `FUN_00589e40`, `FUN_00589f10`, `FUN_0058a020`,
   `FUN_0058a130`, and `FUN_0058a1c0`.
3. `FUN_0058a020` builds the decoy functor when manager `+0x4c` is clear and
   walks the target system's defenders through `FUN_005875e0`,
   `FUN_00587600`, and `FUN_00587620`, which are `FUN_00587640` with
   different category flags.
4. `FUN_00587640` visits, at the system in manager `+8`: its special forces
   (`FUN_005039d0`), its regiments (`FUN_00504c40`), and in each fleet
   (`FUN_004ffe70`) the ships (`FUN_00502db0`) and special forces. It calls
   the functor with (defender, fleet or 0, &stop, ctx) and skips a defender
   with `+0x58` bit 0 when asked.
5. `FUN_00589620` draws a random decoy from the manager's pool
   (`FUN_00588700`, list at `+0x30`) and, if one exists, calls
   `FUN_00588b90(manager, decoy, defender, fleet, ctx)`. There `a` is the
   decoy's effective espionage, `key` and `b` are the defender's slots
   `+0x1bc` and `+0x1c4`, and FDECOYTB applies when the defender is in a
   fleet.

So a decoy is a character attached to a mission. Each enemy defender at the
target is drawn off by a random decoy on a TDECOYTB/FDECOYTB roll, and a
successful decoy decrements the manager's `+0x34` count.

## Open

- The defender's slot `+0x1bc` is the shared stub `FUN_00526f00` (return 3)
  in the regiment vtable and in dozens of others, and the character short at
  `+0x96` it is matched against is unidentified, so the counterpart and `b`
  (slot `+0x1c4`) are not yet named per defender type.
- The remaining phases, the pool's source, and the effects of
  `FUN_00534720`, `FUN_00558070`, and `FUN_005888f0` are not traced.
- The port has no decoy characters on missions. Its invented `is_decoy` roll
  and `MissionSystem::check_decoy` are removed (2026-09-27); the `is_decoy`
  field stays only for the save layout.
