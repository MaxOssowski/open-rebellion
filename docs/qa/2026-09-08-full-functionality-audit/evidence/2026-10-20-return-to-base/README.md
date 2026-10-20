# The Return goes to the character's base, where it joined the mission (F-040)

Date: 2026-10-20. Stage M5g of the "Misje" area.

## The manual (pp. 110-111)

"Each character is attached to a specific base system. ... it is where the
character returns if the system where the mission takes place is not
friendly to your side when the mission ends. ... To change a character's
base, move the character there with the Move command."

## The original (read 2026-10-20)

- Setting RoleOnMission (`+0x78` bit 7, `FUN_00534800`) runs slot
  `+0x268`, `Role_vt268` `FUN_00536490` (the character's override
  `FUN_004f1490` calls it first):
  - joining a mission stores the member's container (`FUN_004f6b70`) in
    `+0x70` (setter `FUN_00534390`);
  - leaving clears it.
- `FUN_00542ee0` tries the container, then `+0x70` (`FUN_005543c0`: the
  move validator), then the search `FUN_00543040`.
- A Return (family `0x42`) needs a target of its own side (`FUN_004f4b60`,
  `1`/`0x28`).
- So the base is where the member started its mission, and a Move changes
  where the next one starts.

## Port

- `return_destination` takes the mission's origin system (`+0x70`) when
  the member's side holds it, before the nearest search.
- port: without the route checks.
- `escape_destination`'s comment now says why an escapee's `+0x70` is
  clear (it is on no mission).
- No save format change.

## Tests

- `missions.rs`:
  `an_agent_returns_to_where_it_joined_the_mission_while_its_side_holds_it`.
  It fails without the change (`fail-without.txt`).
- `cargo test --workspace`: 2486 passed, 0 failed.

## Campaigns (named cause)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:
- every campaign changes;
- the AI's own Move orders fall from 32,470 to 20,412, since agents now
  return to where they were sent from;
- missions resolve about as often;
- `campaigns.txt` has the totals.

## Gates

- The seed-42 golden is unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `869ec1660d8fbe166efbce32f1241919894ff84843d26a754a8bb447ee2f9ea9`).
- The Fleet window gate passes all 48 cases.
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.8 ms | 23.0 ms |
| Fast | 3601 | 0.5 ms | 1.6 ms | 4.6 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff`: 7 mutants, 7 caught.
- `provenance-scan check`: 54 uncited items, unchanged.
- The comment change in `escape_destination` came after the gates; it
  changes no code, and `cargo test` passes after it.

Hashes of the committed files are in `SHA256SUMS`.
