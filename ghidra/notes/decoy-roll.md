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

## Open

- `FUN_00589620`, the only caller of `FUN_00588b90`, has no code or data
  references in the analysed program, so the object class, the pool at
  `+0x30`, and slots `+0x1bc` / `+0x1c4` of the checked object are
  unidentified. The kind-3 walk and the `+0x96` match are unidentified too.
- Until those are known the port's `MissionSystem::check_decoy` (defender
  espionage straight into FDECOYTB) stays unwired: it reads the wrong table
  input and has no recovered call site.
