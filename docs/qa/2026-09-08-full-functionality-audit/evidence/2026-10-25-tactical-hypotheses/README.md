# The tactical withdrawal hypotheses settled from the source (P24)

Date: 2026-10-25.

The withdrawal work of 2026-10-23 left four labelled hypotheses. Three are
now settled from `REBEXE.EXE` (SHA-256 `b3fe3997...ed6ab`).

1. **Jump sound variants.** `FUN_005bae60` fills the `+0x50` variant-count
   table:

   | Events | Variants |
   |---|---|
   | default | 1 |
   | 0..2 | 4 |
   | 3..5 | 2 |
   | 6..8 | 1 |
   | 9..0x13 | 3 |
   | 0x14..0x1f | 1 |

   The jump events 0x1d and 0x1f therefore have one variant each, as the
   port assumed; the hypothesis is now a source read. All port events match
   the table (`original_audio_variant_count`). A0 TAC-01-C012's recording
   has the turbolaser event 0x0e in variants 0, 1 and 2.
2. **Jump factor.** `FUN_005a6b40` picks its factor by
   `(vtable +0x24 + id +0x3b) & 3`. Vtable `+0x24` is the unit's tactical
   ordinal, the index `FUN_005ab0e0` uses for the `+0xa50` resource table.
   The port used only the list index. It now adds the ordinal
   (`original_jump_factor_index`). The object id is still the port's list
   index (`hyp:`).
3. **A task force's target.** `FUN_005d2190` returns the first node of the
   group's member list (`+0x10`, then the node's `+0x10`), and
   `FUN_005c7f70` reads that unit's target. The port took the first member
   that had a target. It now takes the first member's target only, even
   when that is none (`task_force_target`).
4. **Open: fighters' flight while withdrawing.** It is still the port's
   straight flight at the group's sublight rating (`FUN_005c6610`,
   `FUN_005cc300` not yet read in full).

## Gates

| Gate | Result |
|---|---|
| Workspace tests | 2766 passed, 0 failed |
| Provenance scan | 0 uncited items |
| Browser tactical matrix | 144 of 144 passed |
| Replay equivalence | passed |
| Release WASM SHA-256 | `9e32f683f893f1a448e1ca0ebc9ef787dfcbbfc77d6b2530b2f135f17cba5200` |
| `cargo mutants` | 18 of 18 viable mutants caught |
