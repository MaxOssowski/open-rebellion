# Launched squadrons join their group; a jumping ship faces away (P24)

Date: 2026-10-27. The user reported three faults in play:

- "okręt nie odwraca się w prawidłowym kierunku przed skokiem w
  nadprzestrzeń": a ship does not turn the right way before its jump;
- "myśliwce wylatują z okrętów ale za wolno i nie tworzą oddziału do
  sterowania nim": squadrons launch from ships but too slowly, and they
  form no group the player can command;
- "nie da się ich też kliknąć": they cannot be clicked either.

## The cause, from the original

### Launched squadrons

- `FUN_005964c0` puts a carried squadron without a hyperdrive into its
  carrier's hangar. The tactical manager's `FUN_005a9470` adds it to the
  lists `+0x9a0` and `+0x9c4`, so it is one of the battle's units from the
  setup.
- Its launch, `FUN_005a9170`:
  - takes it off `+0x9c4` and onto `+0x988`, the fighters in the field;
  - puts it into the fight (`FUN_005a9380`);
  - reads its force, `+0x174`: when the squadron leads that force, it
    moves the force's mover anchor to the squadron.
  - So the squadron launches in a force it already holds.
- The port dealt the fighter groups (`assign_original_fighter_groups`)
  only to squadrons in the field. A squadron in a hangar kept group 4,
  unassigned (`FUN_005c63f0`'s start). Once launched, it had no group, no
  group mover (`FUN_005adfa0`), no group button, and no selection. Left
  without a mover, it flew alone, which was the slowness.
- The groups come from each squadron's tactical resource (ordinals 29..36),
  whether it is in a hangar or not, so the port now deals them to squadrons
  in a hangar too.

### The jump

- `FUN_005a6b40`, each step of a unit's jump, sets the unit's forward
  (`+0x10`) to its side's vector reversed (`FUN_005a7490(side)`, negated).
  It then moves the unit along that vector (`fStack_18 = side.z * -d +
  z`, `FUN_005c12a0`).
- The port moved the ship's z but left its forward as it was. A ship
  facing the enemy jumped stern first.
- The port now sets the ship's forward and desired forward to the reversed
  side vector on each jump step.
- port: the port's squadrons keep no forward of their own, so nothing
  changes for a jumping squadron.

## Tests

- `a_squadron_in_a_hangar_holds_its_group_and_flies_with_it_once_out`:
  without the change the squadron's group is 4.
- `a_withdrawal_sends_the_fighters_to_their_carriers_and_away` now asserts
  that the jumping carrier faces -z. Without the change it faced z 0.947.

## Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 2997 passed, 0 failed |
| `replay_manifest -- --ignored` (original data) | 2 passed; the golden is unchanged |
| `provenance-scan check` | 0 uncited items, within the baseline |
| Browser tactical matrix (Linux Chromium, 148 cases) | 148 passed, 0 failed |
| Replay equivalence | passed |
| `cargo mutants` | 4 tested: 4 caught |
| Release WASM SHA-256 | `35e8469ca752b961d3d14f9866980960d767a99adb0effa5e15c2f7462aac4df` |

Not checked: no browser fixture has a squadron in a hangar or a ship's
jump on screen, so neither change was inspected in a screenshot. The
launch cadence (at least five turns between launches, `FUN_005b8800`) is
the original's and is unchanged.
