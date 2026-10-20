# Ship repair runs on timer 0x380 (F-035, save v98)

Date: 2026-10-20 (overnight run, "Produkcja i naprawy"). The full reading
is in `ghidra/notes/ship-repair.md`.

## The original

- UnderRepair is Damaged and Usable (`FUN_005578d0`). There is no
  shipyard condition; a travelling ship is not usable.
- The flag arms timer `0x380` on the ship's record (`FUN_005577f0` ->
  `FUN_00557c00`).
- The period is GNPRTB 2052 (2 days) with FastRepair, else 2053 (30)
  (`FUN_005010a0`).
- FastRepair (`FUN_0050c580`, `FUN_0050bb70`, `FUN_00528040(0)`): the
  ship's system is of its side and has a usable, unsuspended yard.
- A change of FastRepair rebuilds the timer from that day
  (`FUN_00557860` -> `FUN_00557c30`).
- Each fire repairs a tenth of the class hull (`FUN_00501c50` ->
  `FUN_00501b70`; the hull is `FUN_00500550`).

## What the port did before

`repair.rs` repaired only at a system with any manufacturing facility,
every day, by the class `damage_control`. None of that had a source.

## Port

- `RepairState` keeps each repairing ship's timer (due day, FastRepair),
  by ship id (save v98).
- `RepairSystem::advance` arms, rebuilds, drops and fires the timers as
  above. Its signature and events are unchanged.
- port:
  - a fleet in hyperspace is in no system's list, which stands for "not
    usable";
  - a ship with id 0 waits for its number;
  - no subsystem nibbles or squadron damage exist in the port, so a fire
    repairs only the hull;
  - the timer's delay spread is taken as 0 (hyp).

## Tests

- `a_damaged_ship_repairs_a_tenth_of_its_hull_every_two_days_at_its_shipyard`.
- `away_from_a_shipyard_a_ship_repairs_every_thirty_days`.
- `a_new_period_rebuilds_the_timer_and_a_healthy_or_travelling_ship_has_none`.
- `an_unnumbered_or_lost_ship_and_an_empty_day_repair_nothing`.
- `a_system_in_revolt_or_destroyed_suspends_fast_repair`.
- On the old per-day `damage_control` rule, three of them fail
  (`fail-without.txt`).
- The save round-trip test now numbers its ship, as the drivers do.
- `cargo test --workspace`: 2489 passed, 0 failed.

## Save v98

- `RepairState` gains the timers, so `SAVE_VERSION` 97 -> 98.
- The seed-42 golden is regenerated for this cause:
  - initial `v1:ba71ff33556e6215` -> `v1:44a4ae74f038e3ea`;
  - final `v1:f7529f2440ec6c60` -> `v1:a2218c2164fce8ed`;
  - two recordings are identical, and only the fingerprints changed.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:
- 4 are identical to the stage before, and 20 change;
- totals over the 24:
  - `ship_repaired` events: 1765 -> 445 (a tenth of the hull per fire,
    not a little every day);
  - `ship_repair_started`: 115 -> 140 (repair now runs away from
    shipyards too);
- all 24 run to day 1500.

## Gates (`f16`)

- The replay equivalence check passes on the new golden with 0 browser
  errors (release WASM SHA-256
  `974ac4f267e85f370a6e2c068d7c914244978ab5cbffd4eddd55f277cb87b520`).
- The Fleet window suite passes 50 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- `cargo mutants --in-diff`:
  - the gate run had 1 survivor in `fast_repair` (the shipyard/side
    test);
  - a test now covers it, and a rerun catches all 33 viable mutants (2
    unviable).
- `provenance-scan check`: 53 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
