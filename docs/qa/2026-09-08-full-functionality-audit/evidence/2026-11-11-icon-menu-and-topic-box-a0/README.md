# The defenses icon's menu and the Topic box against A0

Date: 2026-11-11. Covers CMD-03-C018 (quadrant icon right-click menu) and
the Encyclopedia Topic box of `2026-11-11-finder-prefix-search`. Both
cells stay partial: this stage records A0 only and changes no code.

## A0

Windows 10 VM, original REBEXE, the Empire game of
`2026-11-10-quadrant-icon-art-a0`, days 12 to 106. The captures are in
`.artifacts/a0-vm/2026-11-11-icon-menu-topic/`, with their SHA-256 in
`SHA256SUMS` (39 files). The game was changed for the test: Palpatine, the
six Coruscant regiments and nine TIE squadrons were moved to Sluis Van, and
one TIE squadron to Umgul.

## The defenses icon's Move

Each row is a right-click on the system's defenses icon, the pointer then
moved off the menu.

| Capture | System, day | Defenses team | Move / Confirmed Move |
| --- | --- | --- | --- |
| `00`, `01`, `02` | Coruscant, 12 | Palpatine, 6 regiments, 10 TIE squadrons | grey |
| `07`, `08` | Sluis Van, 14 | 1 regiment | white |
| `10` | Coruscant, 15 | 6 regiments, 10 TIE (Palpatine left) | grey |
| `11` | Sluis Van, 15 | 1 regiment, Palpatine on his way | grey |
| `12`, `13`, `14` | Umgul, 15 | Piett | white |
| `16` | Coruscant, 16 | 10 TIE (regiments left) | grey |
| `18` | Coruscant, 17 | 1 TIE | grey |
| `30`, `31` | Umgul, 18 | Piett, 1 TIE on its way | grey (Scrap grey too) |
| `33` | Umgul, 105 | Piett, the TIE arrived | white |
| `34` | Sluis Van, 105 | Palpatine, 7 regiments, 9 TIE | white |
| `35`, `36` | Coruscant, 105 | Vader | white |

Each member's own menu had Move white on day 12 to 13: Palpatine (`03`),
a regiment (`04`), the first and the last TIE squadron (`05`, `06`). A
squadron on its way has its own Move grey (`31`), and white after it
arrives (`32`).

Coruscant has no shipyard and no training facility (`37`: Ship
Construction 0:0, Troops in Training 0:0), so nothing was being built
there.

**Findings.**
- The port's rule holds where it was tested: a free team of the side has
  Move white (Sluis Van, Umgul, the full team at Sluis Van on day 105). A
  character on its way makes it grey (`11`), as `defenses_move_team`
  already counts.
- **Divergence.** A regiment or squadron on its way to the system also
  makes the icon's Move grey (`30` against `12` and `33`), and Scrap with
  it. The port builds the team from the units on the planet and the
  characters on their way, so it would show Move white at Umgul on day 18.
- Coruscant was grey from day 12 to 17 with every visible member free,
  and even with one TIE left. The likely cause is the same: the Empire's
  opening deliveries on their way to Coruscant. That is inferred, since
  those units were not seen.
- Source to trace: whether `FUN_00504c40` and `FUN_005039d0` (walk mode 3)
  see units on their way as the target's children, as the mission transit
  does for characters (`mission-lifecycle.md`).

## The Topic box

| Capture | Step | Seen |
| --- | --- | --- |
| `20` | the Encyclopedia opened from the console (480, 445) | empty box, All Databases, nothing chosen |
| `21` | `cor` typed with no click on the box | the box reads `cor`; Corellia is chosen and scrolled to the last visible row |
| `22` | `u` typed | `coru`; Coruscant is chosen, on the last visible row |
| `23` | Enter | the Coruscant topic |
| `24` | Index (573, 215) | the box reads `Coruscant` with a caret; the list as before |
| `25` | `s` typed | `Coruscants`; Coruscant stays chosen, no scroll |
| `26` | the System category icon | no caret; Coruscant chosen, on the first row |
| `27` | `b` typed | nothing changes, in the box or in the list |
| `28`, `29` | a click on the box, then `b` | caret; `Coruscantsb` |

**Findings.**
- **Divergence.** A freshly opened Encyclopedia gives the keys to the
  Topic box (`21`). The port's `open_original_encyclopedia_at` sets
  `original_box_focused = false`, so typing does nothing until a press on
  the box or Index.
- Index gives the box the keys (`24`, `25`) and a category icon takes them
  away (`26`, `27`), as `command_focus` does. With the keys on the list,
  a letter does nothing.
- An edit that keeps the chosen row scrolls nothing (`25`), as the port
  does.
- Not yet compared with the port: the chosen row lands on the last visible
  row after an edit (`21`, `22`) and on the first row after a category
  change (`26`); the box shows the topic's name after Index (`24`).

## Not captured

- The missing-movie box: it needs a movie file removed in the guest.
- The fleet window's left list with a scroll bar: it needs about six
  fleets in one system. Sluis Van has one (`38`).
- A defenses icon dropped on another system, and the Confirmed Move
  window.
