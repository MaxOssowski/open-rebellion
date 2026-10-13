# Overnight log, session B (2026-10-14)

Two sessions were started with the same overnight prompt in the same checkout.
This is the second one's log. To keep the two from overwriting each other,
this session works in its own worktree (`.claude/worktrees/overnight-b`) on
branch `overnight/2026-10-14-b`, based on the snapshot commit `03cb5d1d`, and
takes backlog items 7 to 12. The other session keeps the main checkout, branch
`overnight/2026-10-14`, items 1 to 6, and `OVERNIGHT_LOG.md`. Neither takes
item 13. The branches need merging in the morning; they touch different files
except `OVERNIGHT_BACKLOG.md`.

## Decisions

- **Branch before snapshot.** The snapshot commit `03cb5d1d` was made on
  `overnight/2026-10-14`, not `main`, because the prompt also says never to
  modify `main`.
- **Prompt over CLAUDE.md where they conflict.** No pushes, no work on `main`.
- **`cargo fmt` is checked per touched file, not run.** The workspace has 105
  formatting diffs at baseline, so a workspace `cargo fmt` would rewrite
  unrelated code. A touched file must not gain diffs.
- **Clippy is compared against the baseline warning list**, since the baseline
  is not clean.
- **WASM gate per feature is the `cargo build --target wasm32-unknown-unknown
  -p rebellion-app --release` step of `scripts/build-wasm.sh`.** The full
  script also restages assets and overwrites the tracked
  `web/open-rebellion.wasm`; it ran once for the baseline in the main checkout.
- **Seed-42 golden.** It already fails at the baseline (below), so no feature
  here can show "golden unchanged" or regenerate it for a named cause.

## Baseline at `03cb5d1d`

| Gate | Result |
|---|---|
| `cargo build --workspace` | pass |
| `cargo test --workspace` | pass: core 734, render 733 (3 ignored), data 125 (13 ignored), app 72, provenance-scan 20 |
| `cargo clippy --workspace --all-targets` | no errors; existing warnings: core 5 (8 with tests), render 12 (24 with tests), app 6 |
| `cargo fmt --all -- --check` | fails: 105 existing diffs |
| `cargo run -q -p provenance-scan -- check` | pass, 110 uncited items within the baseline |
| `./scripts/build-wasm.sh` | pass, 6.5M (no `wasm-opt` on this machine, so unoptimized) |
| `cargo test -p rebellion-data -- --include-ignored` (needs `data/base`) | 4 existing failures: `seeds::tests::fixed_fleet_tables_only_target_special_systems`, `replay_manifest::original_campaign_replay_matches_after_save_reload`, `telemetry_coverage_all_sys_constants_emit`, and the ignored `PerceptionIntegrator` doctest |

Not available here: `wasm-opt`, `agent-browser`, `cua-driver`. No browser or
native GUI acceptance was run for anything below.

## Entries

### Item 12: `AdvanceTicks` runs real days — done

- Before: the command palette's "advance N ticks" added N to `clock.tick`.
  The skipped days never ran through the simulation, despite the comment
  saying they would catch up.
- Now: `GameClock::queue_days(n)` records the days owed and each
  `GameClock::advance` call completes one of them, at any speed and through a
  pause. The frame loop already simulates whatever `advance` returns, so each
  queued day takes the same path as a day of normal play. The queue is
  `#[serde(skip)]`: the save format is unchanged.
- One day per frame is deliberate. Several systems act once per batch on the
  last tick of the slice they are given, so handing them 1000 days at once
  would not equal 1000 days of play. "Advance 1000" now takes about 1000
  frames.
- Tests: three new clock tests; they fail without the change (the methods do
  not exist). `cargo mutants` on `tick.rs`: 53 caught, 3 unviable, 1 missed,
  and the missed one is the existing stop-day guard at `tick.rs:239`, not this
  change.
- Not verified: the one-line call in `main.rs` (`apply_panel_action`) has no
  test that reaches it, and I could not drive the GUI. Worth one manual check:
  pause, run "advance 10 ticks", watch ten days of events arrive. The roadmap
  box under M2 is left unticked for that reason.

### Item 11: F-005 "victory modal is unreachable" re-check — done (docs only)

- Finding: the audit's evidence is stale. The app's one victory route (the
  `VictorySystem::check` call closing the tick block in `main.rs`) sets
  `GameMode::VictoryModal`; the clock only runs in `GameMode::Galaxy`, so play
  freezes; Continue returns to the main menu. Clippy no longer reports the
  variant as never constructed.
- Changed: F-005 in `audit-report.md` and `audit-report.json` moves from
  "confirmed" to "partially remediated" with the open gaps listed; the M2
  roadmap line notes the same. No code changed.
- This is a code reading, not an acceptance pass. I did not win a game in the
  browser. Open gaps are backlog sub-items 11a to 11d.

### Item 10: research gate for troop and facility classes — done

- Finding: the backlog premise was half stale. `BuildableClass` already loads
  `research_order` for regiments, special forces and all facility files, and
  the player's Build Selection lists already gate on the Troop and Facility
  trees, with an existing test covering both sides.
- The real gap was the AI: `find_troop_class`, `find_defense_facility_class`
  and `find_manufacturing_facility_class` copied the class of any unit the
  side owned, researched or not. They now skip classes the side has not
  researched on that class's tree.
- Test: `the_ai_orders_no_regiment_or_facility_class_its_side_has_not_researched`
  runs both sides and checks the trees are independent and the other side's
  levels do not count. Shown failing with the gate stubbed to `true`.
  `cargo mutants` on the four functions: 12 caught, 3 unviable, 0 missed.
- ASSUMPTION (in code): a class missing from the catalog is not gated. Real
  data always has the catalog; this keeps catalog-less test worlds working.
- Caveat to verify: the research-order gate itself is a hypothesis for the
  player too (audit F-018: "No recovered code gates research"). This change
  makes the AI consistent with the player; it does not add evidence for the
  gate.
- Golden: the seed-42 replay test already fails at baseline, so I could not
  measure whether this moves it. AI production can differ once a side owns a
  unit of an unresearched class.
- F-018 in `audit-report.md` updated; the JSON entry's status
  (`partially_remediated`) still holds, so it is unchanged.
