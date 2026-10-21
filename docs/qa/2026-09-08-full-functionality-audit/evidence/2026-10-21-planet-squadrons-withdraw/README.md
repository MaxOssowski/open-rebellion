# The loser's planet squadrons leave (F-033, W5b)

Date: 2026-10-21. `FUN_00516ef0` sets the loser's fleets and its planet
squadrons (`FUN_00503a50(system, 1, side)`) Autorouting
(`ghidra/notes/battle-flow.md`).

## Change

- `abstract_battle::withdraw_loser` now also moves each of the loser's
  planet squadrons to its side's nearest place (`missions::nearest_place`),
  through `SquadronTransits::move_squadrons`, on the battle's day. The
  move's own checks refuse the winner's squadrons, emptied entries and a
  planet another side holds.
- `port:` a squadron with no destination, or whose destination planet
  another side holds, stays. A squadron's speed is never 0 in the port.
- `port:` the winner's squadrons (`FUN_00556710`, gated by `FUN_00506e60`)
  are not read.
- The app's AI battles and Auto Resolve now land the scrambled squadrons
  back (`squadrons::recover`) before the withdrawal, as the headless
  simulation does. Before, they did it after.

## Campaigns

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days, against
HEAD `16270543`. Nine campaigns change: 2, 8, 14, 15, 16, 17, 20, 21 and
22 (`campaigns.json`).

- Battles: 2092 -> 2080.
- Alliance wins: 755 -> 782.
- Empire wins: 1337 -> 1298.
- Capital ships lost: Alliance 25 -> 18, Empire 7 -> 9.

The cause is the squadrons' departure. In the headless simulation it is
the only behaviour change, because the recovery was already in order
there. The telemetry has no event for it, so the first visible
differences are later AI choices.

## Gates

- Tests: 2 new tests. The loser's squadrons leave and the winner's stay;
  a squadron with only enemy-held planets stays.
  - With the squadron loop neutralized, the first one fails.
  - Workspace: 2556 passed, 0 failed. Replay goldens pass; the only
    failure is the long-standing
    `seeds::tests::fixed_fleet_tables_only_target_special_systems`.
- Mutants:
  - The first run left 3 survivors, in a side and count filter that
    repeated the move's own checks and a speed guard the port cannot reach.
    Both were removed.
  - The final run: 2 caught, 2 unviable, 0 missed.
  - The final campaigns equal the first run's (24 of 24).
- `provenance-scan check`: 0 uncited.
- Browser, Arch Chromium 155.0.8059.84:
  - Fleet window 58 of 58;
  - replay equivalence passed (no errors, 4 requests);
  - frame timing p99 0.8 / 1.6 ms, 0 frames over 50 ms.
- Release WASM: `38479d61e1624c8acb6fbc39bea4586fabeebfa1404153120f1300a46d0cfb2b`.
