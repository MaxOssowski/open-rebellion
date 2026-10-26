# Squadrons ride on their fleet's ships (P24, save v106)

Date: 2026-10-26. The user approved a save-format change so that the
tactical battle can start squadrons in their carriers' hangars
(`../2026-10-26-recovery-executor/`, open question). Until now a fleet's
squadrons were one count per class, on no ship.

Source: static reading of REBEXE.EXE (`~/Games/rebexe-decompile/`,
outside the repository).

## The original

- A fleet's room (slot `+0x78`, `FUN_004fe160`) takes only capital ships
  (galaxy types `0x14..0x1c`). It refuses squadrons (`0x1c..0x20`),
  regiments and characters, which ride aboard ships.
- A ship's room for squadrons (`FUN_00500b40`) is its class's fighter
  capacity (`+0x26c`) less the squadrons aboard. It refuses with status
  `0x14`/1.
- A squadron's container is therefore a capital ship or a system's planet,
  never the fleet.
- **Moves.** A Move (`0x201`) to a fleet asks the destination's `+0x78` for
  each family (`FUN_00550700`) and is refused with status `0x27`. So the
  order names a ship. `FUN_00556390` reparents the squadron into it.
- **Join and split.** These move ships, and a ship's cargo travels with it
  (`FUN_00552300` expands fleet → ships → cargo; `FUN_004feca0`).
- **Battle.** The battle start walks each ship's squadrons
  (`FUN_0040be90`, `FUN_005039d0(ship, 3)`). A squadron whose parent is a
  capital ship passes that carrier to the tactical setup (`0x40bf0b`,
  `FUN_005964c0`).

Untraced:

- the campaign's seeding of fleets with squadrons;
- a delivery to a fleet (the port delivers built squadrons to the planet,
  as the original's system destination does);
- what becomes of the squadrons aboard a scrapped, destroyed or captured
  ship.

## The port

- `FighterEntry::carrier` is the `ShipInstance::id` of the ship a squadron
  rides on: `None` on a planet, and in a fleet for squadrons not yet
  aboard. Save v106 (`agent_docs/save-load.md`); the format has no
  migration, so older saves are refused.
- `squadrons::seat_squadrons` boards a fleet's squadrons that ride on none
  of its living ships. They go onto its living ships in fleet order, up to
  each ship's room. `GameWorld::number_ships` runs it for every fleet after
  numbering new ships. The drivers call it before each day and after the
  campaign's setup.
- `squadrons::aboard` counts a ship's squadrons. `add` adds to the entry
  on no ship.
- `take` now takes across a class's entries, the last first.
- Join (`merge_fleet_into`): entries keep their ship and merge only with
  the same ship's.
- Split (`move_ships_into`): the squadrons aboard the moved ships go with
  them. The port used to leave every squadron in the old fleet.

`hyp:` and `port:` choices:

- `hyp:` the order the ships fill in, since the original's order names its
  ship. It covers moves onto a fleet, the seeding, the sorties' landing and
  the squadrons aboard a lost ship (untraced), which board the fleet's
  other ships.
- `port:` squadrons left over when no ship has room stay in the fleet on no
  ship. The port's seeding and sorties can put them there.
- `port:` the move's room check is still the whole fleet's
  (`squadrons::fleet_room`). The original's names one ship.

## Tests

New:

- `squadrons_board_the_fleets_ships_in_order_up_to_their_room` (two ships
  of room 2, five squadrons);
- `a_lost_ships_squadrons_board_another_ship`;
- `taking_squadrons_draws_on_the_last_entries_first`;
- `moved_ships_take_the_squadrons_aboard_them` (`fleet_join.rs`);
- `a_round_trip_keeps_the_ship_a_squadron_rides_on` (save v106);
- after the first mutation run:
  - `squadrons_of_a_class_on_one_ship_share_an_entry`;
  - `a_merge_keeps_each_squadron_on_its_ship`.

The seed-42 replay golden is regenerated for this save format change. Only
the state fingerprints differ:

- the initial one goes from `v1:06cd3db9594663cc` to `v1:ba9d8c7e73c0e2cd`;
- the final one from `v1:d66014ea2b71379b` to `v1:59b9d1aa26541572`.

`replay_fixture.rs`, `replay_seed42_v1.json`, `tests/replay_manifest.rs`
and `agent_docs/deterministic-replay.md` carry the new values. The first
replay run failed on the old initial fingerprint, as a layout change
must.

The literal `FighterEntry` values in the tests and fixtures gained
`carrier: None`. No assertion changed.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2907 passed, 0 failed; the ignored replay manifest tests (with data) 2 passed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T21-31-18-353Z-3528667`) |
| Browser interface matrix (GID) | 152 of 152 passed (run `2026-10-26T21-43-02-739Z-3563534`; Arch Chromium 155, not the pinned build) |
| Replay equivalence | passed, after the golden's regeneration |
| `cargo mutants` | 44 tested: 38 caught, 6 missed |
| Release WASM SHA-256 | `3df48bf3fb608df583ce14dd5204459c9e9b0c5549453e6f950a56816d1fbe38` |

Of the 6 missed mutants:

- the merge's `&&` and `put`'s `==` were gaps, closed by the two tests
  above;
- the four in `seeds.rs` remove the seeding of fleet squadrons. The seed-42
  replay golden covers them, but it needs the original data and does not
  run under `cargo mutants`.
