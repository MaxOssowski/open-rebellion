# The tactical Death Star browser cases run again (F-017)

Date: 2026-10-20 (overnight run, "Gwiazda Śmierci").

## What was wrong

Every tactical `run.mjs --battle` case failed at fixture setup with "fixture
system is not empty and intact":

- `tactical_test_fixture.rs` looked for SYSTEMSD index 100 or 101;
- the default galaxy loads 100 systems, indices 120..289, so neither
  exists.

## Changes

- The fixture takes the first and second empty, intact systems in SYSTEMSD
  order: the Alliance's case gets the first, the Empire's the second. They
  remain two real picture and palette identities.
- In a Death Star battle the Imperial fleet brings no fighter group.
  `FUN_005ad930` (`a136866e`) enables Attack Death Star only while fewer
  Imperial than Alliance groups are on the field.
- `run.mjs` expects that composition:
  - one Imperial active object and three participants;
  - no Imperial fighter group;
  - no fighter scene once a failed run has destroyed every Alliance group.
- The Death Star entry voice check now polls the console for up to 3 s. The
  line can trail the commit by a frame or two, and was found in the saved
  console when the single check had failed.
- The tactical 3D corpus had never been staged on this machine; resource
  5010 and the meshes were missing. It was staged locally (ignored data,
  not committed):
  - `go run ./tools/stage-ui-assets --tactical-3d-only --source <game>`;
  - then `--tactical-3d-convert`;
  - verified: 87 meshes, 397 type-303 resources.

## Result

All 16 Death Star cases pass on Chromium 154
(`tactical-death-star-cases.json`): presentation, the laser journey, and
trench-run success and failure, each for both sides in both viewports.

- The Alliance's Attack Death Star is enabled with one Alliance group
  against none, and commits order 6.
- The Empire's is disabled.

The other tactical scenarios were not rerun here.

## Gates (`f14`, shared with the destroyed-system art)

- 24 campaigns are identical to the stage before.
- The replay equivalence check passes; seed-42 is unchanged.
- The Fleet window suite passes 50 cases; the Status window gate passes.
- `cargo test --workspace`: 2490 passed, 0 failed; the app's fixture
  tests: 81 passed.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- `provenance-scan check`: 53 uncited items, unchanged.

Hashes of the committed files are in `SHA256SUMS`.
