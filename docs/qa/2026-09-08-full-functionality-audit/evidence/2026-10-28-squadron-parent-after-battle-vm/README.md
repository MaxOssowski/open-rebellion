# After a 3D battle, squadrons in the field return to their carriers (P24, A0 VM)

Date: 2026-10-28. This corrects `../2026-10-28-squadron-parent-after-battle/`
(s179). That stage sent every surviving squadron that was not in a hangar
or docked to the battle system's surface. It followed a static reading of
`FUN_0040d320`, before any clean VM run had shown the case. A clean run
now has, and the original does otherwise.

## The VM runs

Captures are in `.artifacts/a0-vm/2026-10-28-ctrl-squadrons/` (ignored;
hashes below). The save is the original `resource turn 14` (Empire, day
132), unedited. A location patch was tried and had no effect, since Fleet
1 still stood at Averam. Imperial Fleet 1 is Imperial Star Destroyer 1
with 6 TIE squadrons (6 of 6) and Victory Destroyer 2 with 1 (1 of 2). It
was ordered from Averam to the Alliance's Mon Calamari. Two corvettes,
Swift and Quick, met it there, with no fighters (`mc-alliance-forces`).

1. **Simulate Results** (clicked by mistake). Afterwards the 4 surviving TIEs
   were all aboard ISD 1 (4 of 6), and the system showed no squadron of
   its own (`mc-sim-results`, `mc-sim-fleet-fighters`,
   `mc-sim-isd-fighters`). The port's simulated path keeps squadrons in
   their fleet too.
2. **Take Command**, run to the end. All 7 TIEs survived
   (`mc3d-results-fighters`). Afterwards Fleet 1 held 7 of 8, with 6 of 6 on
   ISD 1 (`mc3d-fleet-fighters`, `mc3d-isd-fighters`). The Mon Calamari
   system window showed only Fleet 1.
3. **Take Command again**, to see the squadrons in the field. With the
   camera zoomed out, the Red Group dots (7 squadrons, 84 of 84 fighters,
   `mc3d-red-group`) circled ISD 1 from the first minute
   (`mc3d-ties-launched`) up to the last frame before the results
   (`mc3d-last-frame`, 3 red pixels at the carrier). Afterwards Fleet 1
   held 7 of 8 again, and Victory Destroyer 2 held 1 of 2
   (`mc3d-run2-fleet-fighters`, `mc3d-run2-vd-fighters`).

So a squadron still flying at the end goes back aboard the carrier it
launched from. What docks it before `FUN_0040d320` reads its state is not
traced. That would be a state of 5 or 1 set by something in the battle's
end. Event `0x45` (`FUN_005ba5e0`) is the Death Star's and not it.

## The port

`crates/rebellion-app/src/tactical_flow.rs`, `commit_squadrons`:

- a docked squadron (`Recovered`) rides on the carrier it docked on;
- any other survivor, in its hangar or in the field, rides on the carrier
  of its roster entry, the one it launched from;
- a lost carrier's hangar and docked squadrons die with it. port: a
  squadron it launched stays in the fleet on no ship, and `number_ships`
  seats it, as before s179. No clean run has shown this case;
- a planet's squadron stays in its sortie's fleet, and `squadrons::recover`
  lands it, as before s179.

`main.rs` recovers the sorties after every battle again. The s179
`[tactical_results]` landed counters and the results probe change are
reverted.

## Tests

- `a_surviving_squadron_rides_on_its_carrier_and_a_lost_carriers_dies_inside_it`
  replaces the s179 test. It has four squadrons:
  - one launched from the surviving carrier, which comes back aboard;
  - one docked there from the lost carrier, which rides on it;
  - the lost carrier's hangar squadron, which dies;
  - the lost carrier's launched squadron, which stays in the fleet on no
    ship.

  The planet takes none. The test fails when a squadron from the field
  does not return to its carrier (`left: 1, right: 2`).
- `tactical_space_results_preserve_exact_capital_and_fighter_roster_slots`:
  the surviving squadron stays in its fleet, as it did before s179. The
  emptied roster entry is now dropped, so the test asserts one entry of
  count 1.

## Ctrl clicks on a task force (same session)

With Task Force 1 (ISD 1 and VD 2) selected (`tf-both-selected`):

- Ctrl+left on VD 2 dropped it from the force: its green bracket went
  (`tf-ctrl-leave`).
- A second Ctrl+left brought it back (`tf-ctrl-join`).

Both match `ctrl_click_ship` (`left_task_force`, `joined_task_force`).
The task force's escort drop was not reached. It goes through the same
SetOrder (`FUN_005ca530`) as the fighter group's, which was checked.

## Captures

| Capture | SHA-256 |
|---|---|
| `mc-alliance-forces.png` | `1b9accee2681515507691e0b1cf331abe22f37851b182b961ff6ccd699c07940` |
| `mc-sim-results.png` | `13e5c9a69b4434052ca2cc59fa8be8a2eb78cc622788996e782b9ab24a027103` |
| `mc-sim-fleet-fighters.png` | `448a8d62762514b6c834413e1bc8ecb0f0b728a9b4ff6b7b9b43cdb28a44f671` |
| `mc-sim-isd-fighters.png` | `c5801a62646bfefeae0b565aa9a4b93bccdc4c541076e8f59caf71c93d009da0` |
| `mc3d-results-fighters.png` | `667ce71009c50296cc30ce6fff001463612714bb1092f9314f065465cbaaf799` |
| `mc3d-fleet-fighters.png` | `fbc916982904cd977995d3605b1e43ff75289470cb30dea6b147760f72d8e4f5` |
| `mc3d-isd-fighters.png` | `8e05d51dfe6394ac35ba1b25507641245b81d3f38a94c7ef41b88e7b263968f1` |
| `mc3d-red-group.png` | `42ddb5af718169dfeeec9e731c7880499c6f13b165665ffe9859a32702df9e3c` |
| `mc3d-ties-launched.png` | `b2036c152e5cd0527754fbdfc4a558a48d9b6d73f5a5999fcc940d240a70fc99` |
| `mc3d-last-frame.png` | `c62e82eb70ec63e0fb376b46311b7fe8ccb106b869dbc97aa3555fd12fee3f20` |
| `mc3d-run2-fleet-fighters.png` | `ca9fd4240e2dd4e3884e4cf506ca8e64a13c991d913c0bd7723e3b96bc934071` |
| `mc3d-run2-vd-fighters.png` | `8a4110a7e1c7f75e52a8290f2cb397b888ceed9fad92c6087a1af8cd207e7fa7` |
| `tf-both-selected.png` | `64260d0c339f8e8bd0d6ab3a01f19a8213403a142b190abc97b097da2245a5ac` |
| `tf-ctrl-leave.png` | `e805619cc2ec0ae0950fc1f279321f66abcd3825a6f9f9279a6823a0353ac300` |
| `tf-ctrl-join.png` | `a44197958fb0c0eaeccc08e0433ae189332d71b11567fb795715547280b2db5a` |

## Gates

- Browser `--battle --all`: 148/148 passed (run `2026-10-28T10-45-54-444Z-2244940`).
- Workspace: 3014 tests passed. Provenance scan: no uncited items.
- Replay equivalence and the replay manifest passed; the golden is
  unchanged.
- Mutants (`tactical_flow.rs`, 10): 9 caught, 1 unviable.
- Release WASM SHA-256 `1ab555164c5055a0dc725a3f22beb53d11c8d391efad5e2ed8f94c8070331f3a`.
