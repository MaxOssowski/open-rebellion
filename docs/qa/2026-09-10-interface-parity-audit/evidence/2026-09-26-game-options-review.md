# Game Options review checkpoint

Scope: PR #14, PRE-03 / RE-OPT-01, and P31. This is implementation evidence,
not strict original-game acceptance. The
[structured verification summary](2026-09-26-game-options-review.json) records
portable results and artifact hashes without redistributing original assets.

## Resolved review findings

- The branch is rebased onto current `main`, preserving the existing tactical
  renderer and five display options.
- The authentic bitmap surface exposes six save rows. F8/Ctrl+L and campaign
  F9/Ctrl+S retain access to all ten compatibility slots.
- Overwrite, load, delete, restart, and exit mutations wait for confirmation.
  Escape cancels without changing campaign or save data.
- Payload or metadata keys from browser save versions 9 through 14 count as
  occupied, including corrupt and partial saves.
- Original indexed masks gate bitmap press and release. Missing confirmation
  artwork produces visible bounded fallback controls, while release packaging
  requires REBDLOG 10623 through 10627.
- Context predicates now control both input and artwork. Save and Restart are
  available only from the command center; tactical Save, Load, Delete, Restart,
  and display toggles remain unavailable.
- Native Exit stops music before termination. Browser Exit stops music, replaces
  the stopped canvas with an accessible restart screen, and reloads cleanly.
- Accepted Load must verify its saved fingerprint, restore the live campaign,
  return to Galaxy, and preserve the selected faction.

## Verification

The following gates pass on the rebased branch:

- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `go test ./tools/stage-ui-assets`
- `python3 -m unittest scripts/test_build_runtime_pack.py`
- `npm run test:unit` in `tools/interface-parity`
- `./scripts/build-wasm.sh` with an owned installation staged locally
- `node tools/interface-parity/game-options-smoke.mjs` in muted Chromium

The browser journey passes for both factions and covers save naming, overwrite
cancel, load cancel, fingerprint-verified restoration, slot 10 load/delete,
delete cancel/accept, return/re-entry, corrupt saves, legacy metadata-only
occupancy, exit/restart, and page-error monitoring. The runtime pack contains 52
game files and 2,326 UI bitmaps.

## Acceptance boundary

Strict PRE-03 remains open pending lossless original-executable captures, exact
text and slider comparison, context-specific return artwork, settings
persistence, Holocube behavior, saved-slot faction badges, and inspected release
artifacts. Automated smoke and implementation evidence do not promote those
cells.
