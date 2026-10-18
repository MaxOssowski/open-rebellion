# A prisoner's side in the AI's records (F-036)

Date: 2026-10-19.

`ghidra/notes/ai-mission-planning.md`, "A prisoner's side in the
records".

## The original

`FUN_00401d20` sets the record's side bit from the object's side `+0x24`
and flips it for a prisoner. A capture turns `+0x24` to the side
opposite the character's home (`FUN_004eec60`). So a prisoner counts on
its home side (`+0x30` 2 for its own side, 4 for its captor's), with
`0x40`.

## Port

- `mission_planning::records` read the home side and flipped it, which
  put a prisoner on its captor's side. It now flips the captive's side
  as `FUN_004eec60` does.
- No save format change. The seed-42 golden is unchanged.

## Tests

- `mission_planning.rs`: `a_prisoner_counts_on_its_home_side`. It fails
  with the old reading (`fail-without.txt`).
- The comment in the Abduction test that relied on the old reading is
  corrected; its assertions are unchanged.

## Campaigns (`playtest-prisoner-side.json`)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days,
against the Abduction commit:

| | Before | After |
|---|---|---|
| Rescue orders | 409 | 429 |
| Abduction orders | 558 | 552 |
| Systems at day 1500, summed (Alliance / Empire) | 599 / 540 | 607 / 536 |

## Gates

- `cargo test --workspace`: 2430 passed, 0 failed.
- Replay equivalence passes with the unchanged golden
  (`v1:530e248f2599b293` -> `v1:03a36c82837d1ce1`) and 0 browser errors.
  Release WASM SHA-256
  `7f7eab04f5597e838ccfc16f8831343bff020737c24ab4e17801a3ed3bcb17ae`.
- The Fleet window gate passes all 48 cases (interface WASM
  `49605fd0ee3a3c366018256b9d76950ac81d69d9fb935bf8363ca8f9f8baf5c1`).
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3600 | 0.5 ms | 0.7 ms | 23.1 ms |
| Fast | 3601 | 0.6 ms | 1.6 ms | 4.5 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`mission_planning.rs`):
  - 12 mutants: 10 caught, 1 unviable, 1 missed;
  - the miss is `UNASSIGNED | CHARACTER` to `^`, which is equivalent
    because the bits are disjoint.
- `provenance-scan check`: 86 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
