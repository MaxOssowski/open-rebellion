# An Alliance regiment's Status, against A0 (2026-11-02)

Surface ledger OBJ-10 C001, "troop types".

## A0

The A0 run used the Windows 10 VM and the Alliance's DS_flip_allian save, day 180. The captures are in `.artifacts/a0-vm/troopstatus/`, with hashes in `a0-sha256.txt`.

- **`01-display`.** The Troop Finder's Display on Mon Calamari opens its Defenses window on Trooper Regiments: three Alliance Army Regiments.
- **`02-menu`.** The regiment's menu: Move, Confirmed Move, Encyclopedia, Status, Scrap.
- **`03-status`.** "Trooper Regiment Status" at (108, 79), with "Alliance Army Regiment" on two lines in the name box. Its rows:

  | Label | Value |
  |---|---|
  | Attached | Mon Calamari |
  | Status | Awaiting Orders |
  | Maintenance Cost | 3 |
  | Attack Strength | 3 |
  | Defense Strength | 5 |
  | Bombardment Value | 5 |
  | Detection Value | 10 |

## The port

The `regiment` case of `status-window.mjs` now measures, for the Alliance, the title and the seven labels against A0 `03`. Every box is within 1 pixel. The Empire's Stormtrooper Regiment was already compared against `status/12-troop`.

Run `status-window-2026-11-02T19-51-24-228Z-850801`: pass.
