# The seat of power's leadership bonus (F-038)

Date: 2026-10-19.

`ghidra/notes/seat-of-power-leadership.md`. This is the parity tracker's
item "Premia przywództwa (pole +0x92)", read and ported in one stage.

## The original

- `+0x92` is a percent bonus to leadership. Its one writer is
  `FUN_005618d0`:
  - it writes GNPRTB 3076 (50) while `SeatOfPower` holds and the character's
    class is the Empire's alone (`FUN_004edc80` == 2);
  - otherwise it writes 0.
- It runs for every character when `SeatOfPower` changes (slot `+0x1d8`),
  and for a newly placed character.
- `FUN_004eede0` computes the effective leadership `+0x88` (slot `+0x1f4`):
  leadership + leadership × `+0x92` / 100, at most `0x7fff`.
- The manual's "Seat of Power Effect" says the same: the Emperor on
  Coruscant, not on a fleet, not captured, and Coruscant the Empire's; then
  all Imperial characters get a leadership bonus.
- `SeatOfPower` is the character manager's `+0x58` mask 1, that is bit 0
  (`FUN_0053a640(1, ...)`; `FUN_0053a610` takes a mask). The port's
  `seat_of_power.rs` and `story-message-classes.md` had called it bit 1,
  and are corrected.

## Port

- `seat_of_power::leadership_bonus` and `seat_of_power::effective_leadership`.
- These read them:
  - `missions::member_skill` (leadership), which serves the missions, the
    uprising terms and the AI's agent records;
  - `character_command::leadership`, which serves the bombardment and the
    assault;
  - the Status window's "Leadership Rating:" row (`+0x88`). That row had
    shown the base `+0x64`.
- port: the bonus is read from the world as it stands, not stored, so
  there is no save change.
- port: the class sides are the port's `is_alliance` and `is_empire`. A
  defection, or the interface fixture's side flip, rewrites them where the
  class record would not.

## Tests

- `seat_of_power.rs`:
  - `the_seat_of_power_adds_half_again_to_an_imperial_character_s_leadership`;
  - `a_seat_of_power_general_s_bonus_reaches_the_missions_and_the_duel`.
- `status_window.rs`:
  `leadership_rating_is_the_effective_leadership_with_the_seat_of_power`.
- All three fail without the bonus, or with the old base row
  (`fail-without.txt`).

## Browser (the Status window gate)

The gate passes on both sides. Its fixture puts Mon Mothma on the player's
side, and the Emperor is seated at Coruscant:
- on the Empire's side her "Leadership Rating:" reads 150 (100 + 50%);
- on the Alliance's side it reads 100.

The screenshots are not committed (original artwork). Their SHA-256 are in
`screenshots.sha256`, and they are kept in
`.artifacts/interface-parity/status-window-2026-10-19T18-12-52-927Z-3780772/`.

## Campaigns (named cause: the seat of power's bonus)

`rebellion-playtest data/base --dual-ai`, seeds 1 to 24, 1500 days:
- 21 of the 24 runs change; `campaigns.txt` has the counts.
- The Empire's leadership missions gain. Subdue Uprising succeeds 298 times
  in 1170 resolved missions, against 243 in 1564 before.
- The `story_final_battle` steps that had run in 3 campaigns run in none.
  The campaigns diverge.

## Gates

- `cargo test --workspace`: 2474 passed, 0 failed.
- The seed-42 golden is unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256 `787b7461c0d194ce50d081afefd660ad0553bb7806621aaa4ea623b377d020f7`).
- The Fleet window gate passes all 48 cases (interface WASM
  `140c5e668ed494d69ca9423a1359a0bd4c0c75da7d63834ca94e534e6cd79833`).
- The Status window gate passes.
- The release build ran 60 s at Very Slow and 60 s at Fast:

| Speed | Frames | Median | 99th percentile | Longest |
|---|---|---|---|---|
| Very Slow | 3599 | 0.5 ms | 0.8 ms | 24.8 ms |
| Fast | 3601 | 0.5 ms | 1.7 ms | 4.8 ms |

- The performance run also checked:
  - no frame took over 50 ms;
  - the console has 152 `[ai]` lines and 0 errors;
  - 4 requests, all HTTP 200.
- `cargo mutants --in-diff` (`rebellion-core`, `rebellion-render`):
  23 mutants, 22 caught, 1 unviable.
- `provenance-scan check`: 85 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
