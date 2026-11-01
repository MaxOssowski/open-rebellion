# Scrapping a fleet in A0 (VM Alliance game, day 318)

Fleet 1 at Kimanan (the Alliance's own system), from its Fleet window
(`23-fleet1-*.png`): Corellian Corvette 1, Medium Transport 1, no
squadrons, two Alliance Fleet Regiments, no personnel.

- `24-fleet1-menu.png`: its menu shows Scrap white. Fleet 4, which had
  just withdrawn and was still arriving, shows every order grey but
  Encyclopedia and Status (`22-fleet4-menu.png`), Scrap included.
- `25-fleet1-scrap-confirm.png`: "Are you sure you want to scrap the
  following units?" and one line, "Fleet 1".
- `26-after-scrap.png`, the same day: Fleet 1 is gone. The top bar's
  second readout went from 223 to 243 (+20) and the third from 173 to 220
  (+47); the first stayed at 162.

From the DAT (`CAPSHPSD`, `TROOPSD`; names from TEXTSTRA):

| Unit | Refined cost | Maintenance |
|---|---|---|
| Corellian Corvette (0x45, 10053) | 14 | 23 |
| Medium Transport (0x46, 10054) | 10 | 12 |
| Alliance Fleet Regiment (0x01, 9280), x2 | 8 each | 6 each |

Half the refined costs: (14 + 10 + 8 + 8) / 2 = 20, the second readout's
rise. The maintenance: 23 + 12 + 6 + 6 = 47, the third readout's rise. So
scrapping a fleet scraps every ship and every regiment aboard it, each
refunding half its refined cost (reason 0x14 passed to each child by
`FUN_004fb520` / `FUN_004f8710`; `FUN_00530270`).
