# Who holds a system (`FUN_0050b610`)

Read 2026-10-17. The port's `economy::resolve_system_control` had misread
this function. It compared a system's energy with GNPRTB 7760 and returned
"Uncontrolled" for every unpopulated system. As a result, a planet that
regiments had occupied (step 3b) went back to neutral the next day.

## The word `+0x84`

`FUN_0050b610(system)` copies the system's word `+0x84` and fills three
two-bit fields in it:

| Bits | Setter | Value |
|---|---|---|
| 2-3 | `FUN_00558910` | the support side (`FUN_0055a080`) |
| 4-5 | `FUN_00558930` | the regiments' side |
| 0-1 | `FUN_005588f0` | the holder |

Each value is 1 for the Alliance, 2 for the Empire, 3 for no side, or 0.
`FUN_0050a300` then stores the word. When the word changed it also runs
the side handlers (`FUN_0050fa90`, slot `+0x208`).

- **The support side.** An unpopulated system (`+0x88` bit 0 clear) gets
  3. Otherwise `FUN_0055a080(+0x58)` decides, where `+0x58` is the
  Alliance's support (0 to 100) and `DAT_006bb4c0` is GNPRTB 7760 (60,
  `parameters.tsv`; its only reader is `FUN_0055a080`):
  - support ≥ `DAT_006bb4c0`: 1 (the Alliance);
  - `100 - support` ≥ `DAT_006bb4c0`: 2 (the Empire);
  - else 3 (no side).
- **The regiments' side.** `FUN_00504cc0(system, 1, side)` walks each
  side's regiments, and `FUN_005131b0` says whether there is one. The
  walk does not check strength.
  - Only the Alliance's regiments: 1.
  - Only the Empire's: 2.
  - Both sides': the previous bits are kept.
  - None: 0.
- **The holder.** It is bits 4-5 when they are not 0, otherwise bits 2-3.

Callers:
- `FUN_00508250`, the full state rebuild;
- `FUN_00508660`;
- `FUN_005109f0` and `FUN_00510e30`, the system's handlers when its
  contents change. So the holder follows the regiments at once.

## Port (2026-10-17)

- `economy::resolve_system_control(sys, alliance_troops, empire_troops,
  gnprtb, difficulty)` follows the rules above.
- `economy::system_holder(world, sys)` counts the regiments as
  `FUN_00504cc0` does, at any strength.
- The economy's step 5 uses it. The simulation's step 3b and the app's
  `tactical_flow::occupy` use it too, for each system with regiments on
  its planet. That stands in for `FUN_005109f0`'s call when a system's
  contents change.

hyp:
- The port keeps no regiment bits. With both sides' regiments, the
  previous side is taken from the side holding the system, if any.
- A planet the economy gives no side keeps a neutral holder
  (`Controlled(Neutral)`) as it is.

port: a side's uprising (`ControlKind::Uprising`) stays while that side
holds the system. The port's `ControlKind::Contested` is no longer
produced here.

## Against the port before

- The energy test had no source: GNPRTB 7760 is the support threshold.
- With both sides' regiments, the port returned `Contested`, where the
  original keeps the previous side.
- An unpopulated system was always `Uncontrolled`, even with regiments on
  it.
- Step 3b's occupation counted only regiments with strength above 0.
