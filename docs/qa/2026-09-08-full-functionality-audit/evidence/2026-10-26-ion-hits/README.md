# Ion hits disable systems, never the hull (P24)

Date: 2026-10-26. Before this stage the port treated an ion hit like any
other: the damage beyond the shields came off the hull. In the original,
ions never touch the hull. Past the shields they stun the engines and
knock out weapon arcs.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository, checked against the `objdump` disassembly).

## The original

### Which handler

`FUN_005b3f10` posts one hit event per family, two turns after the shot:

| Family | Count at | Fire event | Hit event | Target's slot |
|---|---|---|---|---|
| laser | `+0x17c` | `0x0d` | `0x11` (`FUN_005b2a20`) | `+0xd8`, `FUN_005b5100` |
| ion | `+0x180` | `0x0f` | `0x12` (`FUN_005b2a70`) | `+0xdc`, `FUN_005b5b60` |
| turbolaser | `+0x184` | `0x0e` | `0x13` (`FUN_005b2ac0`) | `+0xe0`, `FUN_005b5f50` |
| torpedo | | `0x10` | `0x14` (`FUN_005b2b10`) | `+0xd4`, `FUN_005b5750` |

`ghidra/notes/tactical-fighter-combat.md` lists `FUN_005b5b60` as the
turbolaser handler. By the events it is the ion handler.

### `FUN_005b5b60`

For a target with hull above 0:

- **Damage.** It is the event's strength, at least 1.0. A fighter target
  first divides it by its maneuverability ratio, but no ion ever fires at
  a group.
- **The shields hold** (shields less damage ≥ 0):
  - the shields lose the damage;
  - for a capital ship (slot `+0x20` = 0, `+0x124` clear), the share lost
    is `100 / before × (before − after)`, at least 1;
  - a draw of 0 to 100 (`0x5b5ed8`) at most that share posts a shield hit
    (`0x1e`).
- **They fail:**
  - the shields drop to 0. The hull is not touched;
  - for a capital ship, each point beyond them (the loop subtracts 1.0
    while above 0) draws 1 to 10 (`0x5b5d14`):

    | Draw | Effect |
    |---|---|
    | 1 | a second draw of 30 to 50 is added to `+0x3b0` |
    | 2 | a second draw of 30 to 50 is added to `+0x3b4` |
    | 3 to 10 | arc `(draw − 3) >> 1` is flagged |

  - event `0x16` (`FUN_005ce500` at the turn's stamp, slot `+0x2c`)
    carries the two sums and the four flags.

### Event `0x16`, `FUN_005b2340`

- `+0x3b4` and `+0x3b0` grow by the event's sums.
- Each flagged arc is emptied (`FUN_005b6980`: ready and energy to 0) and
  queued (`FUN_005b64e0`).

### What the timers do

- **`+0x3b4`:**
  - a capital ship's speed (`FUN_005b17f0`) is 0 while it is not 0;
  - each turn, slot `+0x9c` (`FUN_005b0330`) lowers it by the turn's
    1000 ms × 0.001 (`DAT_0066c48c`), not below 0;
  - `FUN_005b7d90` also reads it, against 10000, which 30 to 50 a hit
    cannot reach in a battle.
- **`+0x3b0`:** nothing in the executable reads it. Only the decay,
  `FUN_005b2e40` and the saves (`FUN_005b6a00`, `FUN_005b6eb0`,
  `FUN_005b7370`) touch it. Its draws still advance the stream.

### Recharge of an emptied arc

`FUN_005b6320` hands the recharge to the queued arcs in order. A queued
arc whose energy reaches its capacity becomes ready, an arc with no guns
too: its capacity is 0, so it is full at once.

## The port

- `hit_weapon_shot` sends an ion hit on a capital ship to
  `hit_capital_with_ions`, which follows `FUN_005b5b60` with its draws.
- `TacticalTurnEvent::IonDisable` is event `0x16`.
- `TacticalShip::ion_engine_stun` is `+0x3b4`:
  - `original_tactical_mobility` gives no engine power while it is set;
  - the turn's unit loop lowers it before the ship's repair.
- The recharge now makes a full queued arc ready even with no guns. It
  used to require a capacity above 0.

`port:` the shields and hull stay integers, so the damage and the points
beyond the shields are whole numbers, as before.

## Tests

New, in `abstract_battle.rs`:

- `ions_drain_the_shields_and_never_the_hull`:
  - the share boundary (draws 25 and 26 against 25 %);
  - the minimum of 1;
- `an_ion_overflow_stuns_engines_and_knocks_out_arcs`: four points with
  draws 2 (41 s), 1 (33, dropped), 4 (arc 0) and 10 (arc 3);
- `an_ion_stun_stops_the_engines_for_its_seconds`:
  - the event's effects;
  - no engine power while stunned;
  - a second a turn;
  - a gunless queued arc ready after its recharge.

The tests use the new event and field, so they do not compile without the
change.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2894 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T19-27-18-424Z-2430364`; Arch Chromium 155, not the pinned build) |
| Replay equivalence | passed |
| `cargo mutants` | 43 tested: 40 caught, 1 unviable, 2 missed |
| Release WASM SHA-256 | `5384943bf82d42f2d5d6ac6307d56ebeb6ee76165c3a9561029d014afc83151e` |

Both missed mutants are equivalent:

- `ion_engine_stun > 0.0` replaced by `>=` in the turn loop: at 0 the
  decay gives `(0 − 1).max(0)` = 0 again;
- `!ship.alive || hull <= 0` replaced by `&&` in `hit_capital_with_ions`:
  `hit_weapon_shot` has already left out a target out of the fight, so a
  dead ship never gets there.
