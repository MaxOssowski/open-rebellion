# The GID's system marks (read 2026-10-22)

`FUN_0042b330(view, system id, system, ..)` sets one system's mark for the
display chosen in the GID menu (the view's `+200`, the command ids of
`cockpit::GidMode::command_id`). Display Off (`0x80`) draws nothing. The
mark's side `s` starts as the system's holder (`+0x24` bits 6..7).
`FUN_00426e70(value, s, a, b, c, d, e)` picks the mark from side `s`'s
table (`+0x294` Alliance, `+0x2a4` Empire, `+0x2b4` side 3; side 0 has
none):

- largest when `value >= a`;
- large when `c <= value <= b`;
- medium when `e <= value <= d`;
- else smallest.

| Display | Value | `a, b, c, d, e` |
|---|---|---|
| `0x11` Popular Support | the player's support (`FUN_00507270`) | 81, 80, 60, 59, 50 |
| `0x12` Uprisings | system `+0x88` bit 2 | 1, 1, 1, 1, 1 |
| `0x21` / `0x22` fleets | `FUN_004feea0(system, side, 1 / 2)`, both sides | 3, 2, 2, 1, 1 |
| `0x43` / `0x44` personnel | see below | 1, 1, 1, 1, 1 |
| `0x51` energy | `+0x5c - +0x60` | 6, 5, 3, 2, 1 |
| `0x52` raw material | `+0x64 - +0x68` | 3, 2, 2, 1, 1 |
| `0x53` / `0x54` mines, refineries | `FUN_0052d370`, `FUN_0052cf40` | 6, 5, 3, 2, 1 |
| `0x61` | `FUN_0052cc20 > 0` | 1, 1, 1, 1, 1 |
| `0x62..0x64` yards | `FUN_0052c8c0`, `FUN_0052c5a0`, `FUN_0052c270` | 5, 4, 2, 1, 1 |
| `0x65..0x67` idle yards | 15 when the player's system has an idle area (`FUN_00509670(0 / 2 / 1)`, `+0x58` = 0), else 0 | 5, 4, 2 (3 for `0x67`), 1, 1 |
| `0x71`, `0x72`, `0x74`, `0x75` | troops, squadrons, shields, batteries (`FUN_005044f0`, `FUN_00503550`, `FUN_005276d0`, `FUN_005273d0 + FUN_00527150`) | 6, 5, 3, 2, 1 |
| `0x73` Death Star shields | `FUN_0051c0d0` | 1, 1, 1, 1, 1 |

**Fleets** (`0x21`, `0x22`): the player's count and the enemy's. With both,
the mark is the special art `0x27b0` and the value their sum. With only
the enemy's, the side is the enemy's. With neither, it is the holder's
for `0x21`; for `0x22` it is the holder's only when `+0x88` bit 1 is set,
else 0.

**Personnel** (`0x43` Idle, `0x44` Active), with `s` reset to the player's
side:

- the player's characters on the planet (`FUN_004f2640(system, 3, s)`) and
  special forces (`FUN_005040c0`);
- every character and special force aboard the capital ships of every
  fleet the system holds (`FUN_004ffe70`, `FUN_00502db0`, `FUN_004f25c0`,
  `FUN_005040c0`), counted for the player or the enemy by its side.

A character is busy (Active) when:
- it is not usable (`+0x50` bit 0);
- it is on a mission (`+0x78` bit 7);
- it is captive (`+0xac` bit 0);
- it is injured (`+0x94`);
- or it holds a command (`+0x96`).

A special force is busy when it is not usable or is on a mission. The
side is the player's when it counts anyone, else the enemy's when it
counts anyone, else the holder's.

Unexplored systems (`+0x88` bit 1 clear) and destroyed ones (`+0x50`
bit 3) skip the switch for most displays (the `0x17` branch at the top).

## Ported (2026-10-22)

`rebellion-render/src/lib.rs`:
- `gid_metric_size` follows the table;
- `gid_personnel` is the personnel count with its colour;
- `gid_marker_for_side` picks a side's mark.

Before this change:
- raw material used the 6/3/1 bands;
- the idle yards counted facilities;
- the personnel displays counted only the player's people, in bands.

Since the same day the fleet displays follow the rule above
(`gid_fleets`, `gid_fleet_mark`; STRATEGY 10160 is `0x27b0`). port: an
idle fleet orbits the system; an en route one has a move order to it.
The `+0x88` bit 1 test for Fleets Enroute with no fleet is not kept: an
unexplored system shows the unexplored mark first.

Open:
- display `0x61`;
- the `0x13..0x17` and `0x91..0x93` displays the port's menu lacks.

port: a killed character counts as not usable. The port's fleets carry
no special forces. The fleets a system holds are those in orbit and
those en route to it (`movement::listed_fleets`).
