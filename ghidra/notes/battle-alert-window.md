# The Battle Alert window's texts (read 2026-10-22)

The Battle Alert dialog (class `0x659d28`, built by `FUN_0044f860`;
buttons in `abstract-battle.md`) reads a report object (vtable
`0x0065bc40`). `FUN_00488d20` builds that object with `FUN_0049be60(system,
viewer side)`. `FUN_0049be60` keeps the viewer's side at `+0x48` (1, 2,
else 0) and fills the texts through `FUN_0049c0d0`:

- `+0x54`, the title: `0x7020` "Battle at {1:1}".
- `+0x4c`: the system's last arrived side (`+0x78` bits 6..7).
- `+0x60`, the summary, by the system's side (`+0x24` bits 6..7):
  - the viewer's own system: if the viewer's side arrived last, `0x7022`
    "The {viewer} fleet is attempting to break the {enemy} blockade of
    {system}." Otherwise `0x7021` "The {enemy} fleet is threatening
    {system}. {viewer} forces are moving to intercept.";
  - a neutral system (side 0 or 3): `0x7025` "{viewer} and {enemy} forces
    are about to engage in battle near the {system} system.";
  - the enemy's system: if the enemy arrived last, `0x7024` "The {enemy}
    fleet is attempting to break the {viewer} blockade at {system}".
    Otherwise `0x7023` "The {viewer} fleet has entered the {system}
    system. {enemy} forces have been detected on an intercept course."
- It then walks the system's fleets, ships, squadrons and regiments
  through `FUN_0049c5d0`. Not read further; the tabs below fill their own
  list.

The paint (`FUN_0044f860`) draws the scene (`0x29d6` / `0x29d7` by the
viewer's side) at (12, 13). Into it go the title at (0, 16), 400 wide,
centred, font `0xb`, and the summary at (36, 216), 328 wide, font 6, over
a black shadow at (37, 217). Both are in the viewer's colour (`0xff`, or
`0xff00` for side 2). guide-055 is the Empire's view of Deyer, an
Alliance system the Empire entered: `0x7023`.

Ported in `tactical_view.rs` (`battle_alert_lines`): the title and the
summary come from TEXTSTRA with the world's system side and
`System::last_arrival`. The summary has its shadow. Before this, the port
always wrote that the enemy's fleet had entered. Since the same day the
title and the summary sit where the paint puts them
(`battle_alert_scene_text`): the title's top at window y 29, the summary
at window (48, 229), 328 wide. The port had them at about (38, 211),
340 wide. port: the font sizes stay the port's earlier ones.

## The list tabs (read 2026-10-22)

`FUN_0044f860` makes a list control (`FUN_00607ea0`) at (83, 40), 256 by
234, with rows 40 (`0x28`) high, white text (`FUN_00609950(0xffffff)`) and
a scroll bar (`0x29fc`). The command handler `FUN_00450540` sends `200`
to `FUN_00450660`. Tab 1 hides the list and shows surface `0x32` (the
scene with the title and the summary). Tabs 2, 3 and 4 show surface
`0x33` (the scene with the title only) and fill the list through
`FUN_00450770` with 1, 2 and 0:

- 1 or 2, a side's forces: the heading `0x1923` "Alliance Forces" or
  `0x1924` "Imperial Forces" (font `0xb`). For each of the side's fleets
  at the system (`FUN_004ffef0(system, 1, side)`): the fleet's row (font
  5, no picture). Then each capital ship (`FUN_00502db0`) in font 5 with
  its picture, and under it the ship's squadrons (`FUN_005039d0`),
  regiments (`FUN_00504c40`) and personnel (`FUN_00536da0`). Those rows
  use font 4 and are indented (`FUN_00450ce0` with `param_4` 1). Then,
  when the side has squadrons on the planet (`FUN_00503a50`), comes the
  system's name in font 5, and those squadrons in font 4, not indented.
- 0, System Assets: the heading `0x1925`, then the system's facilities
  (`FUN_00539d70`), regiments (`FUN_00504c40`) and personnel
  (`FUN_00536da0`), in font 4, not indented.

A row (`FUN_00450e10`): without a picture, `+0x50` = 1 (centred text).
With one, the picture is `FUN_0042c3b0(.., 0, 1)` (the mini), and the
text starts at 70 (`+0x30` = `0x46`). An indented row draws the picture
20 (`0x14`) further right, and its text starts at 90 (`0x5a`).

Ported in `tactical_view.rs` (`battle_alert_rows`, `battle_alert_list`).
port: a fleet keeps its squadrons, regiments and personnel, not each
ship, so they follow all the fleet's ships. Facilities go by kind
(defenses, manufacturing, production). There is no scroll bar: the rows
whose middle fits are drawn.
