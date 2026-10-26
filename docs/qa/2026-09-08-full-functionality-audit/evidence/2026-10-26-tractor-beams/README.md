# Ships use their tractor beams (P24)

Date: 2026-10-26. This follows `../2026-10-26-weapon-targets/`. Until now
the port had the tractor field itself:

- a target's list of sources;
- the cloud;
- the engine pull in `original_tactical_mobility`.

But no ship ever turned its beam on in a battle. Only test fixtures did.
In the original, every weapon turn of a ship with a tractor beam asks for
its force's target.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository, checked against the `objdump` disassembly).

## The original

### The request, `FUN_005b2140`

`FUN_005b3a40` calls it at `0x5b3b72` with the force's target `T` (or 0),
after the direct shots and before the search for fighter groups. An
escort, or a unit whose arcs cannot hurt `T`'s class, has returned before
this point.

It returns at once in three cases:

- the unit is remote (`+0x124`);
- a request is already on its way (`+0x3ac`);
- the unit has no tractor beam (`+0x3b8` is 0, as for every fighter
  group).

Otherwise, with `+0x380` the ship it holds:

| Case | Request (slot `+0x28`) |
|---|---|
| no `T`, holding none | none |
| no `T`, holding a ship | `0x29`: let it go |
| holding `T`, `T` in the fight (`FUN_005ad870`), power (`FUN_005b1790` not 0) and within range (slot `+0x5c`, `+0x39c`) | none: keep it |
| holding `T`, otherwise | `0x29` |
| holding another ship | `0x29` for that one |
| holding none, with power and `T` within range | `0x28`: take `T` |
| holding none, otherwise | none |

The distance is three-dimensional. A posted request sets `+0x3ac`.

### The events

- `0x28` (`FUN_005b2630`): with both units in the fight, the source's
  slot `+0x40` (`FUN_005b23e0`):
  - sets `+0x380`;
  - adds the source to the target's list `+0x360` (`FUN_005b2570`), which
    starts the cloud with the first source;
  - clears `+0x3ac`.

  Otherwise nothing happens, and `+0x3ac` stays set.
- `0x29` (`FUN_005b26a0`): slot `+0x48` (`FUN_005b2440`):
  - clears `+0x380` and `+0x3ac`;
  - removes the source from the target's list (`FUN_005b25d0`).

### Letting go without a request

- A tractor hit (`FUN_005b1970`, case 2) that leaves the beam no power,
  while it holds a ship, posts `0x29` and sets `+0x3ac`.
- A ship's destruction (slot `+0x1c`, `FUN_005b2d20`) runs slot `+0x68`
  (`0x005b0ad0`). It posts `0x29` for the held ship. It also posts `0x2b`
  for its gravity wells, which are not ported.

### The pull

A held ship's speed (`FUN_005b17f0`) loses each source's power
(`FUN_005b1790`): its `+0x3b8` scaled by its hull (`FUN_005b1770`), less a
quarter of it per tractor hit. The port already had this in
`original_tactical_mobility`.

## The port

| Original | Port |
|---|---|
| `+0x39c` | `TacticalShip::tractor_beam_range`, from the class's DAT `tractor_beam_range` |
| `+0x3ac` | `TacticalShip::tractor_request_pending` |
| `FUN_005b2140` | `BattleSession::request_tractor`, called from `fire_unit_weapons` for ships |
| `+0x380` | `tractor_target_of`, read from the field list |
| event `0x28` | `TacticalTurnEvent::TractorOn`, posted at the turn's stamp |
| event `0x29` | `TacticalTurnEvent::TractorOff`, posted at the turn's stamp |
| letting go | `hit_subsystem` on a tractor hit without power, and `release_tractor_of_destroyed` at the three places where a ship is destroyed: a hit, the Death Star, a jump without a hyperdrive or a left-behind ship |

`hit_subsystem` no longer drops the beam at once. The beam lets go
through `0x29`.

`port:` choices:

- A fighter group as the target is not held yet. A ship holding another
  ship lets that one go, as in the table. The next stage adds groups.
- A source gone by its jump pulls no more, since
  `original_effective_tractor_power` needs a live ship. In the original
  it stays in the target's list.

## Tests

New, in `abstract_battle.rs`:

- `a_ship_takes_its_target_into_its_tractor_beam_through_event_0x28`:
  - the request and the wait;
  - the hold;
  - the pull of 10 on the target's engines;
  - no new request while it holds the target in range;
- `a_tractor_beam_needs_range_power_and_both_ships_in_the_fight`:
  - range 20 against targets at 20 and 20.5;
  - no beam, or four tractor hits;
  - a target gone before the event;
- `a_tractor_beam_lets_go_through_event_0x29`:
  - a held target beyond range;
  - no target;
  - another target;
  - a group target;
- `a_tractor_beam_without_power_or_ship_lets_go`:
  - the fourth tractor hit, against the first;
  - the source's destruction;
- `a_turn_ended_by_the_targets_class_keeps_the_tractor_hold`: an ion ship
  ordered against a group returns before `FUN_005b2140` (`0x5b3b05`), so
  it keeps the ship it holds. This also covers the two mutants of
  `unit_has_weapons_against` that were equivalent in the last stage.

The tests use the new fields and events, so they do not compile without
the change.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2890 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T18-43-30-229Z-2153387`; Arch Chromium 155, not the pinned build) |
| Replay equivalence | passed |
| `cargo mutants` | 30 tested: 30 caught |
| Release WASM SHA-256 | `e5321f85666fe252269aa566a687fe87c5728e6d3e954df7fb7ff61499c5fae9` |
