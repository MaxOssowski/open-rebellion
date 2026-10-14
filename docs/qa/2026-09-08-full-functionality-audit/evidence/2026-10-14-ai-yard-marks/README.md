# The AI's yard marks in the save (F-036)

Date: 2026-10-14.

A system record keeps the bits `+0x28 & 0x3e00000` across refreshes
(`FUN_00433e40` zeroes the rest). They are the production modules' yard
marks (`ghidra/notes/ai-galaxy-records.md`, `ai-mission-planning.md`):

| Bit | Mark | Set by |
|---|---|---|
| `0x200000` | shipyard site | modules 7, `0xa`, `0xb` (`FUN_004c8d20`, `FUN_004cacc0`) |
| `0x400000` | training facility site | modules 5, 6 (`FUN_004d0860`) |
| `0x800000` | construction yard for mines and refineries | module 1 (`FUN_004daf10`, `FUN_004db510`; dropped by `FUN_004daca0`) |
| `0x1000000` | construction yard for facilities | module 2 (`FUN_004e2120`) |
| `0x2000000` | construction yard for the facility balance | module 3 (`FUN_004d6d30`) |

## Port (save v40)

- `AIState::yard_marks`: each system's marks, with `yard_marks`,
  `set_yard_marks` (bits outside `0x3e00000` ignored) and
  `clear_yard_marks` (a system left with none is dropped). The constants
  are `YARD_MARKS` and `MARK_*`.
- `Galaxy::build` takes this map, so the records keep the marks the
  modules set.
- No module sets marks yet, so play is unchanged.

## Golden

The save layout changed, so the seed-42 golden was regenerated for this
named cause (save v40). Only the fingerprints changed; the replay's events
did not:

- initial `v1:e271a795cd483ebe` -> `v1:e187411365b9f348`;
- final `v1:33233aedba63db5c` -> `v1:6685c6f382ede031`.

Before the regeneration, `replay_manifest -- --ignored` failed on the old
fingerprints. After it, both tests pass.
`scripts/check-replay-equivalence.py --skip-build --json`
(`replay-equivalence.json`) passes on nine checkpoints, with 0 browser
errors. WASM SHA-256
`02f17015b642c567040d5556c815a9b431f3e2ec4deab1959cf2fe82add60642`.

## Gates

- `cargo test --workspace --no-fail-fast`: all pass. New tests:
  - `yard_marks_stay_until_a_module_clears_them`
  - `the_five_yard_marks_fill_the_kept_mask`
  - the save round trip keeps the marks (v40).
- `cargo mutants --in-diff`: 13 mutants, 13 caught.
- `provenance-scan check`: 103 uncited items, all within the baseline.
- Clippy: no findings in the changed files. rustfmt: no findings in the
  changed lines.

Hashes are in `SHA256SUMS`.
