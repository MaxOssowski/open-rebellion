# An unpopulated system its last regiment left holds no side

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty").

## The original

- `FUN_0050b610` fills the holder bits of a system's control word
  (`system-control.md`):
  - an unpopulated system's support side is 3, no side;
  - its holder is the regiments' side, else the support side.
- `FUN_00508660` reruns it when a regiment joins or leaves the system.
- So an unpopulated system whose last regiment leaves holds no side.

## What the port did before

- The economy's daily run skipped unpopulated systems entirely.
- The simulation's step 3b and the app's `tactical_flow::occupy` only
  look at systems with regiments.
- So an unpopulated system kept its holder after its regiments left.
  `economy-cadence.md` recorded the gap ("port: the economy loop still
  skips unpopulated systems").

## Port

- The economy's run now resolves the holder of an unpopulated,
  undestroyed system too (`resolve_system_control`, which already gives
  it no support side). It raises `ControlResolved` on a change.
- port: the other derived fields still run only for populated systems.

## Tests

- New: `an_unpopulated_system_its_last_regiment_left_holds_no_side`.
  - A held system with no regiment falls to no side; it fails without
    the change (`fail-without.txt`).
  - One side's regiment alone holds an unheld system, for each side.
- `cargo test --workspace`: 2508 passed, 0 failed.
- `cargo mutants --in-diff`: 10 mutants, 9 caught, 1 unviable.

## Gates (`f35`)

- The 24 dual-AI campaigns (1500 days) and the seed-42 golden are
  identical: no campaign leaves an unpopulated system held without
  regiments.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `5218ae45ba66fd59cce372bbed18fccf993852a4cba1fad7e0ba662758cada8a`).
- The Fleet window suite passes 54 cases; the Status window gate passes.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.

Hashes of the committed files are in `SHA256SUMS`.
