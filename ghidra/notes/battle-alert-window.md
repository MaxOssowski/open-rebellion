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
  through `FUN_0049c5d0`, the lists of the forces tabs. Not read further.

The paint (`FUN_0044f860`) draws the scene (`0x29d6` / `0x29d7` by the
viewer's side) at (12, 13). Into it go the title at (0, 16), 400 wide,
centred, font `0xb`, and the summary at (36, 216), 328 wide, font 6, over
a black shadow at (37, 217). Both are in the viewer's colour (`0xff`, or
`0xff00` for side 2). guide-055 is the Empire's view of Deyer, an
Alliance system the Empire entered: `0x7023`.

Ported in `tactical_view.rs` (`battle_alert_lines`): the title and the
summary come from TEXTSTRA with the world's system side and
`System::last_arrival`. The summary has its shadow. Before this, the port
always wrote that the enemy's fleet had entered. The window's positions
and font sizes are the port's earlier ones. The forces tabs are still the
port's.
