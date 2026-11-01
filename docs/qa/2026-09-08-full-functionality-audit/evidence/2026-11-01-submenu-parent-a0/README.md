# A submenu parent's enabled state against A0 (2026-11-01)

## A0

- `status/05-fleet-menu.png`: an enemy fleet's menu draws Planetary
  Bombardment grey (128), its arrow beside it, with every other order but
  Encyclopedia and Status grey.
- New capture, VM Alliance game day 313, after the Battle at Yavin
  (`menus/05-after.png`): the player's fleet icon at Kimanan, the
  player's own system. `menus/08-fleet-icon-menu.png`: Planetary
  Bombardment is grey; no target can be bombarded at one's own planet.
  `menus/09-parent-hover.png`: with the pointer on it for over a second,
  the row does not light and no submenu opens.
- `status/14-char-menu.png`: Veers's Command is white.

Hashes in `a0-sha256.txt`; the captures stay in `.artifacts/a0-vm/`.

## The original

`FUN_004aba60` paints a row grey when its `+0x58` is clear, and
`FUN_004aab50` lights (and so opens the submenu of) enabled rows only.
`FUN_004ac5c0` passes `FUN_00442590` each listed order's enabled flag
(`+0x1c`, set by `FUN_0051d990` from the order's `+0x18`). How the parent
row (0x120, 0x160) gets its flag is not settled from the decompile: the
recursion builds a related item with flags 8 (enabled) on one path and
flags 4 (disabled) on the other. The old note's "a parent (flags 8) is
always enabled" does not hold against A0.

## Change

hyp: a submenu parent is enabled when one of its rows is
(`object_menu.rs`). It agrees with all three A0 captures: no target, grey;
Veers has an enabled Command order, white.

Tests: `a_submenu_parent_is_enabled_only_while_one_of_its_rows_is` (new),
and ten menu tests that expected the parent enabled with every row
disabled now expect it grey, citing the captures.

Mutants: `object_menu.rs --re object_menu_rows`: 45 tested, 44 caught,
1 unviable.
