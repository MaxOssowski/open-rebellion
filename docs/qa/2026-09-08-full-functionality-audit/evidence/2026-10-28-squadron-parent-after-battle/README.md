# After a 3D battle, a squadron rides on the carrier it entered, or lands on the system (P24)

Date: 2026-10-28. This follows `../2026-10-28-ctrl-clicks-a0/`. That stage
showed that the VM evidence for keeping launched squadrons in the fleet
came from a host-edited save. The user then chose the original's rule
from the code ("wprowadz regule z kodu orginalu").

## The original

The tactical commit is `FUN_0040c780`. Its second pass calls
`FUN_0040d320` for every squadron of the battle system: those aboard the
fleets' ships, and those on the system itself. The entry check,
`FUN_004f6df0`, always returns 1.

- The squadron's battle record comes from `FUN_005968e0` ->
  `FUN_005b8e00`. Its carrier flag is set when:
  - the `+0x164` carrier is found as a capital unit (`FUN_005a6360`);
  - and the state `+0x170` is 1 (in the hangar) or 5 (docked).
- With the flag, the new parent is that carrier, when it still exists and
  is on the squadron's side (`+0x24 & 0xc0`).
- Without it, the parent is the battle's system (`[esi+8]` ->
  `FUN_004025b0`, `0x40d47c`). That is the planet's surface. Manual p. 129:
  "Fighters in a system that are not a part of a fleet are considered to
  be in hangars on the ground".
- `+0x164` is the last carrier entered: a dock (`FUN_005b8570`) and a
  launch (`FUN_005b8c70`) both set it.
- This pass runs before the ships' hull pass (`FUN_0040cfc0` ->
  `FUN_00501ae0`). A lost carrier's hangar and docked squadrons therefore
  go aboard and die in its cascade (`GameObj_vt11c`). The tactical battle
  has already killed its hangar's squadrons (`FUN_005b2d20` ->
  `FUN_005b8d80`).

So a squadron still flying at the end goes to the system: one that was
launched, or one that jumped out on its own hyperdrive (state 9). This
includes the TIEs, which start deployed (`0x40bf0b`). A squadron that
docked keeps flying with its carrier: a carrier's jump docks its hangar,
state 5 (`FUN_005a90c0` -> `FUN_005b8d10`).

## The port

- `crates/rebellion-app/src/tactical_flow.rs`:
  - `apply_results` first applies every fought fleet's hulls.
  - `commit_squadrons` then places each surviving squadron:
    - one in a hangar, or one docked (`Recovered`, on its recovery target
      or else its home carrier), goes aboard that ship when it lived, and
      otherwise dies;
    - any other goes to `system.fighters`.
  - The lost ships leave their fleets after that.
- `crates/rebellion-app/src/main.rs`: after a tactical commit, the planet
  squadrons' sorties are not recovered again. The commit has already
  placed them, on the system or on the carrier they docked on. A battle
  finished by Simulate Remainder still recovers them.
- `squadrons::put` is public for the commit.

Before this, a launched squadron stayed in its fleet and `number_ships`
seated it on a ship with room.

## Tests

- `a_squadron_rides_on_the_carrier_it_entered_and_any_other_lands_on_the_system`
  replaces `a_lost_carriers_hangar_squadrons_die_and_its_launched_ones_stay_in_the_fleet`.
  It covers four squadrons:
  - one in the hangar of the surviving carrier, which stays aboard;
  - one in the hangar of the lost carrier, which dies;
  - one from the lost carrier that docked on the surviving one, which
    rides on it;
  - one that was launched, which lands on the system.
  It fails with the old placement (`left: 3, right: 2`).
- `tactical_space_results_preserve_exact_capital_and_fighter_roster_slots`
  had a surviving squadron in the field. It now expects that squadron on
  the system instead of in the fleet. This expectation changes for the
  recovered rule.

## Not observed

No clean VM battle has had a launched squadron survive to the end. The
two tries are recorded in `../2026-10-28-ctrl-clicks-a0/`.

## Gates

- Browser `--battle --all`: 148/148 passed (run `2026-10-28T09-50-20-512Z-2167174`).
  The probe `production-battle-results-presentation` now expects five
  squadrons a side to leave the fleet and four to land on the system:
  `attacker_landed`/`defender_landed`, added to the
  `[tactical_results]` log. Before this, it expected four to stay in the
  fleet and failed on the new rule (`5->0`).
- Workspace: 3014 tests passed. Provenance scan: no uncited items.
- Replay equivalence and the replay manifest passed. The golden is
  unchanged.
- Mutants (`tactical_flow.rs`, 10): 8 caught, 2 unviable.
- Release WASM SHA-256 `43199375bf3e9183b35879d7ba873d72be7503b045e5fca75fbd7bda6be461c2`.

**Superseded (2026-10-28, s180).** A clean VM battle showed squadrons
still in the field at the end going back aboard their carriers, not to the
system. See `../2026-10-28-squadron-parent-after-battle-vm/`.
