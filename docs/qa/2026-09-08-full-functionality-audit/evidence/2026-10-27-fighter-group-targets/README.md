# Fighter groups take right-click targets, and squadrons are targets (P24)

Date: 2026-10-27. This follows `../2026-10-27-battle-keys/`. A check of
the manual against the port found three gaps:
- a selected fighter group took no target from a right click;
- no right click could target an enemy squadron;
- Space did nothing for a fighter group.

The manual: "Each ship, task force, or fighter group can target specific
enemy ships. To assign a target, first select the ship, task force, or
fighter group. Next, right-click on the enemy ship to target." Space
attacks "the nearest enemy ship of the same type (fighter or capital
ship)" (chapter 4).

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository).

## The original

### The fighter group's state

A fighter-group button posts command `0xb + n` (`FUN_0059b620`). The
command switch (`FUN_0059fd40`) builds the fighter-group state `0x66bbf8`
(`FUN_005a4640`). That state's slot `+0xc` (`0x005a4bd0`) takes a
right-clicked unit of either type, a capital ship or a squadron:

- **An enemy unit, without Ctrl:**
  - the group's attack voice, `0x44` (Alliance) or `0xbe` (Empire) plus
    the group's slot (`FUN_005a02c0`), while the force's `+0x80` slot
    `+0xc` gives 0;
  - order 0 (the force's slot `+100`);
  - the unit as the target (`FUN_005a8fc0`).
- **An enemy unit, with Ctrl:**
  - a unit the group already targets (`FUN_005ca310`) is dropped
    (`FUN_005a8e80`);
  - another one is added, with the same voice.
- **A unit of our own:** Escort (slot `+0x20` with order 1), already
  ported.

The capital states' slot `+0xc` (`0x005a1af0`, `0x005a24d0`) also takes
any unit, so a capital selection can target an enemy squadron.

### Space for a fighter group

The fighter-group state's slot `+0x1c` (`0x005a50d0`) is the task force's
Space (`0x005a4180`) over the group's squadrons:
- their centroid;
- the nearest enemy capital ship in the fight, else the nearest enemy
  squadron;
- slot `+0xc` with 0.

## The port

`crates/rebellion-render/src/tactical_view.rs`:

- The right-click hit test now also takes enemy squadrons from the
  rendered fighter projections, after the capital ships. Among
  overlapping squadrons the lowest index wins.
- With a fighter group selected, `assign_selected_fighter_group_target`
  holds the rule above.
- With capital ships selected, an enemy squadron becomes their target
  through `assign_selected_focus_target`.
- `attack_nearest_enemy` searches from the fighter group's centroid when a
  group is selected.

Not exact:
- port: a squadron holds one target (`attack_target`), so a Ctrl click on
  a new unit replaces the target instead of queueing a second one.
- hyp: the attack voice plays on every click without Ctrl. The original
  also asks the force's `+0x80` slot `+0xc`, which is not traced.

Still open: right-click navigation points for a fighter group (`0x66bbf8`
slot `+0x10`, `0x005a4e50`). The port's squadrons have no route yet.

## Tests

- `a_fighter_group_takes_an_enemy_unit_as_its_target`
- `a_ctrl_right_click_on_a_held_target_drops_it`
- `a_fighter_target_needs_a_selected_group`
- `space_for_a_fighter_group_targets_from_its_squadrons_centroid`: it
  fails with the s140 version, where a fighter group's Space returned
  nothing.
- `a_right_click_hits_the_first_flying_enemy_squadron`
- `a_fighter_group_order_skips_its_lost_squadrons_and_other_groups`

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2937 passed, 0 failed (render 974 passed after the mutant tests) |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | first run 44 tested: 14 missed. After the two tests and `enemy_squadron_hit`: 47 tested, 43 caught, 2 unviable, 2 missed |
| Release WASM SHA-256 | `52ae4a1412d3f309621edad592613572076a86e208c75b17ef78a7e500577b9c` |

Two mutants survive:
- `members > 0` against `>=` in `attack_nearest_enemy` is equivalent.
  When a group has no flying squadron, the centroid's count is 0 and the
  function returns earlier.
- Replacing `handle_combat_input` with `()` needs live mouse input. Its
  squadron hit test is `enemy_squadron_hit`, and tests catch that.

The browser matrix ran before `enemy_squadron_hit` was pulled out of the
mouse handler. The extraction keeps the same filter.
