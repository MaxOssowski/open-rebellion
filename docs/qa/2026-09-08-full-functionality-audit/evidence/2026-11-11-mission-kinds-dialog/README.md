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

## Troop Training Research against A0 (C017)

The A0 run used the Windows 10 VM with a new Alliance game on day 50. The
captures are `2026-11-11-mission-kinds/10-tallon-on-averam-dialog` and
`11-tallon-averam-list-open`, with notes in `NOTES.txt` and hashes in
`SHA256SUMS`.

- Adar Tallon alone. His Mission was released on his own Averam. Averam has
  one Training Facility, no shipyard and no construction yard.
- Create Mission opens at (168, 38), the port's Alliance place. It shows
  Troop Training Research.
- The kind list holds two items: the thumb covers half the track, and
  Troop Training Research is first. The second item could not be shown.
- The kind's icon, GOKRES 3106 (TEXTSTRA 11298 & 0xfff), sits at the
  window's (70, 86). Rows 0..63 match the resource at 0 of 8,320 pixels.

The port agrees. `FUN_0056c510` lets a research kind list only with an
own-side yard of the tree's area (`research_site`, the training facility's
MANFACSD family 0x29) and a member with the tree's skill. The port needed
no fix.

New fixture `Scenario::TroopResearchTargeting` (102, code 103; renumbered from 100 at the merge) mirrors
`ResearchTargeting`, with these differences:

- the agent has troop training 40 and no ship or facility skill;
- the target's only yard is a training facility (`0x2900_0001`).

Full support ends Diplomacy, and the empty pool ends Recruitment, so the
box lists Troop Training Research above Espionage. That is two kinds, as
in A0. A0's second item is not seen.

The unit test is `the_research_scenarios_list_their_own_tree_alone_of_the_trees`
(formerly the ship-only test). It now also checks the troop scenario for
both sides.

Side by side with A0 (`alliance-troop-research/opened-screen.png` of the
run below), the dialog's box, icon and chrome match. The only difference
is the target: the fixture's F'tral against A0's Averam. The kind's name
strip differs in 823 pixels of glyph ink. That is the accepted text-width
deviation.

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
| `troop-research` | TroopResearchTargeting (103), a troop trainer at an own training facility | TroopTrainingResearch, Espionage | 3106 (A0 exact) / 7202 |

The Empire has no Death Star Sabotage record. The fixture's Alliance Death
Star exists only for this case, so the Empire's case is not run.

Facility Design Research has no browser case. Neither research fixture
has a construction yard or a facility designer. Jedi
Training has none either. The port refuses it for a team under two members
(`jedi_training_refused`), and the `ForceStatus` fixture (94) has one agent,
so its dialog does not list it.

Run `mission-dialog-2026-11-11T14-29-50-272Z-777464` (Linux Chromium
155.0.8059.84, test WASM `6cf03478…4bda5d`): the full gate passes, 21 cases,
with every icon at (70, 86) and 0 pixels differing. The earlier run
`mission-dialog-2026-11-11T14-16-05-599Z-706248`, without the troop case,
also passed. `--only=a,b` now runs
named cases alone, for quick checks.

The icon check compares the port with the GOKRES resource, not with A0. It
proves the right resource is drawn in the right place, as A0 shows for
Sabotage, Abduction, Assassination, Subdue Uprising, Diplomacy, Espionage,
Incite Uprising and Recruitment.

Unit test:
`rescue_recon_research_jedi_and_death_star_kinds_show_their_textstra_icons`
(`mission_dialog.rs`) pins the seven kinds' icon ids and names.
`cargo mutants --in-place` on `kind_text_id`, `kind_icon` and `kind_name`:
10 mutants, 10 caught. On `interface_test_fixture.rs`
(`Scenario::decode|reports_object_menu`, which matched 144 mutants): 141
were caught and 1 was unviable. The 2 missed are fields of
`FleetLoadWatch::next`, which this change does not touch.

## No port change

Nothing contradicted the port, so no fix was made.

## A0 still needed

For each item, capture the dialog as it opens. Then open the kind list and
take one capture per scroll position.

1. **Ship and Facility Design Research (C016, C015).** As in Tallon's
   capture: a ship designer on an own system with a shipyard (3104), and a
   facility designer on one with a construction yard (3105). Mission
   released on that planet.
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
