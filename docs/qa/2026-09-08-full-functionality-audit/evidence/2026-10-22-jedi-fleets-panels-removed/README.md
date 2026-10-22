# The replacement Force (Jedi) and Fleets panels are removed (P08, P09)

Date: 2026-10-22. Decided by the user: remove each replacement panel once
the original windows cover it.

## The Force panel (key J)

| Panel content | Original |
|---|---|
| A character's Force value and ranking | Character Status, "Force Ranking:" (`FUN_004486f0`), ported |
| `ForceAware`, `ForcePotential`, the discoverer | not shown to the player |

## The Fleets panel (key F)

| Panel content | Original |
|---|---|
| The side's fleet roster | Fleet Finder (`fleet-finder.md`), ported |
| A fleet's ships, squadrons and characters | Fleet window (`fleet-window.md`), ported |
| Go to the fleet's system | Fleet Finder's Display (`FUN_00429440`), ported |
| Assign or remove a character at once | none: a character boards a fleet by its Move order (`0x201` -> `0x241` to the fleet), ported |

## Change

- Deleted `panels/jedi.rs` and `panels/fleets.rs`, their state and keys,
  and `PanelAction::AssignCharacterToFleet` / `RemoveCharacterFromFleet`.
- `FocusFleetSystem` stays: the Encyclopedia's system entry uses it.
- The GOKRES miniature tables the Fleet, System, Manufacturing, Missions,
  Defenses, Status and tactical windows draw moved, with their tests, to
  `rebellion-render/src/minis.rs`.
- The remaining replacement panels keep their keys: N, B, D, L.

## Gates

- Workspace 2675 passed, 0 failed. The mini tables' tests moved with
  them.
- Mutants: no new logic; the change removes code and moves tables.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole GID catalog: 152 of 152.
- Replay equivalence passed.
- Release WASM `c5ede528c8585271db08047b1c93c1f6c18aa370b45b32b04c07af6a65e5f7d3`.
