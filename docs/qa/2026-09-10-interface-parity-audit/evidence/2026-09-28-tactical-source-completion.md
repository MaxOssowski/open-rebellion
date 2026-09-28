# P58-B22 tactical source completion

P58-B22 closes the bounded practical space-battle launcher implementation. It
does not accept the original tactical interface. All 106 strict cells remain
open until owned lossless native captures pass the complete comparison gate.

## Implementation result

- `FUN_0061a310` and `FUN_005a8a70` now drive one ordered, source-compatible
  tactical random stream instead of per-effect hashes.
- Campaign and fixture entry provide one deterministic battle seed. Exact
  whole-process continuity remains open because the wider simulation does not
  yet use the executable's global random state.
- `FUN_00501510` and `FUN_005015a0` now have bounded shield and weapon nibble
  setters; combat entry consumes persisted allocations and source defaults.
- Waypoint arrival, exhausted orders, target destruction, capital loss, and
  fighter loss queue their exact faction and group voice families.
- `FUN_005cfec0`, `FUN_005d04e0`, `FUN_005d03f0`, and `FUN_005d0430` now drive
  the 120-second trench-run timer, maneuver damage, commander-rating result,
  ordered nine-slot chatter, source casualty rules, and MDATA 201/202 route.
- Campaign battles use `FUN_005ad7e0`'s unassigned commander fallback of 1.
  Persisting the original tactical commander slot is a separate strategic gap.

## Verification

The durable machine-readable results are in the
[artifact summary](p58-b22-tactical-source-completion/summary.json). The final
fresh-process muted run passed 144 of 144 cases with four-request startup, no
timeouts, and all browser processes closed. Independent medium-effort browser
review inspected 88 full-resolution screenshots across both factions and both
viewports and found no P0 or P1 visual blocker; its exact scope and observations
are in the [acceptance record](p58-b22-tactical-source-completion/browser-acceptance.json).
Raw browser screenshots, console and network logs, runtime packs, and owned
game resources remain outside Git.

| Gate | Result |
|---|---|
| `cargo test --workspace` | pass |
| Focused tactical tests | 115 renderer passed, 3 ignored; 13 app passed; core power-allocation tests passed |
| `npm run check` | 106/106 A1 mappings; fixture and proprietary-A0 exclusions pass |
| `npm run test:unit` | 26 passed |
| `cargo run -q -p provenance-scan -- check` | pass; no baseline growth |
| WASM build and development package | pass |
| Fresh muted browser matrix | 144/144 passed; four requests each; zero timeouts; all processes closed |
| Independent live-browser review | pass; 88 full-resolution images; no P0/P1 visual blocker |

## Acceptance boundary

- Bounded practical launcher implementation: complete.
- Deterministic A1 mapping: 106 of 106 cells runnable.
- Strict A0 coverage and acceptance: 0 of 106.
- Still open: owned lossless A0 comparison, exact native beam pixels, audible
  native playback and mixing, whole-process RNG continuity, strategic
  commander binding, exact planet framing, and rare warning/ejection callers.
