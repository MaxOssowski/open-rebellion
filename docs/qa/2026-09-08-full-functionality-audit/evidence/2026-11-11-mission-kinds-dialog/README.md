# Mission kinds without A0 in Create Mission (2026-11-11)

Surface ledger OBJ-11: C005 Death Star Sabotage, C010 Jedi Training,
C011 Reconnaissance, C013 Rescue, C015 Facility Research, C016 Ship
Research, C017 Troop Research.

## A0 searched

None of the existing A0 sets shows the Create Mission dialog with one of
these kinds, offered or chosen. The sets searched under
`.artifacts/a0-vm/`:

- `kinds/`. Vader, Needa, Veers and the Emperor alone on their own Coruscant
  (`31-needa-*`, `32-veers-*`, `32-emperor-*`, every scroll frame). They list
  Recruitment, Diplomacy and Espionage only. The Emperor, a Force user, is
  not offered Jedi Training on his own.
- `kinds2/`. Han on Bortras (`01`, `02`): Incite Uprising and more, but the
  list was not scrolled. `70` is a recon team's menu, with Mission grey: the
  team was already on its mission.
- `kinds-empire/`, `mission-kinds-679/`, `research-display/14..17`,
  `menus2/11..13`, `menus3/53..61`. Diplomacy, Recruitment, Espionage,
  Incite Uprising and Subdue Uprising only.
- Missions windows, not the dialog. `specforces/02` shows a running
  Reconnaissance, and `personnel-states/29` and `37` show Ship and Facility
  Design Research. These rows draw the kind's mini (`0x5c00` range), not the
  dialog's 130 by 65 icon. They show only that the kinds exist for the
  Alliance.
- `2026-11-11-mission-kinds/` (new, an Alliance game on day 7; hashes in its
  `SHA256SUMS`). In `00-dodonna-on-yavin-dialog` and
  `01-dodonna-yavin-kinds-list`, Jan Dodonna alone on his own Yavin (HQ, no
  shipyard) is offered Espionage alone. So no research kind is offered
  without a yard of its area. This agrees with the port's rule
  (`FUN_0056c510`: a research kind needs an own-side yard of the tree's area
  and a member with the tree's skill). The port's case for this exact
  state, with Yavin's facilities and Dodonna's skills as in A0, was not run.

## Browser

`tools/interface-parity/mission-dialog.mjs` now runs four more cases on
each side, through the original entry: right-click the agent in the
Defenses window, choose Mission, and release on the target. Each case
checks the following:

- the kinds offered;
- the dialog's chrome against the STRATEGY bitmaps;
- that the shown kind's icon matches its GOKRES resource at the window's
  (70, 86), where A0 has the other kinds. This is TEXTSTRA id & 0xfff, plus
  0x1000 for the Empire (`FUN_0046a9c0`);
- that Cancel starts nothing.

| Case | Fixture | Kinds offered | Icon (Alliance / Empire) |
|---|---|---|---|
| `rescue` | RescueTargeting (58) | Rescue | 3089 / 7185 |
| `death-star` | DeathStarTargeting (60), Alliance only | DeathStarSabotage | 3137 |
| `recon` | ReconTargeting (63), a recon special force | Reconnaissance | 3093 / 7189 |
| `research` | ResearchTargeting (64), a ship designer at an own yard | ShipDesignResearch, Espionage | 3104 / 7200 |

The Empire has no Death Star Sabotage record. The fixture's Alliance Death
Star exists only for this case, so the Empire's case is not run.

Facility and Troop Training Research have no browser case. The fixture's
designer has only the ship skill, and its target has only a shipyard. Jedi
Training has none either. The port refuses it for a team under two members
(`jedi_training_refused`), and the `ForceStatus` fixture (94) has one agent,
so its dialog does not list it.

Run `mission-dialog-2026-11-11T14-16-05-599Z-706248` (Linux Chromium
155.0.8059.84, test WASM `efdc3ba7…99b2b`): the full gate passes, 20 cases,
with every icon at (70, 86) and 0 pixels differing. `--only=a,b` now runs
named cases alone, for quick checks.

The icon check compares the port with the GOKRES resource, not with A0. It
proves the right resource is drawn in the right place, as A0 shows for
Sabotage, Abduction, Assassination, Subdue Uprising, Diplomacy, Espionage,
Incite Uprising and Recruitment.

Unit test:
`rescue_recon_research_jedi_and_death_star_kinds_show_their_textstra_icons`
(`mission_dialog.rs`) pins the seven kinds' icon ids and names.
`cargo mutants --in-place` on `kind_text_id`, `kind_icon` and `kind_name`:
10 mutants, 10 caught.

## No port change

Nothing contradicted the port, so no fix was made.

## A0 still needed

For each item, capture the dialog as it opens. Then open the kind list and
take one capture per scroll position.

1. **Research (C015..C017).** Alliance. A character with ship, facility or
   troop design skill on an own system that has a shipyard, a construction
   yard and a training facility. Mission released on that own planet. The
   port lists the research kinds first, then Espionage. Expected icons:
   3104, 3105 and 3106. One capture per tree is enough if the system lacks
   some yards.
2. **Reconnaissance (C011).** An idle recon special force: the Longprobe
   Y-wing Recon Team (Alliance, 3093) or the Imperial Probe Droid (Empire,
   7189). Mission released on an enemy or neutral planet in a sector
   window. The port offers Reconnaissance alone.
3. **Jedi Training (C010).** Empire. Vader and the Emperor as one team at
   Coruscant, with Mission released on Coruscant. The port lists Jedi
   Training (7234) only for a team of two or more, where the pupil's Force
   is below the teacher's. A capture with it listed, or absent, settles the
   rule.
4. **Rescue (C013).** Either side. A free agent's Mission released on the
   side's own captured character, in the holding system's Defenses
   personnel window (3089 / 7185).
5. **Death Star Sabotage (C005).** Alliance. An agent's Mission released on
   the Death Star item in its Fleet window (3137). This needs a late game.
