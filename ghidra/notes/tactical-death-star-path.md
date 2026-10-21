# Tactical Death Star path

This note records the tactical Death Star state recovered from the owned
English `REBEXE.EXE` with SHA-256
`b3fe3997cab9a6e96403d638875dcba25484e4d8601751afec748471ac0ed6ab`.
The object is separate from the 29-entry capital-ship mesh registry.

## Recovered functions and resources

| Function | Role |
|---|---|
| `FUN_005ab0e0`, `FUN_005ab650` | Select manager sprite 5010 or 5020 from field `+0x74` |
| `FUN_005ba420` | Death Star operational predicate |
| `FUN_005ba5e0` | Commits the Death Star action and updates attached objects |
| `FUN_005ba7f0` | Advances the laser charge |
| `FUN_005b8b10`, `FUN_005b8bd0` | Route weapon/damage events to the separate Death Star object |
| `FUN_005afe40` | Assigns the Attack Death Star executor and target |
| `FUN_005c0b60` | Adds the operational Death Star to the target list |
| `FUN_005caf20`, `FUN_005caf50` | Decode `TACTICALRESULT_UPDATE` and forward its state to the live result object |
| `FUN_005cfec0` | Arms the 120-second trench-run presentation and dispatches result states 6 and 7 |
| `FUN_005d04e0` | Selects participants, applies maneuver damage, rolls the commander rating, and chooses success or failure |
| `FUN_005d03f0`, `FUN_005d0430` | Emit the nine thresholded chatter slots, advance the timer, and enter state 6 or 7 |
| `FUN_005ad7e0` | Returns the tactical commander rating clamped to 1 through 9, with 1 as the unassigned fallback |
| `FUN_0061a310`, `FUN_005a8a70` | MSVC-compatible tactical random stream and inclusive range helper |
| `FUN_005c0fb0`, `FUN_0059cc60` | State 6 wrapper; requests original movie ID `0x71` |
| `FUN_005c0ff0`, `FUN_0059cc80` | State 7 wrapper; requests original movie ID `0x70` |
| `FUN_00421ba0` | Maps `0x71` to `MDATA\\MDATA.201` and `0x70` to `MDATA\\MDATA.202` |
| `FUN_005c4ed0` | Builds the laser gauge and 1021/1022/1023 control |
| `FUN_005df110` | Builds Attack Death Star resources 1176/1177/1178 |
| `FUN_005d4d10` | Builds the separate 5030 authored sequence surface |
| `FUN_005d6700` | Constructs the tactical 3D window with the standard arrow cursor (`IDC_ARROW`) |

The exact manager fields are:

| Offset | Meaning established by use |
|---|---|
| `+0x68` | Live value; zero makes the object non-operational |
| `+0x6c` | Laser charge, capped at 100 |
| `+0x74` | Destroyed/state flag; selects resource 5020 instead of 5010 |
| `+0x78` | Committed-action flag; value 1 makes the object non-operational |

`FUN_005ba420` is true only when `+0x74 == 0`, `+0x68 != 0`, and
`+0x78 != 1`.

## Charge and controls

`FUN_005ba7f0` advances the charge only while the live value is positive and
the destroyed flag is clear:

```text
charge += elapsed_milliseconds * (1 / 3) * 0.001
charge = min(charge, 100)
```

This is a 300-second zero-to-full charge. The laser control uses 1021 at rest,
1022 when fired/pressed, and 1023 while disabled or loading. The mission panel
uses 1176 at rest, 1177 pressed, and 1178 disabled. It is initially disabled;
the reconstructed mission control becomes active only for a selected fighter
group with a hostile operational Death Star.

## Implemented boundary

The tactical session now creates one separate Death Star from
`Fleet::has_death_star`, transports its faction and battle side, renders the
5010/5020 resource at its retained position, applies the exact operational and
charge predicates, routes the selected fighter-group Attack Death Star order
to it, persists destruction to the strategic fleet, and keeps a Death-Star-only
fleet alive. The production renderer also decodes resource 5030 as the sparse
440 by 438 tactical star surface, composes the owner-only 1024 gauge and
1021/1022/1023 control, arms a right-click capital target, commits the manager
action, resets charge, renders a delayed visible beam, and resolves destruction.
The deterministic browser journey proves the ready, held, armed, committed,
loading, and resolved states for both ownership cases and viewports.

## Trench-run result routing

The executable distinguishes the films from campaign victory and defeat.
`FUN_005caf20` decodes a `TACTICALRESULT_UPDATE` payload as an object ID plus
state, `FUN_005caf50` forwards it to the active tactical result object, and
`FUN_005cfec0` dispatches the result:

| Result state | Wrapper | Internal movie ID | File | Meaning |
|---:|---|---:|---|---|
| 6 | `FUN_005c0fb0` / `FUN_0059cc60` | `0x71` | `MDATA.201` | Successful trench run; Death Star destroyed |
| 7 | `FUN_005c0ff0` / `FUN_0059cc80` | `0x70` | `MDATA.202` | Failed trench run; attacking fighter participants destroyed |

`FUN_005cfec0` initializes a 120,000 ms timer. `FUN_005d04e0` retains at most
1,001 active Alliance participants, draws maneuver failure from 1 through 10,
and applies either 100,000 damage at maneuverability 0 or 1, or an inclusive
1-through-maneuverability draw multiplied by `_DAT_0066d078 = 8`. Each survivor
then compares an inclusive 1-through-100 draw with `FUN_005ad7e0`'s commander
rating. The rating is clamped to 1 through 9; the source returns 1 when the
tactical commander slot is unassigned.

The result object selects one of three exact nine-slot chatter sequences. The
slots occur at 10% intervals over the two-minute timer; `0x13d` is the silent
sentinel. State 6 destroys the Death Star and removes the first bounded half of
the participants: none for groups of zero or one, one for groups of two or
three, and `floor(n / 2)` thereafter. State 7 destroys every participant. The
result then routes to MDATA 201 or 202 and returns to the paused tactical view.

Open Rebellion now follows that producer with one source-compatible tactical
RNG stream, the exact timer, ordered chatter, success rating, maneuver damage,
casualty rules, and movie route. Campaign entry seeds the tactical stream once.
The wider strategic model does not yet persist the executable's tactical
commander assignment slot, so an ordinary campaign battle uses the source's
unassigned rating of 1; deterministic fixtures set the rating explicitly.

Exact retained-mode beam raster comparison, whole-process RNG continuity,
strategic commander-slot binding, audible native playback, and lossless A0
comparison remain open. The current beam shape is bounded A1 implementation
evidence. A custom target cursor is not an open requirement:
`FUN_005d6700` calls `LoadCursorA` with `IDC_ARROW` for the native tactical 3D
window.

## The Death Star shield (read 2026-10-20)

- The Death Star shield is DEFFACSD record 4, family `0x25` (TEXTSTRA
  8832, Empire only; `FUN_0051c0c0` returns `0x25`). The manual (p. 124
  and the facility list) calls it the only protection of a Death Star
  from the Alliance's ships and fighters, "a foolproof defense against a
  Death Star being attacked by fighters".
- `FUN_0040bcd0`, at the tactical setup beside `FUN_0040a700`, counts the
  battle system's usable family `0x25` facilities (`FUN_0051c270(system,
  1)`, no side filter) and calls `FUN_00595ef0(..., not completed,
  shielded, strength)`. `FUN_00595ef0` swaps the two flags into
  `FUN_005af4e0`, which stores `+0x70` = shielded and `+0x74` = the Death
  Star object's completed bit (`+0x50` bit 2) clear. So `+0x74`, which
  selects resource 5020, is "not completed" at creation; this note's table
  called it a destroyed flag (the port also sets it when the trench run
  destroys the Death Star).
- `FUN_005ba450` is `FUN_005ba420` (operational) plus `+0x70 == 0`. The
  Attack Death Star checks call it: `FUN_005ad930` (the mission control),
  `FUN_005afcc0` and `FUN_005d14d0` (the tactical AI).
- `FUN_005ad930` also requires (read 2026-10-20, ported):
  - no trench run under way: `FUN_005d0ae0` returns `DAT_006bc978`, set
    by the result object's constructor `FUN_005d0820` and cleared by
    `FUN_005d0950`;
  - fewer Imperial than Alliance fighter groups on the field: over the
    manager's object list `+0x988`, class 1 (slot `+0x20`, a fighter
    group, `tactical-attack-targeting.md`), side `+0x168` (0 is the
    Alliance: `FUN_005a2920` queues voice event `0x84` for it, `0x102`
    otherwise);
  - none of the player's groups (`*(manager + 8) + 0x70`) with order or
    pending order 6 (`+0x68`, `+0x64`).
  The manual (p. 151) agrees: "there are more Alliance than Imperial
  fighters currently launched. Only one fighter group at a time may
  attempt the Death Star mission." `FUN_005afcc0` repeats the count.
  port: the run counts as under way from state 5's timer to its result.
- Outside the tactical code the family `0x25` checks (`FUN_0051c0d0`,
  `FUN_0051c270`) are called only by the interface (system and sector
  displays, `FUN_00446250`, `FUN_004a7f50`, `FUN_004a9ce0`) and the AI
  (`0x4ae732`..`0x4ec4cd`). No strategic battle code reads a shield.
- Ported 2026-10-20: `TacticalDeathStar::shielded` (`+0x70`) and
  `attackable()` (`FUN_005ba450`) gate the fighters' Attack Death Star.
  The port's strategic `DeathStarState::shield_generator_active`, which
  made a Death Star of family `0x34` immune to hull damage in an
  auto-resolved battle and had no source, is removed with the whole
  strategic Death Star state (save v96).

## The browser fixture (2026-10-20)

- `tactical_test_fixture.rs` looked for SYSTEMSD index 100 or 101, which
  the default galaxy (100 systems, indices 120..289) does not load. Every
  tactical battle case stopped at setup. It now takes the first and second
  empty, intact systems in SYSTEMSD order.
- A Death Star battle's Imperial fleet brings no fighter group, as
  `FUN_005ad930` requires fewer Imperial than Alliance groups for Attack
  Death Star. `run.mjs` expects that composition: one Imperial active
  object, three participants, and no fighter scene once a failed run has
  destroyed every Alliance group.
- The tactical 3D corpus must be staged locally
  (`go run ./tools/stage-ui-assets --tactical-3d-only --source ...`, then
  `--tactical-3d-convert`), or the Death Star resource 5010 is missing.


## The Death Star's identity (read 2026-10-21, F-025)

- Every CAPSHPSD record carries an object family at `+0x10`
  (`tools/dat-dumper` `family_id`). It is `0x14` for every record except
  record `0x88` (136, TEXTSTRA 10120 "Death Star"), which has `0x18`.
- The original finds a fleet's Death Stars by that family:
  - `FUN_005003d0` walks a fleet's objects of families `0x18..0x1b`;
    `FUN_005001b0` is the capital ships' walk.
  - `FUN_0040b0e0` hands each Death Star to `FUN_0040bcd0`, which builds
    the battle's Death Star object.
  - The bombardment's `0x223` needs one (`bombardment-order.md`), and the
    smuggling count `FUN_00509140` counts them (`economy-cadence.md`).
  - The capital ship classes are vtables `0x0065d650` (`0x14`) and
    `0x0065d930` (`0x18`) (`build-delivery.md`).
- Family `0x34` is the Emperor (MJCHARSD `0x34000280`; `FUN_00560d50`
  routes it to the seat of power, `FUN_005617b0`). The port's
  `CapitalShipClass::is_death_star` also accepted family `0x34`, which no
  shipped class has. That clause is gone (F-025). The port keeps a class's
  record id, not its family, so record 136 stands for family `0x18`.
  Carrying the family itself would need a save change.
