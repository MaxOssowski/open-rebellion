# Tractor beams hold fighter groups too (P24)

Date: 2026-10-26. This follows `../2026-10-26-tractor-beams/`, where a ship
held only capital ships, and a fighter group as its target was left aside.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository).

## The original

- `FUN_005b2140` looks its target up with `FUN_005a6340`, which finds any
  unit. So a ship whose force is on Attack Fighters asks for its target
  group like any other target.
- Both unit classes share the list slots:
  - the event `0x28` handler (`FUN_005b2630`) calls the source's slot
    `+0x40`;
  - `FUN_005b2570` adds the source to the target's `+0x360`;
  - the fighter class (`0x66c028`) has the same slots `+0x40` / `+0x48`
    (`0x5b23e0`, `0x5b2440`) as capital ships.
- A group's speed (slot `+0x90`, `0x5ba190`) subtracts its sources' power
  (`FUN_005b1790`) from the commander term plus its sublight rating. The
  result is at least 0.
- The cloud is the unit's own (`FUN_005c1360` with the unit's id).

While reading this, two speed terms were settled. Both stay 0 for a group:

- the hits `+0x36c`: subsystem events `0x1e`..`0x22` are posted only when
  slot `+0x20` is 0 (`FUN_005b5100`, `FUN_005b5750`). For a group that
  slot is `0x40f340`, which returns 1;
- `+0x3b4`: it is raised only through event `0x16`, from an ion hit on a
  capital ship (`FUN_005b5b60`). A group's slot `+0x9c` (`0x5ae1e0`) is
  empty, so it never decays either.

## The port

- `TacticalFieldEffect::target` is now a `TacticalUnitRef`.
- `TractorOn` and `TractorOff` carry one, and `tractor_target_of` returns
  one. `request_tractor` takes a group as its target, by the same table.
- `original_fighter_speed` subtracts the pulls of the ships that hold the
  group. Its comment now says why the hits and `+0x3b4` stay 0.
- The cloud is drawn on the group's position, with the group's render id.
- The fixture's field report names a group target with
  `target_is_fighter`, written only when true. The first matrix run
  wrote it always and failed 8 cases (`production-projectile-field-presentation`
  and `production-subsystem-field-command-presentation`, both factions and
  sizes), whose probes compare the report whole.

## Tests

- New: `a_ship_holds_a_fighter_group_in_its_tractor_beam`:
  - the request;
  - no hold of a group in its jump;
  - the hold and the pull of 10 on the group's speed;
  - the release beyond range.
- `a_tractor_beam_lets_go_through_event_0x29` keeps its group case: the
  ship holding another ship lets that one go first.
- The tests that build fields by hand now name the target as a ship.
  Their assertions are unchanged.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2891 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 140 of 148 in the first run (`2026-10-26T19-06-08-167Z-2297533`); the 8 failures above passed after the fix (runs `2026-10-26T19-22-52-868Z-2421447` and `2026-10-26T19-22-39-601Z-2420769`, 4 of 4 each). The fix is in the fixture report only, which the release build does not contain. Arch Chromium 155, not the pinned build |
| Replay equivalence | passed |
| `cargo mutants` | 26 tested: 23 caught, 3 unviable |
| Release WASM SHA-256 | `41dbe1cbace8621e61292f2051a95529c0a6d2514d68100231d6f6cca14a2997` |
