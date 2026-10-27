# Squadrons left in their hangar are not destroyed in the results (P24)

Date: 2026-10-27. This follows `../2026-10-27-jump-turn-list-scene-result-marks/`.
The user reported that squadrons which had not launched from their carrier
before the battle ended were listed as Destroyed.

## The cause

- The port keeps a squadron in its hangar (state 1, `FUN_005b69d0`) and a
  docked one (state 5, `FUN_005b8630`) out of the fight with `alive`
  false.
- The results' Fighters page counted every squadron that was not alive and
  not jumping as Destroyed (state `0xb`).
- The commit into the galaxy (`tactical_flow.rs`) already kept these
  squadrons aboard, and only a lost carrier's hangar died with it
  (`../2026-10-27-lost-carrier-squadrons/`). Only the window was wrong.

## The fix

- A squadron in its hangar or docked is aboard its carrier: its home
  carrier (`+0x164`) or, once docked, the carrier that took it in.
- It is Destroyed only when that carrier was lost (hull 0, or gone without
  having jumped out). It stays Operational when the carrier survived or
  escaped.
- Test: `a_squadron_still_in_or_back_in_its_carriers_hangar_is_not_destroyed`.
  It failed before the change, with all seven squadrons listed as
  Destroyed.

## Gates

- Browser `--battle --all`: 148/148 passed.
- Workspace: 3007 tests passed. Provenance scan: no uncited items.
- Replay equivalence and the replay manifest passed.
- Mutants: 17 in all, 16 caught and 1 unviable. The one that first
  survived (`||` to `&&` in the lost-carrier check) is caught by the
  added case of a carrier lost with hull left.
