# Quadrant icons: the facility icon's Reserved and drop

Date: 2026-11-10. Cells CMD-03-C018 (quadrant icon right-click menu) and
CMD-03-C019 (quadrant icon drag). Both stay partial. This stage is code
and unit tests only. It has no A0 capture and no browser gate.

## The original

The menus come from `FUN_0050f2e0` (`ghidra/notes/sector-icon-menus.md`):

- facilities: Destination, Encyclopedia, Status, Reserved and Scrap;
- defenses: Move, Confirmed Move, Encyclopedia, Status and Scrap;
- fleets: Move, Confirmed Move, Planetary Bombardment, Planetary Assault,
  Encyclopedia, Status and Scrap;
- missions: Encyclopedia, Status and Abort.

`FUN_00512700` gives each order its team. The team is the icon's objects
of the order's side.

**Reserved (`0x216`) on the facility icon.** The order class is
`0x006612a8`. It builds one `Event216` for each of the system's managers.

- Enabled: each manager passes `ManuMgr_vt1d8` (`FUN_004f9860`), and
  `FUN_00553770` refuses an empty team.
- Checked: `FUN_0051fd30` was read from the image. The order's `+0x54` is
  `FUN_006158b0`, which returns 0. So the row is checked when any manager
  is Reserved (`ManuMgr_vt1d4`).
- Chosen: `Class_006612a8_vt50` gives every manager the opposite of the
  mark (`Event216_Fire` -> `ManuMgr_vt1fc`).

**Drags** (`FUN_00422ce0`, the `0x29a` handler for window type 1):

- A kind 4 entry (the facility icon) becomes a `0x214` Destination
  against the drop window's `+0x68`. `FUN_00537180` refuses a destination
  that is not a system (`1`/`0x25`). `FUN_00487cc0` has no case for
  `0x214`, so nothing asks and Ctrl does nothing.
- Every other kind becomes Move (`0x201`), or Confirmed Move (`0x202`)
  with Ctrl, against `+0x70`:
  - defenses: regiments, squadrons, characters and special forces, split
    into `0x241` and a `0x204` group;
  - fleets: the system's fleets;
  - missions: kind `0x40` ignores the order kind, so the team is the
    members on visible missions. hyp: this is refused because the members
    are busy (`FUN_00522b30`).

## The port before

These already worked:

- Scrap on every icon;
- the fleet icon's Move, Confirmed Move, Bombardment and Assault, and its
  drop;
- the facility icon's Destination from the menu;
- the mission icon's Abort.

These did not:

- The facility icon's Reserved was always disabled and never checked.
- The facility icon's drop did nothing.
- The defenses icon's Move and Confirmed Move were disabled, and its drop
  did nothing.
- The missions icon's drop did nothing.

## The change

- `ManufacturingState::any_reserved` and `toggle_system_reserved`
  (`rebellion-core/src/manufacturing.rs`).
- `quadrant_icons::facility_icon_has_managers`: the port's stand-in for
  "the system has managers of the side". It is true when the system has a
  manufacturing facility of the side. The icon's Destination gate now uses
  it too.
- `object_menu_rows`: the facility icon's Reserved is enabled by
  `gates.reserved` and checked by `gates.reserved_checked`, the same as a
  band's.
- `main.rs`: `order_gates` fills both gates for the icon.
  `PanelAction::ToggleReserved` now has `area: Option<_>`; with no area it
  toggles every area of the system.
- `targeting::destination_drop_target` and `drop_destination`: the
  facility icon's drop sets every area's Destination to the system that
  `+0x68` names. A point that names an object sets nothing, and the port
  logs `command=0x214 ... status=refused drop=1`.

## Tests

Each test failed with its change reverted, then passed with the change
restored:

- `manufacturing::tests::a_facility_icons_reserved_is_checked_when_any_area_is_reserved`
- `manufacturing::tests::a_facility_icons_reserved_sets_every_area_to_the_opposite_of_its_mark`
- `object_menu::tests::a_bands_build_stop_and_destination_follow_their_rules`.
  Its old assertion "an icon's Reserved is not ported" was replaced, based
  on the trace above.
- `quadrant_icons::tests::a_facility_icons_production_orders_need_a_yard_of_the_players_side`
- `targeting::tests::a_facility_icon_dropped_on_an_object_sets_no_destination`

All tests pass in `cargo test -p rebellion-render --lib` (1412),
`-p rebellion-core --lib manufacturing` and `-p rebellion-app`. The wasm32
check builds, and `provenance-scan check` is ok.

## Not done

- No A0 capture of either feature:
  - the facility icon's Reserved row: grey, white, and checked after a
    band was reserved;
  - a facility icon dropped on another system's planet, then that
    system's Manufacturing window Destination;
  - a defenses or missions icon drop.
- No browser gate case. `sector-quadrants.mjs` or `fleet-window.mjs`
  needs one for the icon's Reserved and for the facility drop.
- The defenses icon's Move, Confirmed Move and drop are not ported. They
  need the mixed team's move: a `0x241` for characters and special forces,
  and a `0x204` group with `FUN_0053d430`'s `1`/`0x28` rule. Confirmed Move
  needs one confirmation for all of them.
- The missions icon's drop does nothing. The trace suggests a refusal
  (hyp), but no A0 capture confirms it, and the refusal's advisor reaction
  (`FUN_00487c90`) is not played.
- port: `+0x68` is read through the Mission release's hit test. Over a
  planet's squadron, an unfinished unit, or a Missions window's mission,
  the port gives the system where the original names the object and
  refuses.
- port: the managers are not modelled. A system "has" them when it has a
  manufacturing facility of the side.

## Mutants

- `rebellion-core` diff (copy mode): 5 mutants, 5 caught.
- In the batch, the render diff: 17 mutants, 14 caught, 1 unviable, 2
  missed. Both were `destination_drop_target`, which no test drew. The
  release test `a_mission_released_on_a_defenses_row_targets_that_object_unless_it_is_on_the_team`
  now also drops a facility icon over the Defenses window: its
  background gives the system, a character's cell and the bare map give
  none. It fails with the function returning `None`.

## Batch b177

23 of 23 interface gates on Linux (`sector-quadrants`
`sector-quadrants-2026-11-10T17-57-55-791Z-1563455`); workspace tests
3507 passed, fixture tests 140. Tactical matrix
`2026-11-10T18-40-14-395Z-1632601`: 148 of 148.
