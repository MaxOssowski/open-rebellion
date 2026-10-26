# Squadrons start in their carriers' hangars and launch (P24)

Date: 2026-10-26. This follows `../2026-10-26-squadron-carriers/`, which
gave each squadron the ship it rides on (save v106). Until now every
squadron started the battle deployed, and a recovery knew no home carrier.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository, checked against the `objdump` disassembly).

## The original

### The hangar

The ship constructor builds each capital ship's hangar (`+0x418`) with
`FUN_005b82a0`, called at `0x5b05f8`:

| Field | Meaning |
|---|---|
| `+4` | the carrier's id |
| `+8` | squadrons inside |
| `+0xc` | squadrons inside, plus reservations |
| `+0x10` | capacity, the class's fighter capacity |
| `+0x14` | the next launch stamp; 0.0 means unset (`DAT_0066c690`) |
| `+0x18` | the interval, 5.0 (`DAT_0066c68c`) |
| `+0x1c` | the squadrons inside, in order |

### The setup

- `FUN_0040be90` walks each ship's squadrons. At `0x40bf0b`, a squadron
  whose parent is a capital ship passes flag 1 and the carrier's id to
  `FUN_005964c0`.
- If the squadron has no working hyperdrive (`FUN_005b1b70`):
  - `FUN_005b69d0` puts it in state 1 (`+0x170`), out of the fight and the
    fighter tree;
  - `FUN_005b8570` adds it to the counts and the list and sets `+0x164`.
    When the timer is unset, it sets the timer to `+0x87c` (3 at the setup)
    plus a draw of 0 to 8 plus 5.0.
- With a working hyperdrive, it starts deployed (`FUN_005b69b0`).
- Squadrons on a planet start deployed.

### The hangar's turn, `FUN_005b8800`

`0x005a9de0` runs it for each capital ship in the fight, after the repair
and before the weapons:

- with no squadron inside, it does nothing;
- with the timer unset, it sets the timer (stamp + draw 0..8 + 5) and
  returns;
- before the timer, it returns;
- otherwise:
  - it posts event `0x19` (the hangar, the first squadron inside) at the
    stamp, through slot `+0x2c`;
  - since `+8` still counts that squadron, it sets the timer again in the
    same way.

So launches come at least 5 turns apart.

### The launch

- Event `0x19` (`FUN_005b8b10`) needs:
  - the squadron's group not on order 2 (Recover);
  - the carrier in the fight (or the Death Star operational).
- Then `FUN_005b8c70`:
  - takes the squadron out of the list;
  - places it at the hangar's slot `+4` point, the carrier's underside
    (`0x5b0b80`);
  - sets `+0x164` to the carrier;
  - lowers `+8` and `+0xc`;
  - puts it in the fight (`FUN_005b69b0`, `FUN_005a9170`, sound `0x1b`).

### The recovery

- `FUN_005cf980` tries a member's `+0x164` before its side's ships.
- A hangar's room (`+0xc`) includes its squadrons inside.
- `0x5c7060` flies a member on order 2 with a carrier toward it in any
  state.

## The port

| Original | Port |
|---|---|
| `+0x164` | `TacticalFighter::home_carrier` |
| state 1 | `TacticalFighter::in_hangar`. Such a squadron is not `alive` while inside, like a docked one (state 5): out of every fight, target and draw list, not drawn, and its craft kept for the result |
| the hangar's list and `+0x14` | `TacticalShip::hangar`, `TacticalShip::hangar_launch_stamp` |
| the setup | `expand_fleet` maps each squadron's `FighterEntry::carrier` to its tactical ship; `start_hangar_timers`, run when the battle is made, does the setup's timer draws |
| `FUN_005b8800` | `hangar_turn`, in the turn's ship loop between the repair and the weapon draws |
| event `0x19` | `TacticalTurnEvent::FighterLaunch` and `launch_fighter` |
| the recovery | `carrier_room` counts the hangar; `advance_recovery_executors` tries the home carrier first; in state 0 a member flies to its home carrier |

`port:` choices:

- The Death Star has no hangar in the port.
- What a lost carrier's hangar does with the squadrons inside is untraced.
  The port leaves them inside: their craft count for the result, and the
  strategic side boards them onto another ship the next day
  (`../2026-10-26-squadron-carriers/`).

## Tests

New, in `abstract_battle.rs`:

- `carried_squadrons_without_a_hyperdrive_start_in_the_hangar`:
  - two carried squadrons without a hyperdrive inside;
  - one carried squadron with a hyperdrive deployed with its home;
  - one on no ship deployed;
  - the timer 3 + 0..8 + 5;
- `a_hangar_launches_its_squadrons_one_at_a_time`:
  - event `0x19` at the timer's turn;
  - the next timer 5 to 13 turns on;
  - the launch three turns later at the underside, the home set;
- `a_squadron_stays_inside_while_recovering_or_its_carrier_is_lost`;
- `a_recovery_tries_the_home_carrier_first`, with a full hangar;
- after the first mutation run:
  - `an_unset_hangar_timer_is_the_stamp_plus_a_draw_plus_five`;
  - `a_waiting_member_flies_to_a_live_home_carrier_of_its_side`.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2913 passed, 0 failed (before the two added tests) |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T22-10-12-918Z-3815304`; Arch Chromium 155, not the pinned build) |
| Replay equivalence | passed |
| `cargo mutants` | 39 tested: 38 caught, 1 unviable (first run: 3 missed, closed by the two tests) |
| Release WASM SHA-256 | `2f550527aacc55b386ee553b0c1d548f87b42f33220d6b9914cf94d0c1e6cc3d` |
