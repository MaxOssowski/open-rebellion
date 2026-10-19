# A captured mission member's remove request (F-038, save v90)

Date: 2026-10-20. The user granted all permissions for this run, save
format changes included.

This is stage D of the tracker item "Wydarzenia fabularne z oryginału":
the Final Battle's `DarthToLuke` step repeated daily in campaign 14.

## The fault

In seed 14 Luke left Coruscant on a Move order (day 575). The mission's
detection set an encounter with Vader, who captured him (slot `+0x20c`).
The port let the Move go on, so Luke reached Corsin, an Alliance system,
as the Empire's prisoner. `FinalBattleReady` then held each day, and
Vader's Move order to Corsin ended at once with code 8 (Move takes only an
own-side target, MISSNSD `0x41000001`). There were 76 orders from day 582.

## The original

- The capture's status change runs the handler `FUN_004f18e0` (character
  slot `+0x334`). A character on a mission takes a remove request
  (`Role_SetMissionRemoveRequest`, `FUN_005345d0`, role `+0x78` bit 2).
- The validator's rule 1 (`FUN_00522480.c:42-52`) counts only a team
  member with role `+0x78 & 0xc` clear. A mission whose every team member
  was captured therefore ends with code 5 at its next validation.
- `FUN_005888f0.c:61-84` gives a resign request only to a member without a
  remove request.
- Read into `ghidra/notes/decoy-roll.md` ("The capture's remove request").

## Port

- `ActiveMission::removing` (save v90): the detection's capture
  (`evade_or_capture`) sets it.
- Rule 1 (`running_end_code`), the detection's member count and the decoy
  count read it; `request_resign` skips a member that has one.
- port: the capture's autorouting is not applied here.
- `final_battle.rs`: the module doc no longer says `DarthPickup` is
  unported (it is in `jabba.rs`).

## Save v90

- `SAVE_VERSION` 89 -> 90.
- The seed-42 golden is regenerated for this cause:
  - initial `v1:29ee3368cb892d9e` -> `v1:0d8727a7d2a2ee81`;
  - final `v1:fb78593dbd336659` -> `v1:3f8a4e8cc1d73a06`;
  - two recordings are identical, and only the fingerprints changed.

## Tests

- `mission_detection.rs`:
  - `a_captured_member_takes_a_remove_request_and_no_resign_request`;
  - `a_team_whose_every_member_has_a_remove_request_ends_with_code_five`.
- Both fail without the change (`fail-without.txt`).
- `cargo test --workspace`: 2468 passed, 0 failed.

## Campaigns (named cause)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:
- 1 of 24 is unchanged. In seed 3 the first difference (day 112) follows
  the first capture of a mission member (day 111, a Move with no resign
  right), which the port used to carry on;
- seed 14: no repeated order. Vader is with Luke at Coruscant, and the
  battle is fought there (day 576);
- seed 12: a Final Battle now happens too (day 606);
- `campaigns.txt` has the Final Battle steps by seed.

## Gates

- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `a6bed48b579bc7122ababb5ce3150d8beb90ffc19b1d147da6fae9206756bd78`).
- The Fleet window gate passes all 48 cases (interface WASM
  `c22f62da6e25172f04d05c2b49de57f4851b250fd5daa0d519bf09e069027d76`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.9 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.8 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`: 22 mutants, 22 caught.
- `provenance-scan check`: 54 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
