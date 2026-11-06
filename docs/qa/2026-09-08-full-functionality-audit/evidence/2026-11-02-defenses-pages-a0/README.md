# The Defenses window's Shields and Fighters pages against A0 (2026-11-02)

Surface ledger OBJ-02: C007 "fighter tab" and C008 "shield tab".

## A0

The A0 run used the Windows 10 VM, the Empire's save and Coruscant's Defenses window at (196, 91). The captures are in `.artifacts/a0-vm/status/` (2026-10-30):

- **`17-def-tab`.** The Planetary Shields page, empty, its label "Planetary Shields" at the window's (67..172, 50..63).
- **`19-tab3`.** The Fighter Squadrons page: nine TIE Fighters in rows of three, with a scroll bar. Its label is at (65..168, 50..66).

## The port

The `defenses` case of `sector-quadrants.mjs` now clicks the Shields and Squadrons tabs after the tab tips. For each page it measures the label's ink box relative to the window:

| Page | A0 | Port |
|---|---|---|
| Shields | 67..172, 50..63 | 67..166, 51..63 |
| Squadrons | 65..168, 50..66 | 65..168, 51..63 |

- **Tolerance.** Left and top within 2 pixels, width within 15%. The Shields label is 6 pixels narrower: GDI's hinting, the accepted text deviation. A0's 66 is the descender of q and g.
- **Earlier checks.** The tabs' art (normal, selected, empty) was already compared by the same gate.
- **Run.** `sector-quadrants-2026-11-02T18-32-44-125Z-784713` (Arch Chromium, both sides): pass.

## The Personnel and Regiments pages (OBJ-02 C005, C006), day 184

**A0** (Alliance, DS_flip_allian, `.artifacts/a0-vm/personnel-states/`):

- **`16-pzob-personnel`.** Pzob's Defenses window at (118, 86), on its Personnel page. The label "Personnel" is at the window's (89..144, 51..63). Coruscant's window in `status/13` puts it at the same place.
- **`15-mon-calamari-regiments`.** Mon Calamari's window at (305, 86), opened from the Troop Finder on its Trooper Regiments page. The label is at (63..170, 51..65).

**The port.** The `defenses` case of `sector-quadrants.mjs` now measures both labels as it does the Shields and Squadrons labels: left and top within 2 pixels, width within 15%.

| Page | A0 | Port |
|---|---|---|
| Personnel | 89..144, 51..63 | 89..144, 51..63 |
| Regiments | 63..170, 51..65 | 65..170, 50..65 |

Run `sector-quadrants-2026-11-02T22-31-18-193Z-1201960` (Arch Chromium, both sides): pass.

**Open: the garrison line.** On the Regiments page the port draws "Garrison Requirement: N" under the label. It follows `FUN_004a90d0`: page 2, and the system's holder (`+0x24 >> 6 & 3`) equal to the player's side (`FUN_0041cdb0` → `FUN_00435fc0`).

- A0's Empire window of Coruscant shows the line (`status/11`).
- A0's Alliance windows of Mon Calamari and Pzob do not (`15`, `17-pzob-regiments`, day 184). Both systems are the Alliance's and the player is the Alliance.

- **`18-umgul-imperial-garrison`.** In the same Alliance game, Umgul's window shows the line: "Garrison Requirement: 0". Umgul is the Empire's, opened from the Troop Finder's Imperial tab.

So in both games A0 shows the line on the Empire's systems, and in the Alliance game not on the Alliance's. That does not fit the reading above. `FUN_0041cdb0` returns `DAT_006b14e4 +0xc +0x24`, the side `FUN_00435bc0` gives the player object (`FUN_004861b0(side)`). Why the line follows the Empire is not traced. The port keeps the reading. The gate compares only the first line, rows 50..65.

**Further trace (2026-11-03).** `FUN_00435bc0(param_2, param_3)` builds `DAT_006b14e4`:

- with `param_3 == 1`, `+0xc` (and `+0x10`) is `FUN_004861b0(1)`, the side-1 player object, and `+0x14` is side 2;
- otherwise `+0xc` (and `+0x14`) is side 2 and `+0x10` is side 1.

So `FUN_0041cdb0` returns the local player's side, as read. That predicts the line on the Alliance's systems in an Alliance game. A0 shows the opposite, five times:

- Coruscant, Empire game: shown;
- Mon Calamari and Pzob, Alliance game: none;
- Umgul, Alliance game: shown (`18-umgul-imperial-garrison`).

Two explanations remain open:

- the regiment page's `param_2` is not 2 here;
- `FUN_004a90d0`'s `iVar2` is not the window's system at that point.

The port keeps the code's reading until one of them is checked in the asm.

**Settled (2026-11-06).** Neither explanation was needed. DS_flip_allian builds its messages for the Empire behind an Alliance cockpit, so `FUN_0041cdb0` returned the Empire there. An unedited Alliance game shows the line on its own Yavin. See `../2026-11-06-garrison-line-clean/`.
