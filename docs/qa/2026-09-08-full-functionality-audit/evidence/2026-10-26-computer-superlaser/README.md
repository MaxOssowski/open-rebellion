# The Empire's commander fires a full superlaser (P24)

Date: 2026-10-26. Follows `../2026-10-26-computer-targets/`.

## The original

- On each review, after the withdrawal test, `FUN_005c8780`
  (`0x5c8bbc`..`0x5c8c33`) lets the Empire's commander fire. The
  conditions:
  - the Death Star is present and operational;
  - its charge `+0x6c` is exactly 100.0 (`DAT_0066cbd8`).
- It walks its target list `+0x90` in priority order (`FUN_005c8e10`,
  `FUN_005ed8e0`). It takes the first unit that is not a fighter and is
  still in the fight (`FUN_005ad870`).
- It fires with `FUN_005ba720`:
  - the charge drops to 0;
  - event `0x44` is posted, with the kill a turn later (`FUN_005ce500`).
- `FUN_005ba720` queues no voice. The firing voice `0x111` comes only from
  the interface's fire control (`Class_0066be80` slot `+0xc`, `FUN_005a5740`
  line 75), after its own `FUN_005ba720`.

## The port

- `commit_death_star_shot_by(owner, target, stamp)` holds the shot. The
  player's `commit_death_star_shot` calls it at the command stamp and then
  queues the firing voice.
- `commander_superlaser`, at the end of a review (the computer's or the
  observer's), fires for the Empire at exactly full charge.
  `abstract_battle::commander_superlaser_target` gives the target: the
  first capital ship in the commander's priority list.
- Before, the voice was queued for any shot; now it belongs to the player's
  shot only.

## Tests

- `the_computer_empire_fires_a_full_superlaser_at_its_first_priority_ship`:
  - no shot short of full;
  - at full, the higher-priority ship (index 0) takes the beam at stamp 3
    and the charge resets;
  - with that ship dead, the target falls to the next;
  - a destroyed or Alliance-owned Death Star does not fire;
  - the commander's shot queues no firing voice;
  - an Alliance computer has no superlaser branch.
- `death_star_ready_voice_fires_once_only_when_the_players_laser_crosses_full_charge`
  passes unchanged. It failed while the firing voice was still queued for
  the computer's shot.

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2849 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 148 of 148 passed (run `2026-10-26T07-40-46-438Z-1546829`) |
| Replay equivalence | passed |
| `cargo mutants` | 29 tested: 29 caught |
| Release WASM SHA-256 | `d8722c90cba8c0d36acfec673a1cc6cd0eaabb923ec382e2cb23ed5eb4225aed` |

The first run left 3 mutants. The test then compared the target with the
function itself and lacked the inoperative cases; it now covers both.
