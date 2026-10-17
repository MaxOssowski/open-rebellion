# The informant incident (timer `0x38e`)

Read 2026-10-17 for F-029. The port had raised the informant incident
whenever a held system was short of regiments; the original runs it on a
per-system timer.

## Arming

- `FUN_0050c820(old_side, new_side)` arms timer `0x38e` on the system's
  record `+0x54`+`0x40` twice, with `old_side != 3` and then
  `new_side != 3` (`FUN_0053fa60`). Arming with an unchanged flag changes
  nothing, so the timer runs while the system has a side and stops when it
  has none (side bits 3).
- Callers: the side hook `FUN_00510820` (System slot `+0xf0`) and the
  system's setup `FUN_0050e200`, which first sets the record's minimum to
  GNPRTB 7703 (30) and spread to 7704 (970) (disassembly `0x50e38e`..
  `0x50e39b`). So it fires every 30 to 1000 days, with a new delay each
  time (`FUN_005862a0`).

## Firing

`FUN_0050cbe0`:
- with side bits other than 3, it takes the holder's support
  (`FUN_00507270(side)`) and calls `FUN_00559f90`: `FUN_0053e2f0(support)`,
  a 0..99 draw below the support, and the incident happens when that fails.
  So the chance is `100 - support` percent;
- it then pulses `+0x88` bit 17, InformantIncident (`FUN_0050aac0(1)`,
  then `(0)`).

The bit's hook (System slot `+0x24c`, `FUN_00511860`) syncs the bit to both
sides' views, which post notification `0x153`, Informants Provide
Information (builder `FUN_00491060`, picture `0x3e8`, TEXTSTRA 29008
"Informants Provide Information" and 29009 "Informants have provided us
with information about {system}."). On the rising edge it runs
`FUN_0050d510`.

## The information

`FUN_0050d510`:
- draws `FUN_00559db0`: INFORMTB (table `0x2a`) at GNPRTB 7709 (1) +
  rand(0..=7710 (7));
- passes the code to `FUN_0050d5a0` with side 2 when the holder is side 1,
  else side 1 (side 3 stays 3), so the informants tell the other side.

INFORMTB's rows (threshold, code): (1,1) (2,2) (3,3) (4,4) (5,7) (6,8)
(7,9) (8,10). The draw is 1..8, so codes 5, 6, 11 and 12 never occur.

`FUN_0050d5a0(code, side)` syncs parts of the system to `side`'s view
(`FUN_004f7e30` per type range, and the system's own fields):

| Code | What the side learns |
|---|---|
| 1 | explored, populated, support and control fields |
| 2 | energy and raw materials |
| 3 | units `0x10..0x20` and defense facilities `0x22..0x28` |
| 4 | managers `0xa0..0xb0` and facilities `0x28..0x30` |
| 7 | fleets `0x08..0x10` and all objects twice (`1..0xff`) |
| 8 | everything, through the side's slot `+0x24` |
| 9 | characters and special forces `0x30..0x40` |
| 10 | everything (`1..0xff`), through slot `+0x24` |

## Port (2026-10-17)

`informant.rs`: `InformantState` (each held system's timer, save v66),
`InformantSystem::advance`. The headless simulation runs it after the
uprisings and reveals the system in the fog it keeps when that is the
recipient's. The app reveals it in the recipient's fog and posts the
message to the recipient.

port: the port's fog knows systems, not their objects or fields, so every
code reveals the system; the code is kept in the report. The economy's
invented trigger (a troop deficit) is gone.
