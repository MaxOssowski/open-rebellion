# Game Options review checkpoint — 2026-09-26

PR #14, PRE-03 / RE-OPT-01, P31, local issue `orlocal-ddb`.
Rebased onto upstream `ec5d6a52`; the PR remains a draft. This is an
implementation checkpoint. No original-parity acceptance cell is promoted.
The [durable verification summary](2026-09-26-game-options-review.json) contains
commands, results, artifact hashes and restored campaign fingerprints without
redistributing original game assets.

## Review disposition

| Review item | Result |
|---|---|
| Rebase | Additive save naming, occupancy and confirmation work ported into upstream P58-B12. Upstream geometry, sliders, five display flags and context transitions retained. |
| Save capacity | Six original bitmap rows; all ten compatibility slots available through F8 / Ctrl+L and campaign F9 / Ctrl+S. Slot 10 load/delete is exercised in both factions. |
| Tactical visibility | `tactical_view.rs` and `tactical_assets.rs` match upstream. Only decorative impacts are suppressed; Death Star, tractor and gravity rendering remain intact. |
| Missing dialogs | Packaging validates all five indexed REBDLOG 10623–10627 bitmaps. The runtime uses visible standard confirmation buttons if any required image or hit mask is unavailable. |
| Exit | Menu Quit, main-menu Escape and confirmed Options Exit share cleanup. Native music stop runs before loop exit. Browser exit hides the canvas and offers Restart game. |
| Hit masks | Original bitmap masks gate press and release. Tests reject transparent corners and exact outer edges. |
| Accepted load | Browser assertions require the verified load log and a second fingerprint computed from the restored live campaign after transition to Galaxy. |
| Audit evidence | P24, including B07–B20, is byte-for-byte equivalent as parsed JSON to upstream. P31 describes the implemented window; this report and its portable summary are indexed. |
| Formatting | `cargo fmt --all -- --check` passes. |

The save format stays upstream v14. Occupancy checks include payload and metadata
keys for v14 through v9, so partial and corrupt older saves cannot silently bypass
confirmation. Native corrupt slot 10 and browser v13 metadata-only slots are
covered. These changes do not migrate or erase saves.

## Verification

Native Cargo commands used sanitized `PATH=/usr/bin:/bin:/home/will/.cargo/bin`,
`CARGO_TARGET_DIR=/data/tmp/game-options-target`, and the existing ALSA link shim
`LIBRARY_PATH=/data/tmp/orlocal-l7g-1-29-alsa.sjSyRO`.
Contributor-owned fixtures were staged only in ignored local paths.

- `cargo test --workspace`: 760 passed, zero failures; 34 ignored.
- `cargo fmt --all -- --check` and `git diff --check`: pass.
- `python3 scripts/test_build_runtime_pack.py`: five passed. Every missing
  confirmation resource, truncation and unsupported true-color format is rejected.
- `OPEN_REBELLION_TEST_SOURCE=<owned-install> go test ./tools/stage-ui-assets`:
  pass, including real REBDLOG extraction and dimensions. Upstream stages its
  23 numeric resources and skips the unused named corner.
- `bash scripts/package-web.sh options-review-2026-09-26`: pass. The host lacked
  `zip`; an unpacked distribution package supplied it on a temporary PATH.
- `node scripts/validate-interface-parity-ledgers.mjs`: pass, 564 required cells.
- `node scripts/validate-tactical-lookup.mjs`: pass, 29 ships, eight fighters,
  285 voices. The initial unstaged run failed; generating the two DAT JSON files
  and staging TEXTSTRA resolved the fixture gap.
- Native GUI: Xvfb, original Options → Exit → Yes; process exits 0 and logs
  `[quit] audio_stopped=true cleanup=complete`. This host's audio device fails,
  so audible playback/cessation is not established by this run.
- Independent code review: quit cleanup bypass and unsupported hit-mask format
  found and fixed; no remaining critical or important runtime findings.

New confirmation, tactical-context, missing-dialog, bitmap-edge and packaging
regressions were observed failing before their fixes. Scoped mutation testing:

```sh
cargo mutants -f crates/rebellion-render/src/game_options.rs \
  --re 'GameOptionsState::(enabled|request|confirm)|masked_control_at' \
  --timeout 180 -- --lib game_options::tests
```

Final result: 28 mutations, 24 caught, three unviable (`Default` is not implemented
for the generated replacement enums), one survivor. The survivor deletes
`DrawTextureParams.dest_size` in the unchanged upstream drawing helper; headless
state tests do not exercise graphics scaling. It is recorded as a visual coverage
limit, not counted as killed. The first run exposed six save-guard survivors;
added valid/invalid-slot, overwrite, name and modal tests catch all six.

## Browser journeys

The packaged release build runs in headless Chromium at 1280×960 with
`--mute-audio` and in-game music disabled. Both factions exercise save,
overwrite cancel, load cancel, verified load/restore, slot 10 load/delete,
delete cancel/accept, return/re-entry, corrupt payload handling, v13 metadata-only
overwrite cancel, and browser exit/restart. Successful results and full restored
fingerprints are in the portable summary. The package contains original music,
SFX and voice resources, but no tactical meshes; this is not tactical visual acceptance.

The missing-dialog variant removes precisely REBDLOG 10623 from the HTTP runtime
pack response. It exercises the same journeys through the visible fallback.
An initial parallel run timed out at Exit and is retained in private evidence;
the isolated rerun uses a longer pointer hold and captures the exit confirmation.

```sh
CHROMIUM_PATH=<chromium> OPTIONS_TEST_URL=http://127.0.0.1:8774 \
  OPTIONS_TEST_OUTPUT=<private-output> \
  node tools/interface-parity/game-options-smoke.mjs
# Repeat with OPTIONS_TEST_MISSING_DIALOG=1 for the fallback journey.
```

Private screenshots and logs are retained under
`/data/projects/open-rebellion/agent-work/game-options-2026-09-26/`; hashes are
recorded in the portable summary. Original artwork and runtime packs are not
committed. The earlier [September 24 report](2026-09-24-game-options.md) is
retained as historical evidence and explicitly superseded.

## Acceptance boundary

Exact original pixels/text, slider/audio behavior, context-specific artwork,
saved-slot faction badges, settings persistence, native deletion gesture and
complete tactical visual behavior remain separate acceptance work.
Original-executable A0 captures and independent browser acceptance are still
required. The prescribed browser orchestrator is unavailable in this session;
automated smoke and code review do not substitute for that gate.
