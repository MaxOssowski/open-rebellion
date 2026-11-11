# Quadrant icons: the defenses icon's Move, Confirmed Move and drop

Date: 2026-11-11. Cells CMD-03-C018 (quadrant icon right-click menu) and
CMD-03-C019 (quadrant icon drag). Both stay partial. This stage has code
and unit tests only: no A0 capture, no browser gate and no mutants.

## The original

The trace is in `ghidra/notes/sector-icon-menus.md`, "The defenses icon's
Move (read 2026-11-11)".

**The team.** `FUN_00512700` kind 8 for `0x201`/`0x202` was read in full.
The team is the system's regiments (`FUN_00504c40`), squadrons
(`FUN_005039d0`), and characters and special forces (`FUN_00536da0`).
Each list keeps only the order's side (`FUN_00553350`). The walk does not
skip members on missions. Defense facilities are in Scrap's team only.

**The split.** `FUN_0053c4b0` was read in full:
- An empty team is refused (`1`/`0x16`).
- `FUN_0053c810` and `FUN_0053cbc0` put the characters and special forces
  into one `0x241`, which is one Move mission.
- `FUN_0053ca50` puts the rest into one `0x204` group, only when any are
  left.

**Enabled** (`FUN_0053c100`). Each sub-order's `+0x18` must pass:
- The `0x241`'s runs command `0x250`'s `+0x18` (`FUN_0054d1a0` ->
  `FUN_005429e0` -> `FUN_0054c110`, member slot `+0x1c8`). A member must be
  the side's, not travelling and not on a mission. For `0x241`, an
  unrecruited member or a prisoner is not refused. `FUN_0054bf00` needs
  one place.
- The `0x204`'s checks each member's move command (`FUN_00578c00`).

**Validated** (`FUN_0053c1a0`). Any refusal goes to `FUN_00487c90`, and
nothing moves.
- `0x241`: `FUN_0051ff30`, then `FUN_004f4b60` (`1`/`0x28`), then
  `Event250_vt1c` (`FUN_0054d280`, read from vtable `0x00661e28`).
  `FUN_00542b60` -> `FUN_0054c200` -> `FUN_00522b30` refuses a member that
  has a mission.
- `0x204`: `FUN_0053d430`'s `1`/`0x28` rule, then each member's command.

**Asked** (`FUN_00487cc0`, disassembled at `0x00487cc0..0x00487e13`). The
result starts at 0 (`[esp+0x10]`).
- `0x202` always asks.
- `0x201` asks only when the first team id's container is a blockaded
  system of its side. The icon's entry is the system, and its container
  is a sector, so an icon's Move never asks.
- `FUN_0049a350` reads the same member, so there is no evacuation
  warning.
- The window lists the `0x241`'s members first (`FUN_0053c2e0` ->
  `FUN_004f4850`, at the mission's speed `FUN_00548370`), then the
  `0x204`'s (`FUN_0053d200`).

**The drop** (`FUN_00422ce0`, `0x29a`, window type 1). A kind 8 entry
keeps its kind and goes into `0x201`, or `0x202` with Ctrl, against the
drop window's `+0x70`.

**The missions icon's drop.** Kind `0x40` gives the members on visible
missions, whatever the order kind. Each of them has a mission, so the
`0x241` fails at validation (`FUN_0054c110` and `FUN_00522b30`). A drop
on a window that names a target always reaches `FUN_00487c90`. This is
spot-checked from the decompile and not run in A0.

**Manual.** No passage names the defenses or missions icon's Move or drag.
The searches were "Defense icon", "Defenses icon" and "drag the". Fig.
3.50 (p. 109) covers only the mission icon's menu.

## The port before

- The defenses icon's Move and Confirmed Move were always disabled.
- The defenses icon's drop did nothing.
- The missions icon's drop did nothing.

## The change

- `rebellion-core/src/missions.rs`:
  - `MissionState::move_order_enabled` is the `0x241`'s `+0x18`.
  - `team_move_days` gives each member's days at the mission's speed.
    `member_move_days` and the mission's own speed now share `team_speed`.
- `rebellion-render/src/quadrant_icons.rs`: `DefensesTeam` and
  `defenses_move_team` build `FUN_00512700`'s kind 8 team. A member on its
  way to the system counts, as `FUN_0045ce80`'s walk does.
- `rebellion-render/src/object_menu.rs`: `OrderGates::defenses_move`
  enables the icon's Move and Confirmed Move rows.
- `rebellion-render/src/targeting.rs`: `TargetOrder::DefensesMove`.
- `rebellion-render/src/panels/mod.rs`: `PanelAction::MoveMembers` is one
  Move mission with several members.
- `rebellion-app/src/main.rs`:
  - `defenses_move_enabled` sets the gate.
  - The menu's rows start targeting.
  - The release, and the icon's drop (Ctrl for a Confirmed Move), call
    `issue_defenses_move`. It checks every member, and one refusal
    refuses the whole order with a "Move refused" line standing in for
    `FUN_00487c90`.
  - A Move asks nothing. It sends one `MoveMembers` and a
    `MoveRegiment`/`MoveSquadron` for each unit.
  - A Confirmed Move opens one window with a line per member. Its
    checkmark sends one `MoveMembers` when the window holds more than one
    character or special force.
  - The missions icon's drop on a target logs `status=refused` and posts
    the same "Move refused" line.

### Follow-up: two mismatches the trace found

- **The fleet icon's Move never asks.** `FUN_00487cc0` reads the order's
  first team id. A fleet icon's is the system (`FUN_004f5b10`), whose
  container is a sector, so its `0x201` never asks, even out of a
  blockade of its side. Before, the port asked when any fleet's own rule
  (`fleet_move_confirms`) asked. Now `FleetMoveContext::icon` and
  `TargetOrder::FleetMove { icon }` mark the icon's order, and only its
  Confirmed Move asks. One fleet's own Move still asks out of such a
  blockade.
- **A busy character's menu Move is grey.** The `0x241`'s `+0x18`
  (`FUN_0051fe20` -> command `0x250` -> `FUN_0054d1a0` -> `FUN_005429e0`
  -> `FUN_0054c110`) refuses a member on a mission or travelling. Before,
  `OrderGates::character_move` checked only that the character was alive
  and of the side. It now uses `MissionState::move_order_enabled`.
  `move-order.md` had read this as "enabled for any character that
  resolves", and is corrected.
- **A busy special force's menu Move is grey.** The same `0x241` check
  asks the unit's slot `+0x1c8` (`FUN_00533ce0`, over `FUN_004f9860`).
  Before, `OrderGates::special_force_move` checked only the side. It now
  uses `MissionState::move_order_enabled`. No fixture unit is on a
  mission, so `personnel-orders.mjs` "special-force-move-refused" keeps
  its enabled Move.
- Browser gates: no `tools/interface-parity/*.mjs` case relies on the old
  behaviour (listed, not run).
  - `fleet-window.mjs` orders the fleet icon's Move outside a blockade.
  - `fleet-move.mjs` "blockade-drag" drags one fleet, not the icon.
  - `personnel-orders.mjs` moves a free character.

## Tests

Each test failed with its change reverted, then passed with the change
restored.
- Core: `move_order_enabled` stubbed to `mission_order_enabled`, and
  `team_move_days` at each member's own speed.
- Render: the two `defenses && gates.defenses_move` terms removed, and
  `defenses_move_team` returning an empty team.
- App: `issue_defenses_move` returning at once, and
  `defenses_move_enabled` returning false.

The tests:
- `missions::tests::a_team_moves_at_the_speed_its_whole_mission_takes`
- `missions::tests::a_moves_member_order_is_enabled_for_free_members_of_the_side_at_one_place`
- `object_menu::tests::a_defenses_icons_move_rows_follow_its_mixed_teams_move_rule`
- `quadrant_icons::tests::a_defenses_icons_move_team_is_its_sides_regiments_squadrons_and_personnel`
- `fleet_move_tests::a_defenses_icons_move_sends_one_member_mission_and_the_group_at_once`
- `fleet_move_tests::a_defenses_icons_confirmed_move_asks_once_with_a_line_per_member`
- `fleet_move_tests::one_busy_member_keeps_the_whole_defenses_team_home`
- `fleet_move_tests::a_defenses_icons_move_is_enabled_for_a_free_team_of_the_side`
- `fleet_move_tests::a_fleet_icons_move_never_asks_even_out_of_a_blockaded_system`.
  Revert: `asks` from every fleet's own rule again.
- `fleet_move_tests::a_characters_menu_move_is_disabled_while_it_is_on_a_mission_or_travelling`.
  Revert: the old alive-and-side gate.
- `fleet_move_tests::a_special_forces_menu_move_is_disabled_while_it_is_on_a_mission_or_travelling`.
  Revert: the old side-only gate.

Results:
- `cargo test -p rebellion-core -p rebellion-render -p rebellion-app`
  passes: core 1722, render 1414, app 147.
- `cargo check --workspace` passes, and so does `-p rebellion-app
  --features interface-test-fixtures --tests`.
- `provenance-scan check` is ok.
- rustfmt is clean on the changed hunks.

## Not done

- No A0 capture of any of these:
  - the defenses icon's menu with Move and Confirmed Move white, and grey
    while a character of the side there is on a mission;
  - a defenses icon dropped on another held system, and the Confirmed
    Move window's lines;
  - a missions icon drop and its advisor reaction.
- No browser gate case (`sector-quadrants.mjs`), no `cargo mutants` and no
  wasm32 check. These are for the batch.
- The missions icon's drop has no unit test. Its code is inline in the
  frame loop.
- port: `FUN_0053d430` is not run as one group check. Each regiment, and
  each squadron class with all its squadrons, runs its own move check.
  The room of a fleet target is checked per kind, not for the whole
  group.
- port: `FUN_00487c90`'s advisor reaction is a "Move refused" message
  line.
- port: the line order within the members follows the port's walk
  (characters, then special forces). The order of `FUN_005425e0` within
  the `0x241` is untraced.
- Build note: the shared `target/agents` directory served another
  worktree's stale `rebellion-core` and `rebellion-render` (path packages
  share metadata hashes). These checks used `target/agents-defenses`.

## Mutants and batch b182

- `rebellion-core` diff (copy mode): 27 mutants, 27 caught.
- In the batch, the render diff missed two mutants in
  `DefensesTeam::is_empty` (`&&` to `||`). The test
  `a_defenses_team_with_any_one_kind_of_member_is_not_empty`, added
  after the run, catches both: a team of regiments alone would read
  empty.
- 23 of 23 interface gates on Linux (`sector-quadrants`
  `sector-quadrants-2026-11-11T00-44-11-156Z-3038434`); workspace tests
  3538 passed, fixture tests 148. Tactical matrix
  `2026-11-11T01-25-09-246Z-3124742`: 148 of 148.
