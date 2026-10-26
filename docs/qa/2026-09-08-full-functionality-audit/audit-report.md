---
title: "Open Rebellion Full Functionality Audit"
description: "Repository status, verified evidence, release blockers, and feature-by-feature acceptance plan"
category: qa
created: 2026-09-08
updated: 2026-09-28
commit: fc25634be905839dfa6fb477d5fff0faa49d8ae9
tags: [qa, audit, functionality, parity, bitmap, wasm, astra, fable]
---

# Open Rebellion Full Functionality Audit

## Executive conclusion

Open Rebellion is feature-rich and has substantial unit-test coverage, but it
is not yet demonstrably 100% functional. The repository is at post-implementation
integration and acceptance closeout. Several user-facing paths are incomplete,
the long-running campaign still fails combat-distribution and full-loop gates,
and the existing CI pipeline does not establish browser or visual correctness.
The [original-interface sister audit](../2026-09-10-interface-parity-audit/)
additionally confirms that the in-campaign strategy, object, report, tactical,
and multiplayer surfaces do not yet match the original game.

This audit was performed against:

- Commit: `fc25634be905839dfa6fb477d5fff0faa49d8ae9`
- Branch: `main`, synchronized with `origin/main`
- Audit date: 2026-09-08
- Platform: macOS, Apple Silicon
- Independent reviews:
  - GPT-6-Astra through `codex-orchestrator`, medium effort, read-only
    researcher profile
  - Claude Fable 5.1 through the verified Fable CLI lane, repository-read-only
    audit

No tracked source files were modified during the audit. Four pre-existing
evaluation files were untracked: `EVALUATE.md`, `edd.config.json`, `program.md`,
and `run-eval.sh`.

## Where work left off

PR #11 [corrects the cockpit command routing](../2026-09-10-interface-parity-audit/evidence/2026-09-14-cockpit-routing-correction.md): `0x131` and F7 identify Encyclopedia, `0x132` opens and closes GID, and the side-globe `0x133` plus F1 identify Game Options for both factions. Encyclopedia deliberately fails closed. The [2026-09-26 Game Options review checkpoint](../2026-09-10-interface-parity-audit/evidence/2026-09-26-game-options-review.md) replaces the options placeholder with the shared bitmap surface; strict original-evidence acceptance remains open.

The current continuation has verified the authentic bitmap main menu,
save/load/delete, deterministic browser packaging, save continuation, exact
native/WASM replay, fleet redispatch protection, and authoritative fleet
position with compatible-arrival consolidation. Ordinary player fleet dispatch
passes both-faction browser acceptance through the shared departure helper.
F-007D resolves all opposing fleets at system scope and closes the permanent
five-tick combat backlog. The first F-007E checkpoint also closes production
ownership, friendly cycling, transit, repair-start, and troop-class defects.
The second F-007E checkpoint adds save-v13 troop transport, landing, continuing
ground combat, occupation, and provisional character capture. Player troop
selection and dispatch passed in the following checkpoint (reopened 2026-10-02:
its fleet-panel picker lost its caller in 23d15da and was deleted in F-007C
phase 6a; restored 2026-10-04 through the original order, a regiment's Move
released on the Fleet window). The campaign-history
review then reopened the victory and capture model. The next checkpoint now
distinguishes Alliance control of Coruscant from Imperial destruction and
occupation of the mobile Alliance HQ, preserves Standard leader conjunctions,
makes Death Star loss nonterminal, removes the uncited grace period, and covers
every current bombardment entry path. M1 continues with capture/evasion,
faction liveness, the wider campaign loop, target acquisition, and five-seed
cross-runtime proof.

The current campaign screenshots also exposed a separate visual truth. The
original shuttle main menu remains a passing surface, but the strategy screen
still uses synthetic map primitives, incomplete system-window contents,
approximate rail thumbnails, and incomplete advisor-shell integration. The
first original sector and system shells now replace the sidebar. The sister
audit inventories 43 surface families,
retains 370 original references, and makes UIP-T01 Strategic Cockpit Truth the
highest-priority presentation tranche. Earlier scoped browser checks retain their
behavioral and loading evidence; they are not evidence of original interface
identity.

The repository's records describe different scopes and are not a unified
acceptance record:

| Record | Actual status |
|--------|---------------|
| `README.md` | Explicitly distinguishes implementation estimates from final release acceptance and links to this audit. |
| `archive/progress.archived-2026-04-07-native-video.json` | Archived record of the April 7 native-video task; it reports lint as false and is superseded by this audit. |
| April 12 Tammuz plan | Described as completed elsewhere, but its functional and quality acceptance checkboxes remain open. |
| April 6 test-infrastructure plan | Baseline investigation is complete; implementation milestones remain open. |
| March 24 QA inventory | Historical v0.15 browser observations, not acceptance evidence for the current build. |

## Reproduced audit baseline (2026-09-08)

| Gate | Result | Evidence |
|------|--------|----------|
| Workspace tests | PASS | 465 passed, 0 failed, 17 ignored. |
| App/playtest automated tests | GAP | Neither binary has direct test coverage. |
| Native BMP decode | PASS | All 2,231 staged BMP files decoded with ImageMagick. |
| HD PNG decode | PASS | All 234 PNG files decoded with ImageMagick. |
| Production HD replacements | PARTIAL | 74 production DLL PNGs: Common 53, Gokres 13, Strategy 7, Tactical 1. |
| Raw WASM release compile | PASS WITH WARNINGS | `wasm32-unknown-unknown` build completed with 30 warnings. |
| Formatting | FAIL | Formatting drift was detected across approximately 83 files. |
| Strict clippy | FAIL | At least 98 errors in `rebellion-core`, with additional workspace findings. |
| Packaged WASM browser boot | NOT RUN | The raw compilation result does not exercise `scripts/build-wasm.sh` or a browser. |
| Visual bitmap correctness | NOT PROVEN | Decode success establishes file integrity, not in-game display or identity. |

The local `cc` command is a tmux/Claude launcher rather than the system C
compiler. Verification commands must use a sanitized `PATH` so Rust links with
Apple clang:

```bash
env PATH=/Users/tomdimino/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin \
  cargo test --workspace
```

## Campaign evaluation

A fresh 5,000-tick dual-AI campaign was run with seed 42.

| Metric | Observed |
|--------|----------|
| Quality score | 0.2851 |
| Parity result | FAIL: 19 pass, 5 fail, 1 skip |
| Victory | None by tick 5,000 |
| Total events | 2,001,963 |
| Initial fleets | 5 |
| Fleets in transit at end | 980 |
| AI attack orders | 306,012 |
| Empire AI actions | 326,092 |
| Alliance AI actions | 6,332 |
| Space/ground/bombardment events | 655 / 2 / 1 |
| Battles at Yavin | 655 of 658 combat events |

The run also reported missing `TroopClassDef` entries. Parity failures covered
the original AI interval, Death Star construction/fire/shield events, and
research completion events; victory evidence was skipped because no victory
occurred.

This is release-blocking behavior even though the evaluator did not classify
the run as formally degenerate.

F-007A reran the same seed after protecting active fleet orders. Accepted
moves fell to 153,462 against 152,513 arrivals, and attack orders fell to
78,946. The remaining 950-fleet arena and 949 in-transit fleets confirm that
production, arrival merging, and combat backlog are separate open causes
([evidence](evidence/2026-09-09-fleet-redispatch.md)).

F-007B then made transit orders authoritative, rebuilt orbit indexes before
production, and consolidated compatible arrivals. Across five 5,000-tick
seeds, final fleet arenas are no more than 1.4 times initial size and accepted
moves are no more than 1.002 times arrivals. Combat remains concentrated in at
most two systems and no seed reaches victory
([evidence](evidence/2026-09-09-fleet-position-consolidation.md)).

F-007D resolves every hostile task force at a system in one bounded engagement,
persists fighter losses, corrects fighter launch and shield handling, and stops
unchanged stalemates until system composition changes. Seed 42 falls from 603
space events, including 602 recurring Xyquine draws, to two decisive
engagements with no backlog
([evidence](evidence/2026-09-10-system-combat-resolution.md)).

The first F-007E checkpoint then gates AI production by system ownership,
removes friendly cycling and transit fan-in, retains HQ defense and surface
garrisons, resolves compound troop-class IDs, and persists repair episodes in
save v12. Five current runs finish with 0% transit, exactly one move per
arrival, at most 1.2× their initial fleets, and 0–5 repair starts
([evidence](evidence/2026-09-10-ai-campaign-logistics.md)).

The second F-007E checkpoint carries regiments within living ship capacity,
preserves cargo through transit and consolidation, lands after orbital control,
continues unresolved surface battles, and occupies systems. Its deterministic
character capture and occupation-based Imperial HQ result are verified current
behavior but not parity-correct. Save v13, all 567 workspace tests, the exact
native/WASM replay, five 5,000-tick transport runs, and Astra medium two-faction
bitmap/browser acceptance pass for the scoped transport path. Player troop
dispatch passed in the next checkpoint (reopened 2026-10-02 with F-007C;
restored 2026-10-04 through the Fleet window). The
following victory checkpoint
corrected the Imperial HQ, Death Star, Standard-conjunction, and timing rules;
capture/evasion, faction liveness, battle diagnostics, and five-seed
cross-runtime proof remain open
([transport evidence](evidence/2026-09-10-troop-transport-occupation.md);
[victory evidence](evidence/2026-09-10-victory-contract.md)).

## Confirmed findings

### F-001: Save, Load, and Delete UI wiring

- Severity: P0
- Status: remediated; browser and native GUI verified (2026-10-18)
- Evidence: the app now handles `PanelAction::SaveGame`, `LoadGame`, and
  `DeleteSave` against the full live campaign before the general action
  dispatcher. Main-menu Load Game opens the slot picker.
- Verified tranche: Astra-medium r1 passed 27/28 checks and found that an empty
  slot could be selected in load mode. The panel now disables empty load rows
  and verifies the selected slot is occupied before enabling Load. All 486
  workspace tests pass, including three focused panel regressions.
  Astra-medium r2 then passed 33/33 real-UI save/delete/reload assertions with
  intact bitmaps, complete v10 key removal, an unselectable empty slot, and zero
  strict browser/network/asset errors. See
  `evidence/2026-09-09-save-delete.md`.
- Acceptance: perform UI-driven save, restart, load, compare full campaign
  state, delete the slot, and verify disk/browser storage changes. Browser is
  verified; retain a native GUI restart smoke test in P31.
- 2026-10-18 (native): the release build saved a day-100 campaign to slot 2
  from the in-game panel (`saves/1.reb`), restarted, loaded it from the
  Game Options screen with the same fingerprint `v1:60b8fee5fbe796af`, and
  deleted it (the file is gone) (`evidence/2026-10-18-native-acceptance/`).

### F-002: Native faithful-HD acceptance remains open

- Severity: P1
- Status: profile and manifest foundation implemented; visual acceptance pending
- Evidence: original-parity rendering is now the default, uses nearest sampling,
  and ignores `data/hd`. Native faithful HD is opt-in and accepts only approved
  source-bound manifest records. Verified-byte decode, mutation fallback, and
  original-parity browser regression gates pass in
  `evidence/2026-09-10-faithful-hd-foundation.md`. Visual acceptance using a
  reviewed asset from the owned corpus remains open.
- Acceptance: approve a provenance-complete HD asset, render it in the explicit
  faithful-HD profile, revoke it, and prove original-BMP fallback on the same
  resource without changing original-parity output.

### F-003: Browser faithful-HD packs remain absent

- Severity: P1
- Status: confirmed
- Evidence: WASM now fails closed to original parity. The runtime pack does not
  yet carry approved HD manifest records or optional surface chunks.
- Acceptance: load a reviewed optional HD surface pack, verify its manifest and
  source identity, render its approved asset, and atomically fall back to the
  original family when approval or payload validation fails.

### F-004: WASM omits troop data prefetch

- Severity: P1
- Status: remediated
- Evidence: `TROOPSD.DAT` is included in the deterministic runtime pack. Native
  and browser integration resolve the original troop-class records without
  missing-class fallback diagnostics. This does not establish combat-formula
  parity, which remains P25.
- Acceptance: native and browser worlds report matching troop-class counts and
  no fallback diagnostics.

### F-005: Victory modal is unreachable in normal play

- Severity: P1
- Status: partially remediated; cutscene policy, loss wording, and browser pass remain
- Evidence: normal victory handling transitions directly to a cutscene or main
  menu; the compiler reports `VictoryModal` is never constructed.
- Re-check (2026-10-14, code reading only): the original evidence is stale.
  The one victory route in the app, the `VictorySystem::check` call at the
  end of the tick block in `crates/rebellion-app/src/main.rs`, now sets
  `GameMode::VictoryModal`, and the build no longer warns that the variant is
  never constructed. The clock only advances in `GameMode::Galaxy`, so the
  modal freezes play; its Continue button sets `GameMode::MainMenu`.
- Open:
  - No victory or defeat cutscene plays. `CutsceneKind` has `Intro`,
    `TrenchRun`, and `Story` only, and none chains into or out of the modal,
    so the "accepted cutscene policy" is neither decided nor implemented.
  - The modal words every result from the winner's side ("Alliance Victory!",
    "Imperial Victory!"); a player who lost sees no defeat wording.
  - The modal is port-owned egui, not an original result screen.
  - `PanelAction::ForceVictoryCheck` reports an outcome to the message log
    without resolving the game or opening the modal.
  - Whether Continue leaves a clean menu (no stale campaign state on the next
    New Game) is untested, and no browser or native pass has run.
- 2026-10-18: the game is decided by the victory conditions word
  (`FUN_005180e0`): `VictorySystem::check` reads its game-over and win
  bits. Where the binary differs from the manual and the 2026-09-10
  contract, the port follows the binary: no Imperial assault after the
  HQ is destroyed, a destroyed Coruscant counts for the Alliance, and any
  captor counts. In 24 campaigns of 1500 days no game ends, as before.
  Golden unchanged (`evidence/2026-10-18-victory-word/`).
- 2026-10-19: in 5 AI-against-AI campaigns of 5000 days at `39a75042`,
  two games end by the word: the Empire wins at day 4172 (Death Star on
  the Alliance HQ, Luke and Mon Mothma captured), and the Alliance wins
  at day 4744 (Vader and the Emperor captured, Coruscant taken). They
  are the port's first AI games to end
  (`evidence/2026-10-19-long-campaigns/`).
- Acceptance: every win and loss route reaches the result UI, freezes play,
  follows the accepted cutscene policy, and returns cleanly to the menu.

### F-006: Interactive combat does not establish core combat parity

- Severity: P1
- Status: confirmed design divergence
- Evidence: tactical space and interactive ground combat use simplified
  calculations separate from core auto-resolution.
  - Ground combat read 2026-10-16 (`ghidra/notes/planetary-assault.md`,
    module 8, stage 5e): the original fights on the ground only in the
    Planetary Assault event (`0x234`, `FUN_00563390` -> `FUN_0058c120`):
    the system's defenses fire at the landing regiments, each attacking
    regiment duels a drawn defender (`FUN_0055eb00`: a 0..10 draw plus
    TROOPSD attack less defense, the generals' leadership / G`0x201`; lost
    at or below G`0x203` 4, the defender at or above G`0x204` 6), duels
    cause collateral damage (G`0x205` 8%), and the winners land, at most
    6. The port's `CombatSystem::resolve_ground`, run whenever both sides
    have regiments at a system, rests on a misread (`ground-combat.md`'s
    family ranges) and has no counterpart.
  - Planetary Assault ported 2026-10-16 (module 8, stage 5f,
    `assault.rs`, `AIAction::Assault`); removed by the user's choice: the
    automatic landing on another side's planet, the automatic ground battle
    (`CombatSystem::resolve_ground`) and the app's interactive ground-combat
    screen. Regiments land only on their side's planet (`FUN_005073d0`);
    one side's regiments alone still take a planet (port). Until the kind 4
    plan is ported the AI orders no assault, so it no longer takes the
    other side's planets by landing (`evidence/2026-10-16-planetary-assault/`).
  - The player's Planetary Assault 2026-10-17: the object pop-up menu's
    `0x234` on a fleet or a fleet icon is enabled by
    `assault::order_enabled` (`FUN_00581ae0`). A click carries it out at
    once, with no confirmation (`FUN_00487cc0`'s default), through the
    AI's `BombardEnv::assault`. A browser gate orders it for both sides
    (`evidence/2026-10-17-assault-menu/`). The Assault Summary window is
    not ported.
  - The Assault Summary window read 2026-10-21
    (`ghidra/notes/assault-summary-window.md`): a report message (vtable
    `0x0065b870`) collects the system's and the attacking fleet's units,
    the destroyed and the captured, and writes its title and outcome
    (TEXTSTRA RCDATA `0x7090..0x7094`); it opens at once for the attacker
    (kind 7) and is filed as a message for the defender (kind 3). The
    window (`FUN_0044c410`) shares the Battle Results frame, rail, Close and
    Display page, with six categories. The same report serves the orbital
    bombardment (`0x7098..0x709a`).
  - The Assault Summary window ported 2026-10-22 (`assault_report.rs`,
    `assault_summary.rs`): the attacker's side sees it at once, the other
    side gets a Conflict message that opens it. A browser gate opens it
    for both sides, shows each side's forces and closes it with Escape
    (`evidence/2026-10-22-assault-summary/`). The orbital bombardment's
    summary followed the same day (`0x7098..0x709a`, pictures
    `0x2b9a..0x2b9c`; `evidence/2026-10-22-bombardment-summary/`).
  - The Battle Summary's forces page follows `FUN_0046f360` /
    `FUN_004714e0` since 2026-10-22 (`ghidra/notes/battle-summary-window.md`).
    It has Operational and Destroyed columns (Survivors, Captured and
    Killed for personnel), the category bar and the original empty-column
    lines. The port's Damaged column is gone
    (`evidence/2026-10-22-battle-summary-forces/`). The summary page,
    the report's TEXTSTRA sentences for the viewer and the Display page
    (message line, instruction, system and viewer-fleet buttons) followed
    the same day (`evidence/2026-10-22-battle-summary-page/`,
    `2026-10-22-battle-report-texts/`, `2026-10-22-battle-display-page/`).
    The Battle Alert's title and summary come from TEXTSTRA `0x7020..0x7025`
    by the system's side and its last arrival
    (`evidence/2026-10-22-battle-alert-texts/`). Its other tabs list each
    side's fleets, ships and units aboard, and the System Assets
    (`evidence/2026-10-22-battle-alert-lists/`). Its title and summary sit
    where the original paints them (`evidence/2026-10-22-battle-alert-layout/`),
    and Retreat is offered by the original's test: no enemy Interdictor, a
    unit that can move and somewhere to go
    (`evidence/2026-10-22-battle-alert-retreat/`). The alert's list and the
    Battle Summary's forces list scroll with the wheel; neither draws the
    original scroll bar yet (`evidence/2026-10-22-battle-alert-scroll/`,
    `2026-10-22-battle-summary-scroll/`). Both windows play the original
    score from REBEXE's track table: the alert MDATA.307 or 313, the summary
    308, 309, 314 or 315 (`evidence/2026-10-22-battle-window-score/`). The
    summary's scene is picked for the viewer, so a withdrawal shows the
    fleet beaten off (`evidence/2026-10-22-battle-summary-scene/`).
- Acceptance: deterministic fixtures prove that all entry paths use the
  accepted mechanics and apply identical permanent losses and conquest state.

### F-007: Long-running AI simulation exhibits runaway fleet behavior

- Severity: P0
- Status: partially remediated
- Evidence: seed 42 grew from 5 initial fleets to 980 fleets in transit,
  generated 306,012 attack orders, and produced no victory by tick 5,000.
  F-007A now rejects replacement orders, preserves elapsed travel, excludes
  transit and same-pass reservations from AI dispatch, and emits AI telemetry,
  movement messages, or departure audio only for accepted moves. Explicit
  player retries retain accurate in-transit feedback. Its 5,000-tick rerun
  produced 78,946 attack orders and a 1.006 accepted-move/arrival ratio.
  F-007B removes transit fleets from orbit indexes, reconciles stale saves,
  attaches production only to orbiting garrisons, preserves significant task
  forces, and merges anonymous same-faction arrivals deterministically
  (retired 2026-10-05: no original rule merges fleets the player did not
  join, `ghidra/notes/fleet-join-split.md`). The
  then-current five-seed runs passed the fleet-arena and move/arrival bounds.
  F-007C (reopened 2026-10-01: its only entry, the egui system context
  menu's Move Fleet Here, lost its caller in 23d15da, so the player cannot
  move a fleet; the original entry is the Move order; restored 2026-10-02
  through the Fleet pop-up menu's Move and Confirmed Move (0x201/0x202,
  FUN_0044f060's confirmation window) and the system window drag (0x214),
  with a 12-case two-sided browser gate and 8 of 8 hand-applied main-loop
  mutants caught; the blockade bit follows FUN_0050b8e0, under which the
  confirmation's blockade branch meets only a stale bit, as in the original;
  regiments load through the original Fleet window (type 4, FUN_004a2630),
  opened from the sector window's fleet icon, with an 8-case two-sided gate;
  the sector window's other quadrant icons open the System window, the
  System Defenses window (type 10, FUN_004a7790) and the Missions window
  (type 11, FUN_0049f130), with a 10-case two-sided gate; a regiment
  dragged out of the Fleet window's Troops tab unloads or boards at once in
  its own system and otherwise travels on its own at GNPRTB 1 (FUN_00556390,
  FUN_00556430; another side's populated destination is refused,
  FUN_0053d430), with a 16-case two-sided gate; fleets join and split on
  the Fleet window, a ship's or fleet's Move onto a fleet joining it
  (FUN_004ffc90, FUN_004feca0) and a ship's Create Fleet (0x270) making it a
  fleet of its own, with a 34-case two-sided gate, and F-007B's merge of
  compatible arrivals is retired as unsourced; the Fleet Finder (window
  type 0x15, FUN_00461960) opens from the cockpit or F3 and opens a chosen
  fleet's or ship's Sector and Fleet windows (FUN_00429440), with an 18-case
  two-sided gate. The Finder stays above a modeless Fleet window, and an
  empty-list double-click over a covered sector planet leaves the selected
  system unchanged. Remaining transport-limited native journeys stay open)
  wired
  ordinary player dispatch through the same validated departure helper, fixes
  map/context-menu click ordering and stale targets, and reports authoritative
  destinations and countdowns in the fleet panel. Astra medium passed both
  factions with four requests each, intact bitmaps, and zero runtime or asset
  errors. F-007D now resolves every hostile fleet in one bounded system
  engagement, writes fighter losses back, corrects fighter launch/shield
  handling, and suppresses unchanged five-tick stalemates. The 541-test
  workspace, exact native/WASM replay, and Astra bitmap regression gates pass.
  The first F-007E checkpoint now passes fleet-arena, transit, move/arrival,
  repair-start, and troop-class gates. All 551 workspace tests and the updated
  exact native/WASM replay pass. Astra medium also passed the two-faction bitmap
  flow, save-v12 reload, replay 9/9, exact music-control boundary, muted test
  output, and clean network/runtime gates. The second F-007E checkpoint adds the
  authoritative troop transport and occupation path, exact tactical survivor
  persistence, provisional enemy-character capture, and political occupation.
  Five 5,000-tick runs now emit landings, ground battles, control changes, and 5–6
  captures. All 567 workspace tests, save-v13 migration, exact replay, and Astra
  bitmap/browser gates pass for that scoped implementation. Source review then
  reopened Imperial HQ destruction, Death Star terminal outcomes,
  capture/evasion, and the 200-tick grace period. The next F-007E checkpoint
  (reopened 2026-10-02: no player path loads regiments onto a fleet since the
  chooser's picker was deleted; restored 2026-10-04 through the original
  regiment-to-fleet order, a regiment's Move released on the Fleet window)
  adds player regiment selection to the bitmap fleet chooser, enforces 0/0,
  2/2, and 3/3 live capacity in the browser, and carries selected cargo through
  dispatch and automatic landing. All 568 workspace tests and the packaged
  WASM pass; Astra observed 12/12 HTTP 200 responses and zero runtime or asset
  errors. The victory-contract checkpoint adds separate `HqCaptured`,
  `HqDestroyed`, and `DeathStarVictory` outcomes; preserves both Standard
  leader conjunctions; makes Death Star loss nonterminal; removes the
  unsupported 200-tick delay; and applies HQ destruction before contested,
  unopposed, manual, auto-resolved, or tactical occupation paths. Its 574-test
  workspace, two original-data fixtures, nine-checkpoint native/WASM replay,
  final package, and Astra source/browser acceptance pass. Terminal victory UI
  was not visually exercised. Alliance attacks and battle spread remain below M1 bounds, and the AI
  does not yet acquire every randomized Standard victory target. See
  `evidence/2026-09-10-victory-contract.md`.
- Historical baseline: the cited
  [Rebellion/Supremacy campaign-history reference](../../reference/campaign-history/)
  now defines the official campaign contract, human campaign chronology, and
  observed computer-opponent profile. The 50–400 battle range and geographic
  spread remain provisional engineering guards because no located source
  provides original AI-versus-AI telemetry or those numeric constants.
- Acceptance remains open: fleet, transit, and player troop-dispatch bounds
  pass. Faction liveness, target diversity, battle diagnostics, complete
  capture/evasion, the wider campaign loop, and five-seed cross-runtime proof
  do not yet pass. The source-backed victory rules pass dedicated fixtures;
  P30 remains open for end-to-end result-screen and campaign acceptance.

### P58-B11: Tactical trench-run launch and strategic persistence

- Status: partial A1 coverage; strict acceptance remains open
- Evidence: the production Attack Death Star control now proves that committed
  source order `6` enters the live trench-run lifecycle. Battle Results proves
  exact three-to-two capital and fighter persistence for both strategic fleets
  before destination routing.
- Matrix: 98 of 106 cells have deterministic A1 scenarios through 80 browser
  journeys and 18 snapshots. Eight scenarios and all 106 lossless A0 captures
  remain open. See
  `../2026-09-10-interface-parity-audit/evidence/2026-09-23-tactical-trench-persistence.md`.
  Qualitative behavior is adjudicated against the historical reference.

### P58-B12: Tactical Game Options and empty space

- Status: partial A1 coverage; strict acceptance remains open
- Evidence: the original COMMON Game Options surface now serves the shuttle,
  command center, and tactical Battle Options routes. Tactical display controls
  are disabled during a battle, and the empty-space fixture retains its
  starfield while omitting the planet.
- At this checkpoint, 100 of 106 cells had deterministic A1 scenarios through
  81 browser journeys and 19 snapshots. P58-B15 later completed the A1 mapping
  at 106 of 106; all 106 lossless A0 captures remain open. See
  `../2026-09-10-interface-parity-audit/evidence/2026-09-23-tactical-game-options-empty-space.md`.

### P58-B13: Tactical withdrawal confirmation

- Status: partial A1 coverage; strict acceptance remains open
- Evidence: the original TACTICAL 1310 panel, TEXTTACT title and prompt, and
  controls 1113 through 1116 now mediate withdrawal. Cancel closes without
  retreat; confirm starts withdrawal; the disabled repeat remains inert.
- Matrix: 101 of 106 cells have deterministic A1 scenarios through 82 browser
  journeys and 19 snapshots. Five scenarios and all 106 lossless A0 captures
  remain open. See
  `../2026-09-10-interface-parity-audit/evidence/2026-09-23-tactical-withdraw-confirmation.md`.

### P58-B14: Tactical detail and Escort

- Status: partial A1 coverage; strict acceptance remains open
- Evidence: TACTICAL 3368 supplies a complete destroyed presentation, panel
  1302 displays compact GOKRES assignments, and direct friendly right-click
  assigns source order-code 1 Escort with retained target, marker, follow,
  opportunity fire, and invalid-target cleanup.
- Matrix: 104 of 106 cells have deterministic A1 scenarios through 85 browser
  journeys and 19 snapshots. Battle Alert entry, tactical audio, and all 106
  lossless A0 captures remain open. See
  `../2026-09-10-interface-parity-audit/evidence/2026-09-23-tactical-detail-escort.md`.

### P58-B15: Tactical Battle Alert and audio

- Status: complete deterministic A1 mapping; strict acceptance remains open
- Evidence: the source-built faction Battle Alert now precedes tactical
  command, its four tabs and three command families use recovered resources,
  Take Command enters paused combat, MDATA 307 supplies the battle score, and
  TACTICAL WAVE 13054 is extracted, packaged, and routed. P58-B16 corrects the
  provisional label from ship destruction to torpedo impact and restores all
  eight weapon events and 22 WAVE variants at 13033–13054.
- Matrix: all 106 cells have deterministic A1 scenarios through 87 browser
  journeys and 19 snapshots. A0 coverage and strict acceptance remain 0/106;
  the strict gate was not run. See
  `../2026-09-10-interface-parity-audit/evidence/2026-09-24-tactical-battle-alert-audio.md`.

### P58-B16: Tactical weapon audio bank

- Status: complete within the deterministic A1 boundary; strict acceptance
  remains open.
- Evidence: production capital and fighter combat emit the recovered laser,
  turbolaser, ion, and torpedo fire/impact events. Exact TACTICAL WAVE
  13033–13054 resources are extracted and routed through native and browser
  backends.
- Verification: workspace tests, pack tests, harness checks, and the focused
  four-case muted browser journey pass. Exact original RNG sequencing, audible
  native comparison, and strict A0 acceptance remain open. See
  `../2026-09-10-interface-parity-audit/evidence/2026-09-24-tactical-weapon-audio.md`.

### P58-B17: Tactical command voice bank

- Status: complete within the deterministic A1 boundary; strict acceptance
  remains open.
- Evidence: the original faction battle-ready calls and all task-force/RGBY
  maneuver, attack, formation, and mission acknowledgements now select exact
  VOICEFXA/VOICEFXE events and WAVEs. Unsupported strategic aliases to these
  tactical recordings were removed.
- Verification: focused Rust tests, exact 90-file owned-DLL extraction, the
  packaged WASM build, four muted browser cases, and independent visual review
  pass. Remaining tactical voice families, audible native comparison, mixing,
  interruption, and strict A0 acceptance remain open. See
  `../2026-09-10-interface-parity-audit/evidence/2026-09-24-tactical-command-voice.md`.

### P58-B18: Complete tactical voice-bank transport

- Status: complete transport and selected production dispatch within the
  deterministic A1 boundary; strict acceptance remains open.
- Evidence: the executable's continuous event table now maps and transports
  all 285 VOICEFXA/VOICEFXE resources: 153 Alliance and 132 Imperial. Tactical
  withdrawal, battle result, selected Death Star, and RGBY trench-run
  transitions emit their source event in addition to the command paths from
  P58-B17.
- Verification: the complete workspace, exact owned-DLL extraction and hash
  cross-check, mapping validator, packaged WASM, four muted browser cases, and
  independent visual review pass. Remaining production completion,
  destruction, recovery, warning, and ordered trench chatter callers, audible
  native comparison, mixing, interruption, and strict A0 acceptance remain
  open. See
  `../2026-09-10-interface-parity-audit/evidence/2026-09-25-tactical-complete-voice-bank.md`.

### P58-B19: Mixed-task-force target rejection

- Status: complete within the deterministic A1 boundary; strict acceptance
  remains open.
- Evidence: `FUN_005a24d0` requires one task-force ordinal before a hostile
  focus target can be assigned. A mixed capital selection now preserves its
  manual, active, and Escort targets and queues exact Alliance event `0x84` /
  WAVE `14101` or Imperial event `0x102` / WAVE `15105`.
- Verification: behavior-first and focused Rust tests, the scoped mutation
  gate, packaged production and fixture WASM, four fresh muted browser cases,
  and independent visual review pass. Audible native comparison and strict A0
  acceptance remain open. See
  `../2026-09-10-interface-parity-audit/evidence/2026-09-25-tactical-mixed-task-force-target.md`.

### P58-B20: Tactical recovery and withdrawal feedback

- Status: complete within the deterministic A1 boundary; strict acceptance
  remains open.
- Evidence: `FUN_005b8630` now queues the exact RGBG acknowledgement after the
  last live player fighter-group member recovers. Recover queues one exact
  RGBG capacity warning when no friendly carrier slot remains. Confirmed
  withdrawal follows `FUN_005a0240` and `FUN_005b1b70` to queue the exact
  faction warning when a hyperdrive-disabled capital must remain behind.
- Verification: the complete workspace, focused renderer and fixture tests,
  packaged production and fixture WASM, twelve fresh muted browser cases, and
  independent visual review pass. Audible native comparison and strict A0
  acceptance remain open. See
  `../2026-09-10-interface-parity-audit/evidence/2026-09-26-tactical-recovery-withdrawal-feedback.md`.

### P58-B21: Shared tactical post-battle orchestration

- Status: complete within the deterministic A1 boundary; strict acceptance
  remains open.
- Evidence: played and automatic space-battle outcomes enter one production
  route for bombardment, troop landing, contested ground continuation,
  unopposed occupation, and Battle Results destination routing.
- Verification: seven focused tactical-flow tests, 794 complete-workspace
  tests, packaged production and fixture WASM, four fresh muted browser cases,
  and independent review of all 40 retained PNGs pass. Exact sequencing,
  native playback, remaining special-state and audio paths, and strict A0
  acceptance remain open. See
  `../2026-09-10-interface-parity-audit/evidence/2026-09-27-tactical-post-battle-orchestration.md`.

### P58-B22: Bounded tactical source completion

- Status: practical launcher implementation complete within the deterministic
  A1 boundary; strict original-interface acceptance remains open at 0/106.
- Evidence: one source-compatible tactical random stream, persisted shield and
  weapon power nibbles, exact completion and destruction voice families, and
  the source-derived 120-second trench-run producer now share the production
  battle path. The trench run includes ordered chatter, maneuver damage,
  commander-rating resolution, casualty rules, and MDATA 201/202 routing.
- Verification: see the retained
  [P58-B22 evidence](../2026-09-10-interface-parity-audit/evidence/2026-09-28-tactical-source-completion.md).
  The workspace and WASM/package gates pass, as do all 144 fresh-process muted
  browser cases. Independent review of 88 full-resolution captures found no
  P0/P1 visual blocker.
  Owned lossless A0 comparison, exact native beam pixels, audible native
  playback and mixing, whole-process random-state continuity, strategic
  commander binding, and rare warning/ejection callers remain open.

### F-008: Browser media and mods are incomplete

- Severity: P1 if browser parity is claimed
- Status: confirmed platform gap
- Evidence: original advisor idle frames now render in the browser, but
  briefing frames, EData images, cutscene playback, mods, advisor action/voice,
  and parts of audio integration still use stubs, empty collections, inferred
  behavior, or immediate completion.
- Acceptance: implement and demonstrate each feature, or explicitly exclude it
  from the supported browser contract and qualify all completion claims.

### F-009: Release quality gates are not enforced

- Severity: P1
- Status: confirmed
- Evidence: the former CI ran native check/tests and raw WASM compilation only.
  Its GitHub Actions definitions were intentionally removed from tracking on
  2026-09-08, so all gates are currently manual. Formatting, strict clippy,
  packaging, browser execution, screenshots, parity, coverage, and asset
  integrity are not enforced by a hosted pipeline.
- 2026-10-19: the Fleet window gate failed in 4 of 5 runs, two causes:
  - The list took egui's global double click, so a click on a popup
    menu and then on a fleet counted as one. Win32's
    `WM_LBUTTONDBLCLK` needs both clicks in one window within 4 by 4
    pixels.
  - The fixture read the AI's own arrival line.
  - Both are fixed, and the gate passes 5 of 5
    (`evidence/2026-10-19-fleet-window-double-click/`).
- Acceptance: the release pipeline runs the same versioned commands and fixtures
  used by local acceptance, retains artifacts, and blocks regressions.

### F-010: Bitmap files decode, but display correctness is unproven

- Severity: P1
- Status: open acceptance gap
- Evidence: all staged files decode, but no current resource ledger connects
  every consumer to a runtime cache hit and screenshot. On 2026-09-08, an
  Astra-medium browser check proved that the proposed Strategy 11016–11027
  cockpit mapping loads successfully but displays character portraits rather
  than the intended control art. Common 11001–11275 are grouped cockpit
  animations, not sequential logical-button triplets. The unverified mapping
  was therefore rejected; controls retain explicit labels while the authentic
  command-to-animation table remains open.
- Evidence bundle: `evidence/2026-09-08-cockpit-pr-audit.md` retains the
  resource adjudication, package hash, Astra R7 results, and screenshots.
- Verified tranche F-010B: commit `4589d2e` maps all 38 fighter and capital-ship
  records to their authentic GOKRES miniatures. Astra-medium R9 verified seven
  representative resources across both factions, zero matte-blue pixels,
  correct aspect ratio, successful requests and manifest identity, and working
  expand/collapse/navigation interactions. The failing R8 baseline, R9
  screenshots, hashes, pixel scans, and test log are retained in
  `evidence/2026-09-08-fleet-miniatures.md`.
- Acceptance: complete the bitmap proof protocol below for every image-bearing
  surface on every claimed platform.

### F-011: Deterministic replay is not established

- Severity: P0 for multiplayer; P1 for reproducible single-player acceptance
- Status: partially remediated; F-011A and F-011B1 through F-011B4 pass
- Evidence: replay format v1 now records and executes command streams with
  per-command state checkpoints. Exact-artifact native/WASM execution now
  matches for the seed-42 fixture, but headless and interactive paths still
  consume randomness differently, interactive seeds are wall-clock-derived,
  and automatic and tactical execution paths are not yet proven equivalent.
- Verified tranche F-011A: save format v9 persists a versioned canonical
  logical-state fingerprint, rejects mismatches, preserves native v8 saves as
  explicitly unverified, and stores lossless browser metadata. The seeded
  two-run probe and 16 save tests pass. Astra-medium r3 passed all 34
  save/reload/load, metadata, bitmap, network, and error assertions with the
  matching fingerprint `v1:e2ad73730e68434a`; evidence is retained in
  `evidence/2026-09-08-state-fingerprints.md`.
- Verified tranche F-011B1: save format v10 persists the simulation RNG,
  optional second AI, repair state, automatic-combat cooldowns, and active
  configuration. Human-readable fingerprints now canonicalize populated
  typed-key maps without changing historical bincode layout. All 483 workspace
  tests pass, including an actual 718-byte v9 artifact and eight-value RNG
  continuation proof. Astra-medium r2 passed 40/40 browser storage, reload,
  load, continuation, bitmap, network, and error assertions with fingerprint
  `v1:6bb217229d3c4c60`; evidence is retained in
  `evidence/2026-09-09-state-continuation.md`.
- Verified tranche F-011B2: replay format v1 records the seed, RNG and roll
  contract, configuration identity, typed `{tick, sequence, actor}` commands,
  and unambiguous checkpoint positions. The shared native/WASM data-manifest
  function fingerprints all 51 simulation DATs in canonical order; the local
  fixture covers 50,597 bytes with aggregate `5facb1c7ba0e81ad`. Seven focused
  unit tests, the original-data fixture, and the WASM app check pass. Evidence
  is retained in `evidence/2026-09-09-replay-contract.md`.
- Verified tranche F-011B3: the recorder derives command positions from the
  runtime and captures a state fingerprint after every command. The executor
  independently rejects engine, seed, data, configuration, initial-state,
  command-position, and checkpoint mismatches. Stable key ordering now covers
  manufacturing completions, simultaneous arrivals, blockade transitions, and
  equal-count AI reinforcement choices. A nine-command, 25-tick original-data
  campaign matched after save-v11 reload in five fresh native processes.
  F-007B subsequently changed tick-10 onward state as intended; the same
  reviewed stream now ends at `v1:b8a40a56246c1314` and passes the native
  fixture. All 518 workspace tests and the supported WASM compile gate passed
  at the original tranche. Evidence is retained in
  `evidence/2026-09-09-replay-execution.md`.
- Verified tranche F-011B4: native and packaged browser WASM decode the same
  13,482-byte replay artifact and independently reconstruct its 51-DAT seed-42
  campaign. The initial fingerprint, all nine ordered checkpoints, tick-25
  final fingerprint, and original artifact text match exactly. The automated
  gate also proves invalid queries and missing packs fail without partial
  state, while normal startup retains four requests and reaches the semantic
  bitmap menu. Astra medium independently passed the gate with zero browser
  errors. The F-007B revalidation passes all 530 workspace unit tests. Evidence
  is retained in `evidence/2026-09-09-replay-wasm-equivalence.md`.
- 2026-10-14 golden restored: after `7b34b8c1` made the loader honour the
  chosen galaxy size, the fixture loaded 100 systems and failed its own
  guard. `replay_fixture::seed42_seed_options()` now selects the full
  200-system galaxy for the native test, `replay-gate`, and the browser
  runner. The golden is regenerated for that cause and for the simulation
  and save-v33 changes since `e7808e5f`: `v1:24c73c2d80dea3f6` ->
  `v1:01685e44ab44b004`. Native and browser reports match on all nine
  checkpoints; the invalid-query, missing-pack, and normal-startup browser
  cases were not rerun. Evidence is retained in
  `evidence/2026-10-14-seed42-golden/README.md`.
- Acceptance: repeated native runs and native-versus-WASM runs produce the same
  versioned state fingerprints for the same seed and command stream, including
  after save/load.

### F-012: Interactive and headless simulation loops diverge

- Severity: P0
- Status: confirmed design divergence
- Evidence: the app duplicates simulation sequencing instead of calling the
  headless tick entry point. Ground follow-up, bombardment, battle-memory
  updates, AI availability, Death Star effects, telemetry, and `AdvanceTicks`
  behavior differ between paths.
- Acceptance: one authoritative simulation entry point produces identical
  results from the same seed and commands in app, playtest, native, and WASM.

### F-013: The packaged browser artifact omits runtime data

- Severity: P0 for browser release
- Status: remediated by F-014A
- Evidence: the deterministic release package carries 52 game-data entries,
  2,231 bitmaps, and five audio files in `runtime.orpk`; a clean browser boot
  loads it in four requests and verifies the expected hashes. See
  [F-014A evidence](evidence/2026-09-08-runtime-pack.md).
- Acceptance: a clean unpacked artifact boots offline from its own contents,
  loads the expected data hashes, and passes browser smoke tests.

### F-014: Browser startup, memory, and rendering are not release-scaled

- Severity: P1
- Status: partially remediated; F-014A verified 2026-09-08
- Closed tranche: a deterministic pack now carries 52 game-data entries and
  2,231 bitmaps. The release package boots with four total requests and one
  `runtime.orpk` request, retains lazy bitmap decode, includes hashes for all
  five shipped files, and preserves a loose-file development fallback. Astra
  medium passed both factions, fleet interactions, and ten zoom steps per
  faction with zero loading, bitmap, console, or WebGL errors. See the
  [F-014A evidence](evidence/2026-09-08-runtime-pack.md).
- Remaining evidence: the pack is not compressed; raw bytes and decoded
  textures have no eviction policy; HD assets are absent; high-DPI mode is not
  enabled; the galaxy can run two egui passes per frame; sector hulls are
  recomputed each frame; and Firefox/Safari and memory/frame budgets have not
  passed.
- Acceptance: the browser asset pack, lazy decode, LRU limits, one UI pass, and
  cached geometry meet the budgets below in Chrome, Firefox, and Safari.

### F-015: Browser persistence is not production-safe

- Severity: P1
- Status: confirmed design gap
- Evidence: synchronous base64-encoded bincode in `localStorage` is vulnerable
  to quota limits and main-thread stalls. Save v12 captures the currently known
  deterministic continuation envelope, campaign setup, and active repair
  episodes, but browser
  persistence remains synchronous and quota failures are not yet exercised end
  to end.
- Acceptance: versioned, compressed, asynchronous IndexedDB saves round-trip
  complete state, expose quota/corruption errors, and preserve fingerprints.

### F-016: Original main-menu parity and documented extension pass

- Severity: P1
- Status: remediated; P03 and P04 pass
- Evidence: the former stretched, dimmed bitmap and replacement text buttons
  are gone. The browser now composes COMMON 20001 with the binary-mapped control
  resources, exact logical hit regions, 4:3 scaling, direct faction starts, and
  the original menu audio. Astra-medium R2 exercised the full matrix and
  exposed a URL-plugin ABI diagnostic plus audio surviving Quit; after both
  fixes, R3 passed 11/11 focused assertions with zero console/page/request/
  texture errors and zero active audio loops after Quit. F-016B then persisted
  faction, difficulty, galaxy size, and victory mode through save v11 and wired
  Standard principal-leader capture versus Headquarters Only rules. Workspace
  F-016C adds Credits, the explicit M4 multiplayer destination, gain/mute,
  return-to-menu audio, clean second-campaign reset, the correct 35.34-second
  `MDATA.300` *Return of the Jedi* cue, and four original `COMMON.DLL` button
  effects. Workspace tests passed 500/500; Astra-medium verified the actual
  WebAudio buffers, navigation, reset, and error-free packaged artifact; the
  final focused R4 passed 14/14 and marked the candidate safe to commit. See
  `evidence/2026-09-09-main-menu-cockpit.md` and
  `evidence/2026-09-09-game-setup-propagation.md`, and
  `evidence/2026-09-09-main-menu-completion.md`. F-016D adds one clipped
  semantic navigation landmark for all 14 authentic hotspots without visible
  replacement controls. Workspace tests now pass 502/502; Astra-medium passed
  nine of nine browser gates, including 84/84 focus transitions, six exact
  keyboard activations, all destinations, the campaign return, `MDATA.300`,
  all four original effects, four-request startup, and zero runtime errors. See
  `evidence/2026-09-09-main-menu-semantics.md`. F-016E records binary evidence
  that the original had no standalone mute control, then adds one clearly
  separated Open Rebellion extension. Its 30×22 hard-beveled housing and cyan
  projection use the Jiff Gorda/SWG Project Thorn reference; verified Fable 5.1
  review set the canopy clearance, restrained motion, color, and exact hit-area
  rules. The user accepted the native presentation, all 504 tests pass, and
  Astra-medium passed 10/10 browser gates across four viewports with 24/24
  outside-edge probes inert, music gain zero while muted, SFX gain one, and no
  runtime errors. A narrow follow-up found a one-pixel bevel overflow below
  native size; clipping the full device to the shared hit rectangle corrected
  it, and Astra R2 passed 10/10 at 320×240 and 480×360 with 8/8 outside-edge
  probes inert. See `evidence/2026-09-09-main-menu-music-toggle.md`.
- Reference: [`agent_docs/main-menu-parity.md`](../../../agent_docs/main-menu-parity.md)
  records the binary-confirmed geometry, resources, commands, settings, music,
  responsive transform, and Astra matrix.
- Acceptance: native and browser reproduce the assembled cockpit, every mapped
  hover/click/keyboard behavior and selection, direct faction start, menu music,
  4:3 scaling, and all navigation with no blank aperture or missing resource.
  The original 14-control contract remains intact; the optional music-only
  control is documented as an extension. P03 and P04 are complete.

### F-017: The Death Star can never fire

- Severity: P1
- Status: remediated; browser verified (2026-10-20)
- Evidence: `DeathStarSystem::fire` (`crates/rebellion-core/src/death_star.rs`)
  refuses while `shield_generator_active` is true, and the only other writer
  (`crates/rebellion-app/src/tactical_flow.rs`) sets it to true;
  `DeathStarState::destroy_shield` has no caller. Nothing calls
  `start_construction`, so no Death Star fleet is ever built. The shield gate
  may itself be a wrong model: in the original the shield protects the
  Death Star rather than preventing it from firing. Found by the 2026-09-25
  test-pruning pass.
- 2026-10-21: the Death Star's destruction message takes the original
  texts (`0x182`, `FUN_004950d0`), for both sides:
  - TEXTSTRA `0x70c8` "{2:1} Destroyed!";
  - `0x70c9`, naming the side the system had (kind 4, `FUN_0052d920`),
    or `0x70ca` for a neutral system.
  The browser `destroy-system` case sees "The Death Star has destroyed
  the Alliance system Akrit'tar."
  (`evidence/2026-10-21-death-star-destroyed-message/`).
- Acceptance: construction, completion, planet destruction, and the shield's
  real role follow recovered Ghidra evidence, with tests that fail without
  each rule and a browser pass of the fire command.
- Fix (2026-09-25): the superlaser no longer waits on the shield generator.
  `FUN_005617b0` never references it; the shield only absorbs hull damage in
  battle. The Death Star is CAPSHPSD record 136 (TEXTSTRA 10120), an ordinary
  research-order-0 capital ship built at shipyards. Every Death Star check
  tested DatId family `0x34`, but seeded classes carry the record id, so no
  real Death Star was ever recognized in combat or firing.
  `CapitalShipClass::is_death_star` accepts both; a completed Death Star
  build now marks its fleet so it can fire, and a destroyed one clears it.
  Tests fail without each change.
- Open: `DeathStarState::start_construction` and its uncited 1,825-tick timer
  are now a second construction path beside ordinary manufacturing, so
  Death Star Sabotage delays a timer nothing starts. The fire command still
  needs a browser pass. Battle shield absorption stays limited to family-`0x34` ids:
  nothing destroys the shield generator (`destroy_shield` has no caller), so
  applying it to the seeded Death Star would make it unkillable in
  auto-resolved battles.
- Correction (2026-09-26): `FUN_005617b0` is not a superlaser check, and the
  2026-09-25 fix and the fire() docs were wrong to cite it. It recomputes the
  CharacterMgr SeatOfPower flag: `FUN_005070d0` finds `0x34000280`, which
  MJCHARSD and TEXTSTRA 10368 name Emperor Palpatine, and the flag is set when
  he is alive (`+0xac` bit 0 clear) and active (`+0x50` bit 0) at `0x90000109`
  (Coruscant) under Empire control (`+0x24 & 0xc0 == 0x80`). `FUN_0055f650`
  is decompiled: it stores the flag in `+0x58` bit 0 and notifies
  `CharacterMgrSeatOfPowerNotif` (`FUN_00562450`, event `0x230`). The real
  superlaser path is unrecovered (`DEATHSTAR_FIRE` appears only in the
  tactical constructor `FUN_005a7500`), so every `fire()` precondition lacks
  a source. Family `0x34` is the Empire major characters, not the Death Star;
  see F-025.
- Search (2026-09-26): the binary has no Death Star construction or planet
  destruction notifier. Its only Death Star strings are the tactical
  `DEATHSTAR_FIRE`, `DEATHSTAR_UPDATE`, and `DEATHSTAR_WITHDRAW` and the
  system flag `SystemDeathStarNearbyNotif` (`FUN_00512480`). The destroyed
  reasons are `DestroyedSabotage`, `DestroyedAssassination`,
  `DestroyedAutoscrap`, and `DestroyedOnArrival`, raised through
  `GameObjDestroyedNotif` (`FUN_004fc080`, event `0x302`). A sabotage success
  therefore destroys an object; nothing in the binary delays a build, so the
  port's `add_sabotage_delay` has no source either. The superlaser, the
  1,825-tick timer, and sabotage stay open until the tactical fire path
  (`FUN_005a7500`) and the sabotage mission handler are decompiled.
- Construction (2026-10-20, save v94): the second construction path is
  removed. CAPSHPSD record 136 (cost 584, research order 0) is built by a
  shipyard like any capital ship, and its completion already marks the
  fleet (`apply_build_completion_inner`). `DeathStarState::under_construction`,
  `start_construction`, the 1,825-tick timer, `ConstructionCompleted` and
  its telemetry, message and panel view had no source and no caller outside
  one telemetry fixture. Golden regenerated for save v94 (initial
  `v1:f47443f73b78e129`, final `v1:caadbc1bab82cbda`)
  (`evidence/2026-10-20-death-star-construction/`).
- Sabotage (2026-10-20): a successful Death Star Sabotage already destroyed
  the Death Star (`FUN_005746e0`, slot `+0xac(6)`; manual p. 105), and the
  invented construction delay was gone (`70916608`). A fleet the sabotage
  emptied stayed in the world, though `FUN_004fe870` -> `FUN_004fe630`
  disbands a fleet left without a capital ship; `destroy_target` now goes
  through `scrap::remove_ship` / `disband_if_empty`, and
  `Fleet::has_death_star` follows the remaining Death Star hulls. Seeds 1,
  6, 16 and 24 of 24 change from the first fleet a sabotage emptied
  (`evidence/2026-10-20-death-star-sabotage/`).
- Superlaser and nearby warning (2026-10-20, save v95): a planet falls only
  to the Destroy System order (`0x223`, `FUN_0058d990`, ported), and the
  tactical superlaser is the tactical session's
  (`tactical-death-star-path.md`). The strategic `DeathStarSystem::fire()`,
  the panel's Fire and Move buttons, `reconcile_death_star_result` and the
  300-unit `NearbyWarning` had no source and no reachable caller: nothing
  set `DeathStarState::death_star_fleet`. They are removed; DeathStarNearby
  stays `economy::death_star_nearby` (`FUN_005178b0`). The port-only panel
  (D) lists the side's Death Star fleets read-only. Golden regenerated for
  save v95 (initial `v1:803494e978e8f71f`, final `v1:5eb68d3167d6a252`)
  (`evidence/2026-10-20-death-star-superlaser/`).
- Shield (2026-10-20, save v96): the Death Star shield is DEFFACSD 4
  (family `0x25`). `FUN_0040bcd0` counts the battle system's shields into
  the tactical Death Star's `+0x70` (`FUN_00595ef0` -> `FUN_005af4e0`), and
  `FUN_005ba450` bars the fighters' Attack Death Star while it is set
  (manual p. 124); now `TacticalDeathStar::shielded` / `attackable`. No
  strategic battle code reads a shield, so the auto-resolved battle's
  hull-damage immunity (`shield_generator_active`, keyed on family
  `0x34`, F-025) is removed with the whole strategic `DeathStarState`.
  Golden regenerated for save v96 (initial `v1:82332ef108ec7e21`, final
  `v1:49a3738dca7ca496`) (`evidence/2026-10-20-death-star-shield/`).
- Browser pass (2026-10-20): the fixture scenario `FleetDestroySystem`
  (code 74) and the `destroy-system` case of `fleet-window.mjs` order
  Destroy System from the fleet icon's Planetary Bombardment submenu. The
  Alliance's row is disabled (no Death Star, `FUN_00582440`). The Empire's
  destroys Akrit'tar: the fleet stays, two characters go adrift, no enemy
  regiment is left, and the line "Akrit'tar DESTROYED by Death Star
  superlaser!" is posted. Both runs: 0 errors, 4 requests at 200. The
  Fleet window suite passes 50 cases. F-017's acceptance is met; the
  tactical Attack Death Star's launched-fighter count (`FUN_005ad930`)
  stays a known gap (`evidence/2026-10-20-destroy-system-browser/`).
- Attack Death Star count (2026-10-20): the gap is closed. `FUN_005ad930`
  also requires no trench run under way (`FUN_005d0ae0`, `DAT_006bc978`),
  fewer Imperial than Alliance fighter groups on the field (list `+0x988`,
  class 1, side `+0x168`, 0 the Alliance), and none of the player's groups
  already on order 6 (manual p. 151); `attack_death_star_enabled` now
  checks all three. The tactical browser scenarios could not run: their
  fixture wants SYSTEMSD index 100/101, which the default 100-system
  galaxy (indices 120..289) does not load, so all 16 Death Star cases stop
  at setup, independently of this change
  (`evidence/2026-10-20-attack-death-star-count/`).
- Tactical fixture (2026-10-20): the fixture takes the first and second
  empty, intact systems of the loaded galaxy instead of SYSTEMSD 100/101; a
  Death Star battle's Imperial fleet brings no fighter group, which
  `FUN_005ad930` requires; with the tactical 3D corpus staged locally, all
  16 tactical Death Star browser cases pass on Chromium 154
  (`evidence/2026-10-20-tactical-death-star-fixture/`).
- Destroyed-system art (2026-10-20): `FUN_0045bbb0` draws a destroyed
  system as STRATEGY 10240, an asteroid field, with the unheld name colour
  `0x2ffff00`; `sector_window::planet_art` does the same, seen in the
  browser after Destroy System (`evidence/2026-10-20-destroyed-system-art/`).
- Victory fields (2026-10-20, save v97): `VictoryState::death_star_active`
  and `death_star_location` had no source and no reader; both are removed.
  Golden regenerated for save v97 (initial `v1:ba71ff33556e6215`, final
  `v1:f7529f2440ec6c60`) (`evidence/2026-10-20-victory-death-star-fields/`).

### F-018: Research never limits which ships can be built

- Severity: P1
- Status: remediated (2026-10-20: the gate's comparison and start traced)
- Evidence: `ResearchSystem::ship_class_is_available` and
  `fighter_class_is_available` (`crates/rebellion-core/src/research.rs`) had
  no callers, so manufacturing offered every class regardless of research level.
- Fix (2026-09-25): the manufacturing panel and AI production offer only
  capital ships and fighters whose `research_order` is at or below the side's
  Ship level. Fighters use their own FIGHTSD.DAT order. Three tests fail
  without the gate, scoped `cargo mutants` catches all 13 gate mutants, and
  the seed-42 golden changes from tick 10 for this cause.
- Open: the `<=` comparison and the level-0 start rest on the `rebellion2`
  prototype and the DAT data.
- Fix (2026-10-14): the "troop and facility classes do not load
  `research_order`" gap is closed. `BuildableClass` carries the order from
  TROOPSD, SPECFCSD, DEFFACSD, MANFACSD, and PROFACSD, and
  `build_selection::listed_classes` already gates the Training Facility list
  on the Troop tree and the Construction Yard list on the Facility tree for
  both sides. The AI was the remaining hole: it ordered a regiment, defense,
  or yard class by copying one it owned, without the gate. It now skips a
  class its side has not researched on that class's tree
  (`AISystem::catalog_class_is_researched`), with a test over both sides that
  fails without the gate; scoped `cargo mutants` catches all 12 viable
  mutants. The seed-42 golden already failed before this change (the default
  galaxy loads 100 systems), so its effect on the golden is unmeasured.
- Correction (2026-09-26): `FUN_0052e4f0` and `FUN_0052e510` are not empty and
  are not buildability checks. Each returns `FUN_005839e0(this+0x88 / +0x8c,
  key)`, a count of matching list entries. Their only caller, `FUN_005330b0`,
  validates that each object in families `0x2d..0x2e` and `0x28..0x2b` sits in
  its side's list exactly when its `+0x58` is 1. No recovered code gates
  research.
- Source (2026-10-20): the `<=` comparison and the level-0 start now
  rest on the original, not the prototype. `FUN_0052e580` ->
  `FUN_0051cb20` -> `FUN_00567880` lists a class of the side whose
  research order (`+0x50`) is below or equal to the side's level, and
  skips one above it. The side's constructor `FUN_00533300` zeroes the
  three levels (`+0x9c`, `+0xa0`, `+0xa4`), and only `FUN_0052f280`
  raises them. The port's `is_unlocked` and zero start already match;
  only citations change (`manufacturing-build-selection.md`).
- 2026-10-21: research posts the original messages, replacing "tech
  advanced to level N".
  - Research Complete (`0x125`-`0x127`, `FUN_00499aa0`) names the first
    class whose order equals the new level (`FUN_00567a90` ->
    `FUN_00567880`, "equal" only; TEXTSTRA `0x7048` / `0x7049`). It says
    nothing when no class has that order.
  - Research Exhausted (`0x129`-`0x12b`, `FUN_0048abc0`,
    `0x7218`-`0x721b`) follows a tree turning Done (`FUN_0052f440`).
  - Evidence: `evidence/2026-10-21-research-messages/`.
- 2026-10-21: the yards' own research (the timer `0x381` tally) posts the
  same Research Complete and Research Exhausted messages as a research
  mission, through the same points hook
  (`evidence/2026-10-21-yard-research-messages/`).
- Acceptance: build lists and manufacturing orders respect the recovered
  research-order gate for both factions.

### F-019: Subdue, guarded dispatch, and initial Force awakening are never called

- Severity: P1
- Status: partially remediated; phases 0-7 of the mission port have landed
  (the decoy rule and the Subdue success check now run, and the dialog opens
  from its original entry and passes its browser gate). A native check of
  that entry, and targets other than systems, remain
- Evidence (before the 2026-09-25 fix): `UprisingSystem::try_subdue`
  (`uprising.rs`), `MissionSystem::dispatch_guarded` and `check_decoy`
  (`missions.rs`), and `JediSystem::apply_initial_awakening` (`jedi.rs`) had
  no production callers.
  Subdue Uprising missions never end a revolt, dispatch does not refuse busy or
  mandatory-mission characters through the guarded path, and no character
  starts Force-aware from `jedi_probability`.
- Acceptance: each path runs in the simulation with recovered rules and a test
  that fails without the call.
- Fix (2026-09-25): player, AI, and integrator dispatch now use
  `dispatch_guarded`, which refuses a character already on a mission or a
  mandatory mission and marks the dispatched character busy, matching the
  original role flags (`RoleOnMissionNotif` `FUN_00536b00`,
  `RoleOnMandatoryMissionNotif` `FUN_00536b80`). Cancelling releases the
  character. The seed-42 golden changes for this cause.
- Corrections: seeding already rolls `jedi_probability` for initial Force
  awareness, so the duplicate `apply_initial_awakening` was removed and the
  seeding roll gained a test. A successful Subdue Uprising mission ended the
  revolt only in the headless integrator; the app's own mission handler
  dropped both the uprising clear and Death Star sabotage delay. Both paths
  now share `apply_mission_state_effects`.
- Review follow-up: the guard also refuses a character who already has an
  active mission, which covers saves written before dispatch set the flag,
  and the player's commander list omits busy characters, with a message if a
  dispatch is still refused.
- Pending: the app's call to `apply_mission_state_effects` runs inside the
  frame loop, which no unit test reaches; a Subdue Uprising success needs a
  browser pass to confirm the uprising clears.
- 2026-10-20: the pending browser pass is done. Fixture scenario 72
  (SubdueSuccess) revolts the player's primary system, garrisons it with
  eight regiments and sends the agent on Subdue Uprising with the clock
  Fast; on both sides the app's frame loop ends the revolt through
  `apply_mission_state_effects` (log uprising_subdued at Akrit'tar, day 9
  and day 6; `evidence/2026-10-20-subdue-success/`). The native check of the
  dialog's entry passed on 2026-10-14 (cua-driver), and targets other than
  systems landed in phases 7g-7i.
- Recovery (2026-09-26): `FUN_0058b420` assigns the table ids (resource
  `0x642` `GDATA\` plus an `RT_RCDATA` file name): TDECOYTB 10, FDECOYTB 11,
  the ten mission tables `0x14..0x1d`, UPRIS1TB `0x28`, UPRIS2TB `0x29`,
  ESCAPETB `0x2c`. See `ghidra/notes/uprising-incident.md`.
- Correction: UPRIS1TB and UPRIS2TB are not subdue probabilities.
  `FUN_00559ce0` reads both during the uprising incident (system `+0x88`
  bit 16, slot `+0x248` `FUN_00511840`, `FUN_0050d030`), turning one score
  into two outcome codes that `FUN_0050d150` applies as facility or regiment
  losses and character effects (F-026). `try_subdue`'s premise is
  contradicted, so it was removed (see the F-026 follow-up below); the Subdue
  Uprising mission table is SUBDMSTB (`FUN_0055c780`).
- Decoy recovery (2026-09-26, `ghidra/notes/decoy-roll.md`): `FUN_0055e410`
  rolls TDECOYTB, or FDECOYTB when the checked object is in a fleet, on
  `decoy espionage - b - counterpart espionage * GNPRTB[3588] / 100`
  (GNPRTB 3588 is 35), and succeeds on `random(0..=99) < value`. Both
  espionage values are character slot `+0x1e0`, `FUN_004edc00`: the short at
  `+0x7e`, which `FUN_004eecf0` sets to `base(+0x5a) * (100 + +0x8c) / 100`.
  The decoy is a random pool member (`FUN_00588700`); the counterpart is the
  first system-holder object whose `+0x96` matches slot `+0x1bc` of the
  checked object (`FUN_00509330`), and `b` is its slot `+0x1c4`.
  `FUN_00589620`, the only caller of `FUN_00588b90`, then had no known
  reference (resolved below). The port's `check_decoy` fed the defender's
  espionage straight into FDECOYTB, which is the wrong input, and was never
  wired.
  The `is_decoy` mission branch is invented too: it cuts FDECOYTB by a flat
  GNPRTB 3588 percent, and only tests set `is_decoy`.
- Decoy call chain (2026-09-27, `ghidra/notes/decoy-roll.md`): `FUN_00589620`
  is slot `+4` of the functor vtable `0x0066a878`, a pointer Ghidra did not
  turn into a reference. Mission execution `FUN_00547f60` runs phase
  `FUN_0058a020`, which walks every defender at the target system
  (`FUN_00587640`: special forces, regiments, and each fleet's ships and
  special forces) and lets a random decoy character from the mission's pool
  roll TDECOYTB or FDECOYTB against each. The original decoy is a character
  attached to a mission. The port has none, so the invented `is_decoy` roll
  and `check_decoy` are removed with their two tests; the field stays for the
  save layout. Porting mission decoys needs decoy characters on
  `ActiveMission`, a save-format change.
- F-026 follow-up: `try_subdue` and the invented UPRIS1TB start roll are
  removed. A Subdue Uprising success now raises support by `FUN_0055cb10`
  (1..20 on its own side's system, 1..10 when contested) and ends the revolt
  through `FUN_0050c910` once regiments cover the undoubled garrison. Two
  divergences remain: the port's Subdue success check uses diplomacy, while
  `FUN_00569b90` rolls SUBDMSTB on leadership, support, and the Stormtrooper
  count; and `BetrayalSystem` still reads UPRIS1TB as a loyalty table.
- Review fix (2026-09-27): the Subdue gain draw took the next completing
  mission's outcome roll, and the budget of one roll per mission left the
  last mission on the 0.5 fallback. Callers now reserve `ROLLS_PER_MISSION`
  (2) rolls per mission; outcomes read the first half and gains the second.
  A test fails without the change.
- Recovery pass 3 (2026-09-28, `ghidra/notes/decoy-roll.md`): the decoy and
  detection phases run on mission phases 2, 3, 7, and 9. The mode word is
  the code constant `0x4112` (`FUN_005236e0`) for every class except Adrift,
  not a DAT column. Betrayal runs only in phase 9. The MSTB success roll is
  phase 10 (`FUN_00592f50`), once per team member through slot `+0x274`,
  characters and then special forces; decoys and captives never roll. The
  requester supplies decoys as an explicit key list (`FUN_0054bb90`, command
  `0x250`). Open: the sender of that command.
- Recovery pass 4 (2026-09-28, `ghidra/notes/ai-mission-planning.md`,
  `mission-lifecycle.md`): both the player and the AI fill the decoy list.
  - A mission order (`0x240`..`0x242`) holds a team `+0x2c` and decoys
    `+0x58`. `FUN_004f4a00` copies them into command `0x250`.
  - The player's dialog (`FUN_0046c3c0`, cases `0xca`/`0xcb`) moves chosen
    characters between the two lists.
  - The AI planners (`FUN_0042f830`, `FUN_004bced0`) pick decoys for
    Sabotage, Rescue, Incite, Espionage, DS Sabotage, Abduction, and
    Assassination, up to `(record.+0xbc + 2) / 2`. Diplomacy, Recruitment,
    Subdue, Recon, and Research pick none.
  - Phases chain within one tick except phase 4 (transit) and phase 8
    (timer `0x38b`, MISSNSD min + rand(spread)).
  - The port lands per phase and covers the port's 10 agent kinds.
- Phase 1 (2026-09-28, save v17): missions hold team, decoy, and captured
  lists of characters or special forces.
  - Guarded dispatch applies `FUN_0054bb90`: prisoners go to the captured
    list and an empty team is refused.
  - It also applies `FUN_00522b30`: members must be on one side, have no
    current mission, share one location, and appear only once.
  - MISSNSD records load with their timer and flags. SPECFCSD classes load,
    and seeded special forces roll their skills (`FUN_00535e40`).
  - The uprising leadership term averages every member (`FUN_00520cd0`).
  - The resolver still rolls one lead character until phase 4. A 1500-tick
    dual-AI playtest produces identical telemetry before and after.
  - Review fixes (2026-09-28): a killed character is refused and hidden from
    the mission panel. Prisoners leave both lists before the team, decoys,
    and captured are added in `FUN_0054c200` order. MISSNSD records are found
    by id, since Research and Vacation share families across records. Save
    v17 has a round-trip test for the new fields.
- Phase 2 (2026-09-29, save v18): missions step through the recovered
  phases instead of counting down.
  - The stepper `FUN_005227d0` chains phases in one tick except phase 4
    (members travel from the origin to the target, `FUN_00556430`, and stay
    there) and phase 8 (timer `0x38b`, MISSNSD min + rand(spread)).
  - Phase 2 sets Han Solo's speed, GNPRTB 3083, when he is a free member and
    no special force is (`FUN_00548370`). Phase 10 resolves and repeats to
    phase 8 when the record repeats; phase `0xb` ends with code 1 when no
    rule fired.
  - The validator `FUN_00522480` reads the MISSNSD target columns: lost
    container 7, target side 8, lost, travelling, or departed target 6,
    unpopulated container `0xd`, the uprising and prisoner columns, and
    Diplomacy's full support `0xf`. A destroyed container or target runs it
    at once (`FUN_00545240`).
  - Open: Recruitment's `0x10` (the recruit pick, phase 4), resign (5), and
    the end's observation level (phase 3). The roll is still the interim
    lead-character roll.
  - A 1500-tick seed-42 dual-AI playtest resolves 2644 missions instead of
    1823, mostly repeating Incite on its 2..12-day timer. The seed-42 golden
    is regenerated for this cause.
  - Review fixes (2026-09-29, save v19):
    - `FUN_00520ac0` reads the mission's phase (`+0x54 -> +0x1c`), not a
      class, so a destroyed container gives code 7 only past phase 6. The
      check runs once per destruction.
    - Phase 4 ends unless every living member still travels
      (`FUN_00522280`), and a null origin moves no one (`FUN_00556430`).
    - Leaving phase 8 validates before disarming the timer. The app frees
      both AIs' mission members through one shared helper.
    - Open: before phase 5 the validator should read the side's own copy of
      the target (`FUN_00521160`, `FUN_005211c0`), which the port does not
      keep.
    - The 1500-tick playtest now resolves 3076 missions. Of the 60 seeded
      characters, 51 unrecruited ones have no location, and they now stay
      nowhere instead of landing at their first target. The recruit pool
      (phase 4) takes them out of play. The golden moves to
      `v1:d8edefc22cfdf7cd` -> `v1:6f404bcf7868244a` for save v19 and the
      new mission field.
- Phase 3 (2026-09-29, save v20): the detection run `FUN_00547f60` follows
  each phase change (`crates/rebellion-core/src/mission_detection.rs`).
  - Setup `FUN_00589a40` picks mode 2, 1, or 4 for phases 2, 3 and 7, and 9,
    and which members and defenders count.
  - Decoys draw off defenders on TDECOYTB/FDECOYTB (espionage - detection,
    `FUN_00589620`) or are exposed. Defenders roll FOILTB on the team's
    average espionage - detection - special forces - G3584 (`FUN_005896e0`).
    Phase 9 finds a traitor on `draw(0..99) < 100 - loyalty`
    (`FUN_00589f10`).
  - A detected team ends 3, or 4 past phase 4, when members may resign.
    Each member faces a random defender and rolls RLEVADTB: an evader
    resigns and takes the injury roll (`FUN_0053e990`), a captured
    character is injured and held, and a captured special force is
    destroyed (`FUN_00503eb0`). Validator rule 1 ends 5 once no team member
    stands without a resign request.
  - port: no officer ranks, so the officer terms are 0; the draws come from a
    SplitMix64 stream seeded by a fifth per-mission roll.
  - The seed-42 playtest never detects: its teams are unplaced pool
    characters that no location filter counts (phase 4's recruit pool). It
    resolves 2949 missions instead of 3076 because the fifth roll shifts the
    later systems' draws. The golden moves to `v1:acbd440f1734677c` ->
    `v1:276e317d8c392b5e` for save v20 and the fifth roll.
- Phase 4a (2026-09-29): phase 10 rolls every team member
  (`FUN_00592f50`). Team characters roll first, then team special forces.
  Decoys and captives never roll.
  - The chance (slot `+0x274`) is a step lookup (`FUN_00595090`) of the
    class's MSTB table on its recovered input (`member_chance`). A missing
    table, row, or target gives 0. The draw `0..99 < chance` comes from the
    mission's seeded stream.
  - Each success raises the base skill by GNPRTB 6156..6168. Special forces
    never raise, because their slot `+0x1d8` returns 0 (`FUN_006158b0`).
  - The in-roll actions:
    - Rescue frees the target.
    - Abduction captures it with the member as captor.
    - Assassination kills it.
    - Incite runs the uprising incident `FUN_0050d030`. The result is 2
      while the holder keeps a regiment (`FUN_00509020`).
    - Subdue rolls only during an uprising, adds the `FUN_0055cb10` gain,
      and runs `FUN_0050c910`.
  - Slot `+0x280` turns result 0 into 2. Diplomacy wins `FUN_0055cac0`
    support: G6183 + rand 6184 at its side's own system, G6185 + rand 6186 at
    a neutral one (side bits 3, `FUN_004f8c60`), and none at the opponent's.
    Espionage reveals the system.
  - Deleted: the invented quadratic, foil probability, defense score,
    covert flags, the interpolating MSTB lookup, fixed effect sizes,
    `MissionOutcome::Foiled`, and the integrator's control flip. The AI and
    the panel preview read `member_chance`. The provenance baseline shrinks
    by 9, to 119.
  - The earlier Subdue gain mapped side 3 to contested. It is neutral.
  - port, interim until phases 4b and 4c: Recruitment's recruit, and the
    fixed Sabotage and Death Star Sabotage effects.
  - Roll slots per mission drop from 5 to 3: the creation timer, the repeat
    timer, and the stream seed.
  - The 1500-tick seed-42 playtest resolves 1212 missions instead of 2949.
    Incite and Recruitment now end with code 8, because the invented
    uprising start and control flip no longer run. The golden moves to
    `v1:acbd440f1734677c` -> `v1:f5bfb82d01bda045` for the per-member roll
    and the three roll slots.
- Phase 4b (2026-09-29, save v21): Recruitment signs a pool character
  (`FUN_0056b9a0`).
  - A character carries `recruited`, the original's `+0x50` bit 1.
    port: a character placed at game start begins recruited. The other 51
    of the 60 form the pool.
  - Each successful member picks `draw(0..=n-1)` among its side's living
    minor characters (families `0x38..0x3c`, MNCHARSD) without that bit (`FUN_0055ef30`, `FUN_0055fc80`,
    `FUN_0053e290`). The recruit joins at the target (`FUN_0055fe70`: slot
    `+0xa8`, then `FUN_004f7480` sets the bit). The result becomes 3, and
    leadership rises by GNPRTB 6159. An empty pool leaves the result unset,
    so the member fails.
  - Taking the last one sets the side's `+0xb8` (`FUN_0052f590`), and the
    validator then ends Recruitment with `0x10` (`FUN_0056b370`).
  - port: the pool is walked in `DatId` order. A later mission in the same
    step skips an earlier mission's pick, because the world applies recruits
    after the step.
  - The 1500-tick seed-42 playtest resolves 1053 missions instead of 1212,
    fewer Diplomacy (327, was 444) and Incite (160, was 207). A run that
    marks recruits without placing them reproduces 1212 exactly, so placing
    the recruits at their targets accounts for the whole change. The golden
    moves to `v1:c6a7cdfec25f7b55` -> `v1:2ceb75524906c42b` for save v21 and
    the recruit pool.
  - Review fix: the pool holds minor characters only (families
    `0x38..0x3c`, MNCHARSD, through `FUN_0056f450`), never a major.
- Phase 4c (2026-09-30, save v22): Sabotage and DS Sabotage name a target
  object (order `+0x4c`, read back by `FUN_00521030`).
  - The object is a defense, manufacturing, or production facility, a
    regiment, a special force, or a fleet's Death Star hull (class `0x88`,
    family `0x18`).
  - Dispatch refuses a Sabotage without an object or naming the Death Star
    (`FUN_0056a110`, `0x40`/`0x28`), and a DS Sabotage naming anything else
    (`FUN_005744c0`, `0x40`/`0x29`).
  - On result 3, `FUN_005746e0` destroys the object once (slot `+0xac(6)`).
    The validator's object rules (`FUN_00593500`) read its side, its system,
    and whether it stands.
  - Deleted: the interim removal of a system's first facility, and the
    invented 50-tick Death Star construction delay (`add_sabotage_delay`).
  - port: capital ships other than the Death Star, and fighter squadrons,
    have no identity in the port and cannot be named. A removed object
    reads as the opponent's and destroyed, so rule 4 ends the mission with
    6. The AI names the first enemy manufacturing facility at its chosen
    system (the planner's own choice is phase 5).
  - The 1500-tick seed-42 playtest resolves 1069 missions instead of 1053.
    In 4b the AI sabotaged Wistril 109 times, because removing the system's
    first facility left the enemy yard standing. Naming the yard destroys it
    within 7 missions, so Wistril completes 17 builds instead of 32. The
    golden moves to `v1:8ff65dd8cf6fd7b2` -> `v1:41144ee79484fa1a` for save
    v22 and the target objects.
- Phase 5a (2026-09-30, save v23): mission legality and the AI planners'
  team and decoy selectors (`ghidra/notes/ai-mission-planning.md`).
  - MISSNSD records gain their member rules (`+0x40..+0x4c`). Dispatch
    refuses members the record does not admit (`FUN_005830a0`,
    `FUN_00583320`): special-force mission bits outside `+0x48`, a character
    where `+0x4c` lacks `0x10000`, mixed sides, or a side the record does not
    run for. DS Sabotage is Alliance-only and Assassination Empire-only. A
    running mission is never ended by this check.
  - `crates/rebellion-core/src/mission_planning.rs` ports the AI records
    (`FUN_00401d20`, `FUN_00402230`), the candidate query (`FUN_00403460`),
    the ranking (ascending by skill, a coin flip on ties, positions from
    `FUN_0041c230`), the pick (`FUN_004357b0`: the highest sum of positions,
    only candidates every query found), and the ten kinds' selectors with
    their sizes, thresholds, and decoy caps (4 for Rescue, Abduction, and DS
    Sabotage, `FUN_004047d0`).
  - `AIAction::DispatchMission` carries the team and decoy lists. Deleted:
    the invented skill thresholds, the 30% success gate, and the Jedi skip;
    the `diplomacy_skill_threshold`, `espionage_skill_threshold`,
    `covert_min_success_prob`, and `covert_target_popularity_threshold`
    tuning fields. Assassination and Abduction go to the target character's
    system instead of the enemy's most popular one (`FUN_004bd0a0`).
  - port: the mission kind and target stay the port's (side `+0x318` is
    untraced). The planners run in one cycle, so the nearness query never
    joins and an order keeps only the members standing with its first (the
    original gathers them in state 7). A plan with no team keeps no decoys,
    a member one plan sends is not taken by the next, and the tie draws come
    from a stream seeded by the day and side.
  - hyp: the posture that sets `+0xbc` is not ported, so Espionage, Incite,
    Assassination, and Sabotage take one decoy. The side controller's leader
    and research bits are read as clear, their constructor values, so the
    four leaders stay off Incite, Rescue, Sabotage, Assassination, and DS
    Sabotage teams, and every character legal for Research (all of them) is
    reserved from Incite, Rescue, Sabotage, and Abduction teams. A character
    in a fleet holds an officer rank.
  - The 1500-tick seed-42 playtest resolves 480 missions instead of 1069
    (Diplomacy 341, Espionage 89, Recruitment 32, Sabotage 18), from 372
    orders instead of 1971. Characters reserved for research leave Sabotage
    and Incite to special forces, and Espionage takes special forces only.
    Manufacturing completes 62 builds instead of 33. The golden moves to
    `v1:cff529f786c2ecc1` -> `v1:7e1061cadd1e6f22` for save v23 and the AI
    selection.
- Phase 5b (2026-09-30): the player's original mission dialog
  (`ghidra/notes/mission-dialog.md`).
  - `crates/rebellion-render/src/mission_dialog.rs` ports the 259 by 355
    window (`FUN_0046a750`, `FUN_0046a9c0`, `FUN_0046c3c0`). It uses the
    STRATEGY page panels, title bars, tabs and buttons, the GOKRES mission
    icons and minis, and the TEXTSTRA text. The first tab holds the
    mission-kind box, its "Missions" drop-down, and the target; the second
    holds the agents and decoys with the move arrows (and double clicks).
    "Begin Mission" (`0x66`) submits and closes, "Cancel" (`0x65`) and the
    close box (`0x64`) discard the order, and "Encylopedia" (`0x67`) opens
    the Encyclopedia.
  - `missions::available_kinds` lists the kinds as `FUN_005422f0` does: the
    records in MISSNSD order that are not hidden, whose member rules admit
    the team and decoys, and whose class validator accepts a temporary
    mission (`FUN_0054c590`). With no kind, nothing opens (`FUN_0042a320`).
  - `PanelAction::DispatchMission` carries the agents and decoys. The
    missions panel's pickers open the dialog instead of dispatching, and
    lose their invented kind buttons and success preview.
  - port: until phase 7 builds the drag, those pickers stand in for the
    drop, so the target is a system and only system-target kinds are
    listed. The scroll bars and item status overlays are not drawn; the
    wheel scrolls one item. The Encyclopedia opens without the kind's entry.
  - hyp: the window is centered in the galaxy view (`FUN_00606980`), and
    choosing a drop-down item closes it.
  - The replay golden is unchanged (`v1:7e1061cadd1e6f22`); the dialog is
    player input only.
- Phase 6 (2026-09-30): telemetry and browser acceptance.
  - `EVT_MISSION_DISPATCHED` was defined but never emitted. Each applied AI
    `DispatchMission` now reports its kind, side, target, team, and decoys;
    a refused order reports nothing. `integrator::member_name` names a
    special force by its class.
  - A new test catches the pre-existing mutant in the fleet-move rollback:
    troops embarked for a move that cannot depart land again.
  - `FUN_00602150` buttons blit their bitmaps at native size
    (`FUN_00602d30` -> `FUN_005fc140` with a zero width), clipped by the
    control. The 66 by 33 "Encylopedia", "Begin Mission", and "Cancel"
    bitmaps sit in 64 by 33 controls, so they show their left 64 columns.
    The dialog had stretched them. It also sits on whole pixels now; both
    640 by 480 galaxy views center it on a half pixel (hyp placement).
  - Fixture scenarios `MissionDialogMission` and `MissionDialogAgents` (codes
    43 and 44) open the dialog for the fixture's first character at its
    primary system. `tools/interface-parity/mission-dialog.mjs` runs four fresh
    muted Chrome for Testing 151.0.7922.34 processes (two sides, two pages)
    and clicks the Agents tab once. 277,588 of 277,588 static-chrome pixels
    match 23 owned STRATEGY resources (aggregate `19e85900…f9f3ce`); text,
    the kind item, the target art, and the lists are masked and kept as
    captures. 16 of 16 requests return 200, with zero console or page
    errors. With the bitmaps stretched again, five captures differ by 1,169
    pixels each. The sixth also missed its tab click, because one-frame
    clicks were flaky; the gate now spreads a click over frames, and three
    runs in a row pass. The fixture WASM is `020a8010…cfa7853` and the runtime pack
    `02244b24…cf7d90f4`. Evidence is kept under the ignored
    `.artifacts/interface-parity/mission-dialog-2026-10-01T06-42-20-347Z-31048/`.
  - Finding: in the 1500-tick seed-42 dual-AI playtest, 372 dispatches
    (Diplomacy 220, Espionage 78, Recruitment 32, Sabotage 23, Incite 13,
    Assassination 6) carry one member and no decoys. The selectors picked
    only 4 decoys and 2 second members, and phase 5a's co-location rule
    dropped them all, because the research reservation (hyp) and the few
    special forces leave tiny candidate pools. Not fixed here.
  - Native: the dialog's input is covered by the egui harness tests. A live
    native capture waits for the phase 7 drag, which is the dialog's only
    original entry point.
- Phase 7 (2026-10-01): the original entry and cleanup.
  - Correction: a drag from the system window onto the map is a move
    (`0x201`/`0x202`), not the dialog's entry. A mission starts with a right
    click on an object, Mission, the targeting cursor, and a click on the
    target (manual pp. 39, 57, 100; `ghidra/notes/object-popup-menu.md`).
  - 7a: `game_menu.rs` draws the Game Menu Window (`FUN_00442860`) for any
    owner; the speed menu uses it, and its first two items were swapped.
  - 7b: a right click on a character or special force in a system window
    opens the object pop-up menu (`FUN_004ac5c0`, `FUN_0051d990`) with the
    STRATEGY records' rows. Mission follows `FUN_0051fe20`; Encyclopedia
    opens the Encyclopedia. port: Move, Confirmed Move, Command, Status, and
    Retire are drawn disabled.
  - 7c: Mission starts the galaxy view's targeting mode (`FUN_00429320`,
    `FUN_00422ce0`): the view holds the pointer, REBEXE.EXE cursor 1002 is
    staged and drawn with its hotspot on the pointer, and a release on a map
    system opens the dialog through `available_kinds`. port: only a map
    system is a target (no `+0x68` object hit test), Shift's pass-through is
    not ported, and Escape cancels.
  - 7d: the invented entry points are gone: Send Diplomat, Send Spy,
    `PanelAction::OpenMissionTo`, the missions panel's Dispatch tab, and
    `PanelAction::OpenMissionDialog`.
  - 7e: a refused order shows no text in the original; the side's advisor
    schedules a reaction (`FUN_00487c90` to advisor slot `+0xc`,
    `ghidra/notes/mission-dialog.md`, "Refusal"). port: the message-log lines
    stand in until P34 plays the reactions.
  - 7f: the unreachable egui system and fleet context menus and
    `PanelAction::InitiateFleetMove` are deleted (see F-007C). The browser
    gate found the pop-up menu and the dialog drawn under the system window
    they open over; both now take the order above the modeless windows.
    Fixture scenario `MissionTargeting` (code 45) and the gate's third
    scenario reach the dialog through the original entry for both sides: six
    of six fresh muted Chrome for Testing 151.0.7922.34 runs pass, 378,406
    of 378,406 checked pixels (dialog chrome and cursor 1002's 89 opaque
    pixels), 24 of 24 requests, zero console or page errors. Fixture WASM
    `b384974078a2…`, runtime pack `d1989cd84df5…`, cursor `d4196586faef…`.
    Evidence: the ignored
    `.artifacts/interface-parity/mission-dialog-2026-10-01T18-07-25-702Z-95298/`.
  - Open: a native check of the entry and of the speed menu's colors;
    character and object targets
    (Rescue, Assassination, and Abduction need them); the advisor's refusal
    reactions (P34).
- Phase 7g (2026-10-14): character and object targets from the Defenses
  window.
  - A Mission release asks the window's `+0x68`. The Defenses window answers
    the character, special force, regiment or defense facility in the list
    cell under the point, else its system (`FUN_004aa380`). A team member
    answers as its system. `available_kinds` lists a kind only for the
    target its class takes, so a prisoner lists Rescue and an enemy
    regiment Sabotage, and the dispatch carries the target.
  - A prisoner now lists with its captor's personnel, as it carries the
    captor's side (`FUN_004eec60`); before, it was listed with its home
    side, so no Rescue target was visible.
  - Tests: 5 new tests (core 2, Defenses window 1, targeting 1, dialog 1).
    The Defenses test fails without the prisoner-side change; the others
    call API that did not exist before. Scoped `cargo mutants`: core 8
    caught, 4 unviable; render 14 caught and 2 missed, both caught after two
    added cases.
  - port: the dialog names an object target but does not draw its portrait
    (`FUN_0042c3b0(.., 1, 1)`). The System, Fleet and sector windows still
    reduce their objects to the system, so manufacturing and production
    facilities and the Death Star are not yet targetable by the player.
  - Browser (2026-10-14): fixture scenario `RescueTargeting` (code 58)
    runs the whole flow on both sides with `agent-browser` and headless
    Chromium 153: the agent's menu, Mission, a click on the prisoner in the
    enemy Defenses window, a dialog offering Rescue alone and naming the
    prisoner, and an active Rescue after Begin Mission; 4 of 4 requests,
    0 console errors. Evidence:
    `evidence/2026-10-14-rescue-targeting/README.md`.
- Phase 7h (2026-10-14): the System and Fleet windows' targets.
  - The System window answers the facility in a facility page's cell
    (`FUN_00458b80`); the Fleet window its left entry, its right item, or
    else the right list's selection (`FUN_004a2f80`, now read). The port
    can target a facility and the Death Star hull there; a fleet, a
    squadron, any other ship, a queued unit and an empty deposit answer
    their system (port). The sector window's `+0x68` (`FUN_0045c6b0`)
    answers the planet's system, as no icon over it is a port target.
  - Fix: the dialog listed both Sabotage and Death Star Sabotage for any
    object, because only the dispatch ran the class creation checks
    (`FUN_0056a110`, `FUN_005744c0`). `FUN_0054c590` builds the dialog's
    trial mission through the class factory, so `available_kinds` now runs
    them too; a test fails without it.
  - The dialog names a facility or the Death Star by its class.
  - Tests: 4 new (System window, Fleet window, the Death Star dialog list,
    target names). Scoped `cargo mutants`: 13 core, 28 render and 7 dialog
    caught, the last after one added case; 0 missed.
  - Browser: fixture scenarios `SabotageTargeting` (code 59, both sides)
    and `DeathStarTargeting` (code 60, Alliance; only the Alliance has the
    mission) run the whole flow with `agent-browser`: Sabotage alone for an
    enemy Orbital Shipyard, Death Star Sabotage alone for the Death Star,
    and the mission active after Begin Mission; `RescueTargeting` reruns
    on both sides. 0 console errors and no failed request in any run.
    Evidence: `evidence/2026-10-14-object-targeting/README.md`.
  - Native (2026-10-14): `cua-driver` 0.31.0 on Linux (X11 through
    XWayland) drove the release build through a save with a prisoner, an
    enemy Orbital Shipyard and the Death Star at one enemy system. From the
    galaxy map, quadrant icons, the agent's menu and Mission, a click on
    each target offered Rescue, Sabotage or Death Star Sabotage alone, named
    the target, and Begin Mission took the agent off its personnel page.
    Evidence: `evidence/2026-10-14-native-mission-targets/README.md`.
- Phase 7i (2026-10-14): the targets the port used to reduce to a system.
  - The creation checks are read (`FUN_005868c0`, `FUN_00586e20`,
    `FUN_00593500`, `FUN_0056a110`, `FUN_005744c0`): no class takes a fleet
    or an unfinished object, and Sabotage takes an ordinary capital ship or
    a fighter squadron. A Fleet window now answers a ship
    (`MissionTarget::CapitalShip`) or squadron (`MissionTarget::Fighter`) as
    a Sabotage target, which destroys one hull or squadron of that class;
    a fleet entry, and a System window's queued unit or empty deposit, drop
    the order, so no dialog opens, where the port used to open one for the
    system. port: a ship or squadron names its class aboard the fleet, as
    the port keeps no per-hull or per-squadron identity.
  - Tests: 2 new core tests and the Fleet and System window tests rewritten
    to the recovered rules (they asserted the system before). Scoped
    `cargo mutants`: core 35 caught after one added case, render 33
    caught; 0 missed.
  - Browser: fixture scenarios `ShipTargeting` (code 61) and
    `FleetTargeting` (code 62), both sides, plus reruns of codes 58 to 60:
    a ship offers Sabotage alone ("Strike Cruiser"); a fleet opens no
    dialog and dispatches nothing. Evidence:
    `evidence/2026-10-14-ship-and-fleet-targets/README.md`.
  - Open: the object portrait, and a native rerun of the ship and fleet
    cases.

### F-020: A mod with a missing dependency fails silently

- Severity: P2
- Status: remediated; native GUI verified (2026-10-18)
- Evidence: `ModRuntime::enabled_sorted` (`crates/rebellion-data/src/mods.rs`)
  prints load-order errors to stderr and returns an empty list;
  `ModError::MissingDependency` was never constructed, so `ModRuntime::errors`
  and the Mod Manager showed nothing.
- Fix (2026-09-25): `discover`, `toggle_mod`, and `refresh` now record
  `MissingDependency` and `VersionMismatch` for each enabled mod, counting a
  disabled dependency as missing. The Mod Manager matches errors by
  `ModError::mod_name()` and shows text such as "requires 'x', which is not
  installed and enabled" instead of a Debug dump. A dependency cycle or
  duplicate name records a `LoadOrder` error on every enabled mod it blocks.
  Five tests fail without the change, and scoped `cargo mutants` catches every
  mutant in the new code.
- Acceptance: missing-dependency and version-mismatch failures reach
  `ModRuntime::errors` and the Mod Manager names the mod and dependency.
- 2026-10-18 (native): with three test mods, the Mod Manager names the
  missing dependency, the version mismatch (">=2.0.0, but 1.0.0 is
  enabled"), and a disabled dependency as not installed and enabled
  (`evidence/2026-10-18-native-acceptance/`).

### F-021: The blockade troop-destruction event is never raised

- Severity: P1
- Status: remediated; browser pass pending
- Evidence: before the fix, `BlockadeEvent::TroopDestroyed` (event `0x340`,
  `FUN_00504a00`) was matched by the integrator and the app but never built.
- Recovery (`ghidra/notes/blockade-troop-withdrawal.md`): `FUN_00504a00` only
  notifies. The loss is the regiment's withdraw-percent roll. `FUN_0050b310`
  keeps a system at 100 unless it is blockaded with no active KDY-150; then it
  is `max(0, 100 - ships * GNPRTB[7684] - fighters * GNPRTB[7685])`. A regiment
  added to a system copies that value, landing resets it to 100, and entering
  transit keeps it only if `random(0..=99) < percent` (`FUN_00504990`).
- Correction: `FUN_00504a00` is regiment vtable slot `+0x1f8`, not `+0x204`;
  `FUN_004ff3c0`'s `+0x204` call reaches `FleetBlockadeNotif` on fleet views.
- Fix: `BlockadeSystem::running_regiments` observes embarked regiments after
  arrivals and landings and again at the blockade step, and `resolve_running`
  rolls those leaving; both run in the headless step and the app frame loop.
  Destroyed regiments leave their fleet's cargo and emit
  `blockade_troop_destroyed` telemetry. Nine tests, all 37 viable mutants
  caught, seed-42 golden unchanged.
- Fix (2026-09-26): save v15 persists `BlockadeState`'s embarked-regiment
  tracking, so a regiment keeps its orbit and withdraw percent across a load;
  a v14 save migrates with empty tracking. Tests cover the round trip and the
  migration.
- Review fix (2026-09-27): the browser's `slot_occupied` skipped the v14
  keys, so a slot holding only a v14 save was overwritten without
  confirmation. Occupancy and deletion now derive their keys from
  `STORED_VERSIONS`.
- Open: the app path needs a browser pass. (The model's missing
  system-based fighters are closed below.)
- Squadrons on planets (2026-10-15): the open model item is closed. A
  built squadron stands on its system's planet (`FUN_0052a140`, slot
  `+0xa8`), and `withdraw_percent` counts the planet's squadrons with the
  fleets' (`FUN_0050b310` via `FUN_005039d0`). A squadron on the planet also
  makes its side present (`FUN_00509710` → `FUN_00503a50`), so the holder's
  squadrons turn an enemy fleet's blockade into a battle. Save v46
  (`evidence/2026-10-15-planet-squadrons/`).
- Acceptance: a regiment carried into a blockaded system without a KDY-150
  rolls against the recovered percent when it leaves, garrisons and
  surface-loaded regiments are never rolled, and a browser pass shows the
  loss message.

### F-022: Two tactical tests needed untracked bitmaps

- Severity: P3
- Status: remediated in `9901c34`
- Evidence: two `tactical_view` hit-mask tests read the gitignored
  `data/base/ui` and failed in a clean checkout. They are now `#[ignore]`d with
  a reason, and `make test-assets` runs them when extracted bitmaps exist.
- Acceptance: met.

### F-023: The economy applies the blockade troop-withdraw formula as a KDY production modifier

- Severity: P2
- Status: remediated
- Evidence: `economy.rs` step 4 computes
  `clamp(100 - capships * GNPRTB[7684] - fighters * GNPRTB[7685])` as a
  production modifier, citing `FUN_0050a480`, a community-dump address inside
  a `CALL` operand in our binary. The community function is our
  `FUN_0050b310` + `FUN_0055a020`, the blockade withdraw percent ported under
  F-021. No recovered code applies it to production; the value feeds only
  telemetry.
- Fix (2026-10-20, save v99): the modifier, its GNPRTB 7684/7685
  constants and its tests are removed. The withdraw formula stays where it
  belongs (`blockade.rs`, F-021). The campaign snapshot's
  `production_modifier` now reports the real `+0x70` (`FUN_0050b2c0`).
  The 24 campaigns are identical apart from that telemetry value. Golden
  regenerated for save v99 (initial `v1:fe91e55dd0f9a2af`, final
  `v1:b51db94b116e8495`) (`evidence/2026-10-20-kdy-modifier-removed/`).
- Acceptance: the modifier is dropped or re-sourced, with any golden change
  named.

### F-024: The Emperor's 1.5x battle damage bonus has no source

- Severity: P2
- Status: remediated (2026-10-21: the bonus went with the round formula,
  and no recovered battle code reads the Emperor)
- Evidence: `combat.rs` multiplies the Empire's pending weapon damage by 1.5
  when a character named Palpatine or Emperor is in the fleet, citing
  `FUN_00542050`. Ours is a two-line thunk; the community function maps to our
  `FUN_005438a0`, a named-character check (Leia, Luke, Han, the Emperor, Vader,
  Chewbacca). Neither shows a damage modifier.
- Acceptance: the bonus is removed or traced to recovered code.
- 2026-10-21: remediated.
  - The bonus was in `CombatSystem::resolve_space`, which W2g
    (`c94ef9d8`) removed with the round formula.
  - No recovered battle code reads the Emperor (record `0x34000280`): the
    abstract step (`FUN_005ae9c0`, `FUN_005d14d0`, `FUN_005b7780`), the
    battle's build and result (`FUN_0040b0e0`, `FUN_0040c780`), and the
    modules in `0x0059xxxx`..`0x005dxxxx`. The one hit, in
    `FUN_00592f50`, is a vtable slot `+0x280`. The only character term is
    the trench run's commander (`FUN_005afc00`).
  - The test `the_emperor_aboard_changes_no_battle` resolves one battle
    with and without the Emperor in the Imperial fleet and finds the same
    losses. With an Emperor-dependent effect injected, it fails.
  - No golden or campaign changes: the change is a test.

### F-025: Death Star combat checks key on the Empire major-character family

- Severity: P2
- Status: remediated (2026-10-21: identity by the CAPSHPSD family `0x18`
  record; the shield's role since save v96)
- Evidence: `combat.rs` derives `is_death_star` and `CombatEntityKind::DeathStar`
  from DatId family `0x34`. MJCHARSD and TEXTSTRA show `0x34000280` is Emperor
  Palpatine and `0x35000281` Darth Vader; `FUN_00560d50` routes family `0x34`
  to the SeatOfPower check. No shipped hull uses `0x34`, so shield absorption
  never runs on real data.
- Acceptance: Death Star detection uses the recovered class identity and the
  shield's battle role follows recovered code.
- 2026-10-21: remediated (`evidence/2026-10-21-death-star-identity/`).
  - CAPSHPSD record `+0x10` is the object family: `0x14` for every record
    but 136, the Death Star, `0x18`. The original finds Death Stars by it
    (`FUN_005003d0`, families `0x18..0x1b`; `FUN_0040b0e0` ->
    `FUN_0040bcd0`).
  - `is_death_star` drops the family `0x34` clause (the Emperor's family);
    record 136 stands for family `0x18`.
  - The shield's role follows `FUN_005ba450` since save v96.
  - Campaigns are unchanged.

### F-026: The disaster and uprising incidents have no effect

- Severity: P2
- Status: remediated; browser verified (2026-10-21)
- Evidence: `economy.rs` raised uprising and disaster flags from invented
  thresholds and only emitted messages; `UprisingSystem` started revolts on an
  invented UPRIS1TB roll and handed the system to the rebels.
- Correction: the earlier ledger text used system vtable base `0x0065e638`;
  the base is `0x0065e640` (constructor `FUN_00507130`), so bit 16 is the
  uprising incident (`FUN_0050aa50`, slot `+0x248`) and bit 18 the disaster
  (`FUN_0050ab30`, slot `+0x250` `FUN_00511930`).
- Recovery (`ghidra/notes/uprising-incident.md`): a revolt (`+0x88` bit 2)
  starts when a held, populated system is short of troops (`FUN_0050b800`)
  and never changes control (`FUN_0050a130`). It ends only on a Subdue
  Uprising success once all regiments cover the undoubled requirement
  (`FUN_0050c910`), or when the system is lost or emptied. Its incident timer
  fires every 30 to 100 ticks (GNPRTB 7701/7702, `FUN_00586130`).
  `FUN_00559ce0` scores two draws of `1..10`, the support shortfall below 60,
  regiments (doubled for a strongly held Empire system), Stormtroopers, and
  the Incite and Subdue agents' leadership; UPRIS1TB and UPRIS2TB step
  lookups give two codes, which `FUN_0050d150` applies as a lost facility or
  regiment, an injured character, or freed prisoners, before a -2 support
  change while an Incite mission is active (`FUN_0050c9f0`, halved per
  `FUN_00559be0`). A disaster (event `0x38f`, every 1 to 400 ticks) picks a
  random system with energy or raw materials, erodes both (`FUN_00559e10`),
  and destroys each facility not en route, of either side, at 10 percent.
  The garrison requirement is halved only for a strongly supported Empire
  system and doubled only in a revolt (`FUN_00559fe0`).
- Fix (2026-09-26): `UprisingSystem::advance` implements the lifecycle, the
  incident, and the disaster timer; `apply_uprising_event` applies the losses,
  freed prisoners, support, and resources in the headless integrator and the
  app. Save v15 stores each revolt's incident timer and the disaster timer.
  The invented economy uprising and disaster triggers and
  `EconomyEvent::NaturalDisaster` are removed. Each timer fires at most once
  per advance, so a second fire never reads the world before the first one's
  losses. The Uprisings filter, loyalty panel, and sector window read the
  revolt from `UprisingState`. Twenty-eight uprising tests and
  four economy tests fail without the change; scoped `cargo mutants`
  over the bundle's diff catches 307 of 331 viable
  mutants. The 24 survivors: the two `UprisingState` migrations, caught by the
  `rebellion-data` save tests; boundary swaps equivalent at a zero change or a
  shipped injury of at least 1; the informant and resource triggers (F-029);
  the unrecovered strong-support boundary; WASM-only save paths; and egui
  draw paths awaiting the browser pass. The seed-42 golden changes for this named cause.
- Review fixes (2026-09-27): the troop surplus that starts a revolt counted
  only the holder's regiments; `FUN_0050b500` counts every regiment at a
  held system (`FUN_00504c40`, no side filter) and leaves 0 elsewhere, as the
  end check already did. The uprising roll slice was one roll per system,
  so a busy tick ran dry and later draws returned their maximum;
  `UprisingSystem::roll_budget` now reserves each due incident's and the
  disaster's worst case. New tests pin the Empire regiment weight, the
  Empire-only halving of the Incite loss, code 5 freeing every prisoner, the
  gain-before-check order, and the step-lookup clamp. Scoped `cargo mutants`
  over the fixes catches 28 of 34 viable mutants; the seed-42 golden catches
  the two mission-budget multiplier swaps, and the four WASM-only save
  mutants need the browser pass. The seed-42 golden changes from tick 15 for
  these causes and now ends at `v1:301752058a3627d8`. The port has no en-route facility state, so every
  listed facility is a disaster candidate.
- Open: the injury at character `+0x94` has no port field, so an injured
  character is reported but unchanged. The app path needs a browser pass.
- Character pick (2026-09-27, `ghidra/notes/object-state-flags.md`): codes 3
  to 5 walk only the system's direct children (`FUN_00513120`,
  `FUN_005130d0`) in mode 1, `+0x50` bit 0, usable: existing, complete, and
  not en route (`0x004f7b80`). Characters in a fleet or on a mission belong to
  that fleet or mission, so the pick now also skips mission agents, and the
  prisoner codes skip captives aboard fleets. Two tests fail without it.
- En route: `FUN_00511930` spares a facility with `+0x50` bit 4. The port
  places a finished facility at once, so it never has one en route; the
  missing delivery phase is F-030.
- Browser (2026-10-21): the fixture scenario `UprisingIncident` (code 76)
  and `fleet-window.mjs` `incident` show, for both sides, the player's
  Akrit'tar in revolt, its incident ("Uprising incident at Akrit'tar")
  with the revolt still on, and a natural disaster on a random system.
  0 errors; the Fleet window suite passes 54 cases
  (`evidence/2026-10-21-uprising-browser/`).
- Texts (2026-10-21): the uprising and disaster messages take the
  original texts.
  - Uprising Begins / Ends (`FUN_00499460`, TEXTSTRA `0x7030`-`0x7033`)
    speaks for both sides. The holder is named by the Side's text provider
    (`FUN_0052d920`, kind 4 strings `0x4d70` / `0x4d71`).
  - Natural Disaster (`FUN_0048e940`, `0x7190`-`0x7193`, with a `0x719b`
    line per destroyed facility) goes to the holder, and says nothing when
    nothing was lost.
  - Browser runs struck Bothawui and Akrit'tar with the original lines
    (`evidence/2026-10-21-uprising-disaster-texts/`).
- Near Uprising (2026-10-21, `0x148`, `FUN_00498cb0`) is ported.
  - `FUN_0050a230` notifies a troop surplus change through
    `FUN_00511fd0`, with the old and the new value.
  - The class warns the holder's side when a garrisoned system (`+0x88`
    bit 8) that is not in revolt falls to a surplus of exactly 0
    (TEXTSTRA `0x7070` / `0x7071`).
  - `EconomyEvent::TroopSurplusChanged` carries both values. A class's
    slot `+0x24` takes the new value first, as System Joins
    (`FUN_00499760`) shows.
  - The campaigns and the golden are unchanged
    (`evidence/2026-10-21-near-uprising-message/`).
- 2026-10-21: notification `0x152` (SystemUprisingIncidentNotif,
  `FUN_00512580`) has no message type in `FUN_00488de0`, so the incident
  has no message of its own. The port's "Uprising incident at X" stays in
  the log, off the Message Index
  (`evidence/2026-10-21-uprising-incident-off-index/`).
- 2026-10-21: a revolt ended by Subdue Uprising posts Uprising Ends
  (`0x14b`, `FUN_00499460`, TEXTSTRA `0x7031` / `0x7032`) for both sides,
  in place of the port's "Uprising subdued at X"
  (`evidence/2026-10-21-subdue-uprising-ends/`).
- Acceptance: both incidents trigger and apply their recovered effects with
  failing-without tests, and a browser pass shows a revolt, an incident, and
  a disaster.

### F-027: Community-dump citations named the wrong functions

- Severity: P3
- Status: remediated
- Evidence: `disassembly.zip` came from a different REBEXE.EXE build; its
  addresses shift by region (`+0x360` to `+0x19c0`). Thirty-two cited
  addresses were not function entries in our binary.
  `ghidra/notes/community-address-remap.md` matches 43 community functions to
  ours, and the citations now name our addresses. A five-slice check labelled
  716 citation sites: 505 confirmed, 131 vague, 23 unsupported, and 57
  contradicted. Review overturned 2 (the `FUN_005c81d0` formation order is
  correct); the other 55 are corrected or ledgered above.
- Follow-up (2026-09-27): render tests of original behaviour gained comment-only
  source citations, about 90 in the three tactical files and 40 across ten
  other render files, each naming a function, resource, or manual page already
  recorded by the runtime code or the parity evidence. Three tactical tests stay
  uncited for lack of a recovered source: capital-selection wrap-around,
  one-shot trench-run recording, and the command-panel hit regions. The
  auto-resolve projection and canvas letterbox tests cover our own code.
- Review follow-up (2026-09-27): the order-code and target-control tests now
  cite `FUN_005ca6d0` and resources 1058/1059; the mesh Z reflection is our
  own conversion. Five render tests named "original" or "recovered" with no
  recovered source are renamed, and the light-rig test notes that its
  intensities are the port's reading.
- Acceptance: met for citations.

### F-028: Probability tables interpolate where the original steps

- Severity: P2
- Status: remediated
- Evidence: `MstbTable::lookup` interpolates between thresholds. The original
  lookup, `FUN_00595090`, returns the value of the largest threshold at or
  below the argument and clamps below the first row.
- Fix (2026-09-26): `MstbTable::step_lookup` implements the step lookup, and
  the uprising incident uses it for UPRIS1TB and UPRIS2TB, with a test that
  fails under interpolation.
- Fix (2026-10-14): betrayal and the captive escape check use `step_lookup`,
  each with a test that fails under interpolation, and the interpolating
  `MstbTable::lookup` is removed. Every table lookup now follows
  `FUN_00595090`.
- Open: the golden change is not named. On the tree the fix was made on, the
  seed-42 replay test already fails before the change (the fixture loads 100
  systems, and a 200-system galaxy does not match the reviewed initial
  fingerprint), so the golden was not regenerated.
- 2026-10-21: the golden change is named. `83facd86`, the first golden
  regenerated after `d6adacfe`, lists the step lookups among its causes.
  On today's tree an interpolating `step_lookup` leaves the seed-42 golden
  unchanged (its commands never fall between two rows), while all 24
  dual-AI campaigns diverge between day 21 and day 168
  (`evidence/2026-10-21-step-lookup-golden/`).
- Acceptance: every table lookup follows `FUN_00595090`, with any golden
  change named.

### F-029: The informant and resource incidents fire on invented triggers

- Severity: P3
- Status: remediated; browser and native GUI verified (2026-10-18)
- Evidence: `economy.rs` `evaluate_incident_flags` raises the informant and
  resource flags from a troop deficit and from overcapped energy or raw
  materials. The original runs them
  on timers: resource (bit 19) on event `0x390` every 1 to 500 ticks
  (GNPRTB 7719/7720) through `FUN_00556be0` and RESRCTB; informant (bit 17) on
  a per-system timer (`+0x40`, GNPRTB 7703/7704) that rolls support in
  `FUN_0050cbe0` before INFORMTB (`FUN_0050d510`).
- 2026-10-17: the informant incident is ported (`informant.rs`,
  `ghidra/notes/informant-incident.md`). Each held system's timer `0x38e`
  (GNPRTB 7703 (30) + rand(0..=7704 (970)) days, `FUN_0050c820`) fires it
  with `100 - support` percent (`FUN_0050cbe0`, `FUN_00559f90`). INFORMTB
  at 1 + rand(0..=7) (`FUN_00559db0`) then tells the other side about the
  system (`FUN_0050d510`, `FUN_0050d5a0`): the port reveals it in that
  side's fog and posts the Informants message (`0x153`). The troop-deficit
  trigger is gone. Save v66; golden final `v1:21ecf34ac9e1f828` ->
  `v1:e212b88bcf82db21` (`evidence/2026-10-17-informant-incident/`). The
  resource incident stands.
- 2026-10-17: the resource incident is ported (`uprising.rs`,
  `resolve_resource`). Event `0x390` every GNPRTB 7719 (1) + rand(0..=7720
  (499)) days picks a random existing system (`FUN_00556be0`), and RESRCTB
  at rand(0..=99) (`FUN_00559ee0`) moves its raw materials or energy by one
  within GNPRTB 7711-7714, posting New Resources or Resources Depleted
  (`0x155`). The economy's overcap incident and its "resource discovery"
  on rising output are gone. Save v67; golden final `v1:e212b88bcf82db21`
  -> `v1:c02d4ee44749e195` (`evidence/2026-10-17-resource-incident/`).
  Both incidents of this finding now follow their recovered timers and
  tables.
- 2026-10-18, closed by the user's decision: the informant incident runs on
  each held system's timer `0x38e` with the `100 - support` roll and
  INFORMTB, and the resource incident on event `0x390` with RESRCTB, each
  with failing-without tests and browser runs. Native GUI acceptance was
  not run.
- 2026-10-18 (native): a 1000-day release-build campaign posted
  Informants on days 286, 520 and 747 and New Resources on day 776
  (`evidence/2026-10-18-native-acceptance/`).
- 2026-10-21: the resource message (`0x155`, `FUN_0048e940`) reads its
  title and body from TEXTSTRA through `story_messages::resource_texts`:
  `0x7194` / `0x7199` / `0x719a` for a gain, `0x7196` / `0x7197` /
  `0x7198` for a loss. Before, the app held them as English strings
  (`evidence/2026-10-21-resource-message-texts/`).
- 2026-10-21: the Informants message (`0x153`, `FUN_00491060`, TEXTSTRA
  `0x7150` / `0x7151`) and Maintenance Shortfall (`0x304`, `FUN_00498970`,
  `0x7080`-`0x7082`, the system as `0x7081`'s third substitution) read
  their texts from TEXTSTRA. Before, the app held them as English strings
  (`evidence/2026-10-21-informant-maintenance-texts/`).
- Acceptance: both incidents follow their recovered timers and tables with
  failing-without tests.

### F-030: Manufactured objects arrive without an en-route phase

- Severity: P3
- Status: remediated; browser verified (2026-10-20)
- Evidence: the integrator's manufacturing completion inserts a finished
  facility, regiment, or ship straight into its system or fleet. The original
  keeps an en-route state (`+0x50` bit 4, `GameObjEnrouteNotif` `FUN_004fc240`;
  bit 5, `GameObjEnrouteActiveNotif`; `GameObjDestroyedOnArrivalNotif` for an
  object lost on arrival), and an en-route object is not usable
  (`0x004f7b80`), so the disaster (`FUN_00511930`) and the incident pick skip
  it. See `ghidra/notes/object-state-flags.md`.
- Open: the delivery rule (who sets bit 4, the travel time, and the arrival
  handler) is not yet recovered. Storing en-route objects changes the save
  format.
- Remediated 2026-09-28: a queue item may name a destination. On completion it
  departs (`FUN_0052bee0`) and arrives after the `FUN_0055d8c0` transit at its
  own speed: capital ship `FUN_00500820`, fighter FIGHTSD hyperdrive
  `FUN_00502f80`, others GNPRTB 1. An object whose destination is destroyed is
  lost on arrival (`0x303`, `FUN_004fc080`). Save v16 stores destinations and
  `DeliveryState` (`delivery.rs`, `ghidra/notes/build-delivery.md`).
  - hyp: the port holds a travelling object outside the world and places it
    on arrival, so every walk skips it. The original keeps it in the
    destination container, marked en route.
  - port: the production panel picks the destination from a list of held
    systems. The original order path is unrecovered, and AI builds stay at
    their facility.
- 2026-10-15: a completed or delivered squadron joins its destination
  system's planet, not a fleet there (`FUN_0052a140`, `FUN_0052a380`; manual
  p. 114); the player or the AI moves it onto a carrier with Move `0x201`
  (`evidence/2026-10-15-planet-squadrons/`).
- Browser (2026-10-20): the fixture scenario `ProductionDelivery`
  (code 75) and `fleet-window.mjs` `delivery` show, for both sides, a
  capital ship built at Akrit'tar for F'tral. It departs ("Construction
  complete at Akrit'tar, en route to F'tral"), is at neither end while en
  route, and arrives ("Unit arrived at F'tral", one ship there). 0 errors;
  the Fleet window suite passes 52 cases
  (`evidence/2026-10-20-delivery-browser/`).
- Autoroute (2026-10-20, save v100): a product travelling to a system
  that is destroyed is not lost. The destroyed hook (`FUN_004fb520`) sets
  an EnrouteActive child Autorouting (`FUN_004f8710`), and the autoroute
  search (`FUN_005545e0`) sends it to the nearest place of its side.
  `DeliveryState::advance` now does so (`Delivery::side`,
  `DeliveryAdvance::rerouted`), and only a delivery with nowhere to go is
  lost. `0x321` is the manager's current-product notification, not a
  redirect. Seed 10 of 24 changes
  (`evidence/2026-10-20-delivery-autoroute/`).
- Facility Idle (2026-10-21): `0x160` (`FUN_0048be60`) is per manager.
  - Each area whose queue empties reports to the system's holder, while
    a facility of its family remains: `0x28` shipyards (`0x71fc` /
    `0x71fd`), `0x29` training (`0x71fa` / `0x71fb`), `0x2a` construction
    (`0x71f8` / `0x71f9`).
  - `ManufacturingAdvance::newly_idle` carries `(system, area)`. The port
    had reported once per system, after its last area.
  - The idle telemetry now has one record per area (18557 -> 32484 over
    24 seeds); the simulation is otherwise unchanged.
  - The browser `delivery` case sees "Shipyard Idle on Akrit'tar"
    (`evidence/2026-10-21-facility-idle/`).
- Units Deployed (2026-10-21): a completed or delivered unit posts `0x107`
  (`FUN_00497940`) with the texts of its family, replacing "Construction
  complete at X" and "Unit arrived at Y":
  - ships: TEXTSTRA `0x7086` / `0x7087`;
  - the Death Star: `0x708e` / `0x708f`;
  - squadrons: `0x708a` / `0x708b`;
  - regiments and special forces: `0x7088` / `0x7089` with a `0x707f`
    line;
  - facilities: `0x708c` / `0x708d`.
  The browser `delivery` case sees "Mon Calamari Cruiser Deployed at
  F'tral" and "Strike Cruiser Deployed at F'tral"
  (`evidence/2026-10-21-units-deployed/`).
- Facility Lost (2026-10-21): a delivery lost on arrival (`0x303`) posts
  Facility Lost (`FUN_00497690`, message type `0x10` in `FUN_00488de0`)
  only for a facility (family `0x20`..`0x2f`): TEXTSTRA `0x70b0` /
  `0x70b1`, to its side. Other units lost on arrival post nothing, as in
  the original (`evidence/2026-10-21-facility-lost-message/`).
- Arrival (2026-10-21, timing corrected):
  - `FUN_0052bee0` sets Deployed (`0x107`) on completion, with the
    product already a child of its destination. So Units Deployed names
    the destination at departure.
  - The arrival clears EnrouteActive (`0x105` -> type `0xd`), and Unit
    Arrival (`FUN_004981c0`) speaks: TEXTSTRA `0x7076` / `0x7077` for
    ships, `0x707a` / `0x707b` for other units, and a `0x707e` line.
  - The browser `delivery` case sees "<class> Deployed at F'tral" at
    departure and "Ships arrive at F'tral" on arrival
    (`evidence/2026-10-21-unit-arrival/`).
- Rails (2026-10-21): Units Deployed (`FUN_00497940`) and Unit Arrival
  (`FUN_004981c0`) set their category mask (`+0x34`) by family:
  - ships `0x80` Fleet;
  - squadrons and regiments at a system, and defense facilities, `0x40`;
  - special forces `0x10`;
  - manufacturing facilities `0x08`;
  - mines and refineries `0x04` when deployed, `0x08` when arriving.
  The app had filed them all on the Manufacturing rail; now
  `story_messages::unit_message_rail` picks the rail
  (`evidence/2026-10-21-unit-message-rails/`).
- Acceptance: completed objects travel en route under recovered rules, the
  disaster and incident skip them, and a test fails without the phase.

### F-031: Simulation rules carry numbers with no source in the original

- Severity: P2
- Status: remediated (2026-10-21)
- Evidence: `tools/provenance-scan` finds 135 uncited items carrying 289
  literals or roll draws (baseline `scripts/provenance-baseline.json`).
  - `missions.rs` `foil_prob` decides every covert mission with a quadratic
    from rebellion2 `Mission.cs`; the original rolls FOILTB (table id 12) in
    `FUN_0058a130` (`ghidra/notes/decoy-roll.md`).
  - `MissionKind::coefficients` are rebellion2 or placeholder curves. They run
    only when an MSTB table is missing, so they should fail closed instead.
  - `ai.rs` has 18 uncited items, among them the production queue cap
    (`queue_len >= 3`), the troop-deployment radius (`150 * 150`), and
    `ESPIONAGE_SKILL_THRESHOLD` 50.
  - `economy.rs` `SupportTier::from_support_int` splits support at
    20/30/40/60 with no source.
- 2026-10-21 (economy and incidents): the five economy GNPRTB ids are
  cited to their readers:
  - 7761 and 7762: `FUN_0055a050`;
  - 7763: `FUN_00559c10`;
  - 7680: `FUN_00559fe0` and `FUN_00559c40`;
  - 7682: `FUN_00559fe0`.
  The blockade event ids are cited to their senders (`0x14e`
  `FUN_005122d0`, `0x340` `FUN_00504a00`), and the uprising roll budget is
  tagged `port:`. The baseline shrinks from 51 to 43 uncited items.
- 2026-10-21 (events, ids, world): each notification and event id in
  `events.rs` is cited to its sender (`FUN_0053fdd0`/`FUN_0053fcf0`
  notifications, `FUN_0053f9c0` events) and to the message class that
  answers it. Three keep a `hyp:` for the port's reading:
  - `0x100` is `GameObjControlKindNotif` (`FUN_004fbbd0`), not a support
    band change;
  - `0x305` comes from the blockade hook (`FUN_00511300`), not a
    saboteur;
  - `0x306` comes from the Autorouting hook (`FUN_004fba10`), not a death.
  The `DatId` layout, the GNPRTB.DAT and SDPRTB.DAT columns and the shield
  and weapon nibbles (`FUN_00501510`, `FUN_005015a0`) are cited. The event
  engine's evaluator and the old-save difficulty default are tagged
  `port:`. The baseline shrinks from 43 to 19.
- 2026-10-21 (AI, tuning, combat caps):
  - `ai.rs`'s three rebellion2 constants (`DIPLOMACY_SKILL_THRESHOLD`,
    `DIPLOMACY_TARGET_POPULARITY_CAP`, `MAX_CONSTRUCTION_YARDS`) had no
    reader. They are removed with the stale heuristics header.
  - The tuning defaults are tagged `port:`. `max_construction_yards` was
    marked Parity, but the original has no yard cap.
  - The auto-resolve round cap is tagged `port:` and the headless battle
    retry `hyp:`.
  The baseline shrinks from 19 to 11. What remains is `combat.rs`: the
  family ranges, the phase flags and the fighter rules. They need the
  combat area's reading.
- 2026-10-21 (combat remainder, read only): the 11 `combat.rs` items rest
  on `ghidra/notes/space-combat.md`, whose seven "phases" are not combat.
  In the typed corpus, `FUN_005442f0`, `FUN_00544030`, `FUN_00544130`,
  `FUN_00544da0` and `FUN_005445d0` are `MissionMgr` methods.
  - They test the story flags at `+0x58`: LukeDagobahRequired,
    HanCapturedAtPalace and FinalBattle.
  - They set LukeDagobah, HanCapturedAtPalace, LukePalace,
    FinalBattleReady and DarthPickup.
  The phase flags and the pipeline citations need the combat area's
  re-read; they are not tagged here.
- 2026-10-21: the invented support bands are removed (save v101).
  - `SupportTier` (edges 20/30/40/60), `SystemEconomy::last_support_tier`
    and `SupportChanged` had no recovered source, and only the telemetry
    read them.
  - `0x100` is `GameObjControlKindNotif`, ported as System Joins in
    `dfb840f2`.
  - Golden initial `v1:eb6515d682ec8a5d` -> `v1:b77445f2a1a777c0`, final
    `v1:eea438fc7e6c6fbc` -> `v1:4a994816eae0685d`.
  - The 24 campaigns lose only their 1270 `support_change` records
    (`evidence/2026-10-21-support-bands-removed/`).
- 2026-10-21 (combat remainder): the 11 `combat.rs` items are tagged
  `hyp:` with their reason. Comments only.
  - The phase-flag register is read from `MissionMgr` story steps (above).
  - The auto-resolve rules are the port's; the strategic combat is
    unrecovered (F-033).
  - The family table conflicts with the recovered iterators
    (`coverage-map.md`, item 6).
  `provenance-scan check` passes with an empty baseline, so F-031 is
  remediated. The `hyp:` items stay open under F-033.
- Acceptance: each item is recovered and cited, or tagged `port:`/`hyp:` with a
  reason; `provenance-scan check` passes and the baseline shrinks to the
  port-owned remainder.

### F-032: Fleet transit time and bombardment rest on an invented formula and a misread

- Severity: P1
- Status: partially remediated (transit)
- Evidence:
  - movement.rs computes ceil(distance * DISTANCE_SCALE 2 / slowest
    hyperdrive) with MIN_TRANSIT_TICKS 10 and DEFAULT_FIGHTER_HYPERDRIVE 60,
    none sourced
  - The original moves each object on its own: FUN_00514a60 -> FUN_00556430 ->
    FUN_0055d8c0, ticks = max(1, isqrt(dx^2+dy^2) / GNPRTB 5120 * speed(+0x34)
    / 100), arrival event 0x387 (ghidra/notes/build-delivery.md)
  - bombardment.rs and ghidra/notes/bombardment.md read that transit chain as
    a bombardment formula: the short pair from FUN_00509620 is the system
    position, not combat strength; the real bombardment function is
    unrecovered
  - blockade.rs matches its decompiles; fog.rs has no source and is port-owned
    until tagged
  - evidence/invention-review-r1/movement.md
  - Transit remediated 2026-09-28: movement.rs ports FUN_0055d8c0 with the
    original isqrt (FUN_0053e1d0), GNPRTB 5120 and 1, the ship speed
    FUN_00500820, and the fleet speed FUN_004fd900. A fleet without a capital
    ship cannot enter hyperspace (FUN_004fda10). The per-ship damage nibble is
    not modelled. Bombardment and fog remain open.
  - Bombardment read 2026-10-16 (`ghidra/notes/bombardment-order.md`): the
    Fleet menu's orders `0x220..0x223` send a bombardment that the game
    resolves in `FUN_005637c0` -> `FUN_0058d800` -> `FUN_0058d770`: a Death
    Star destroys the system (`0x223`); the ion cannons and batteries fire
    at the bombarding ships; firepower less the shields gives shots, each
    destroying a drawn target whose bombardment defense loses to a 1-10
    draw; civilian damage costs support. The original never bombards after
    a battle on its own.
  - Bombardment ported 2026-10-16 (save v55, `bombardment.rs`): the four
    phases replace the transit misread; the automatic bombardment after a
    battle is gone; the AI's plan objects order bombardments and the
    player's Bombard button orders General Bombardment (the sub-menu is
    next). port: the draws come from a stream seeded by the tick; no officer
    ranks (both leadership factors 1); the sector and galaxy support events
    are not ported (`evidence/2026-10-16-bombardment/`).
  - Bombardment sub-menu 2026-10-16 (module 8, stage 4f): the object
    pop-up menu's Planetary Bombardment is a parent (`FUN_00442590`) whose
    window opens beside it (`FUN_004ab3a0`) on the menu's 500 ms timer
    (`FUN_004aab50`); a fleet or a fleet icon lists the four orders, each
    enabled by `bombardment::order_enabled`, and a chosen one is carried out
    at once (`FUN_00486fb0` -> `FUN_00487740`). A browser gate orders General
    Bombardment from it for both sides
    (`evidence/2026-10-16-bombardment-menu/`).
  - Destroyed system's contents 2026-10-20: the system's destruction
    (`FUN_005080d0`, destroyed hook `FUN_00510950` -> `FUN_004fb520`) calls
    slot `+0xb4` on each direct child: units and facilities are destroyed
    (`FUN_004f8710` -> `FUN_004f7ed0`), a fleet stays with its ships and
    the characters aboard (`FUN_00524fb0`), and a character on no mission
    goes adrift (`FUN_004ef2e0`). `FUN_0058d990` then kills each minor
    character standing there that GNPRTB `0x606` (50) hits. The port had
    removed the fleets, killed everyone aboard and cancelled the trips
    there; `cleanup_destroyed_system` and
    `bombardment::destruction_hits` now follow the original (manual p. 96).
    Campaigns unchanged: the AI orders no Destroy System
    (`evidence/2026-10-20-death-star-destruction/`).
  - 2026-10-21: the per-ship damage counters are read
    (`ghidra/notes/ship-subsystem-damage.md`).
    - A capital ship keeps six 4-bit hit counters at `+0x64`: shield,
      weapon, tractor, speed, and primary and backup hyperdrive.
    - A battle starts from them (`FUN_0040b8a0`) and writes them back
      (`FUN_0040cfc0`).
    - A hit primary hyperdrive makes the ship travel at its
      `hyperdrive_if_damaged`; both hit give the default speed
      (`FUN_00500820`).
    - Each repair fire lowers one of the counters.
    - The port keeps none of them.
  - 2026-10-21 (D2, save v103): the counters are ported
    (`evidence/2026-10-21-ship-hit-counters/`).
    - `ShipInstance::subsystem_hits` keeps `+0x64`; a battle starts from it
      and writes it back.
    - `movement::ship_speed` follows `FUN_00500820`.
    - 20 campaigns change (battles 2080 -> 2146); the golden is regenerated
      for v103.
  - 2026-10-21 (D3): repair mends subsystem hits
    (`evidence/2026-10-21-ship-hit-repair/`).
    - A ship with any hit counts as Damaged (`FUN_00501c00`).
    - Each fire, on a roll under `damage_control`, lowers one hit chosen
      evenly (`FUN_00501c50`).
    - `port:` the draws come from SplitMix64.
    - All 24 campaigns change (battles 2146 -> 2025).
  - 2026-10-21 (D4): the AI's ship record uses the recharge after hits
    (`FUN_004738d0`, `FUN_00473910`, GNPRTB 2048; evidence
    `2026-10-21-ai-recharge-hits/`). 14 campaigns change (battles
    2025 -> 1855).
  - 2026-10-22 (D5): the Capital Ship Status rows show the ratings after
    hits and hull damage (`ship_condition.rs`: `FUN_00500e60`,
    `FUN_005010c0`, `FUN_00501270`, `FUN_00500f00`, `FUN_00500f60`;
    evidence `2026-10-22-ship-status-hits/`).
- Acceptance: Transit follows FUN_0055d8c0 per object with a failing-without
  test; bombardment.rs is removed or rebuilt on a recovered function; fog.rs
  is tagged port:.

### F-033: Strategic combat formulas are port-authored

- Severity: P1
- Status: open
- Evidence:
  - combat.rs reads GNPRTB 5120 (0x1400) as a combat difficulty modifier in
    space and ground combat; 5120 is the transit distance divisor
  - Weapon variance, fighter dogfight (atk_str + maneuver / 2, 0.4 attrition),
    anti-fighter screening, and the ground hit formula have no source; the
    original math sits in unrecovered ship-class slots +0x1c4, +0x1c8, +0x1d0,
    +0x1d4
  - The Emperor 1.5x bonus cites FUN_00542050, a one-line runtime thunk;
    FUN_004ee350 is a strength setter, not the ground calculator
  - death_star.rs construction (1825 ticks) and warning radius (300) are
    estimates
  - evidence/invention-review-r1/combat.md
- Battle start (2026-10-15): a battle starts where both sides are present
  by a fleet or a squadron on the planet (`FUN_00509710` → BattlePending
  `FUN_0050a900`; the battle manager `FUN_00516d50` sets Battle); it used to
  need a fleet of each side. port: the planet's squadrons fight inside
  their side's first fleet or a group of their own, and the survivors land
  after it; the loser's autorouting (`FUN_00516ef0` → `FUN_004f78e0`) is not
  ported. The "space combat" pipeline `FUN_005457f0` → `FUN_00549910` is the
  mission manager's story path (`FUN_005438a0`), not the battle's
  (`evidence/2026-10-15-squadron-battles/`).
- 2026-10-21: the 5120 misuse is removed. GNPRTB 5120 (`0x1400`) is the
  transit distance divisor (`FUN_0055d8c0`); `resolve_space` had scaled
  every hull hit by 5 / 100. The modifier and its difficulty argument are
  gone, and a test fails with the scaling put back. 20 of 24 campaigns
  change: battles 960 -> 901, rounds 1855 -> 1650, capital ships lost
  451 -> 401. The golden has no battle
  (`evidence/2026-10-21-combat-5120-removed/`).
- 2026-10-21 (read, `ghidra/notes/battle-flow.md`): the original has no
  separate strategic combat formula.
  - `FUN_0040a050`'s states ask each side's player for the battle type
    (`FUN_004362b0`).
  - Tactical (state 7) and abstract (state 8) both build the battle
    (`FUN_0040b0e0`) and run the tactical module (`FUN_00596820` ->
    `FUN_00597530(10)`, `FUN_0059a530`).
  - `FUN_0040c780` reads its result (`FUN_005969d0`), stores the winner
    (`FUN_0050ceb0`) and applies the losses to fleets, ships and
    squadrons.
  - The acceptance's four slot handlers belong to the tactical module. The
    port's auto-resolve stays `hyp:` until it runs the tactical simulation
    without display.
- 2026-10-21 (read, second pass): the abstract battle steps the tactical
  simulation without display.
  - The mode flag (battle object `+0xc`) reaches the module's `+0x668`
    (`FUN_0040b0e0` -> `FUN_00595d60` -> `FUN_00599c90` / `FUN_0059a0b0`).
  - `FUN_0059a530` then calls `FUN_005ae9c0` at a step of 1.0. Each side
    runs its combat step (slot `+0x14`) and its beaten test (slot `+0x28`).
  - After 1200 (`DAT_0066c3e0`) with neither side's strength (`+0x3c`)
    falling, the weaker side ends (`FUN_005af7a0`; see the next entry: it
    is destroyed unless it can leave).
  - The port's tactical simulation lives in `rebellion-render`
    (`tactical_view.rs`, `BattleSession::step`). Running it for
    auto-resolve needs it moved to `rebellion-core`: an architecture change
    to agree first.
- 2026-10-21 (move, agreed): the tactical battle state and step now live in
  `rebellion-core::tactical`, with no change in behaviour.
  - Moved: `BattleSession` with `step`, the ships, fighters, Death Star,
    RNG, damage, subsystem, order and voice-cue types, and the
    `tactical_resources` joins. Drawing, input, the battle windows and the
    HUD art stay in `rebellion-render::tactical_view`, which re-exports the
    moved names.
  - The two Death Star log lines go to `BattleSession::diagnostic_log`; the
    view prints them right after `step`, the effects advance and the shot.
  - The provenance scanner now sees the moved code: its 100 uncited items
    carry their source (`FUN_`, from `ghidra/notes/tactical-*.md`), `port:`
    for the 2D fallback layout and the port's step, or `hyp:` where the
    writer is not located.
  - 24 of 24 campaigns and the golden are unchanged; 63 of 64 tactical
    screenshots match, the 64th being the in-flight beam frame that differs
    on every run (`evidence/2026-10-21-tactical-sim-to-core/`).
  - Next: auto-resolve runs this simulation (step 1.0, 1200 stalemate).
- 2026-10-21 (read and probe, `ghidra/notes/abstract-battle.md`): the
  port's tactical simulation cannot yet resolve a battle on its own.
  - Each side of an abstract battle is a commander object (vtables
    `0x66c428` / `0x66c3f8`). Its step `FUN_005d14d0` gives the orders
    (`FUN_005c9200`), runs the Death Star, the field damage and the
    retreat, and updates every unit (`FUN_005b6320`).
  - Strength is weapon firepower in the bearing arc, scaled by hull
    (`FUN_005d0ce0`, `FUN_005b7a80`).
  - At the 1200 stalemate, `FUN_005af7a0` destroys the weaker side (state
    `0xb`, 100,000 damage) unless it can leave by hyperdrive (slot
    `+0xc`). It is not a withdrawal.
  - Probe: the port's `BattleSession` run headless on all 67 battles of
    seeds 1-3 dealt no damage in 40,000 steps, because its units act
    only on a player's order.
  - Remaining, in order:
    1. the side commander (`FUN_005d14d0`, `FUN_005c9200`);
    2. the strength;
    3. the abstract step and its result.
- 2026-10-21 (W2d, `crates/rebellion-core/src/abstract_battle.rs`): the
  abstract battle engine is ported, not yet used by the campaign.
  - Each step is one second (`DAT_0066badc`; rates per second through
    `FUN_005a9de0` × 0.001).
  - Each task force with a target deals its summed rate (`FUN_005cb650`,
    `FUN_005b7780`, the arc from `FUN_005b4c60`) through `FUN_005b54d0`.
  - Units recharge (`FUN_005b6320`), and a side with no active unit is
    beaten (`FUN_005d14d0`).
  - Strength leaves out out-damaged ships (`FUN_005d0ce0`,
    `FUN_005b7a80`).
  - 1200 seconds without a fall destroys the weaker side (`FUN_005ae9c0`,
    `FUN_005af7a0`).
  - `hyp:` targeting, until `FUN_005ec740` is ported (read into
    `abstract-battle.md`).
  - Probe over seeds 1-3: all 67 battles end, 28 attacker and 39 defender
    wins, no stalemate, 1 to 1008 steps (median 43).
  - 22 tests; mutants 189 caught, 5 missed, one closed by a test and four
    equivalent (`evidence/2026-10-21-abstract-battle-engine/`).
- 2026-10-21 (W2e): the abstract battle's targeting is ported in place of
  its `hyp:` rule.
  - The enemy list runs by priority, 15 × strength (`FUN_005d1a70`,
    `FUN_005ed8e0`).
  - Laser-led task forces take the fighter list, the others the ship list
    (`FUN_005c9200`).
  - A task force keeps a target under a combined score of 300
    (`FUN_005ed5f0`). Eligible targets are armed (`FUN_005ed550`).
  - Score: (hull × 0.65 + shields) ÷ (rate − shield recharge) + travel
    (`FUN_005ed170`).
  - Pairing (`FUN_005ec740`): alone in 0..180, otherwise combined from the
    tail with the head counted but not linked, then the idle take the
    first eligible target.
  - Probe over seeds 1-3: 67 of 67 battles end, 26 attacker and 41
    defender wins, no stalemate; 55 match the `hyp:` rule
    (`evidence/2026-10-21-abstract-targeting/`).
- 2026-10-21 (W2f): every strategic space battle the port resolves on its
  own is now an abstract battle (`FUN_0040a050` state 8).
  - The integrator puts every fleet of both sides at the system into one
    battle (`BattleSession::new_for_fleets`, `FUN_0040c780`).
  - The app's AI battles and the player's Auto Resolve use the same
    engine. `CombatSystem::resolve_space` decides nothing now.
  - 24 of 24 campaigns change:
    - battles 901 -> 1043;
    - draws 258 -> 1, stalemates 236 -> 3;
    - Alliance wins 118 -> 256, Empire wins 525 -> 786;
    - capital ships lost 401 -> 502.
  - No campaign reaches victory, and the golden has no battle
    (`evidence/2026-10-21-abstract-battle-wired/`).
  - Open:
    - the retreat decision (slots `+0x0c`, `+0x2c`);
    - the Death Star's abstract actions;
    - removing the unused round formula;
    - ground combat.
- 2026-10-21 (W2g): the unused round formula is removed. `combat.rs` now
  holds only the result types the strategic layer applies.
  - Removed: `CombatSystem::resolve_space` and its phases (weapon variance,
    fighter dogfight, screening, recall), `CombatPhaseFlags`,
    `CombatEntityKind` and `extract_difficulty`. All were `hyp:` and none
    was called.
  - Removed with them: 25 tests of that formula.
  - 24 of 24 campaigns are unchanged, and so is the golden
    (`replay_manifest` 2 passed).
  - The release WASM is byte-identical
    (`9cda77d995e1a6f21acaa78c85bc0f385b66b299eabebe87ca72f2cffc288f73`).
  - Browser gates, on Arch Chromium 155.0.8059.84 (the system's update
    the same day):
    - replay equivalence and perf pass;
    - Fleet window 58 of 58;
    - status window pass;
    - tactical Death Star scenarios 4 of 4.
  - Workspace tests: 2524 passed; the 25 removed tests account for the
    difference. provenance-scan: 0 uncited.
  - The agent docs now point to `abstract_battle.rs`.
- 2026-10-21 (W2h): the abstract battle's retreat is ported
  (`abstract-battle.md`, "Retreat").
  - A side that may leave (commander `+0x80`) and has a working hyperdrive
    leaves when its strength falls under 0.33 of the enemy's, or to 0
    (`FUN_005d0f50`, `FUN_005d19a0`).
  - At the stalemate, a weaker side that can leave is marked rather than
    destroyed.
  - In the retreat (`FUN_005d14d0`), ships with a hyperdrive escape and
    the others are destroyed.
  - `+0x80` comes from the battle's `+0x10` / `+0x14`, the sides' answers
    to the battle choice (`FUN_004362b0`). What the answers mean is open,
    so the port allows no retreat yet and campaigns do not change.
  - 6 new tests; mutants on the diff 37 caught, 2 equivalent.
- 2026-10-21 (W2i read): the battle choice is read (`abstract-battle.md`,
  "The battle choice").
  - Battle Alert's Retreat (`0x67`, text 6474) answers "simulate" and sets
    the player's retreat word to 10000.0 (`FUN_00450540`, `FUN_00487830`).
    Simulate Results (`0x65`) answers "simulate" with no retreat, and Take
    Command (`0x66`) asks for the tactical battle.
  - The AI always answers "simulate". Its strategy module `0xd` sets the
    retreat word: 0.9 by default. At its own system with active squadrons
    the word is its fleets' attack over the enemy's, capped at 0.9, and 0
    at its headquarters (`FUN_00485d50`, `FUN_004e77f0` and six copies).
  - The port's Retreat opens the tactical battle with ships marked as
    retreating. That differs from the original and is the next stage's
    work. Read only; no code changes.
- 2026-10-21 (W5 read): the winner and the loser's withdrawal are read
  (`battle-flow.md`, "The winner and the loser's withdrawal").
  - The seven result words (`FUN_005ae9c0`): words 1 and 3 mark a side
    with no active unit, and words 0 and 2 a side whose retreat ran. They
    are not each side's survivors, as first read.
  - `FUN_0040c780`: the side that still has active units wins. A side
    that retreats loses. A tie goes to the system's `+0x78` bits 6..7,
    the side of the last fleet or squadron to arrive (`FUN_00508660` ->
    `FUN_0050a1b0`).
  - `FUN_00516ef0` sets every fleet and planet squadron of the loser
    Autorouting. The event `0x307` handler (`FUN_00515480`) sends each to
    the autoroute search's destination (`FUN_005545e0`).
  - The port counts escaped units as survivors and sends no loser away.
    Read only; no code changes.
- 2026-10-21 (W2j): the winner rule and the loser's withdrawal are ported
  (`evidence/2026-10-21-battle-loser-withdrawal/`).
  - `AbstractBattle::finish` builds result words 0 to 3 and takes the
    winner from them (`FUN_005ae9c0`, `FUN_0040c780`). `port:` with both
    retreat words set there is no winner, because the last arrived side
    is not kept.
  - `withdraw_loser` sends the loser's fleets to the autoroute search's
    destination (`FUN_00516ef0`, `FUN_00515480`, `FUN_005545e0`). It runs
    in the headless simulation, the app's AI battles, Auto Resolve and
    after a tactical battle.
  - No side may retreat yet, so 24 of 24 campaigns are unchanged.
  - Correction to the W5 read entry: the port's winner already counted
    only active units.
  - 4 new tests; mutants 32 caught, 0 missed.
- 2026-10-21 (W2k): the battle choice's retreat orders are ported
  (`evidence/2026-10-21-battle-retreat-orders/`).
  - An AI with a strategy orders a retreat except at its own system with
    squadrons on the planet, where it needs armed own fleets and never
    retreats from its headquarters (`ai_strategy::orders_retreat`,
    `FUN_00485d50`, `FUN_004e77f0`).
  - The Battle Alert's Retreat (`0x67`) runs the abstract battle with the
    player's order; it no longer opens the tactical battle.
  - The integrator's winner is the battle's (`FUN_0040c780`), so a side
    that escaped and is still in orbit loses and is sent away.
  - All 24 campaigns change:
    - battles 1043 -> 2092, 800 of them without losses;
    - draws 1 -> 0;
    - capital ships lost 502 -> 31;
    - seed 24 ends with the Alliance taking Coruscant on day 1350.
  - `port:` Take Command does not yet pass the AI side's order to the
    tactical engine.
  - 7 new tests; mutants 29 caught, 0 missed.
- 2026-10-21 (W2l, save v102): the system's last arrived side is ported
  (`evidence/2026-10-21-last-arrived-side/`).
  - `System::last_arrival` (`+0x78` bits 6..7, `FUN_00508660` ->
    `FUN_0050a1b0`) is set when a fleet departs for a system, a new fleet
    or completed unit appears there, or a squadron lands.
  - A battle with both retreat words set goes to that side
    (`FUN_0040c780`). With no winner and both sides present, the other
    side leaves (`FUN_00516ef0`).
  - The golden is regenerated for this cause (`v1:0174e1ddb8a28d5b` ->
    `v1:00f7cdfb6462bcb2`). Campaigns are unchanged: no tie came up.
- 2026-10-21 (W2i read): the Death Star in the abstract battle is read
  (`abstract-battle.md`, "The Death Star in the abstract battle").
  - The Empire's superlaser charges by dt x 1/3 x 0.001 (`FUN_005ba7f0`).
    At the abstract step of 1.0 it needs 300,000 steps, so in practice it
    never fires before the 1200-step stalemate. At full charge it would
    destroy the first capital ship in the enemy priority list.
  - The Alliance, with more fighter groups than the Empire and an
    attackable Death Star, sends its first fighter task force on the
    trench run (`FUN_005aefc0`).
    - Each visit rolls 1..10 against maneuverability for damage, then
      1..100 against (torpedo x 0.5333 x hull ratio + commander value,
      1..9) x 0.5.
    - Up to 100 visits; then the members left are destroyed.
    - Success zeroes the Death Star's hull and destroys the units it
      carries.
  - Open: the commander value's source field and the Death Star's carried
    lists. Read only; no code changes.
- 2026-10-21 (W2i): the Death Star's abstract actions are ported
  (`evidence/2026-10-21-death-star-abstract/`).
  - The superlaser charges 1/3000 a step and fires at 100, so in practice
    never.
  - The Alliance's trench run (`FUN_005aefc0`) can destroy the Death
    Star, which takes its capital ship from the fleet.
  - `hyp:` the leader kind 4 is a fighter group; the commander term is
    1.0. `port:` the carried units are unknown.
  - Seed 3 changes: on day 1340 the trench run destroys the Death Star at
    Orto. The other 23 campaigns are unchanged.
  - 11 new tests; mutants 57 caught, 0 missed.
- 2026-10-21 (W5b): the loser's planet squadrons leave on Autorouting
  too (`FUN_00516ef0`, `evidence/2026-10-21-planet-squadrons-withdraw/`).
  - The app's battles land scrambled squadrons back before the
    withdrawal, as the simulation does.
  - Nine campaigns change: battles 2092 -> 2080, Alliance wins 755 -> 782.
  - `port:` the winner's squadrons (`FUN_00556710`) are not read.
  - 2 new tests; mutants 2 caught, 0 missed.
- 2026-10-21: the trench run's commander term is ported
  (`trench_commander_term`, `evidence/2026-10-21-trench-commander/`).
  - The Alliance's best Commander by combat gives combat / 20 + 1, clamped
    to 1..9 (`FUN_005afc00`, `FUN_0040c200`, `FUN_00509530`).
  - Campaigns are unchanged. 3 new tests; mutants 17 caught, 0 missed.
- Acceptance: The four slot handlers and the ground caller are decompiled and
  ported with citations, the 5120 misuse is removed, and each formula has a
  failing-without test.

### F-034: Mission resolution departs from the recovered phases

- Severity: P1
- Status: open
- Evidence:
  - Detection collapses into one defense score fed to a rebellion2 quadratic;
    the original rolls FOILTB per defender with a composite input
    (FUN_0058a130)
  - Escape uses loyalty alone as its ESCAPETB input; the original input is a
    four-operand composite (community address, needs remap to our build)
  - Sabotage destroys a facility where the effect carries ticks_lost; effect
    sizes (ticks_lost 10, popularity 0.05, Death Star delay 50, diplomacy
    0.01) and MISSNSD tick ranges are unverified
  - Uprising leadership reads one agent; FUN_00520cd0 averages every team,
    decoy, and captured member; the incite effect skips the support machine
    FUN_0050c9f0
  - evidence/invention-review-r1/missions.md
- Re-check (2026-10-14): the leadership average over every team, decoy, and
  captured member (`FUN_00520cd0`) is ported as `uprising.rs`
  `uprising_mission_terms` with three tests. The other bullets were not
  re-checked.
- Research (2026-10-14): the escape original is recovered in
  `ghidra/notes/captive-escape.md`. Timer `0x38a` rolls each captive every
  GNPRTB 2584 + rand(0..=2585) after capture; the ESCAPETB input is
  espionage + combat minus the average combat of the captor's free
  characters and the count of the captor's usable regiments at the
  captive's location; success clears Captured and restores the home side.
  The freed character then autoroutes from where it was held to the
  nearest unblockaded system of its own side (falling back to a friendly
  fleet, a blockaded friendly system, a neutral one, then an enemy one).
- Fix (2026-10-14): `MissionSystem::advance_escapes` ports that rule and
  replaces the loyalty roll (save v35). 16 tests cover the timer, the
  ESCAPETB step, the input terms, the destination order and distance, and
  the integrator's trip; scoped `cargo mutants` catch all 63 viable mutants.
  Five 5000-day dual-AI playtests (seeds 1 to 5) record 22 captures and 22
  escapes. The seed-42 golden moves for this cause and the save version:
  `v1:18b0d4ae14e00774` -> `v1:501c8c959ff47345`. The escape bullet above is
  closed; the container and `+0x70` candidates and the move validator stay
  open. Evidence: `evidence/2026-10-14-captive-escape/README.md`.
- Re-check (2026-10-20): every evidence bullet is now ported. Detection
  rolls FOILTB per defender (`mission_detection.rs`, `FUN_0058a130`), and
  the rebellion2 quadratic is gone from the code (its module doc was the
  last trace, rewritten). Sabotage destroys the named object
  (`FUN_0056a2d0`, `FUN_00574600`); no `ticks_lost` or invented effect sizes
  remain, and the timers come from the MISSNSD records. Incite starts the
  uprising incident (`FUN_0050d030`) and Subdue runs the support machine
  (`FUN_0050c9f0`). Members are team, decoy and captured lists
  (`FUN_0054c200`). Timer `0x38c` was read (`FUN_005229c0`, `FUN_005236e0`:
  the ended mission is deleted one day after its last member left); the port
  drops it on the day it ends (port:). Still open: the traitor's own fate
  and fields `+0xa0`/`+0x8c`, `FUN_004f9510`, and `+0xa4` bit 1, which lifts
  rule 1 (`ghidra/notes/decoy-roll.md`).
- Read (2026-10-20): the last open detection items. `+0xa4` bit 1, which
  lifts rule 1, is ImpliedTeam (`FUN_005221a0`); only the Autorouting order
  0x43 (`FUN_00547740`) sets it, so it is clear for every kind the port
  runs. The traitor's own fate: `FUN_00588da0` only stores its key, and the
  traitor is exposed with the team. +0x8c is the Force and +0xa0 the
  TraitorDiscovered key (notification 0x361, ported as TraitorIdentified).
  `FUN_004f9510` hands an event to a side's view (hyp: no rule). Nothing in
  F-034's evidence stays open; the status stays open until a review
  confirms the closing.
- Acceptance: Detection, escape, and effects follow their decompiles with
  failing-without tests; mission members become a roster (with F-019).

### F-035: Manufacturing and repair use invented structure and rates

- Severity: P2
- Status: remediated (2026-10-21)
- Evidence:
  - One ProductionQueue per system; the original keeps a manager per facility,
    so two yards build in parallel
  - No queue acceptance filter (FUN_00528890: created, not completed, not
    destroyed, same side); build progress rate per tick is unverified against
    FUN_0052b960 (now decompiled)
  - Repair uses the ship class damage_control; the original runs timer 0x386
    with GNPRTB 7693/7695 and a facility-derived rate (FUN_00509890.c:26),
    gates on the shipyard bit (+0x88 bit 5) rather than any facility, and
    repair runs per frame, not per tick
  - ai.rs:1325 treats refined_material_cost as build ticks
  - evidence/invention-review-r1/manufacturing.md
- Correction (2026-10-20): two evidence lines above are stale or misread.
  - The original keeps one manager per production area of a system (a yard
    finds it through `FUN_00509670`), and every usable, unsuspended yard of
    the area adds points to it, woken fastest first (`FUN_00529dd0`). The
    port has kept a queue per system and area with the same yard
    regulation since 2026-10-13 (`manufacturing.rs`, `YardDay::regulate`;
    `ghidra/notes/manufacturing-manager.md`). So two yards of one area
    already build in parallel, and the per-yard item is closed.
  - Repair is timer `0x380`, not `0x386`. Timer `0x386` is the blockade's
    support shift (`blockade.rs`). Repair is read in
    `ghidra/notes/ship-repair.md`.
- Repair ported (2026-10-20, save v98): `repair.rs` runs timer `0x380` per
  ship. A damaged ship that is not travelling repairs a tenth of its class
  hull every 2 days (GNPRTB 2052) beside a usable, unsuspended yard of its
  side, else every 30 (2053). A FastRepair change rebuilds the timer. The
  invented per-day `damage_control` rule at any manufacturing system is
  gone. 20 of 24 campaigns change
  (`evidence/2026-10-20-ship-repair-timer/`).
- Closed (2026-10-21): the two remaining evidence lines are stale too.
  - The acceptance filter `FUN_00528890` keeps in a manager's queue only
    products that exist and are not completed, destroyed or en route
    (`manufacturing-manager.md`, "Queue membership"). The port's queue
    holds orders, and it makes an object only at completion
    (`apply_build_completion_inner`) or at a delivery's arrival
    (`delivery.rs`). So nothing completed, destroyed or en route is ever
    in it.
  - `ai.rs` passes a class's refined cost as the order's cost in points;
    each point is one yard cycle (`FUN_00530950` -> `FUN_0052a430`), as in
    the original. That is not a tick count.
  - The queues, points and completion follow `FUN_0052b960` (tests since
    2026-10-13), and repair follows timer `0x380` (the acceptance's `0x386`
    is the blockade timer), each with failing-without tests.
  - The port's remaining differences are recorded as port: in
    `manufacturing-manager.md` ("The port"): the daily walk of the
    managers, and the yard stopped in a cycle.
- Acceptance: Queues, progress, and repair follow FUN_0052b960 and timer 0x380
  (read 0x386 when written) with failing-without tests.

### F-036: The AI polls on an invented cadence with invented weights

- Severity: P2
- Status: open
- Evidence:
  - The original AI is driven by notifications and timers; there is no daily
    AI tick and 0x1f0 is a mission UI message (ghidra/notes/timer-
    scheduler.md)
  - AI_TICK_INTERVAL 5 and AiConfig tick_interval 7 are invented; 15 further
    constants (deploy budgets, force ratios, scoring weights, 11 duration_roll
    0.5 placeholders) have no source, and DIPLOMACY_SKILL_THRESHOLD comes from
    rebellion2
  - The aggression curve cites FUN_0053e190 but is linear; system_strength
    weights facilities by 10 where FUN_00502020 does not
  - evidence/invention-review-r1/ai.md
- Cadence (2026-10-14, stage 1, save v38): the first bullet is corrected.
  The AI player's tick `FUN_00485b40` runs once a sub-tick from frame-loop
  phase 8 (`FUN_0041dff0` -> `FUN_0041cf20` -> `FUN_004361a0`; phase 2,
  `FUN_0041e300`, waits for the next sub-tick). A day therefore holds
  600/60/12/4 AI passes by speed (`FUN_0051dea0`), with 10 orders a day
  (`+0x428`, `FUN_00485a50`) cleared at the day boundary (`FUN_00485de0`).
  Ported as `ai::passes_per_day` and `AISystem::pass`, applied pass by pass,
  one side then the other; the port's heuristics run as modules in round
  robin (`FUN_004be980`) and the research planners one per step
  (`FUN_004d2730`). `AI_TICK_INTERVAL` and `ai.tick_interval` are removed.
  The officers module now targets only systems the Diplomacy and Incite
  records admit (`FUN_00523450` rule 3, `FUN_005868c0`), because a refused
  target was ordered again on every pass. Golden `v1:eb82b46bf03b0753` ->
  `v1:4225657ae5e05393` (`evidence/2026-10-14-ai-cadence/`). Open: the
  fourteen modules' identities, their priorities (`+0x34`, `FUN_004bede0`)
  and one-order steps.
- Game loop (2026-10-14): the browser build runs the cadence in the app's
  loop (`[ai]` console line per day): 4 passes a day at Fast and 600 at
  Very Slow, where the longest frame was 18.7 ms. Repeated Sabotage orders
  are detected missions retried, as the original planner does up to 1000
  failures (`FUN_00480df0`) (`evidence/2026-10-14-ai-cadence-game-loop/`).
  Native GUI not run.
- Modules (2026-10-14, stages 2 and 3, save v39): the fourteen modules are
  identified by their orders and record marks (`ghidra/notes/ai-mission-
  planning.md`, "The fourteen modules"), and the port runs them in the
  original order with priorities (`FUN_004bede0`), one order a sub-tick
  (`FUN_00485b40`), and the mission module one entry a step
  (`FUN_004d2730`). The port's heuristics stand in for the modules' contents;
  modules 1, 6, 7, 9, `0xa` and `0xe` order nothing. Golden
  `v1:e271a795cd483ebe` -> `v1:33233aedba63db5c`
  (`evidence/2026-10-14-ai-modules/`).
- World model (2026-10-14): the AI's records the production modules query
  are ported in `ai_records.rs`. That covers class records (`FUN_0042dd00`), fleet records
  (`FUN_00433620` over `FUN_00484630`, `FUN_00433780`), system records
  (`FUN_00433e40`), sector records (`FUN_004321e0`, `FUN_00432350`) and the
  sector and system queries (`FUN_00419a00`, `FUN_00431830`, `FUN_004357b0`)
  (`ghidra/notes/ai-galaxy-records.md`). Nothing reads them yet; the golden
  is unchanged (`evidence/2026-10-14-ai-galaxy-records/`). Module 1's first
  read is in `ghidra/notes/ai-production-modules.md`.
- Scrap order (2026-10-14): `0x200` (`FUN_0053dd20`) is ported for
  facilities and regiments in `scrap.rs`. It checks the object
  (`FUN_004f9860`; a headquarters refuses with `0x17`), destroys it with
  reason `0x14` (`FUN_00534c00`), and refunds half the class's refined cost
  (`FUN_00530270`, reasons `0x14..0x16`). `AIAction::Scrap` applies it. No
  module issues it yet; the golden is unchanged
  (`ghidra/notes/scrap-order.md`, `evidence/2026-10-14-scrap-order/`).
- Yard marks (2026-10-14): the system records' marks `+0x28 & 0x3e00000`,
  which `FUN_00433e40` keeps across refreshes, are kept in
  `AIState::yard_marks` (save v40) and feed `Galaxy::build`. Golden
  `v1:e271a795cd483ebe` -> `v1:e187411365b9f348` for the save layout only
  (`evidence/2026-10-14-ai-yard-marks/`).
- Distance queries (2026-10-14): `FUN_00419b80` (sectors) and
  `FUN_0041a400` -> `FUN_00431c00` (a sector's systems), keyed by the
  squared distance of the records' map positions (`FUN_0041b7b0`), are
  ported as `Galaxy::{sectors_by_distance, systems_by_distance}`. Module 1
  reads them in order 2 through `FUN_004357b0`, which gives the nearest
  (`evidence/2026-10-14-ai-distance-queries/`).
- Records in each pass (2026-10-14): `AISystem::pass` rebuilds the side's
  records as phase 4 (`FUN_00418500`, `AISystem::records`) with the yard
  marks, the planner's person flags and side `+0xa4`, and hands them to
  the modules. Golden unchanged; in the browser, Very Slow's longest frame
  was 2.6 ms and Fast's 1.6 ms (`evidence/2026-10-14-ai-records-pass/`).
- Module 1 (2026-10-15): the construction yard, mine and refinery module
  (`FUN_004da950`, step `FUN_004dabe0`, cycle `FUN_004db2f0`) is ported in
  `ai_module1.rs`, with the yard token (`FUN_004199b0`), the Scrap team,
  Destination and Build orders, their validation and refusal
  (`FUN_004dd810`), and the scheduler's completion rule (`FUN_00485b40`).
  Save v41; golden `v1:e187411365b9f348` -> `v1:ada51196c4d497ba`. port:
  the seven other yard-token modules are not ported, so their turns pass
  at once (`evidence/2026-10-15-ai-module-1/`).
- Module 2 (2026-10-15): the second construction yard module
  (`FUN_004e1e50`, step `FUN_004e2120`) is ported in `ai_module2.rs`. It
  keeps its own sites (mark `0x1000000`) and runs a mine or refinery
  cycle (`FUN_004e25c0`) or a yard cycle (`FUN_004e2400`) that borrows
  module 1's nearest idle yard with the token `0x40`. Its shared steps are
  module 1's, read as identical in assembly. The production heuristic's
  manufacturing facilities move from module 2 to `0xa`. Save v42; golden
  `v1:ada51196c4d497ba` -> `v1:09ab7bb25d2c1611`
  (`evidence/2026-10-15-ai-module-2/`).
- Module 3 (2026-10-15): the defense balance module (`FUN_004d6b20`,
  step `FUN_004d6d30`) is ported in `ai_module3.rs`: a surplus cycle that
  scraps defenses beyond the records' goals (`FUN_004d6da0`), a defense
  and mine cycle at its own sites (`FUN_004d6e70`, Coruscant first,
  `FUN_00418240(4)` raising the shield goal), and a yard cycle with the
  token 1 (`FUN_004d7060`). The heuristic's defenses, which stood in for
  it, are gone; on seed 42 module 3's yard budget stays below one
  construction yard, so the AI builds no defenses there. Save v43; golden
  `v1:09ab7bb25d2c1611` -> `v1:76a53e974120e2d1`
  (`evidence/2026-10-15-ai-module-3/`).
- Module 5 (2026-10-15): the special force module (`FUN_004cfb20`, step
  `FUN_004cfed0`) is ported in `ai_module5.rs`: a cycle that trains the
  four special force kinds in turn at its training facility sites
  (`FUN_004cff20`, budget 70% of 4% of the capacity less their upkeep),
  and a yard cycle that builds training facilities with the token 4
  (`FUN_004d0010`). The shared steps take a yard kind (construction or
  training). Two original quirks are kept: no record carries `+0x34` bit
  `0x800000`, so every kind counts as short, and none carries flag
  `0x2000`, so no Retire is given. The heuristic's regiments now stand in
  for module 6. Save v44; golden `v1:76a53e974120e2d1` ->
  `v1:ed967029485e168a` (`evidence/2026-10-15-ai-module-5/`).
- Module 6 (2026-10-15): the regiment module (`FUN_004dd460`, step
  `FUN_004dd7a0`) is ported in `ai_module6.rs`: Move `0x201` of a
  surplus's weakest regiments to a system short of garrison
  (`FUN_004de6f0`, `FUN_004deba0`; `AIAction::MoveRegiments`, validated by
  `TroopTransportState::check_move_regiment`), training regiments at its
  training facility sites and sending them on (`FUN_004dd8d0`), and the
  training facility yard cycle with the token `0x20` (`FUN_004dda00`).
  port: the two moves that ask module 8's sector planners, and the
  strategy-driven garrison goal changes, give nothing. The heuristic's
  regiments are gone. Save v45; golden `v1:ed967029485e168a` ->
  `v1:6d35df6768400d72` (`evidence/2026-10-15-ai-module-6/`).
- Squadrons on planets (2026-10-15), ahead of module 7: the system record
  counts the planet's squadrons (`FUN_00433e40`: `v[23..28]`, `v[32]`,
  `+0x28` bit `0x80000`, and so `v[38]`/`v[53]`, which are squadrons, not
  regiments, against the goal). `AIAction::MoveSquadrons` carries a
  squadron Move, validated by `squadrons::check_move`; the heuristic
  standing in for module `0xb` boards planet squadrons onto its fleets.
  port: a planet's squadrons take no part in battle yet. Save v46; golden
  `v1:6d35df6768400d72` -> `v1:d33088556c4445a3` (`evidence/2026-10-15-planet-squadrons/`).
- Module 7 (2026-10-15): the squadron module (`FUN_004d2ef0`, step
  `FUN_004d3100`) is ported in `ai_module7.rs`: a planet's surplus goes to
  a fleet with hangar room or a short system as Move `0x201`, else Scrap
  `0x200` (`FUN_004d3170`); squadrons are built at shipyard sites for the
  short system nearest (`FUN_004d3220`, Build `0x211`), raising the side's
  squadron goals `+0x388..+0x390` when none is short; and shipyards are
  built with the token 8 (`FUN_004d3350`). A squadron now travels between
  systems on its own and takes Scrap on a planet. port: fleet record bit 8
  and module 8's planners are not available. Save v47; golden
  `v1:d33088556c4445a3` -> `v1:da11bf56815cc333` (`evidence/2026-10-15-ai-module-7/`).
- Module `0xb` (2026-10-15): the capital ship module (`FUN_004c8820`, step
  `FUN_004c8b30`) is ported in `ai_module_b.rs`: damaged ships go to the
  nearest shipyard site (`FUN_004c8e90`, `AIAction::MoveShips`; ship
  records now take Damaged, `0x1000`), the Death Star returns to
  Coruscant (`FUN_004c94c0`), capital ships are built at its shipyard
  sites by role (`FUN_004c8c20`: troop carriers, the Death Star, armed,
  lasers, carriers, else any shipyard class, within 90% of 30% of the
  capacity less their upkeep) and the cheapest ship is scrapped over
  budget (`ScrapTarget::Ship`), and shipyards are built with the token 2
  (`FUN_004c8d20`). The heuristic's ships and fighters are gone. port: the
  move to a base, the planner-driven roles and `FUN_004c9cb0` need module
  8's planners and the fleet plan objects. Save v48; golden
  `v1:da11bf56815cc333` -> `v1:bde18d3a1df8ed02` (`evidence/2026-10-15-ai-module-b/`).
- Module `0xd`'s posture and sector plans (2026-10-15), stage 1 of module
  8: `ai_strategy.rs` ports the posture (`FUN_004bf490`: the fleets'
  attack against 1.5 times the opponent's, spare regiments against the
  weakest opponent sector, the economy and the yard requests choose one of
  seven strategies, setting controller `+4` `0x800000` or `0x400000`), and
  the strategy's states 1 and 2: each strategy names the sectors the side
  works on with a plan kind (slot `+0x14` of seven classes, the sector
  record's `+0x30`/`+0x3c`, `FUN_00419e90`) and reviews them
  (`FUN_004e5560`). Module 6 now raises and lowers the garrison goals
  `+0x37c..+0x384` under those bits. port: module 8's planners and their
  plan objects are not ported, so each side keeps one planned sector and
  no plan gives an order yet. Save v49; golden `v1:bde18d3a1df8ed02` ->
  `v1:cca1045386d40ca4` (`evidence/2026-10-15-ai-strategy-posture/`).
- Module 8's planners (2026-10-15), stage 2: `ai_module8.rs` ports the
  module's states 1 to 3 (`FUN_004cf630`): a planner is added while a
  planned sector lacks one and every planner has cycled, dropped when it
  holds no plan (`FUN_004cf700`); each cycle takes the next planner
  (`FUN_004cf760`), which keeps its sector or claims the nearest planned
  one, else one by a random key (`FUN_004cf7c0`, `FUN_0041a1d0`), and
  binds the plan's kind (`FUN_004773f0`). The strategy's review now reads
  the planners (`FUN_004e5560`). port: the plan objects are not ported;
  in state 4 the port's fleet heuristics stand in, so module 8 orders only
  while a planner holds a plan. Save v50; golden `v1:cca1045386d40ca4` ->
  `v1:6cc71d2663560368` (`evidence/2026-10-15-ai-sector-planners/`).
- Module 8, stage 3 (2026-10-15): the planners' infrastructure. A
  capital ship has an id (`ShipInstance::id`, numbered before the AI's
  passes); the AI keeps its ship records' kept bits and holder
  (`+0x24 & 0x1b020000`, `+0x30`, `+0x34`; `FUN_00484230`,
  `FUN_00484630`) by that id; fleet records list their ship records;
  `AIAction::CreateFleet` carries out Create Fleet `0x270`; the fleets
  near a sector (`FUN_0041a600`) are a shared query. The reading found
  that a sector planner holds ships (`FUN_00478e00`, `FUN_004790d0`) and
  gets no order reports. No module uses these yet. Save v51; golden
  `v1:6cc71d2663560368` -> `v1:1ec77e5da40c2f8f`
  (`evidence/2026-10-15-ai-ship-records/`).
- Module 8, stage 4a (2026-10-15): the sector planner's step
  (`FUN_004774c0`) in `ai_planner.rs`: the planner holds ships and fleets
  (`FUN_00478e00`, `FUN_004790d0`, `FUN_00478fd0`, `FUN_004791d0`),
  checks them against the records after an order, splits damaged ships
  off, weighs its forces against what kind 3's plan wants
  (`FUN_004b4920`), takes free ships of the nearest fleets or a later
  plan's, gives spare ones back (Create Fleet `0x270` with a target), asks
  for missions and movement, and reports; the strategy's review now
  grows and shrinks its plans. port: the plan object's step and the
  other kinds still run the fleet heuristics; no mission or movement
  planners. Save v52; golden `v1:1ec77e5da40c2f8f` ->
  `v1:030a4e7e7f7902e9` (`evidence/2026-10-15-ai-planner-step/`).
- Module 8, stage 4b (2026-10-16): kind 3's plan object steps
  (`FUN_004adb70`, states 1 to 6) in `ai_plan.rs`: it clears its sector's
  role marks, keeps or chooses a base (`FUN_004af8a0`, `0x40000`), unloads
  a held fleet's regiments at an own system (`FUN_004afce0`), boards the
  base's squadrons onto a held fleet there (`FUN_004b0120`,
  `AIAction::MoveSquadronTeam`), finds the next idle held fleet
  (`FUN_004b04c0`) and moves the flagship's fleet or sets down its people
  (`FUN_004b06b0`, `AIAction::MoveCharacters`); a reset gives up the
  base. port: states 7 to `0xe` (roles, attacks, bombardment) are still
  the fleet heuristics. In seed 42 the planners take bases but no state
  gives an order, so the campaign's events equal stage 4a's. Save v53;
  golden `v1:030a4e7e7f7902e9` -> `v1:a9d12433c6c44f1e`
  (`evidence/2026-10-16-ai-plan-object/`).
- Module 8, stage 4c (2026-10-16): kind 3's plan object runs to its end
  (`FUN_004adb70`, states 7 to `0xe`): its fleet attacks an enemy fleet it
  prevails over (`FUN_004b4560`, `FUN_004ae150`) or takes a system to
  bombard (`FUN_004b4730`, `FUN_004ae320`) and moves there
  (`FUN_004b0f90`); else it joins the nearest held fleet
  (`FUN_004b0c90`, `AIAction::JoinFleet`) or goes home (`FUN_004b4a20`);
  `FUN_004b1070` settles the roles at their targets, and state 1 marks
  them. The fleet record keeps its target `+0x2c`. The fleet heuristics
  no longer stand in for kind 3 and leave held fleets alone. port: the
  bombardments (`0x220`, `0x222`, `0x223`) are not given (F-032). In seed
  42 the Alliance's plan finds no target in its sector and gives no
  order; the heuristics' fleet moves fall from 28 to 15. Save v54; golden
  `v1:a9d12433c6c44f1e` -> `v1:dbb908594619dd4b`
  (`evidence/2026-10-16-ai-plan-roles/`).
- Module 8, stages 4d and 4e (2026-10-16): the Fleet menu's bombardment
  orders read (`ghidra/notes/bombardment-order.md`) and ported
  (`bombardment.rs`, F-032): a plan object's bombarder at its target orders
  `AIAction::Bombard` with Destroy System `0x223` (fleet `0x10`, system
  `v[50]`) or `FUN_004ae320`'s code (`FUN_004b1070` -> `FUN_004b1330`), the
  order stopping the walk at `+0x30`. The automatic bombardment after a
  battle is gone. In 19 campaign runs over 18 seeds no plan binds a
  role, so nothing bombards. Save v55; golden
  `v1:dbb908594619dd4b` -> `v1:b160883b9ed0c807`
  (`evidence/2026-10-16-bombardment/`).
- Module 8, stage 4f (2026-10-16): the player's Planetary Bombardment
  sub-menu (F-032) gives the same orders through
  `bombardment::order_enabled`, the AI's validation now shared with it. No
  simulation change; the golden is unchanged
  (`evidence/2026-10-16-bombardment-menu/`).
- Module 8, stages 5a and 5b (2026-10-16): kinds 1 and 2's plan objects
  read (`ghidra/notes/ai-sector-plans.md`); kind 1 ported: its wants
  (`FUN_004bb510`), a second attack list (`FUN_004bafe0`), three
  bombardment lists (`FUN_004bb300`) and a lone fleet's patrol of the
  systems seen longest ago (`FUN_004bb630`, `FUN_0041a3a0`). Only seed 47
  of nine makes a kind 1 planner, and it holds no fleet; every count and
  the golden are unchanged (`evidence/2026-10-16-ai-plan-kind1/`).
- Module 8, stage 5c (2026-10-16): kind 2's plan object ported: its wants
  (`FUN_004b9fe0`), its attacker remembering the target (`FUN_004b9c90`),
  a lone fleet sent to each system of the opponent once (`FUN_004ba0d0`,
  the visited list `+0x5c`) and the sector marked swept when none is left
  (`FUN_004b9310`, `AIState::swept`). Kind 2 planners now take ships and
  create fleets, so every campaign moves. Save v56; golden
  `v1:b160883b9ed0c807` -> `v1:fb57b4045cb49e83`
  (`evidence/2026-10-16-ai-plan-kind2/`).
- Module 8, stages 5d to 5g (2026-10-16): kind 4's plan object read and
  ported: its target (the opponent's system with the plan's yard kind,
  chosen anew each cycle, `FUN_004b8f80`), base (`FUN_004b8070`), boarding
  of the base's regiments (`FUN_004b8470`, `AIAction::BoardRegiments`),
  attacker, assault role (`FUN_004b8ab0`, `FUN_004ae240`) settled at the
  target as Planetary Assault (`FUN_004b1070`), wants, movement, report
  and retreat. Without the planner's request (`+0x0c`) no regiments reach
  the base, so no fleet carries enough for an assault yet. Save v57;
  golden `v1:fb57b4045cb49e83` -> `v1:a8343f6c6630924e`
  (`evidence/2026-10-16-ai-plan-kind4/`).
- Module 8, stage 5h (2026-10-16): the planner's request (slot `+0x0c`,
  kind 4's `FUN_004b7100`) and module 6's two planned moves of regiments to
  a planner's base (`FUN_004ddb70` defensive, `FUN_004de130` offensive,
  controller bit `0x10`). The source sector of module 6's moves is now the
  farthest (order 1), as the original pushes; the port had the nearest. In
  seed 47 a kind 4 plan gets its regiments and orders the campaigns' first
  Planetary Assault. Module `0xb`'s `FUN_004c9cb0` is read, not ported. No
  save change; golden unchanged (`evidence/2026-10-16-planner-request/`).
- Module 8, stage 5i (2026-10-16): module `0xb`'s fleet cycle state 4
  (`FUN_004c9cb0`) sends the regiments carried by the first fleet of free
  own ships to the first unblockaded base of a planner that asks, else to
  the first planned sector's base. In seed 1 it sends one regiment to
  Sullust on day 22. No save change; golden unchanged
  (`evidence/2026-10-16-carried-regiments/`).
- Module 8, stages 6a and 6b (2026-10-16): kind 5's plan object read and
  ported. It consolidates the side's own sector:
  - its request asks for a regiment per system not yet the side's;
  - its base takes `0x60000`;
  - boarding goes by defense;
  - an occupier role (`0x80000000`) lands one regiment at a time on a
    neutral, unpopulated system;
  - its bombarder targets the opponent's systems;
  - a lone fleet without a base scouts an unexplored system.
  A regiment may now land on an explored, unpopulated, neutral planet
  (`FUN_005073d0`), which the player's Move gets too. No save change;
  golden unchanged (`evidence/2026-10-16-ai-plan-kind5/`).
- Module 8, stages 6c and 6d (2026-10-16): kind 6's plan object read and
  ported. It conquers a sector of the opponent:
  - its request is the sector's summed garrison need plus the base's
    term;
  - its base takes `0x50000`;
  - it boards as kind 4, attacks as kind 3 and bombards as kind 5;
  - its assault takes any opponent system of the sector that a fleet can
    take (by `v[50]`, else keys `0x21` and `0x14`).
  Every plan kind is now ported. In seed 47 (1000 days) the kind 6 plan
  assaults Triton on day 770. No save change; golden unchanged
  (`evidence/2026-10-16-ai-plan-kind6/`).
- 2026-10-17, by the user's decision: the port's fleet heuristics that
  stood in for module 8's plan objects are removed
  (`evaluate_fleet_deployment`, `evaluate_ds_escort`, `select_ds_target`
  and their helpers, `PlannerStep::StandIn`, `Step8::Plan`,
  `Module8::decided`, the battle cooldowns). A planner of a kind outside 1
  to 6 has no plan object and ends its cycle. The campaigns are identical.
  Save v58; golden `v1:a8343f6c6630924e` -> `v1:51f96ff9faae705b`
  (`evidence/2026-10-17-stand-in-removed/`).
- 2026-10-17, by the user's choice: the port's automatic landing of
  carried regiments is retired. Nothing in the original lands cargo
  without an order (`ghidra/notes/regiment-unload.md`), so regiments stay
  aboard until a Move: the simulation's step 3b and the app's
  `tactical_flow::occupy` only occupy now. Module `0xc`'s heuristic sends
  a regiment by its own Move instead of aboard a fleet. Campaigns change
  in five of seven seeds; the occupation's clash with the economy's
  troop-based control (`FUN_0050b610`) now shows as neutral planets
  flipping back and forth. Golden final `v1:d2e439b6e18aff1f` ->
  `v1:3bb9a6063ff07d84` (`evidence/2026-10-17-landing-retired/`).
- 2026-10-17, by the user's decision: the fleets' hold
  (`TroopTransportState::load`/`release`) and `AiConfig`'s
  `alliance_deploy_budget`, `empire_deploy_budget` and
  `ds_retreat_strength_ratio`, unused since the two changes above, are
  removed. The campaigns are identical. Save v59; golden
  `v1:51f96ff9faae705b` -> `v1:e47089c0adbf9615`
  (`evidence/2026-10-17-hold-and-config-removed/`).
- Acceptance: Every AI constant is recovered, or tagged port: where the polled
  architecture has no counterpart, and the two wrong formulas follow their
  decompiles.

### F-037: Economy and research run on an invented cadence and miss recovered terms

- Severity: P2
- Status: remediated; browser and native GUI verified (2026-10-18)
- Evidence:
  - The port recomputes support every tick; the original runs FUN_00508250
    once at setup (stage 0x17) and then updates through field hooks
    (FUN_00510820, FUN_005109f0, FUN_00511740) and timers 0x381, 0x383-0x386
  - FUN_00559be0 divides positive Alliance and negative Empire support shifts
    by GNPRTB 7681; the port omits it, and FUN_0050c9f0 adds the current
    support before clamping
  - resolve_system_control's GNPRTB 7760 energy threshold has no source;
    FUN_0050b610 resolves control from loyalty (FUN_0055a080)
  - FUN_0050b310 is the blockade withdraw percent, not a production modifier;
    the maintenance 0x304 cadence is invented; timer 0x381 (resource tally)
    has no port counterpart
  - research.rs is taken from rebellion2 and cites nothing in REBEXE.EXE
  - evidence/invention-review-r1/economy.md (row 9 on Empire doubling is
    excluded; see the README)
- Re-check (2026-10-14): the `FUN_00559be0` divisor (GNPRTB 7681) and the
  `FUN_0050c9f0` add-then-clamp are ported as `uprising.rs` `support_change`
  and `apply_support_change`, which mission and uprising support shifts go
  through, with tests citing both. The economy's per-tick `SupportDrifted`
  path does not use them, so the cadence bullet stands.
- Research (2026-10-14): `research.rs` no longer follows rebellion2. Its
  projects, ticks and the Research panel are gone; research runs only as
  missions (F-040). Each side's points and Done flag drive its levels
  through the points hook `FUN_005326f0` -> `FUN_00567ba0`: the next order
  of the tree's family, costing the class difficulty times side `+0xc0`
  over 100, with tests citing them. The research bullet is closed; the
  economy bullets stand.
- 2026-10-17: who holds a system (`FUN_0050b610`) read and ported
  (`ghidra/notes/system-control.md`):
  - the regiments' side holds the system; with both sides' regiments the
    previous side keeps it;
  - without regiments the side whose support reaches GNPRTB 7760 (60,
    `FUN_0055a080`) holds it;
  - an unpopulated system follows only its regiments.
  The port had misread GNPRTB 7760 as an energy test, returned
  `Contested`, and made every unpopulated system neutral. That clashed
  with the occupation in step 3b, which now uses the same rule
  (`economy::system_holder`), as the original's handlers do when a
  system's regiments change. Systems flipping back and forth fall from 5
  to 2 over seven 600-day seeds. Golden final `v1:9234ac6ef0f26a8a` ->
  `v1:ef80035d2be8099a` (`evidence/2026-10-17-system-control/`).
- 2026-10-17: the economy's cadence read (`ghidra/notes/economy-cadence.md`).
  The port's per-tick support drift applies `FUN_00559c40`, which is the
  system's SmugglingPercent (`+0x6c`, `FUN_00509ec0`): the original never
  moves support with it. Support moves only through `FUN_0050c9f0`, from
  missions, events and the timers `0x384` (uprising, -1) and `0x386`
  (blockade, ±1). The other derived fields are recomputed by the field
  hooks (`FUN_005109f0`, `FUN_00510820`, `FUN_00511740`, `FUN_00511300`),
  not each tick. The port's "collection rate" is ProductionModifier
  (`+0x70`).
- 2026-10-17, step 1: the support drift is removed. `FUN_0050b230` ->
  `FUN_00559c40` is ported as the system's smuggling percent
  (`economy::smuggling_percent`, save v61): the holder's capital ships,
  planet squadrons and regiments against its support band, gated by its
  Death Stars, and nothing moves support with it. Golden final
  `v1:6f20d000b67b9f3a` -> `v1:8790fcc409096722`
  (`evidence/2026-10-17-smuggling-percent/`). The timers `0x384` and
  `0x386` (step 2) and the hook cadence (step 3) stand.
- 2026-10-17, step 2: the support timers are ported. In revolt, timer
  `0x384` (`FUN_0050caa0`) takes GNPRTB 7697 (-1) from the holder every
  GNPRTB 7698 (30) days. Under blockade, timer `0x386` (`FUN_0050cb20`)
  gives the blockader GNPRTB 7693 (1) when it is the more popular side,
  else 7695 (-1), every GNPRTB 7694 (30) days. The delay records come from
  `FUN_0050e200`'s disassembly. Save v62; golden final
  `v1:8790fcc409096722` -> `v1:07eb8ba0a9c5f765`
  (`evidence/2026-10-17-support-timers/`). The hook cadence (step 3) and
  `+0x88` bit 11 stand.
- 2026-10-17, step 3a: `+0x88` bit 11 is DeathStarNearby
  (`FUN_0050a890`), set on every system of a sector holding an active
  Death Star (`FUN_005178b0`), not the port's "support above 40". It is
  ported as `economy::death_star_nearby` for the four rules that read it
  (`FUN_00559b60`, `FUN_00559ce0`, `FUN_00559fe0`, `FUN_00559be0`). The
  hook map is read (`ghidra/notes/economy-cadence.md`): `FUN_005109f0` is
  the support's hook, `FUN_00511740` bit 11's. Golden final
  `v1:07eb8ba0a9c5f765` -> `v1:cdf8f5db1e744a60`
  (`evidence/2026-10-17-death-star-nearby/`). The hook cadence (step 3b)
  stands.
- 2026-10-17, step 3b: the side hook `FUN_00510820` (System slot
  `+0xf0`, run by `FUN_004f6f40` when the holder changes) clears the
  uprising, so a revolt now ends when its system changes hands. The
  smuggling percent is recomputed only on a support, bit 11 or side
  change, and the collection rate only on a support change. The holder
  and TroopRegRequired have all their inputs among their hooks, so their
  daily recompute matches. Save v63; golden final `v1:cdf8f5db1e744a60`
  -> `v1:653ff1b1bbff8904` (`evidence/2026-10-17-hook-cadence/`). The
  economy bullets of F-037 are closed: support moves only through
  `FUN_0050c9f0` (missions, incidents and the timers `0x384` and `0x386`),
  and the derived fields follow their hooks. Still open: the maintenance
  counter `0x304` (F-029), timer `0x381` and the collection rate's
  neutral value (`FUN_00559c10` gives 100, the port 200).
- 2026-10-17: the "collection rate" is the system's ProductionModifier
  (`+0x70`, `FUN_0050b2c0`): 100 with no holder, else GNPRTB 7763 × 100
  over the holder's support. Its hook `FUN_0050c2b0` copies it to the
  system's mines and refineries, whose cycles it lengthens
  (`FUN_0055e2b0`). Both are ported; builds fall by up to 19% over seven
  600-day seeds. Golden final `v1:653ff1b1bbff8904` ->
  `v1:99cb09fad4a57148` (`evidence/2026-10-17-production-modifier/`).
  Still open: the maintenance counter `0x304` (F-029) and timer `0x381`.
- 2026-10-17: the maintenance shortfall (`0x304`) is
  `GameObjDestroyedAutoscrap`. While a side's maintenance exceeds its
  capacity, its timer `0x382` (GNPRTB 7168, 10 days; `FUN_005329f0`,
  `FUN_00530350`) autoscraps one random maintained object with reason
  `0x15` and a half refund (`FUN_004f8850`, `FUN_00530270`). Ported as
  `autoscrap.rs` (save v64), replacing the economy's invented 30-day
  garrison-deficit counter. The seven 600-day campaigns play the same
  games and never autoscrap. Golden final `v1:99cb09fad4a57148` ->
  `v1:c23f38a8242bca8f` (`evidence/2026-10-17-autoscrap/`). Still open:
  timer `0x381` (the side's per-system list sums).
- 2026-10-17: timer `0x381` is the yards' own research. Every GNPRTB 7169
  (30) + rand(0..=7170 (30)) days, `FUN_00530460` adds the side's usable,
  unsuspended yards of each area, in the systems it holds, to its research
  points (`+0x90` ships, `+0x94` troops, `+0x98` facilities), whose hook
  raises the levels. Ported as `ResearchSystem::advance_tally` (save v65).
  Golden final `v1:c23f38a8242bca8f` -> `v1:21ecf34ac9e1f828`
  (`evidence/2026-10-17-yard-research/`). With it, F-037's economy timers
  (`0x381`..`0x386`) are ported. Still open in F-037: none of the economy
  bullets. The informant (`0x38e`) and resource (`0x390`) incidents stay
  under F-029.
- 2026-10-17, closed by the user's decision: support moves only through
  `FUN_0050c9f0` (missions, incidents and the timers `0x384`, `0x386`),
  the derived fields follow their hooks, bit 11 is DeathStarNearby, the
  ProductionModifier drives the mines and refineries, the maintenance
  shortfall is the autoscrap timer `0x382`, and the yards research through
  timer `0x381`, each with failing-without tests and browser runs. Native
  GUI acceptance was not run.
- 2026-10-18 (native): the same 1000-day release-build campaign raised
  research levels 22 times and refunded 14 autoscrapped units
  (`evidence/2026-10-18-native-acceptance/`).
- 2026-10-20: the Smuggling message (`0x145`, `FUN_00499130`) is ported
  as `story_messages::smuggling_message`. It speaks only when the
  smuggling percent begins (old value 0) or ends (new value 0), on a held
  system: an Alliance system when the old value is at least 0, an Empire
  one when it is below 1. TEXTSTRA `0x7060`-`0x7067` are chosen by
  beginning or ending and by whose system it is, on the Popular Support
  rail. `SmugglingPercentChanged` gains `previous`. The 24 campaigns are
  identical (`evidence/2026-10-20-smuggling-message/`).
- 2026-10-21: the setup pass `FUN_00508250` (stage `0x17`) runs once at
  game creation as `EconomySystem::setup`: in the app's new game, the
  playtest runner and the seed-42 fixture. The first day no longer
  reports every field changed from its default. That was 62 to 81
  economy events a seed, among them "smuggling begins". Golden initial
  `v1:38af87970fe0c218` -> `v1:eb6515d682ec8a5d`, final unchanged. The 24
  campaigns are otherwise identical
  (`evidence/2026-10-21-economy-setup-pass/`).
- 2026-10-21: System Joins / Declares Neutrality (`0x100`,
  `FUN_00499760`, TEXTSTRA `0x7038`-`0x703d`) is ported for a holder change
  the support alone caused (`+0x88` bit `0x2000`, `FUN_00510d70`; port: no
  regiment at the system). The app's `ControlResolved` and occupation now
  hand the facilities over (`FUN_004f6f40`), as the headless integrator
  already did (`evidence/2026-10-21-system-joins-message/`).
- 2026-10-21: an unpopulated system's holder follows its regiments.
  `FUN_0050b610` gives it no support side, and `FUN_00508660` reruns it
  when a regiment leaves, so with no regiment it holds no side. The
  economy had skipped unpopulated systems and left them held; it now
  resolves their holder daily. Campaigns and the golden are unchanged
  (`evidence/2026-10-21-unpopulated-holder/`).
- 2026-10-21: `FUN_0050b230` asks only for a holder, so a system held by
  regiments alone has a smuggling percent too. The economy now runs step 1
  for unpopulated systems (`refresh_smuggling`).
  - Golden initial `v1:b77445f2a1a777c0` -> `v1:e01dc3b9b044c28e`, final
    `v1:4a994816eae0685d` -> `v1:7737eb96955b5cd7`: unpopulated systems
    now carry an economy record.
  - Campaign events are identical; the snapshots gain unpopulated rows
    (`evidence/2026-10-21-unpopulated-smuggling/`).
- 2026-10-21: the blockade messages take the original classes.
  - Blockade (`FUN_004960f0`, `0x14e`) speaks only at the start, with a
    fleet present. The holder reads `0x70a2` / `0x70a3`, naming the other
    side (`FUN_004f3dd0(viewer, 3 - holder)`). The blockader reads
    `0x70a0` / `0x70a1`, naming the fleet.
  - The blockade's end posts nothing.
  - Evacuation Losses (`FUN_00495e90`, `0x340`) takes `0x70a8`-`0x70aa`,
    to the regiment's side.
  - Evidence: `evidence/2026-10-21-blockade-messages/`.
- Browser (2026-10-21): the fixture scenario `BlockadeArrival` (code 77)
  and `fleet-window.mjs` `blockade` show the player's fleet arriving at
  the other side's F'tral, for both sides. The blockader reads "Fleet
  Initiates Blockade of F'tral" (`FUN_004960f0`, TEXTSTRA `0x70a0`). The
  Fleet window suite passes 56 cases
  (`evidence/2026-10-21-blockade-browser/`).
- Browser (2026-10-21): the fixture scenario `SupportFlip` (code 78) and
  `fleet-window.mjs` `joins` show, for both sides, the player's F'tral,
  with no regiment and the other side's support, handed over on the first
  day. System Joins reads "F'tral Joins Enemy" (`FUN_00499760`, TEXTSTRA
  `0x7039`). The Fleet window suite passes 58 cases
  (`evidence/2026-10-21-system-joins-browser/`).
- Acceptance: Support follows the hook and timer chain with failing-without
  tests, and research is recovered or tagged.

### F-038: Character timers are missing and story events are port-authored

- Severity: P2
- Status: open
- Evidence:
  - Injury recovery (timer 0x388, character +0x94, GNPRTB 2563/2564) has no
    port field or timer; timers 0x389 and 0x38a are unported
  - Jedi detection uses flat per-tier probabilities in place of FUN_0055e4d0,
    FUN_0055ff60, and FUN_0058a530; betrayal (interval 50, loyalty - 50), XP
    thresholds, and detection intervals have no source
  - The Jabba chain, carbonite countdown, Dagobah, Emperor arrival, and Final
    Battle sequences use invented tick thresholds and probability gates
  - events.rs reuses real timer ids for story events: 0x384, 0x386, 0x387, and
    the unarmed 0x396
  - Nine notification ids match their decompiles
  - evidence/invention-review-r1/characters.md
- 2026-10-17: injury recovery (timer `0x388`) is ported
  (`ghidra/notes/character-injury.md`). Characters keep their injury
  (`+0x94`, save v68) from missions and the uprising incident; the
  effective combat (`+0x86`: combat + combat × Force / 100 - injury) is
  what missions, the injury roll and the Status window read; a character
  with its own side heals 1 point a day, 2 at Force 80+ (GNPRTB
  2560..2564, `healing.rs`). Golden final `v1:c02d4ee44749e195` ->
  `v1:29ab46197f1887c6` (`evidence/2026-10-17-character-injury/`). Still
  open here: the injury's resign request, command clearing and the
  Injured/Healed messages; timer `0x389` (`0x38a`, captive escape, was
  ported 2026-10-14); Jedi, betrayal and the story events.
- 2026-10-17: the Character Injured / Healed messages (`0x106`,
  `FUN_0048fdf0`) are posted when an injury is added and when healing ends
  (`evidence/2026-10-17-injury-messages/`). The injury's resign request
  needs no port; the command clearing and `Damaged` stay open, as the port
  keeps no command.
- 2026-10-17: Force awareness read (`ghidra/notes/force-awareness.md`).
  `ForcePotential` is rolled once from `jedi_probability`, `ForceAware`
  starts as `is_known_jedi`, and timer `0x389` (100 + rand(0..=1000) days)
  lets a trainer with Force 80+ (Luke later, the Emperor) make the free
  characters of its side beside it aware. Force detection between sides
  is `force1 + force2 - 100` percent (`FUN_0055e4d0`). None of it exists in
  `jedi.rs` yet.
- 2026-10-17, Force stage 1: characters keep `ForcePotential`,
  `ForceAware` and the Force base `+0x8e` (save v69). Seeding rolls the
  potential from `jedi_probability` for every character, and the base
  (class base + rand(0..=variance), `FUN_004eee60`) only for one both
  potential and aware. The Force level `+0x8c` is the base, so only Luke,
  Vader and the Emperor start with Force, not every minor character. The
  effective combat, `FastHeal` and the Force Ranking read it. Potential
  no longer makes a character aware. Golden final `v1:29ab46197f1887c6` ->
  `v1:aa49f78d3f06cc2f` (`evidence/2026-10-17-force-potential/`). Open:
  timer `0x389`, the Force share of diplomacy and espionage, `+0x90`.
- 2026-10-18, Force stage 1b: diplomacy `+0x7c` and espionage `+0x7e`
  carry the Force share, skill + skill × Force / 100 (`FUN_004eecb0`,
  `FUN_004eecf0`, `FUN_0053e940`). The missions read them through slots
  `+0x1dc` and `+0x1e0` (`FUN_00573ff0`, `FUN_00573090`), and so does the
  Status window. Luke's data (50, 50, 90 at Force 50) gives the 75, 75
  and 135 of the manual's Figure 3.46. No save change; golden final
  `v1:aa49f78d3f06cc2f` -> `v1:5531248a4edc88bb`
  (`evidence/2026-10-18-force-share/`). Open: timer `0x389`, `+0x90`,
  `+0x92`.
- 2026-10-18, Force stage 2: timer `0x389` is ported
  (`force_discovery.rs`, save v70). A trainer with Force 80+ that is free
  makes the latent characters of its side beside it aware each day
  (GNPRTB 2582/2583, record `+0x20`; `FUN_00560a30`), with Leia only for
  Luke once he knows his heritage. Two corrections to the 2026-10-17
  reading: the timer fires daily (2584/2585 belong to the escape timer
  `0x38a`), and the trainer at the start is Vader (MJCHARSD 641, Force
  120), the Emperor being 640 (Force 150, no trainer). No discovery
  happens in seven 600-day or five 3000-day campaigns. Golden initial
  `v1:9c1efa24f4e2cd19` -> `v1:58473b5bf4582374`, final
  `v1:5531248a4edc88bb` -> `v1:461650933b75014c`
  (`evidence/2026-10-18-force-discovery/`). Open: message `0x362`,
  `+0x90`, `+0x92`.
- 2026-10-18: each discovery posts notification `0x362` as
  `FUN_0048f1e0` builds it, to the trainer's side: "Future Jedi Discovered"
  with the trainer's portrait and `0x7179`/`0x717a` by Force 100 (GNPRTB
  2581), or "Leia Uses Force" with hers. Both seen in the fixture build
  (scenario 55, both readers). The Healed picture's portrait lookup (side
  passed for the major flag) and the Force Growth ranking (template for
  the Force level) are fixed with it. Simulation unchanged
  (`evidence/2026-10-18-force-discovery-message/`). Open: `+0x90`,
  `+0x92`.
- 2026-10-18: Force growth read (`ghidra/notes/force-awareness.md`).
  `+0x8e` is ForceExperience and `+0x90` ForceTraining; Force is their sum.
  ForceTraining comes only from the Jedi Training mission (`0x58`,
  `FUN_005712b0`): a teacher at Force 100+ gives each pupil a teacher -
  pupil percent chance of rand(0..=(teacher - pupil) / 2) Force.
  ForceExperience grows when Luke or Leia evade capture (+1, GNPRTB
  3081/3082), at Dagobah's end (+60%, or days/2 percent; GNPRTB
  6174/6175) and in the Luke-Vader encounter. None exists in the port.
- 2026-10-18: Luke and Leia gain GNPRTB 3081/3082 (1) ForceExperience when
  they evade capture in a mission's detection (slot `+0x208`,
  `FUN_0056f8b0`, `FUN_0056ffa0`; `MissionEffect::ForceExperienceGained`).
  Defect fixed with it: the detection's injuries were never applied
  (`apply_advance_effects` ignored `CharacterInjured`), so no mission
  injury had landed since the injury step. Seven 600-day campaigns now
  land 19 injuries and Leia gains up to 6 Force. Golden unchanged
  (`evidence/2026-10-18-evade-force/`).
- 2026-10-18: the Jedi Training mission (`0x58`) is ported
  (`MissionKind::JediTraining`, save v71; also F-040). The first member
  who trains Jedi at Force 100+ teaches; every other member needs some
  Force below the teacher's, and the team two members or more
  (`FUN_00570690`). Each pupil hits with teacher - pupil percent and gains
  rand(0..=(teacher - pupil) / 2) `ForceTraining` `+0x90` (`FUN_005712b0`).
  Force is now `+0x8e` + `+0x90`. No AI planner sends it, so the campaigns
  are unchanged. Golden initial `v1:58473b5bf4582374` ->
  `v1:d077dd4c3c2b7247`, final `v1:461650933b75014c` ->
  `v1:1d8dbb48d17c03de` (`evidence/2026-10-18-jedi-training/`).
- 2026-10-18: Luke's Dagobah mission read (`ghidra/notes/dagobah.md`).
  Timer `0x392` (GNPRTB 6146/6147, 300 to 400 days) sets
  `LukeDagobahRequired`; once Luke is free and Han is not held at
  Jabba's palace, `LukeDagobah` (notification `0x221`) creates mission
  `0x71000043` for Luke to the Dagobah abode `0x98000441` (ABODESD 1089),
  100 days. Its end raises Luke's Force by 60 percent on success, else by
  days / 2 percent, and sets `DagobahCompleted` (`0x210`). The abodes
  (two jails, Dagobah, Hermitage, Jabba's Palace) sit in the galaxy, not in
  systems, and the port loads none of them.
- 2026-10-18: travel to an abode read (`ghidra/notes/dagobah.md`). Family
  `0x98` is its own class (vtable `0x00669260`, `FUN_00577530`); it
  resolves to itself as a place, as a system does, and missions accept it
  as a target. A trip between two different places takes 1 day unless both
  are systems (`FUN_00555b30`), so reaching or leaving an abode always takes
  1 day.
- 2026-10-18: abodes, first stage (save v72): `GameWorld::abodes` loads the
  five ABODESD places with the sides `FUN_00519910` gives them, a
  character can be in one (`Character::current_abode`), and the Status
  window names it. Missions do not target them yet; campaigns unchanged.
  Golden initial `v1:d077dd4c3c2b7247` -> `v1:c4aa6c7fc51080c1`, final
  `v1:1d8dbb48d17c03de` -> `v1:0e985b5a40e2bfa8`
  (`evidence/2026-10-18-abodes/`).
- 2026-10-18: Dagobah's end and Luke's return read
  (`ghidra/notes/dagobah.md`). `FUN_00548840` clears `LukeDagobahRequired`
  only when the Dagobah mission ends after phase 8, so an interrupted trip
  repeats. When the mission ends, `FUN_00546370` runs each member's slot
  `+0xb0` (`FUN_004ef260`): a character in a container of another side
  autoroutes home (`FUN_00543040`), as an escaped captive does.
- 2026-10-18: Luke's Dagobah is ported (`dagobah.rs`,
  `MissionKind::Dagobah`, save v73). Timer `0x392` (300 to 400 days)
  requires it; once Luke is free the mission manager sends him to the
  Dagobah abode (1-day trip), he trains 100 days, his Force rises 60
  percent (or days / 2 percent), `DagobahCompleted` is set, and he
  autoroutes home; messages `0x221` and `0x210` are posted and were seen
  in the fixture build. The invented story chain (`0x390`, `0x392`,
  `0x221`, `0x391`, `0x210`) is gone; the events that waited for `0x210`
  wait for `DagobahCompleted`. Over 1200-day campaigns Luke goes on 4 of 7
  seeds and ends at Force 80; on the other 3 an assassination kills him
  first. Golden initial `v1:c4aa6c7fc51080c1` -> `v1:7de82effa8207c75`,
  final `v1:0e985b5a40e2bfa8` -> `v1:ab2d9eddde745c32`
  (`evidence/2026-10-18-dagobah/`).
- 2026-10-18: Force detection and the Luke-Vader encounter read
  (`ghidra/notes/encounter.md`). During a mission's detection each Force
  user of the defenders meets each of the mission's (Luke and Vader first)
  with `force1 + force2 - 100` percent (`FUN_0058a3f0`); a hit detects the
  mission and sets both characters' `Encounter` keys (`FUN_0055ff60`,
  notification `0x360`). A detected member with an `Encounter` then duels
  its partner instead of a random defender (`FUN_00560200`): mutual
  injuries and a combat raise, Luke's `KnowsHeritage` and his wound at the
  first meeting with Vader, and a Force gain for Luke or Leia from Vader or
  the Emperor (GNPRTB 3072..3087). None of it exists in the port.
- 2026-10-18: Force detection and the encounter are ported
  (`mission_detection.rs`, save v74). Force users meet before betrayal;
  a hit detects the mission and sets both `Encounter` keys and the day; a
  detected member with a key confronts its partner (the duel, the combat
  raise, Luke's heritage and wound, and the Force lessons from Vader and
  the Emperor). The loader's characters carry no family byte, so the
  detection now reads `is_major` for the major and minor sets and the
  exposure order. Over 1200-day campaigns Luke meets Vader 7 times (seeds
  2 and 3) and the Emperor meets Luke twice (seed 47). The invented "Final
  Battle Imminent" (`0x396`) still sets Luke's heritage at Dagobah's end,
  before every first meeting. Golden initial `v1:7de82effa8207c75` ->
  `v1:57115a2a37530826`, final `v1:ab2d9eddde745c32` ->
  `v1:e6bb717f2403b483` (`evidence/2026-10-18-encounter/`).
- 2026-10-18: the port's `SetHeritageKnown` action is removed (save v75).
  "Final Battle Imminent" (`0x396`) no longer sets Luke's heritage; only
  his confrontation with Vader (`FUN_00560200`) and Leia's discovery set
  it. In the campaigns Luke learns it at his first meeting with Vader
  (seeds 2 and 3, days 716 and 488). The rest of the Final Battle chain is
  still the port's own. Golden initial `v1:57115a2a37530826` ->
  `v1:ad8518eb172a430f`, final `v1:e6bb717f2403b483` ->
  `v1:e811b12322f7e20e` (`evidence/2026-10-18-heritage/`).
- 2026-10-18: the encounter class's messages are ported
  (`story_messages::EncounterNotes`): Force user Detects Enemy for each
  side, Character Confronts character and Luke Discovers His Heritage,
  built from the day's `0x360` and `0x211` as `FUN_0048d9a0` and
  `FUN_0048d7f0` gather them, with their recovered texts and pictures, seen
  in the fixture build (`evidence/2026-10-18-encounter-messages/`). 12 of
  the 21 story message classes remain unmapped.
- 2026-10-18: the original Final Battle read (`ghidra/notes/final-battle.md`).
  The mission manager sets FinalBattleReady once a captured Luke, not in
  Jabba's Palace, knows his heritage and has Force 60 or more (GNPRTB
  3078), with Vader and the Emperor free. It then sends Vader to Luke and
  both to the Emperor by Move orders, and fights the battle when the three
  meet: Luke with Force below GNPRTB 6151 (100) is injured and stays
  captive; otherwise he goes free and captures Vader and the Emperor. The
  Final Battle message (`0x71e0`..`0x71e2`) follows. DarthPickup sends
  Vader to collect the prisoners of Jabba's Palace (mission `0x73`). The
  port's own chain (`0x393`..`0x396`, `0x220`) has none of this.
- 2026-10-18: the Final Battle is ported (`final_battle.rs`, save v76):
  FinalBattleReady, Vader's Move orders to Luke and to the Emperor, the
  battle (Force GNPRTB 6151) with its injury, captures and barred escape,
  and The Final Battle message. Prisoners take their captor's side in the
  member checks, so the Empire's Move order carries the captive Luke. The
  port's `0x393`..`0x396` and `0x220` events are gone. In 1500-day
  campaigns the battle comes on seeds 13 (day 1151) and 47 (day 854); Luke,
  below Force 100, loses both. Golden initial `v1:ad8518eb172a430f` ->
  `v1:978a2af596955f63`, final `v1:e811b12322f7e20e` ->
  `v1:8c0677e665d1d127` (`evidence/2026-10-18-final-battle/`). Open:
  DarthPickup and the Pickup mission (Jabba's palace).
- 2026-10-18: Jabba's palace read (`ghidra/notes/jabba-palace.md`).
  BountyHuntersActive arms timer `0x393` (300 + rand(300) days); its
  fire creates a Bounty Hunters unit in the palace and the Bounty mission
  (`0x65`) on Han, whose capture sends him to Jabba's Palace by
  autorouting. HanCapturedAtPalace then sends Luke, Leia and Chewbacca on
  Palace missions (`0x64`, chance espionage/3 + combat/3): a success
  frees Han and the palace's prisoners, a failure leaves the rescuers
  captive for good. The port's own chain (`0x380`..`0x39b`, `0x212`,
  `0x231`) runs on tick thresholds instead.
- 2026-10-18: the bounty hunters are ported (`jabba.rs`, save v77):
  BountyHuntersActive and timer `0x393`, the Bounty Hunters unit leaving
  Jabba's Palace on the Bounty mission against Han, his RLEVADTB roll
  against the hunters, the capture with a barred escape, and his trip to
  the palace. The app posts the three `0x212` messages. The port's own
  palace chain (`0x380`..`0x39b`, `0x212`, `0x200`, `0x231`) is gone. In
  1200-day campaigns the hunters strike on seeds 1, 2 and 3 (days 331, 497
  and 561), and Han evades each time. Golden initial `v1:978a2af596955f63`
  -> `v1:5474ae69134d1255`, final `v1:8c0677e665d1d127` ->
  `v1:4cd75bad1e5062b8` (`evidence/2026-10-18-bounty-hunters/`). Open:
  HanCapturedAtPalace, the Palace missions and the Pickup.
- 2026-10-18: the Palace rescue is ported (`jabba.rs`, `missions.rs`):
  HanCapturedAtPalace cancels the hunters and sends Luke, Leia and
  Chewbacca on Palace missions (`0x64000044`, espionage/3 + combat/3). A
  success frees Han and every prisoner of the palace; a failure leaves the
  rescuer captive there, and Jabba Captures is posted. The Dagobah check
  waits while Han is held. No save change; the golden is unchanged. In
  24 seeds of 1500 days Han evades the hunters every time (his combat
  against their 0), so the rescue is shown by tests and the fixture
  (`evidence/2026-10-18-palace-rescue/`). Open: DarthPickup and the
  Pickup mission.
- 2026-10-18: Vader's Pickup is ported (`jabba.rs`, `missions.rs`):
  DarthPickup (Luke, Leia and Han held, Chewbacca held or destroyed, a
  prisoner in the palace, Vader free) sends the Pickup mission
  (`0x73000082`). It takes the palace's prisoners, who autoroute, still
  captive, to Vader's home, and Jabba Delivers Prisoners is posted. Han's
  and Jabba's story now runs on the original's paths end to end, but no
  campaign reaches the palace, because Han evades the hunters. No save
  change; the golden is unchanged (`evidence/2026-10-18-pickup/`).
- 2026-10-18: Han's escapes from the bounty hunters are the original's.
  The hit rolls RLEVADTB at Han's combat 100 less the hunters' 0, which
  gives 98, and the Bounty record does not repeat. So Han is captured in
  about 1 game in 50. The data test pins Han's combat, the RLEVADTB row
  and the four story missions' records (`ghidra/notes/jabba-palace.md`).
- 2026-10-18: the assassination is the original's (`FUN_00576620`,
  `FUN_004ef690`, `FUN_0053eae0`). It injures its target, and only a
  minor character can die of the injury (GNPRTB 2577, 25 percent). A
  major character never dies, as the manual says (p. 105). The port had
  always killed: in 24 campaigns of 1500 days the AI had killed Han 22
  times, Mon Mothma 21, Leia 18 and Luke 5. Now it kills none, and Han's
  palace story happens in play (seeds 16 and 19). The golden is unchanged
  (`evidence/2026-10-18-assassination/`). Open: death by injury after the
  other injuries.
- 2026-10-18: death by injury is ported for every injury that passes
  slot `+0x2f0`: the detection's evasion and capture injuries, the Force
  duel, the uprising incident and the assassination. A minor character
  dies with GNPRTB 2577 (25 percent), a major one never. In 24 campaigns
  26 minor characters die, and seed 16 shows a failed rescue at Jabba's
  palace followed by Luke's success. The golden is unchanged
  (`evidence/2026-10-18-death-by-injury/`).
- 2026-10-18: `story-message-classes.md` corrected. Events `0x305` and
  `0x306` come from the system's blockade hook (`FUN_00511300`) and from
  an autorouting arrival (`FUN_004fba10`) through `FUN_0053f950`, with a
  flag pair as arguments. They are not an assassination or a sabotage.
  The builders of rows 18 (Killed by Imperial Assassins) and 19 (Death
  Star Sabotaged) need object keys that this path cannot carry, so their
  triggers are open. Row 16's (`0x230`) trigger is open too.
- 2026-10-18: the message classes' delivery read
  (`story-message-classes.md`, "Delivery"). The UI queue's posters are
  `FUN_0053fdd0`, `FUN_0053fcf0`, `FUN_0053fe40`, `FUN_0053feb0`,
  `FUN_0053ff20` and `FUN_0053fd60`. None of them is called with `0x304`,
  `0x305` or `0x306` as a literal. `FUN_004fc080`, the Destroyed
  notification, posts them by the DestroyedReason: 6 sabotage (`0x305`),
  7 assassination (`0x306`), `0x15` autoscrap (`0x304`). So rows 18
  (Killed by Imperial Assassins) and 19 (Death Star Sabotaged) are
  ported, and the app keeps a killed character in the arena as the
  integrator does. Row 16's trigger is SeatOfPower (`FUN_005617b0`): the
  Emperor free at an Empire-held Coruscant. Its port needs a save format
  change. The port's own `0x230` event is invented.
- 2026-10-18: Emperor Arrives at Coruscant is ported (`seat_of_power.rs`,
  save v78): SeatOfPower holds while the Emperor stands free at an
  Empire-held Coruscant, and the Empire reads the message when it is set.
  The port's own `0x230` story event is removed. In 24 campaigns the
  Emperor leaves Coruscant by day 15 and returns 0 to 4 times. Golden
  initial `v1:5474ae69134d1255` -> `v1:d8f4a9b46e07169c`, final
  `v1:4cd75bad1e5062b8` -> `v1:fd4bcf87c87325d6`
  (`evidence/2026-10-18-seat-of-power/`).
- 2026-10-18: the victory conditions word read
  (`ghidra/notes/victory-conditions.md`). `FUN_005180e0` builds galaxy
  `+0x74` from the HQ, the four victory characters' captures and
  Coruscant, with the wins by game type (`+0x60`: 0 standard, 1
  headquarters). Both sides copy it to `+0xbc`, and each change posts
  `0x128`, which drives rows 20 (Coruscant Captured) and 21 (Alliance
  Headquarters Destroyed). The port's `victory.rs` has none of this, and
  porting the rows needs a save format change (v79).
- 2026-10-18: the victory conditions word is ported (`victory.rs`, save
  v79). It is checked daily, its changes are notification `0x128`, and
  the app posts Coruscant Captured and Alliance Headquarters Destroyed.
  In 24 campaigns the word changed 292 times from the victory
  characters' captures, but neither the HQ nor Coruscant fell. Deciding
  the game by its win bits stays with F-005. Golden initial
  `v1:d8f4a9b46e07169c` -> `v1:e8ceaa386f41cc96`, final
  `v1:fd4bcf87c87325d6` -> `v1:475c0c0f35968fba`
  (`evidence/2026-10-18-victory-conditions/`).
- 2026-10-18: Rescue Attempt (row 13, `FUN_0048c4c0`) is ported. It
  follows notification `0x200` for a character setting out on a Palace
  mission (the family-`0x64` test at `0x48c5d9`, which Ghidra dropped).
  The app posts it to the Alliance with the rescuer's portrait when a
  Palace mission is dispatched. 20 of 21 story message classes are now
  mapped (`evidence/2026-10-18-rescue-attempt/`).
- 2026-10-18: Character Discovers Traitor (row 17, `FUN_0048c9c0`) is
  ported. After a betrayal each other member learns the traitor with a
  chance of its Force (`FUN_00588e80`), which posts `0x361`, and the
  discoverer's side reads the message. The AI never sends two characters
  on one mission, so it shows only for a player's teams. All 21 story
  message classes are now mapped (`evidence/2026-10-18-discovers-traitor/`).
- 2026-10-18 (F-036): the AI's mission planners persist (save v80). A
  planner holds its team, gathers it at the target or the side's system
  nearest it (`FUN_004bd0a0`), moves the members there (`FUN_004bd570`),
  and orders the mission once all stand there (`FUN_0047b360`). In 24
  campaigns, missions fall from 41,485 to 13,590 and two-member teams
  rise from 3% to 13%; successes go from 17,811 to 14,889. No AI team
  holds two characters yet, because the covert selectors find only
  special forces. Golden initial `v1:e8ceaa386f41cc96` ->
  `v1:5ba66653ae1fb76d`, final `v1:475c0c0f35968fba` ->
  `v1:7869f8168ab1782f` (`evidence/2026-10-18-mission-planners/`).
- 2026-10-18 (F-036): the AI's research reserve follows the side's
  finished trees (`FUN_00402230`, `FUN_004198a0` and siblings). Before,
  the port reserved every character for research for the whole game, so
  none went on covert missions. In 24 campaigns, teams with two
  characters rise from 0 to 613, and characters join Sabotage, Incite,
  Rescue and Abduction. Golden final `v1:7869f8168ab1782f` ->
  `v1:2d45f0777ed8cd0d` (`evidence/2026-10-18-research-reserve/`).
- 2026-10-18 (F-036): the AI's mission planners refill their teams and
  decoys every cycle (`FUN_004bced0` states 3 and 4). In 24 campaigns
  two-member teams rise to 38% of missions (3,233 of 8,605). Golden final
  `v1:2d45f0777ed8cd0d` -> `v1:f7fe1a11ab5f9ffd`
  (`evidence/2026-10-18-planner-refill/`).
- 2026-10-18 (F-036, save v81): the AI's Diplomacy planner is the
  original's. The strategy keeps one planner and one more per five
  candidates (`FUN_004e8290` row 5, `S+0x214`). Each planner picks and
  claims its target (`FUN_00481720`): own systems below 70 support first,
  then neutral ones. The port's popularity-cap and uprising-prevention
  heuristics are removed. In 24 campaigns, Diplomacy orders fall from
  1,381 to 789 with as many results, and research and recruitment rise.
  Golden initial `v1:5ba66653ae1fb76d` -> `v1:6f750b483b72f71f`, final
  `v1:f7fe1a11ab5f9ffd` -> `v1:bec91e71a20394cd`
  (`evidence/2026-10-18-diplomacy-planner/`).
- 2026-10-18 (F-036): the AI's Recruitment planner is the original's. The
  strategy keeps two planners while the side has minor characters
  (`FUN_004e8290` row 1). Each targets the recruiter's own calm system or
  the nearest one (`FUN_0047dcc0`). Side `+0xa4` (`minor_classes`) counted
  none in a real game, because loaded characters carry no family byte;
  fixed. A research re-order fault that was already there now shows in
  22 of 24 campaigns: one designer is ordered to the same site every few
  days. Golden final `v1:bec91e71a20394cd` -> `v1:4a2572231a842e24`
  (`evidence/2026-10-18-recruitment-planner/`).
- 2026-10-18 (F-036): the AI's research order goes to the site.
  `FUN_0047b360` orders on `+0x30`, the site slot `+0x50` copied from
  `+0x34`. The port ordered on the gathering place `+0x38`, so an old
  place with no yard was ordered again every few days, and each mission
  ended with `0xc`. In 24 campaigns research orders fall from 18,414 to
  602, and research successes rise from 7,643 to 8,300. Golden unchanged
  (`evidence/2026-10-18-research-order-target/`).
- 2026-10-18 (F-036): the AI's Incite planner is the original's. The
  strategy keeps one planner and one more per six candidates
  (`FUN_004e8290` row 7). A candidate is an opponent's system where the
  side's support is above 50. The planner targets the candidate nearest
  its last gathering place, or the best supported (`FUN_00480370`). The
  heuristic went where the enemy was most popular. In 24 campaigns
  Incite failures fall from 3,691 to 436. Golden final
  `v1:4a2572231a842e24` -> `v1:1f09d2d3d2c77cf3`
  (`evidence/2026-10-18-incite-planner/`).
- 2026-10-18 (F-036): the AI's Subdue planner is the original's. The
  strategy keeps one planner per own uprising, plus one, while an own
  system is below 70 support (`FUN_004e8290` row 6). The planner targets
  the nearest revolt, or the best supported one in the busiest sector
  (`FUN_0047d060`). The port's AI sent no Subdue before. In 24 campaigns
  there are 330 Subdue orders, and systems falling to no side drop from
  184 to 27. Golden final `v1:1f09d2d3d2c77cf3` -> `v1:7ed462ebd95b759a`
  (`evidence/2026-10-18-subdue-planner/`).
- 2026-10-18 (F-036, save v82): the AI's Reconnaissance planner is the
  original's. The strategy keeps one planner per sector planner
  (`FUN_004e8290` row 11). Each claims a sector and scouts a random
  unexplored system not the side's (`FUN_0047f970`). The Recon selector
  (`FUN_0047f790`) is ported. The heuristic sent Espionage instead. In 24
  campaigns there are 3,006 Reconnaissance orders. Golden initial
  `v1:6f750b483b72f71f` -> `v1:cb9430c7635d6e7e`, final
  `v1:7ed462ebd95b759a` -> `v1:3c9e4e1739d78f43`
  (`evidence/2026-10-18-recon-planner/`).
- 2026-10-18 (F-036): the AI's Espionage planner (subtype 6) is the
  original's. The strategy keeps one planner per sector holding an
  opponent's system, less the sector planners (`FUN_004e8290` row 12).
  Each claims a sector and spies on the opponent's system seen longest
  ago (`FUN_00481110`), with one special force (`FUN_00480f30`). The
  heuristic Espionage on unexplored systems is removed. Golden final
  `v1:3c9e4e1739d78f43` -> `v1:fe6f9d2ef366dfd9`
  (`evidence/2026-10-18-espionage-planner/`).
- 2026-10-19 (F-036, save v83): the standing Diplomacy, Incite and
  Subdue planners read their Mission Reports (`FUN_0047c890`). A
  success, or a failure within 5 or 10, checks the target. Past the
  limit, or with a target that no longer qualifies, the planner aborts
  its running mission (`FUN_0047b250`, order `0x250`). In 24 campaigns
  aborts rise from 216 to 871. Golden initial `v1:cb9430c7635d6e7e` ->
  `v1:530e248f2599b293`, final `v1:fe6f9d2ef366dfd9` ->
  `v1:45ce6a74e7ac0f51` (`evidence/2026-10-19-planner-reports/`).
- 2026-10-19 (F-036): the Recruitment, Reconnaissance and Espionage
  planners read their own report slots (`FUN_0047dad0`, `FUN_0047f720`,
  `FUN_00480eb0`). After a result, scouts and spies drop their target and
  mark it seen. `bind_mission` takes special-force leads, so Espionage
  missions now report to their planners. Golden final
  `v1:45ce6a74e7ac0f51` -> `v1:75f7a5f65cee3222`
  (`evidence/2026-10-19-report-slots/`).
- 2026-10-19 (F-036): the AI's Abduction planner is the original's. The
  strategy keeps one all game (`FUN_004e8290` row 9). It targets the
  first system holding an opponent's main character whose defense its
  gathered decoys can cover (`FUN_00483640`), then a random such
  character there (`FUN_00483d10`). The heuristic, which went for the
  weakest enemy characters, is removed. Golden final
  `v1:75f7a5f65cee3222` -> `v1:03a36c82837d1ce1`
  (`evidence/2026-10-19-abduction-planner/`).
- 2026-10-19 (F-036): a prisoner counts on its home side in the AI's
  records. `FUN_00401d20` flips the side test for a prisoner, and a
  capture turns the object's side to its captor's (`FUN_004eec60`). The
  port had read the home side and flipped it, putting a prisoner on its
  captor's side. Golden unchanged
  (`evidence/2026-10-19-prisoner-side/`).
- 2026-10-19 (F-036): the AI's Rescue planner is the original's. The
  strategy keeps one while a character of the side is a prisoner
  (`FUN_004e8290` row 10). It chooses its prison anew each cycle (the
  keep test at `0x47e800` never holds), where the gathered decoys cover
  half the defense (`FUN_0047e620`), and draws a prisoner there
  (`FUN_0047ecf0`). The last mission heuristic group is removed. Golden
  final `v1:03a36c82837d1ce1` -> `v1:49ba43802d0d2840`
  (`evidence/2026-10-19-rescue-planner/`).
- 2026-10-19 (F-036): a Diplomacy planner trades its diplomat for a
  better free one (`FUN_0047ba40` asking slot `+0x3c`, `FUN_00481ef0`).
  Golden unchanged (`evidence/2026-10-19-diplomat-trade/`).
- 2026-10-19 (F-036): the AI controller's leader bit is ported
  (`FUN_004182a0`). In a Headquarters Only game the records mark no
  leader (`FUN_00401d20`), so leaders may join the missions whose teams
  forbid them. Standard campaigns are unchanged, event for event
  (`evidence/2026-10-19-leader-bit/`).
- 2026-10-19 (F-036): the sector planners ask for missions as the
  original does (`FUN_004b7150`). The mission module takes each request
  and holds the planner for its sector planner (`FUN_004d2610`,
  `FUN_004d2690`, `FUN_00476ba0`). These planners do not order anything
  yet. Save v84. Golden initial `v1:530e248f2599b293` ->
  `v1:20ee82a2951ad9aa`, final `v1:49ba43802d0d2840` ->
  `v1:14ec5ed9881bd79b`. Campaigns are unchanged, event for event
  (`evidence/2026-10-19-sector-mission-requests/`).
- 2026-10-19 (F-036): a sector planner's held Recon and Espionage
  planners take their targets from its plan (`FUN_004bdbc0`,
  `FUN_00476e80`, slots `+0x2c` and `+0x28`). They still order nothing.
  Golden final `v1:14ec5ed9881bd79b` -> `v1:aae0de7bbef402c1`
  (`evidence/2026-10-19-sector-mission-targets/`).
- 2026-10-19 (F-036): a sector planner's held Abduction and
  Assassination planners take a system and a character from its plan
  (slot `+0x24`, `FUN_004af1c0`). They still order nothing. Golden
  unchanged. The Fleet window gate passed in 1 of 4 runs; the
  stage-2 commit also fails it (`join-fleet`), an open problem
  (`evidence/2026-10-19-sector-mission-characters/`).
- 2026-10-19 (F-036): a sector planner's held Sabotage planner takes a
  system and an object from its plan (slot `+0x30`, `FUN_004ae480`). It
  still orders nothing. Golden final `v1:aae0de7bbef402c1` ->
  `v1:dee3c02775ecfb47` (`evidence/2026-10-19-sector-mission-sabotage/`).
- 2026-10-19 (F-036): the sector classes' selectors (`FUN_0042f830` by
  subtype: Espionage `FUN_004808c0`/`FUN_00480aa0`, class `0x65a9b0`) are
  ported. The defense-capped decoy selectors take `(v[29] + 2) / 2` of the
  target's record, replacing a `hyp:` that read `+0xbc` as a posture and
  took one decoy. Golden unchanged
  (`evidence/2026-10-19-sector-selectors/`).
- 2026-10-19 (F-036): the sector planners' mission planners run their
  cycle (`FUN_004bda60` states 3 to 9: `FUN_004bdcd0`, `FUN_004be230`,
  `FUN_0047b360`) and read their reports. They now gather, move and
  order. Missions dispatched in 24 campaigns rise from 11,976 to 19,149
  (Reconnaissance 1,682 -> 6,450). Golden final `v1:dee3c02775ecfb47` ->
  `v1:ed40629cc5780d6d` (`evidence/2026-10-19-sector-cycle/`).
- 2026-10-19 (F-036): the held planners' `+0x20` bits reach their sector
  planner's `+0x64` (`FUN_00478000`), and the port's heuristic Sabotage and
  Assassination are removed with the adoption and `Planner::plan`. Every
  covert mission now comes from an original planner. Golden final
  `v1:ed40629cc5780d6d` -> `v1:1ddf726cecfd8477`
  (`evidence/2026-10-19-sector-bits/`).
- 2026-10-19: timer `0x38a` verified. It is the captive escape timer
  (`captive-escape.md`, ported 2026-10-14). Its pulse of `+0xac` bit 10
  (`FUN_004ef150`, `FUN_004eebf0`) raises event `0x317`, the ESCAPETB roll.
  The pulse's other call, slot `+0x3e8` (`FUN_004f23c0`), only logs.
  `coverage-map.md` rows `0x388` and `0x38a` are corrected from "missing"
  to matched. No code change.
- 2026-10-19: character command read (`ghidra/notes/character-command.md`).
  `+0x96` is `CommandKind` (1 Commander, 2 Admiral, 3 General), set by
  orders `0x260..0x263`. The checks are `FUN_004ed750`: not on a mission,
  injured or captured, and the class allows it. The effect is
  `FUN_00561bb0`: one holder per fleet or system, the same kind again
  clears. `FUN_004eeed0` clears it on injury and capture. The AI's command
  orders are module `0xc`'s. Not yet ported.
- 2026-10-19: character command, stage 2 (save v85, `character_command.rs`).
  `Character::command`, the order's check (`FUN_004ed750`) and effect
  (`FUN_00561bb0`, `FUN_004ef400`), and the loss on injury and capture
  (`FUN_004eeed0`). Nobody gets a command until the menu (stage 3), so
  the 24 campaigns are unchanged. Golden initial `v1:20ee82a2951ad9aa` ->
  `v1:4d8201e965d27089`, final `v1:1ddf726cecfd8477` ->
  `v1:2229fcf258823750` (`evidence/2026-10-19-character-command/`).
- 2026-10-19: character command, stage 3. The Command submenu
  (`0x260..0x263`, STRATEGY records read from the DLL) is enabled per
  `FUN_004ed750` and its base check `FUN_004f9860`, with the held kind
  checked. The Fleet Status window names the fleet's Admiral, General and
  Commander. Accepted in the browser on both sides. The name prefix
  (`FUN_004f17f0`) waits for a decision
  (`evidence/2026-10-19-character-command-menu/`).
- 2026-10-19: character command, stage 3b (save v86). A command renames
  the character (`FUN_004f17f0`) to TEXTSTRA of its name id plus `0x6800`,
  `0x6c00` or `0x7000` ("Admiral Ackbar"), and its loss renames it back.
  The loader keeps the four names on the character. Accepted in the
  browser on both sides. The 24 campaigns are unchanged. Golden initial
  `v1:4d8201e965d27089` -> `v1:094e77fcd3b4b666`, final
  `v1:2229fcf258823750` -> `v1:beb63be8a01b2699`
  (`evidence/2026-10-19-character-command-names/`).
- 2026-10-19: character command, stage 4a. The command's leadership
  readers: the bombardment batteries' general and each fleet's admiral
  (`FUN_0055e930`, `FUN_0055e950`), and the assault duel's two generals
  (`FUN_0058bec0`, `FUN_0055eb00`), through `fleet_holder`
  (`FUN_004fd790`) and `system_holder` (`FUN_00509330`). The 24 campaigns
  and the golden are unchanged
  (`evidence/2026-10-19-character-command-leadership/`).
- 2026-10-19: character command, stage 4b. A detector's officer (slot
  `+0x1bc`: a ship's Admiral, a squadron's Commander, a regiment's General,
  from its fleet or the system) takes its espionage times G3588/G3589 over
  100 off the decoy roll and FOILTB (`FUN_00588b90`, `FUN_00588a90`), and an
  exposed member rolls against its combat and is captured by it
  (`FUN_005888f0`). The 24 campaigns and the golden are unchanged
  (`evidence/2026-10-19-character-command-detector/`).
- 2026-10-19: character command, stage 4c. The AI records' rank bits
  (`FUN_00402230`: `0x4000000`, `0x1000000`, `0x2000000`) follow the
  character's command, replacing the hyp that gave every character in a
  fleet rank 1. The 24 campaigns and the golden are unchanged. Found:
  `FUN_00560d50` clears commands on a move (stage 5)
  (`evidence/2026-10-19-character-command-ai-ranks/`).
- 2026-10-19: character command, stage 5. A character that moves loses
  its command (`FUN_00560d50` from the move event `FUN_00514a60`),
  through `Character::move_to` at every real move; a fleet merge moves
  the ships and so takes the commands aboard. `FUN_004eeed0` read again:
  a character on a mission loses its command (the notes had it the other
  way round). The 24 campaigns and the golden are unchanged
  (`evidence/2026-10-19-character-command-move/`).
- 2026-10-19: character command, stage 6. `keeps` adds `FUN_004eeed0`'s
  mission test, and a death clears the command (the destruction hook,
  slot `+0x11c`, `FUN_004f1340`); the mission hook is slot `+0x268`
  (`FUN_004f1490`). The 24 campaigns and the golden are unchanged
  (`evidence/2026-10-19-character-command-hooks/`).
- 2026-10-19: character command, stage 7 (closing). `FUN_004f9860`'s en
  route test refuses a Command order (`Refusal::EnRoute`). No Command row
  is checked: the menu's fresh order leaves its `+0x40` list empty, so
  `FUN_00580d80` -> `FUN_0051fd30` answers 0 (read from the image),
  replacing the stage-3 hyp. Accepted in the browser on both sides. The
  tracker item is done (`evidence/2026-10-19-character-command-closing/`).
- 2026-10-19: the seat of power's leadership bonus. `+0x92` is GNPRTB
  3076 (50) for an Empire-class character while `SeatOfPower` holds
  (`FUN_005618d0`), and the effective leadership `+0x88` adds that percent
  (`FUN_004eede0`); the manual's "Seat of Power Effect" agrees. The
  missions, the command readers and the Status window read it. 21 of the
  24 campaigns change for this cause; the golden is unchanged
  (`evidence/2026-10-19-seat-of-power-leadership/`).
- 2026-10-19: jedi.rs, stages 1 and 2. The inventory
  (`ghidra/notes/jedi-rs-replacement.md`) finds its tiers, XP, flat
  detection and three story events without source; the original's Force
  rules are ported elsewhere. Stage 2 removes the day step and the three
  events (golden regenerated for this cause: initial
  `v1:6bc0edeba372c686`, final `v1:f2b8571c7dec63a3`); every campaign
  changes through the shifted roll stream
  (`evidence/2026-10-19-jedi-rs-simulation/`).
- 2026-10-19: jedi.rs, stage 3. The J panel ("The Force") and the
  Officers panel read the original's Force fields (`+0x8c`, its Status
  window ranking, ForceAware, ForcePotential, the discoverer) and give no
  training order; the invented tiers, XP and detection risk are gone from
  them. Accepted in the browser
  (`evidence/2026-10-19-jedi-rs-panels/`).
- 2026-10-19: jedi.rs, stage 4 (save v87, approved). `jedi.rs`,
  `ForceTier`, `force_tier`, `force_experience`, `is_discovered_jedi`,
  `JediState`, the event engine's tier conditions and actions and the
  `0x363` event are removed; the provenance baseline shrinks 85 -> 77.
  Golden regenerated for the layout (initial `v1:681087f1af0a0e6d`, final
  `v1:f2946c95622dd970`); the 24 campaigns are unchanged. The tracker item
  is done (`evidence/2026-10-19-jedi-rs-save/`).
- 2026-10-19: event ids. The port defines no story event any more, so
  none meets the original's timers `0x380..0x396`; 21 unread `u32` event
  constants (the invented ids among them) are removed and a test guards
  the range (`evidence/2026-10-19-event-ids/`).
- 2026-10-19 (F-028): betrayal. The original's only betrayal is the
  mission phase `FUN_00589f10` (ported); no function changes a
  character's side, and the manual's Traitors section agrees. The port's
  50-day defection (`betrayal.rs`) is removed, save v88; golden
  regenerated (initial `v1:a2c72fbdc17546ff`, final `v1:55c055d53d7ca5b8`);
  every campaign changes through the shifted rolls, none had a defection
  (`evidence/2026-10-19-betrayal-removed/`).
- 2026-10-19 (F-028): loyalty. `FUN_00561910` shifts every non-exempt
  character's base loyalty for a side's gain; Coruscant taken
  (`FUN_005616a0`, `0..=5` for the new side) and the Alliance HQ lost
  (`FUN_00561780`, `0..=-5` for the Alliance) are ported
  (`loyalty.rs`); 23 of 24 campaigns are unchanged, seed 9's HQ loss
  shifts once (`evidence/2026-10-19-loyalty-coruscant-hq/`).
- 2026-10-19 (F-028): loyalty after a battle. The winner gains its
  enemy's losses less its own over SDPRTB 2560 (`FUN_00560d00`,
  `FUN_0053ec10`; the losses are the lost units' class `+0x58`), in every
  battle path; 15 battles shift loyalty in 24 campaigns
  (`evidence/2026-10-19-loyalty-battle/`).
- 2026-10-19 (F-028): loyalty at a blockade's end (`FUN_00561280`): the
  assault and bombardment losses since the blockade began, by the lost
  units' class `+0x58`, credit the holder against the blockader; save v89
  (golden regenerated: initial `v1:29ee3368cb892d9e`, final
  `v1:fb78593dbd336659`) (`evidence/2026-10-19-loyalty-blockade/`).
- 2026-10-20: Luke taken to the Emperor. `FUN_0048dc50`'s `0x71a2` (read
  from the image) needs Luke and Vader on one mission whose target
  container is the Emperor's, which the Final Battle's DarthToEmperor
  Move order is; ported as `story_messages::taken_to_emperor`
  (`evidence/2026-10-20-luke-taken-to-emperor/`).
- 2026-10-20: a captured mission member takes a remove request
  (`FUN_004f18e0` -> `FUN_005345d0`), so rule 1 ends a mission whose team
  was all captured with code 5; a Move order no longer carries its
  prisoner on (the Final Battle's `DarthToLuke` repeated 76 times in seed
  14). Save v90 (golden regenerated: initial `v1:0d8727a7d2a2ee81`, final
  `v1:3f8a4e8cc1d73a06`) (`evidence/2026-10-20-capture-remove-request/`).
- Acceptance: Character timers are ported, story events are recovered from the
  original or tagged port:, and event ids no longer collide with timer ids.

### F-039: Ghidra notes and agent docs repeat unverified claims

- Severity: P2
- Status: partially remediated
- Evidence:
  - FUN_004927c0 is called the master tick and 0x1f0 the daily AI trigger
    (agent_docs/ghidra-re.md:155, ai-parity-tracker.md:52,92,123);
    FUN_004927c0 formats notification text and only the mission code sends
    0x1f0
  - FUN_00508250 is described as per tick; it runs once at setup
  - Reviews and notes cite community-disassembly addresses, which name
    different functions in our build (ghidra/notes/community-address-remap.md)
  - entity-system.md:383 conflicts with two iterator recoveries on the fleet
    type range
- Fix (2026-10-14): `agent_docs/ghidra-re.md`,
  `agent_docs/systems/ai-parity-tracker.md`, `agent_docs/simulation.md` and
  `agent_docs/architecture.md` no longer call FUN_004927c0 the master tick,
  0x1f0 a daily AI trigger, or FUN_00508250 a per-tick pass; each cites
  `ghidra/notes/timer-scheduler.md`.
- Open: the community-address citations in reviews and notes, and the
  entity-system.md:383 fleet type range.
- Acceptance: Each false claim is corrected at its source and every note
  citation resolves to a .c line in our build.

### F-040: Original mission kinds the port lacks

- Severity: P2
- Status: open
- Evidence:
  - MISSNSD holds 25 records; the port has 10 agent kinds plus Autoscrap.
    The missing kinds, with their outcome slots:
    - Reconnaissance `0x54` (always succeeds, `56bec0`)
    - Research `0x53` (three records, repeat, `56cdb0`)
    - Palace `0x64` (rescue plus free every prisoner there, `56e650`)
    - Bounty `0x65` (`575de0`)
    - Jedi Training `0x58` (`5712b0`)
    - Dagobah `0x71`, Vacation `0x72` and Pickup `0x73`
    - the movement pseudo-missions `0x41..0x44`

    See `ghidra/notes/mission-lifecycle.md`.
  - F-019 ports phases, decoys, and the per-member roll for the port's 10
    kinds only (decision 2026-09-28).
- Reconnaissance (2026-10-14): `MissionKind::Reconnaissance` runs the
  lifecycle on record `0x54000015`: the system rules (`FUN_0056be20`), a
  chance of 100 (`FUN_0056bea0`), result 3 with no raise (`FUN_0056bec0`),
  and observation level 8 at a system target (`FUN_0056bee0`; port: the
  port reveals the system, as for Espionage). Its member masks admit only
  the Longprobe Y-wing Recon Team and the Imperial Probe Droid (special
  force mask `0x104`, character mask 0). The dialog lists it at another
  side's system. Browser: fixture `ReconTargeting` (code 63), both sides,
  offers Reconnaissance alone and dispatches it
  (`evidence/2026-10-14-reconnaissance/`). Open: its AI planners
  (`FUN_0047f790`, `FUN_0047f320`, `FUN_0047f500`).
- Research (2026-10-14): `MissionKind::ShipDesignResearch`,
  `FacilityDesignResearch` and `TroopTrainingResearch` run the lifecycle on
  records `0x53000020..22`. The validator `FUN_0056c510` adds to the system
  rules a creation check (each character needs the tree's skill,
  `FUN_0056c820`) and two running checks: a working yard of the tree's area
  at the target (end `0xc`) and the side's Done flag (end `0xe`). Each team
  member rolls below its skill and on a hit adds 1 + rand(0..=6) points and
  raises the skill by GNPRTB 6158 (`FUN_0056cdb0`); `FUN_0056cf30` adds the
  points to the side's tree. The AI planners `FUN_0047bcd0`, `FUN_0047c2b0`
  and `FUN_0047c900` send one member of skill 10 or more. Browser: fixture
  `ResearchTargeting` (code 64), both sides, dispatches Ship Design
  Research; a seed-42 dual-AI playtest reaches Ship level 4 in 600 days
  (`evidence/2026-10-14-research-missions/`).
- Research AI (2026-10-14): the strategy's choice (`FUN_004e8290`,
  `FUN_0041bc10`, `FUN_0042dd00`) and the planners' target (`FUN_0047be50`,
  `FUN_00433e40`) are read and ported. The AI runs one research mission per
  able character per tree until its Done level, cancels the weakest team
  when it has a surplus or the tree is done, aborts a mission whose system
  stops being a research site, and aims at the best-supported site in the
  sector with the most sites (`AIAction::CancelMission`). A seed-42
  dual-AI playtest has the Empire run two ship research missions and reach
  Ship level 6 in 600 days.
- Research planners (2026-10-14, save v37): the port's simplifications were
  checked against the original and fixed. Persistent research planners hold
  their designer, a strategy cancellation leaves the mission running, the
  abort (order `0x250`, end code 2) follows each Mission Report, equal keys
  fall to coin flips (`FUN_005f59f0`, `FUN_0041c250`), and selectors pass
  over candidates at blockaded systems (`FUN_0047ba90`). Golden
  `v1:a6ee248d7d0194dc` -> `v1:68580f6dd8f9f6b0`
  (`evidence/2026-10-14-research-planners/`).
- Move (2026-10-14): `MissionKind::Move` runs the lifecycle on record
  `0x41000001` (hidden, timer 0, detection on, own-side target only) with
  the base phase handler `FUN_00524b70`, which rolls no one in phase 10,
  and the base rules `FUN_00523450`. The research planner orders it first
  while its designer stands away from the target (state 7,
  `FUN_004bd570`), then the research. Seed-2 dual-AI playtest: Crix Madine
  moves to Umgul on day 28 and researches there from day 70; the seed-42
  golden is unchanged (`evidence/2026-10-14-move-mission/`). Open: Return,
  Autorouting and Adrift (`0x42..0x44`) and the player's character moves.
- 2026-10-18: Jedi Training (`0x58`) runs through the lifecycle
  (`MissionKind::JediTraining`, save v71): the phase-2 teacher, the rules
  of `FUN_00570690`, the chance `FUN_005710b0` and the `ForceTraining` gain
  `FUN_005712b0` (F-038). No AI planner sends it yet
  (`evidence/2026-10-18-jedi-training/`). Open: Palace, Bounty, Dagobah,
  Vacation, Pickup, Return, Autorouting, Adrift.
- 2026-10-18: Dagobah (`0x71`) runs through the lifecycle with an abode
  target (F-038, `evidence/2026-10-18-dagobah/`). Open: Palace, Bounty,
  Vacation, Pickup, Return, Autorouting, Adrift.
- 2026-10-20 (re-check): no AI planner sends Jedi Training in the original.
  `FUN_00402720` builds the key `0x58000042` (`0x402c74`, the image's only
  use of that id) and sets no capability bit, so the port's player-only Jedi
  Training matches (test `no_planner_capability_names_jedi_training`). Since
  the entries above, Palace and Bounty (`148dbd83`, `b43895ec`), Pickup
  (`f05e1831`) and the Reconnaissance planners (2026-10-18) are ported.
  Open: Vacation `0x72` (two records, Alliance only), and the pseudo-
  missions Return `0x42`, Autorouting `0x43` and Adrift `0x44`.
- 2026-10-20: Vacation `0x72` is deliberately excluded with evidence. Its
  records `0x72000045` (Vacation, 60 days) and `0x72000046` (Sabbatical,
  1000 days) are hidden and Alliance only, and nothing in the original
  creates one: neither id appears in the image as a value, the create
  command `0x250` (`FUN_0054d360` -> manager slot `+0x1bc` `FUN_00549bf0`)
  takes its id from an order that the dialog fills only with records that
  are not hidden and the AI planners only with their capability kinds
  (`FUN_00402720`), and the class is reached only through its factory
  (`FUN_0051ebf0`). Open: Return `0x42`, Autorouting `0x43` and Adrift
  `0x44` (`ghidra/notes/mission-lifecycle.md`).
- 2026-10-20 (read): how members leave a mission, and the Return,
  Autorouting and Adrift orders. `FUN_00548120` (run after every phase
  change) asks `FUN_00521220` how each member of an ended mission, or one
  with a request, leaves: MISSNSD column 10 is 0 for the movement orders, 1
  for agent missions (return when the member's container is not its side's)
  and 2 for Dagobah, Palace, Vacation, Pickup and Bounty (always return).
  `FUN_00548910` groups the returning members by destination
  (`FUN_00542ee0`: the container, else the character's `+0x70`, else the
  autoroute search `FUN_00543040`) and sends them on a Return order
  `0x42000002` (`FUN_005475f0`); with no destination, `FUN_00549580` sends
  them to their side's jail (abodes `0x98000440` Alliance Jail and
  `0x98000480` Empire Jail) on an Autorouting order `0x43000003`. The port
  releases members where they are, so an agent stays at another side's
  system after its mission; porting the Return is the next stage
  (`ghidra/notes/mission-lifecycle.md`).
- 2026-10-20: the Return order is ported (save v91).
  `MissionRecord::leave_rule` holds MISSNSD column 10;
  `MissionSystem::finish` sends each team or decoy member of an ended rule-1
  mission that stands in a container of another side on a Return order
  `0x42000002` (`MissionKind::Return`, Move's slots), grouped by destination
  (`FUN_00548910`, `FUN_00542ee0`'s autoroute search; port: the container
  and `+0x70` candidates are skipped, and the jail fallback, rule 2 and the
  Mandatory flag are not ported). 24 dual-AI campaigns: 11,979 Return
  orders, and the AI's own Move orders fall from 44,841 to 32,470. Golden
  regenerated: initial `v1:68bc58d8f3fb696d`, final `v1:5fc59cefe817d1b4`
  (`evidence/2026-10-20-return-order/`). Open: Autorouting `0x43` and Adrift
  `0x44` as orders, and the jails.
- 2026-10-20: a character's Move from its menu. `0x201` expands a character
  into the Move mission order `0x241` (`FUN_0053c810`); `FUN_004f4a00`
  enables it for any character that resolves, and `FUN_004f4b60` refuses a
  movement kind's target of another side (`1/0x28`). The port enables the
  menu's Move for the player's living characters, targets a system, checks
  `missions::move_target_allowed` and dispatches the Move mission; browser
  acceptance on fixtures 69 and 68, both sides
  (`evidence/2026-10-20-character-move/`). Manual pp. 110-111 confirms the
  Return to a base system; open: the base (`+0x70`) that Move sets,
  Confirmed Move, a fleet target, and the drag of a character.
- 2026-10-20: a character's Move aboard a fleet (save v92). The Move order
  `0x241` takes a fleet as its container; the validator reads the fleet's
  side, and `FUN_004f7640` lands the members in it. The port keeps
  `ActiveMission::target_fleet`, reads the fleet in the validator, and
  boards the member on landing (MemberBoarded); port: no rerouting when the
  fleet moves. Browser acceptance on fixture 70, both sides. Golden
  regenerated: initial `v1:e4659ba5e1a2e054`, final `v1:21001b04a1c4b3b9`
  (`evidence/2026-10-20-character-board/`).
- 2026-10-20: a character in the Fleet window. The Personnel tab lists each
  character as an object: a right click opens its menu (manual p. 56,
  Command) and a drag out of the list issues the Move `0x201` -> `0x241`
  against the window under the release (`FUN_006083c0`, `FUN_00422ce0`;
  manual p. 111). Browser acceptance on fixture 71, both sides
  (`evidence/2026-10-20-character-drag/`). Open: the drag out of a Defenses
  or system window (`0x214`).
- 2026-10-20: a character's Confirmed Move (manual pp. 110-111). `0x202`
  expands to the same Move mission order `0x241`; `FUN_00487cc0` opens the
  confirmation with the trip's days (`FUN_0053c2e0` -> `FUN_004f4850`), and
  `FUN_0049a350` shows the evacuation warning `0x7056` for a character in a
  blockaded system. Ported with `missions::move_days` and MoveConfirmation's
  characters; browser acceptance on fixtures 69, 70 and 68, both sides
  (`evidence/2026-10-20-character-confirmed-move/`).
- 2026-10-20: the Return goes to the character's base. `+0x70` is where the
  member joined its mission: setting RoleOnMission (`FUN_00534800`) runs
  Role_vt268 (`FUN_00536490`), which stores the member's container in +0x70
  (`FUN_00534390`) and clears it on leaving; `FUN_00542ee0` takes it before
  the nearest search, and a Return needs its own side's target
  (`FUN_004f4b60`). The port takes the mission's origin while the member's
  side holds it (manual pp. 110-111, the base). 24 dual-AI campaigns: the
  AI's own Move orders fall from 32,470 to 20,412
  (`evidence/2026-10-20-return-to-base/`).
- 2026-10-20: a Move's traveller follows its target fleet. The original
  reroutes a traveller whose target container moves (`FUN_004fbc20`,
  `GameObjParentReroutedNotif`); the port, on arrival, sends the member on
  to the fleet's new system and moves the mission's place with it, and the
  member boards there (`evidence/2026-10-20-fleet-reroute/`).
- 2026-10-20: a character dragged out of the Defenses window moves.
  `FUN_00422ce0` issues `0x201` for window types 4 and 10 and `0x214` only
  for type 9, the System window, which lists facilities; so the Defenses
  window's character drag is the Move mission order `0x241` (manual p. 111).
  Ported with `DefensesWindowAction::DragCharacter`; browser acceptance on
  fixtures 69 and 68, both sides (`evidence/2026-10-20-defenses-drag/`).
- 2026-10-20: the Adrift order `0x44000004` and its detection.
  `MissionKind::Adrift` exists, and `mission_detection` reads the mode word
  per kind: Adrift's `0x4103` (`FUN_00576a20`) gives phase 2 mode 3, whose
  two passes run mode 1's body and then mode 2's with the flags kept
  (`FUN_00589a40`); other kinds run one pass as before (24 campaigns
  identical). Nothing creates an Adrift order yet; the battle's adrift path,
  which replaces the integrator's unsourced capture of a losing fleet's
  characters, is the next stage (`evidence/2026-10-20-adrift-detection/`).
- 2026-10-20: a destroyed fleet's characters go adrift, not captive (save
  v93). The integrator's unsourced capture of a losing emptied fleet's
  characters is removed; as `FUN_00556710` -> `FUN_004ef2e0` does, they go
  adrift where the fleet stood, and `MissionSystem::send_adrift`
  (`FUN_0054c7d0`, `FUN_00549230`) settles one at a standing system of its
  side and sends the others on an Adrift order `0x44000004` to the nearest
  place of their side, where the order's detection may capture them. Golden
  regenerated: initial `v1:57cfc0d727199c15`, final `v1:4dee6ff02fdbe5bc`;
  campaigns unchanged, as the AI puts no characters aboard fleets yet
  (`evidence/2026-10-20-adrift-path/`).
- 2026-10-20: the Autorouting order and the jails are excluded with
  evidence. A traveller already in hyperspace lands though its order ends
  (`FUN_00522fc0`), as the port's direct trip does, and the order's
  mandatory flag cannot matter while the traveller is off the map; the port
  keeps its `EscapeeTransit` (port:, cited in code). The jails
  (`FUN_00549580`) need a member with no destination, but the move validator
  the autoroute search uses (`FUN_005571c0` -> `FUN_005513a0`) has no side
  test, so an Alliance or Empire character always finds the other side's
  systems while one stands; only a side-3 character could reach a jail, and
  none is sent home (`ghidra/notes/mission-lifecycle.md`).
- 2026-10-20: the player's tactical battle sets a destroyed fleet's
  characters adrift too. `tactical_flow::apply_results` kept a copy of the
  unsourced capture; it and the integrator now share
  `missions::cast_adrift` (`FUN_00556710` -> `FUN_004ef2e0`). The port's
  other fleet removals leave no character to set adrift: a merge moves
  them, scrap disbands only a fleet without characters, and the Death
  Star kills them (Death Star area). Campaigns and replay unchanged
  (`evidence/2026-10-20-adrift-tactical/`).
- Acceptance: Each original kind runs through the F-019 lifecycle with its
  recovered slots, AI planner, and legality rules, or is documented as
  deliberately excluded with evidence.

## Fable 5.1 audit synthesis

The Fable review confirmed the original blockers and sharpened several
interpretations:

- For the non-original menu-music extension, Fable judged the behavior
  shippable after a one-function visual pass: six-pixel canopy clearance,
  Project Thorn-style cyan confined to the projection, a rectangular inset
  status lamp, static rest art, restrained 5 Hz hover interference, pressed
  displacement, gold-only focus, and congruent paint/hit bounds. F-016E
  applies and verifies each requirement.
- The `19 pass / 5 fail / 1 skip` parity score is not five equivalent gameplay
  failures. The original AI interval is a documented augmentation, three Death
  Star checks expose structural AI/event gaps, and the research mismatch may be
  an oracle-ID mismatch. The run remains practically degenerate despite the
  evaluator's `false` flag because it only detects the absence of combat.
- The Fable finding that main-menu Load Game skipped slot selection and faction
  restoration has since been remediated and browser-verified under F-001.
- Browser main-menu music and effects are now packaged, but advisor voice and
  cutscene/media parity remain incomplete. Release packaging must continue to
  stage owned runtime assets explicitly.
- Auto-resolve and tactical calculations can still produce different outcomes.
  Tactical ground results now persist exact survivor damage and occupation
  captures characters, but full path convergence remains M2.
- Autoresearch parameter tuning should remain paused until faction liveness,
  capture/evasion, the wider campaign loop, and target acquisition are
  established.

## Optimization and parity roadmap

The order below makes the acceptance ledger executable. Milestones are gated;
later work must not hide failures in an earlier invariant.

| Milestone | Scope | Exit criteria |
|-----------|-------|---------------|
| M0 — Truth and bleeding | Wire save/load/delete and Load Game selection; fix native HD root and `TROOPSD.DAT`; ship browser data; attach evidence to README claims; add deterministic replay gates. Browser Save/Load/Delete, paths, a self-contained four-request package, F-011A fingerprints, the F-011B1 continuation envelope, the F-011B2/B3 replay pipeline, and F-011B4 native/WASM fixture equivalence are verified. Native GUI restart and persistence hardening remain open. | Persistence works on native/WASM, the packaged site boots from a clean directory, and claims link to current evidence. |
| M1 — Simulation correctness | F-007A–D close redispatch, fleet-position, player-dispatch, and combat-backlog defects. F-007E closes ownership, friendly cycling, transit fan-in, troop-class lookup, blockade garrison, repair state, troop transport, political occupation, and the source-backed asymmetric victory contract. Capture/evasion, faction liveness, target acquisition, and the wider campaign loop remain open. | Across five fully identified 5,000-tick seeds: transit ≤10% of fleets, move orders ≤1.5× arrivals, fleet arena ≤3× initial, both factions remain capable of productive action, every contract-correct victory fixture passes, and native/WASM checkpoints match. Record the provisional encounter volume/spread numbers as diagnostics until calibrated. Qualitative behavior matches the historical campaign reference. |
| M2 — One game engine | Route app and playtest through one tick API and event sink; make combat resumable from core state; construct victory UI; remove or correctly simulate `AdvanceTicks`. | Same seed plus command stream yields identical checkpoints and final state across interactive, headless, native, WASM, auto, and tactical paths. |
| M3 — Browser excellence | Extend the verified deterministic `runtime.orpk` foundation with Brotli compression, bounded raw/decoded caches, HD entries, high DPI, one egui pass, cached geometry, IndexedDB, gesture-unlocked audio, owned advisor assets, and cross-browser input suites. | Cold start ≤3 s at 50 Mbps/30 ms, ≤4 requests before menu, combined heap/WASM ≤256 MB after 10 minutes, no visual/input failures in current Chrome/Firefox/Safari. |
| M4 — Multiplayer | Introduce validated, tick-stamped commands; authoritative host simulation; faction-filtered fog-safe deltas and snapshots; secure WSS transport; prediction/reconciliation; reconnect; persistence and observability. | Two clients run 5,000 ticks with matching server checkpoints every 250 ticks; at 200 ms RTT there are no input stalls and ≤1 reconciliation per 100 commands; reconnect within 60 s; all illegal commands rejected; hidden state absent from client memory. |
| M5 — Continuous proof | Enforce format/clippy/build/browser checks; short and long campaign gates; resource and screenshot ledgers; app integration tests; package boot and data/save hashes. | Every supported P00–P40 pass is green from release artifacts, with reproducible evidence retained by CI. |
| v1.0 — Protected Cloudflare release | Deploy the self-contained browser build to Cloudflare Pages with Functions middleware, `SITE_PASSWORD` and `SESSION_SECRET` secrets, signed secure cookies, asset headers, preview/production environments, and rollback instructions. | Anonymous requests cannot retrieve HTML, WASM, DAT, bitmap, save, or multiplayer endpoints; valid login survives navigation; invalid/expired/tampered sessions fail closed; logout works; independent browser acceptance verifies gameplay and bitmap evidence through the deployed URL in current Chrome, Firefox, and Safari. |

### Browser performance budgets

- Startup: menu interactive within 3 seconds at 50 Mbps and 30 ms RTT.
- Simulation: ≤4 ms per native tick and ≤12 ms per WASM tick at 1,000 fleets.
- Rendering: ≤8 ms per 1280×800 galaxy frame at the reference fixture.
- Distribution: optimized WASM ≤5 MB; initial resource requests ≤4.
- Memory: combined JS heap and WASM memory ≤256 MB after 10 minutes.
- Regression policy: fail CI when a tracked metric regresses by more than 10%.

### Multiplayer target architecture

Single-player should execute through the same validated `Command` boundary used
by multiplayer. A server owns the authoritative simulation and emits
fog-filtered `Delta` or `Snapshot` messages per faction; clients never receive a
complete hidden `GameWorld`. Each command carries `{player, tick, sequence}` and
is validated and rate-limited server-side. Begin with an in-process transport
for deterministic tests and secure WebSockets for deployment. Add WebRTC only
as an optional trusted lockstep mode after deterministic replay is proven.

The existing `net_protocol.rs` is a notification vocabulary, not a complete
network envelope. Isolate protocol and server responsibilities in dedicated
`rebellion-net` and `rebellion-server` crates rather than coupling sockets to UI
panel actions.

## Feature-by-feature acceptance plan

Each row is a bounded pass. A pass closes only after the user-visible behavior
and underlying state mutation are both demonstrated.

| Pass | Features | Required acceptance |
|------|----------|---------------------|
| P00 | Supported scope | Define native and WASM contracts. Explicitly include or exclude browser audio, video, advisor, EData, and mods. |
| P01 | Build infrastructure | Formatting, warning-free all-target check, strict clippy, all required tests, native build, actual WASM packaging, and artifact inspection. |
| P02 | Data and startup | Every required/optional DAT table, string names, entity counts, troop classes, clean boot, and missing/corrupt-data errors. |
| P03 | Main menu | **Pass.** Assemble and operate the original 14 cockpit controls, animation, navigation, keyboard and screen-reader access, `MDATA.300` music, original button effects, responsive 4:3 hit testing, and the documented optional music-only extension per [`main-menu-parity.md`](../../../agent_docs/main-menu-parity.md). Native and browser acceptance pass. |
| P04 | Game setup | Use the cockpit controls for both factions, three difficulties, three original galaxy sizes, both game types, direct campaign start, correct state propagation, and clean second-campaign reset per [`main-menu-parity.md`](../../../agent_docs/main-menu-parity.md). |
| P05 | Clock | Pause and every speed, focus loss, browser background/resume, modal/combat/cutscene tick behavior. |
| P06 | Galaxy navigation | Reproduce the original galaxy/GID, sector, and system-window graph, selection, object menus, pan/zoom semantics, resizing, high-DPI transform, and cockpit input boundaries per the [interface audit](../2026-09-10-interface-parity-audit/). First-pass sector and system shells plus the rail lifecycle work; the synthetic galaxy, complete item compositions, commands, uncommon states, and exact rail thumbnails remain open. |
| P07 | Fog and overlays | Both factions, original intelligence/control colors, sensor/recon visibility, every GID filter and matching legend, fleet/facility/system markers, and no hidden-information leakage. Current primitive markers fail visual parity. Since 2026-10-22 the marks' size bands and the personnel count follow `FUN_0042b330` (`evidence/2026-10-22-gid-marker-bands/`), and the fleet displays mark both sides and the counting side's colour (`evidence/2026-10-22-gid-fleet-marks/`). The 22 legend layouts (`FUN_004522f0`) load with their texts (`evidence/2026-10-22-gid-legend-layouts/`), and choosing a display opens its legend window with the original frame, close button, legend button and drag (`evidence/2026-10-22-gid-legend-window/`). The marks reveal what `FUN_0042b330` reveals: it asks nothing of the player's knowledge, gives unexplored and destroyed systems the unexplored mark, and lets Fleets Enroute and Active Personnel count unexplored ones (`evidence/2026-10-22-gid-visibility/`); an explored, unpopulated system takes its holder's (no-side) marks (`evidence/2026-10-22-gid-unpopulated-marks/`); a click on a system closes a visible legend first (`evidence/2026-10-22-gid-legend-map-click/`), and the system opens on the release, as `FUN_00422ce0` does (`evidence/2026-10-22-map-opens-on-release/`). The legend's texts use the original's face, Arial (`FUN_0060ee20`), drawn with its metric match Liberation Sans at the original cells (`evidence/2026-10-22-arial-metric-font/`). The legend drags live and is moved back inside on the drop (`FUN_00607010`). The advisor's opening narration (`FUN_004c0fc0`, reference captures 0119..0149) shows displays `0x13`, `0x14`, `0x15`, `0x17`, `0x20` and `0x91..0x93`, which the port lacks; they are to be ported with the narration (P34) (`evidence/2026-10-22-legend-drop-and-display-senders/`, `ghidra/notes/gid-markers.md`). The side-0 flag settles the last mark assumptions (`evidence/2026-10-22-gid-side-zero-flags/`). Since 2026-10-23 the counting displays count the system holder's units for either viewer, as `FUN_0042b330` does (`evidence/2026-10-23-gid-holder-counts/`). |
| P08 | Personnel | Reproduce Personnel Finder, Character Status, special forces, every portrait/state, selection, assignment, availability, captivity, injury, death, Jedi state, item menus, and detail refresh. Since 2026-10-22 the replacement Officers panel is removed; its contents are in Personnel Finder, Character Status and the Command submenu (`evidence/2026-10-22-officers-panel-removed/`). The Personnel Finder's two views were swapped and now follow `FUN_004637a0`/`FUN_00464e10`, and its rows carry Injured, the command word and abodes (`evidence/2026-10-22-personnel-finder-views/`). A character's Retire order works as `FUN_004ed460`/`FUN_0049a350`/`FUN_00534c00` do, save v104 (`evidence/2026-10-22-character-retire/`), and a special force's as `FUN_00533ba0` does (`evidence/2026-10-22-special-force-retire/`); a special force's Move and Confirmed Move work as a character's (`evidence/2026-10-22-special-force-move/`). The replacement Force panel is removed; the Status window's Force Ranking row is the original's only Force display (`evidence/2026-10-22-jedi-fleets-panels-removed/`). Since 2026-10-23 special forces board fleets by Move, travel and merge with them, stand at the system when the fleet is destroyed (`FUN_00503e20`), and are listed on the Fleet window's Personnel tab and as a fleet row in the Personnel Finder (`FUN_00464e10`); save v105 (`evidence/2026-10-23-special-forces-aboard-fleets/`). |
| P09 | Fleets | Reproduce Fleet/Ship Finders, original Fleet and status windows, tabs, icons, selection, move, cancel, split/merge/transfer, cargo, transit, invalid operations, arrivals, duplicate prevention, and post-combat refresh. Current custom panels remain visual failures. Since 2026-10-22 the replacement Fleets panel is removed; its contents are in the Fleet Finder and the Fleet window (`evidence/2026-10-22-jedi-fleets-panels-removed/`). Since 2026-10-22 the missions icon's Abort (`0x250`) asks and ends the side's visible missions there (`FUN_00521600`, `FUN_005233a0`), and the last replacement panels (Missions, Bombardment, Death Star, Loyalty) are removed (`evidence/2026-10-22-mission-abort-panels-removed/`). |
| P10 | Economy | Income, collection, support drift, maintenance, shortfall, incidents, nonnegative invariants, and faction ownership effects. |
| P11 | Manufacturing | Enqueue, cancel, prioritize, capacity, costs, each product category, blocked production, completion, and usable world insertion. |
| P12 | Diplomacy | Legal targets, probability boundaries, success/failure, support/control change, cancellation, messages, and persistence. |
| P13 | Recruitment | Eligibility, success/failure, unique recruitment, repeated-order prevention, cancellation, telemetry, and persistence. |
| P14 | Espionage | Legal targeting, intelligence visibility, success/failure, foiling, informants, messages, and persistence. |
| P15 | Sabotage | Normal and Death Star sabotage, damage/delay, defense interaction, success/failure, telemetry, and persistence. |
| P16 | Character operations | Assassination, abduction, rescue, capture, health, death cleanup, escape, decoys, foiling, and autoscrap. |
| P17 | Uprisings and betrayal | Incite, subdue, thresholds, occupation, control transitions, allegiance changes, suppression, and telemetry. |
| P18 | Research | Ship/troop/facility trees, assignment exclusivity, progression, unlock events, no duplicate projects, and mid-project save/load. |
| P19 | Jedi | Force discovery, eligibility, training tiers, trainer loss, captive/dead states, story interactions, and persistence. |
| P20 | Movement | Distance timing, Han bonus, order/cancel/reorder, references, arrival, battle triggering, and no duplicate orders. |
| P21 | Blockade and repair | Enter/exit, ownership/economy effects, breach outcomes, hull recovery, cost/cap, interruptions, and persistence. |
| P22 | AI | Both factions; validator pass/reject boundaries; budgets; research; production; troop deployment; recon; defense; retreat; target deconfliction; Death Star escort/targeting. |
| P23 | Core space combat | Seven phases, weapon classes, shields, ion effects, recharge, carriers, fighters, officers, Emperor, retreat, destruction, and result application. |
| P24 | Tactical space combat | Placement, selection, formations, movement, focus fire, pause/speed, retreat, visual state, accepted formulas, and galaxy result application. [Per-hull/fighter result identity and shared entry](evidence/2026-09-12-tactical-result-identity.md) are verified partial tranches. P54 through P57B2C2B stage and decode the original 3D corpus and recover its camera, placement, transform, palette, light, and retained-mode state. [P58A](../2026-09-10-interface-parity-audit/evidence/2026-09-14-tactical-resource-join.md) joins every ship and fighter DAT identity to its original resources and transports all 87 meshes and 397 textures. P58B through P58F13 render live capital/fighter families and restore their interaction and command paths. The [P58-B06 checkpoint](../2026-09-10-interface-parity-audit/evidence/2026-09-22-tactical-completion-bundle.md) adds capital and fighter combat, collision, automatic groups, retained formations, the separate Death Star, original result/options panels, superlaser journey, both trench-run routes, and exact strategic roster, capture, and Death Star-state application. P58-B15 through P58-B21 restore Battle Alert, weapon and command voice banks, mixed-task-force rejection, recovery/withdrawal feedback, and shared post-battle orchestration. [P58-B22](../2026-09-10-interface-parity-audit/evidence/2026-09-28-tactical-source-completion.md) completes the bounded practical launcher with source-compatible tactical randomness, persisted power allocation, completion/destruction callbacks, and the timed source-derived trench-run producer. [P58-B07](../2026-09-10-interface-parity-audit/evidence/2026-09-22-tactical-106-matrix-contract.md) generates and validates the exact 106-cell denominator, and [P58-B08](../2026-09-10-interface-parity-audit/evidence/2026-09-22-tactical-a0-ingestion.md) adds fail-closed original-capture provenance and ingestion. Deterministic A1 mapping is runnable at 106/106. Owned lossless A0 coverage is 106/106 since 2026-10-24 (the last 12 cells from editor-assisted saves with a Death Star, `evidence/2026-10-24-death-star-a0/`); acceptance remains 0/106. Exact native beam pixels, audible native playback and mixing, whole-process random-state continuity, exact planet framing, rare warning/ejection callers, and strict battle acceptance remain open. Since 2026-10-23 the trench run's commander rating comes from the Alliance's battle commander (`FUN_005ad7e0`, `FUN_005a93e0`), as in the abstract battle (`evidence/2026-10-23-tactical-commander-rating/`). Since 2026-10-23 the app's battles (AI against AI, Simulate Results, Retreat and Take Command) take every fleet of both sides at the system, and the tactical result returns to each unit's own fleet (`FUN_0040c780`; `evidence/2026-10-23-battles-of-every-fleet/`). Since 2026-10-23 the AI side of a tactical battle keeps its retreat order and leaves under the abstract battle's rule (`FUN_005d19a0`, `FUN_005d14d0`), and a ship that escaped the tactical field keeps its hull (`evidence/2026-10-23-tactical-retreat/`). Since 2026-10-23 the player's withdrawal lasts the side's time, 30 s or 150 s for the Empire with an operational Death Star (`FUN_005ad530`, `FUN_005c8780`; `evidence/2026-10-23-tactical-withdrawal-time/`). Its fighter groups return to their carriers and leave with them (`evidence/2026-10-23-tactical-fighter-withdrawal/`). Since 2026-10-23 only the groups without a hyperdrive do (order 2, `FUN_005cf980`); a group with one takes order 3 and jumps when the time ends (port: it holds meanwhile; the original heads for the side's exit, `FUN_005c6610`, read in `ghidra/notes/abstract-battle.md` "The jump out"). Since 2026-10-23 the player's withdrawing ships fly to their side's edge (`FUN_005af5f0` side vectors, `FUN_005cc300`) and, once the time ends, each jumps within 8.0 of it for 4000 ms (`FUN_005a6b40`); the battle lasts until the last jump (`evidence/2026-10-23-tactical-withdrawal-jump/`). Since 2026-10-23 the AI side withdraws the same way: the tactical battle's own commanders (`FUN_005be160`, `FUN_005c8780`) run `FUN_005ad530` for it, rather than the abstract departure the port used, which removed its units at once (`evidence/2026-10-23-tactical-ai-withdrawal/`). The AI commander reviews on the first step and every sixth after, counts the enemy task forces' real targets, wants the enemy's strength above 0, and waits out a trench run (`FUN_005c8780`, `FUN_005c7f70`; `evidence/2026-10-23-tactical-ai-review/`). Since 2026-10-23 withdrawing fighter groups with a hyperdrive also fly to their edge and jump there for 4000 ms after the time (`FUN_005c6610`; `evidence/2026-10-23-tactical-fighter-exit/`). Since 2026-10-23 a withdrawing ship keeps firing and can be hit until its jump starts (`FUN_005b3a40`, `FUN_005cdf50`); the port had left every withdrawing ship out of the fight (`evidence/2026-10-23-tactical-withdrawing-in-the-fight/`). Since 2026-10-23 every ship of the withdrawing side withdraws, and when its last jump ends every unit of the side without a working hyperdrive is destroyed (`FUN_005c8780` state 4, event `0x2f`, `FUN_005b2720`, `FUN_005b2d20`); the player cannot select a withdrawing group (`evidence/2026-10-23-tactical-left-behind/`). A withdrawal also aborts its side's trench run, restoring the runners' hull and shields (`FUN_005cfec0` state 8, `FUN_005d08d0`; `evidence/2026-10-23-tactical-withdrawal-aborts-trench-run/`). The jump now flies its unit away with the original's four acceleration factors and plays TACTICAL.DLL WAVE 13065 (capital ship) or 13063 (fighter group) (`FUN_005a6b40`, `FUN_005bae60`; `evidence/2026-10-23-tactical-jump-flight/`). A jump also needs engine power (`FUN_005b1b70`; `evidence/2026-10-23-tactical-jump-needs-engines/`); the warning of a ship left behind follows `FUN_005a0240`, and the port's invented R-key partial retreat is removed (`evidence/2026-10-23-tactical-withdrawal-warning/`). Since 2026-10-24 lossless A0 captures of the original exist (94 of 106 cells, ignored store); by them the right-hand panels sit at (482, 28), the emblem shows with no selection, the Battle Options panel is 1306, the left order button opens Maneuvers, and the selected ship's meters show hull left and shield right, blue remaining and red lost for both factions (`evidence/2026-10-24-tactical-right-panel-a0/`); the lower HUD's navigation, pause, highlight, options, zoom and camera controls sit at their captured positions, with the memorize and recall art the right way round (`evidence/2026-10-24-tactical-lower-hud-a0/`); the group bar's slots are 29 px apart, with the fighter header at x 296 (`evidence/2026-10-24-tactical-group-bar-a0/`); the panel texts use Arial (Liberation Sans) at the original's fonts 8 and 10, at the captured positions, and the fighter panel shows group, orders and tactics (`evidence/2026-10-24-tactical-panel-text-a0/`), with its count line "X-wings: 1 12/12" built as the original does (`evidence/2026-10-24-tactical-fighter-count-line/`); a battle opens with both side highlights off, "Battle Paused" takes the player's colour and "Observe Battle" shows while observing (`evidence/2026-10-24-tactical-status-text-highlights/`); the holocube, never drawn before, is a blue wireframe cube one battle extent wide (`FUN_005d4d10`, `FUN_005d58c0`; `evidence/2026-10-24-tactical-holocube/`); the Battle Alert and Battle Summary windows open where the captures show them per side, their texts in Arial at the original fonts and places, and their art keyed (`evidence/2026-10-24-battle-windows-a0/`); the summary's forces page takes the original's fonts 0xb, 6 and 4 (`FUN_00471200`) and the Display page its title (`evidence/2026-10-24-battle-summary-pages-a0/`), with the Display page's second line in bold font 7 (`evidence/2026-10-24-display-page-bold-line/`), and the Death Star's laser control sits at (611, 2) with a pure green on blue meter that starts full, a committed shot shows 1023, the Death Star is the backdrop decal opposite the planet (`FUN_005d4d10`), and the battle itself plays no score (`FUN_00450540`, `FUN_00417640`; `evidence/2026-10-24-death-star-a0/`), the camera orbits the vertical axis with the pitch as its height (`FUN_005d9640`; the port had read the yaw axis as Z), the planet is an extent-wide decal at `+2e` x and the Death Star at `-2e` x (`FUN_005d4d10`), matching A0 TAC-01-C002, C008 and TAC-03-C020 (`evidence/2026-10-24-tactical-camera-backdrop/`), hull meshes load three times their authored size and turn to face their heading, with the collision box taken from the scaled base + 1 model before the first frame (`0x005d5b10`, `0x005d3500`, `FUN_005d26c0`; `evidence/2026-10-25-tactical-hull-frame/`), the superlaser beam grows pure green from the Death Star decal to its target in 1 s (`FUN_005da150`, `0x005da830`; `evidence/2026-10-25-death-star-beam/`), the opening and Death Star voice announcer follows `FUN_005ada10` (battle ready, Alliance warnings 0x90-0x94, the Empire's 0x11c six seconds later, as in A0 TAC-01-C012's recording; `evidence/2026-10-25-tactical-voice-announcer/`), and the withdrawal's jump variants, jump factor and task-force target follow `FUN_005bae60`, `FUN_005a6b40` and `FUN_005d2190` (`evidence/2026-10-25-tactical-hypotheses/`), and a withdrawing fighter group flies a full unclamped step until within 1.0 of its line and only then tries its jumps (`FUN_005c6610`, `0x005ec0d0`; `evidence/2026-10-25-fighter-withdrawal-flight/`), and a withdrawing capital-ship task force jumps only its first member, once arrived, one ship after another (`FUN_005cc300`, `0x005c6070`; `evidence/2026-10-25-capital-withdrawal-order/`), and the superlaser's beam starts four 1000 ms event turns after the shot, the target dying one turn later (`0x005ba720`, `FUN_005a96b0`, `0x005b2b60`; `evidence/2026-10-25-death-star-shot-timing/`), and the battle keeps the original's event turn queue, through which a withdrawal jump now waits for its events `0x2d` and `0x2e` (`FUN_005b1e40`, `FUN_005b26d0`; `ghidra/notes/tactical-event-turns.md`, `evidence/2026-10-25-tactical-event-queue/`), and the battle's logic (commanders, withdrawal decisions, recharge, repair and weapons) runs once a 1000 ms turn with its 1000 ms (`0x005a9d10`, `0x005a9d40`, `0x005a9de0`; `evidence/2026-10-25-tactical-turn-updates/`), and the units move only from turn 3 and are held within +-2/3 of the battle extent (`0x005a9ec0`, `0x005bef1d`; `evidence/2026-10-25-tactical-frame-movement/`), and the trench run flies its approach (`FUN_005c6610`'s order step, each frame) before state 5 starts the 120 s run with its follow-up line or the Empire's 0x11d (`FUN_005cfec0`; `evidence/2026-10-25-trench-run-approach/`), and each weapon shot shows its projectile at its fire event (+1) and deals its damage at its hit event (+2) through the queue (`FUN_005b3f10`, `FUN_005b1f60`, `0x005b2ac0`; `evidence/2026-10-25-weapon-shots-queue/`), and its subsystem hits follow as queued events 0x1e-0x22 (`FUN_005b5f50`; `evidence/2026-10-25-subsystem-hits-queue/`), whose hull branch above 100 cannot be reached (`evidence/2026-10-25-hull-score-above-100/`), and the hulls show their lit outer faces with an unclamped light and pick their model by depth along the view against the field (`FUN_005d3770`; A0 TAC-01-C003; `evidence/2026-10-25-hull-rendering-a0/`), with FUN_005d3770's depth as the hull's camera-space position dotted with the camera's world direction (`evidence/2026-10-25-hull-detail-depth/`), and a player's order is set at once but its executor runs on the command turn through event 0xa (`SetOrder` `0x005ca530`, `FUN_005ca6d0`; `evidence/2026-10-25-player-orders-queue/`), as do the withdrawal's orders and its trench-run abort (`FUN_005ad530`; `evidence/2026-10-25-withdrawal-orders-queue/`), with each withdrawing ship deciding its own jump in a force of its own (`FUN_005aded0`; `evidence/2026-10-25-withdrawal-own-forces/`), and an order to one clicked unit gives it a force of its own while several units form a task force in the first free slot (`0x005a1a60`, `0x005a2c70`; `evidence/2026-10-25-selection-task-forces/`), and a Surround tactic takes effect on its 0x3d turn with the original's 22 world-axis slots of 12.5 (`0x005c9c50`, `0x005eb8d0`; `evidence/2026-10-25-surround-formation/`), and capital forces now close on their targets to their tactic's band of distance (`FUN_005cc300`, `FUN_005edf80`; `evidence/2026-10-25-closing-on-targets/`), as do fighter groups, which circle a target once within reach (`FUN_005c6610`; `evidence/2026-10-25-fighters-closing/`), and a computer force of more than 7 ships in Stand Off switches itself to Surround (`0x005ccdda`, `FUN_005c9be0`; `evidence/2026-10-25-ai-surround-switch/`), and each side starts facing the other, as in A0 TAC-01-C003 (`FUN_005ab650`, `FUN_005b5ad0`; `evidence/2026-10-25-side-facing/`), and the superlaser cone rolls as `LookAt` with the Z constraint does (`d3drm.dll` `FUN_6d81f86b`; `evidence/2026-10-26-beam-cone-roll/`), and every capital force moves on its own anchor at 0.95 of its slowest member's speed, its members steering to their places (`FUN_005ea740`, `0x005eb1a0`, `0x005eb540`, `FUN_005b2f30`; `evidence/2026-10-26-formation-movers/`), and hulls reach the 8-bit screen through the RGB device's 6×6×6 colour cube on the battle palette, as in A0 (d3dim `FUN_7403e4f0`, `FUN_74029f67`; `evidence/2026-10-26-rgb-cube-output/`), lit once per face as d3drm's default `D3DRMRENDER_FLAT` device quality makes them (`0x6d81a08b`, `FUN_6d827ec2`; `evidence/2026-10-26-flat-hull-shading/`), and the browser canvas draws without multisampling, so hulls and field lines keep A0's solid palette colours (`evidence/2026-10-26-browser-no-multisampling/`), and members climb within the nearby ships' height bands (`FUN_005cdbf0`, `FUN_005b2f30`; `evidence/2026-10-26-height-bands/`), and a held member of a still formation takes the original's detours (`FUN_005ea740`, `0x5eb040`; `evidence/2026-10-26-formation-detours/`), and hulls are lit as d3dim lights them, ambient 128, light clamped before the texture, specular added (`FUN_6d837223`, `FUN_7402029b`, `FUN_7403a950`; `evidence/2026-10-26-d3dim-hull-lighting/`) with each hull's own view direction (`FUN_74017fc4`, `FUN_7402029b`, `FUN_6d8410d6`; `evidence/2026-10-26-object-view-direction/`), and units are drawn between their last two 250 ms steps, so they move smoothly (`evidence/2026-10-26-smooth-unit-motion/`), and the computer's commander links its task forces to targets each review, three turns ahead, so the computer side closes and fights (`FUN_005c8780`, `FUN_005c9200`, `FUN_005a8fc0`; `evidence/2026-10-26-computer-targets/`); the commander also keeps busy forces out (`evidence/2026-10-26-busy-forces-keep-out/`), takes the player's side under Observe (`evidence/2026-10-26-observe-battle/`), fires the Empire's full superlaser (`evidence/2026-10-26-computer-superlaser/`) and sends the Alliance's Death Star run (`FUN_005ad930`; `evidence/2026-10-26-computer-death-star-run/`), and fighter groups move as one force on the original's mover anchor and slots (`FUN_005adfa0`, `FUN_005c72b0`, `0x005ec0d0`, `0x5ebee0`; `evidence/2026-10-26-fighter-forces/`), their withdrawal steps that anchor (`evidence/2026-10-26-fighter-withdrawal-anchor/`), and each squadron turns and flies its own frame to its place (`0x5b8f70`, `0x5b9a20`, `0x5b97a0`, `0x5ebe30`; `evidence/2026-10-26-fighter-squadron-frame/`), at the commander term plus sublight speed (`0x5ba190`; `evidence/2026-10-26-fighter-speed/`), stepping out of the ships' height bands (`evidence/2026-10-26-squadron-height-bands/`), escorting by closing on and circling their ship (`evidence/2026-10-26-fighter-escort/`), recovering through their frames to the carrier's underside (`evidence/2026-10-26-fighter-recovery-flight/`), and approaching the trench on the group's anchor (`evidence/2026-10-26-trench-run-anchor/`); each unit's weapon turn opens only with a ready arc and a draw of 0 in 0 to 2, and never on Escort, against the manual's opportunity fire (`FUN_005b3a40`, `FUN_005a8a70`; `evidence/2026-10-26-weapon-turn-gate/`), and fires only at its force's target, its force's nearest enemy and, with lasers or on Attack Fighters, the fighter groups nearby, never picking a capital ship by itself (`FUN_005b6530`, `FUN_005cb6e0`, `FUN_005cdf50`; `evidence/2026-10-26-weapon-targets/`), and a ship with a tractor beam takes its force's target within range through event 0x28 and lets it go through 0x29, also on a tractor hit without power or its own destruction (`FUN_005b2140`, `FUN_005b2630`, `FUN_005b26a0`, `0x005b0ad0`; `evidence/2026-10-26-tractor-beams/`), fighter groups too, whose speed then loses the beam's power (`0x5ba190`; `evidence/2026-10-26-tractor-beams-on-groups/`), and an ion hit only drains shields, its points beyond them stunning the engines for 30 to 50 seconds or knocking out weapon arcs (`FUN_005b5b60`, event 0x16, `FUN_005b2340`; `evidence/2026-10-26-ion-hits/`), and a capital escort closes on its ship as an attack would and, when that ship is lost, keeps its order and stands (`FUN_005cc300` `+0x8c`, `+0xb0`; `evidence/2026-10-26-capital-escort/`); the battle's frame uses only its palette's colours (A0 TAC-01-C003: palette 5537), drawn by the Ramp device the original prefers on 8-bit displays, with the exact 8-bit output still open (`evidence/2026-10-25-battle-palette-output/`), and in the abstract battle it escapes on its own (`FUN_005ad530`, `FUN_005d14d0`; `evidence/2026-10-23-tactical-fighter-hyperdrive/`). |
| P25 | Ground combat | Troop attack/defense, facilities, officers/difficulty, selection, casualties, conquest, visuals, and parity between automatic and interactive paths. |
| P26 | Bombardment | Eligibility, shields, losses, popularity, ownership, messages, persistence, and visual feedback. |
| P27 | Death Star | Construction, sabotage, escort, retreat, shielding, targeting, firing, cooldown, destruction, cleanup, contribution to the Imperial HQ objective, and nonterminal Alliance destruction behavior. |
| P28 | Generic events | Every condition/action branch, one-shot behavior, simultaneous events, notification art, state changes, and save/load. |
| P29 | Story and cutscenes | Every story chain, all eight cutscene mappings, heritage branches, queueing, audio/video sync, skip/end/error behavior, and replay prevention. |
| P30 | Victory and defeat | Every win/loss condition for both player factions, simulation freeze, result screen, cutscene policy, Continue, and clean restart. |
| P31 | Save/load/delete | UI actions, slot refresh, populated round-trip, native restart, browser restart, corruption, compatibility, quota errors, delete, and deterministic continuation. |
| P32 | Mods | Discovery, dependency order, cycles, versions, enable/disable/reload, New Game reapplication, save mismatch, hot reload, and additive-feature scope. |
| P33 | Audio | Music, SFX, voices, context transitions, gain/mute, missing files/devices, browser user-gesture policy, and platform scope. |
| P34 | Droid advisors | Both factions, original embedded chrome, every decoded sequence, exact frame IDs/order/timing, priority, message/audio behavior, missing frames, and packaged browser assets. The authentic idle-frame transport/rendering tranche passes; shell overlap and authored behavior remain open, including the mission-refusal reactions F-019 traced (`ghidra/notes/mission-dialog.md`, "Refusal"). Since 2026-10-23 the advisor's messages are read (GDATA C3POACT.SPT / IMP22ACT.SPT, `FUN_004c2c70`, `FUN_004c0ba0`; byte-exact round trip), and the opening briefing is identified as message 0x29 / 0x4a: map actions and briefing voice clips in turn (`ghidra/notes/advisor-briefing.md`, `evidence/2026-10-23-advisor-message-codec/`). Playing it is not ported. The briefing's map displays (`0x13..0x17`, `0x20`, `0x91..0x93`, `FUN_0042b330`) are drawn since 2026-10-23 (`evidence/2026-10-23-briefing-displays/`). |
| P35 | Encyclopedia and EData | Original Index/Topic surfaces, every category/entity, exact EDATA identity, navigation, system focus, fallback, and browser loading. [P62](../2026-09-10-interface-parity-audit/evidence/2026-09-28-encyclopedia-artwork-transport.md) transports all 187 owned images. Source cross-checking reclassifies [P63](../2026-09-10-interface-parity-audit/evidence/2026-09-28-message-index-shell.md) as Message Index evidence. [P64](../2026-09-10-interface-parity-audit/evidence/2026-09-30-encyclopedia-index-shell.md) proves the production-dormant index shells and controls through 32 exact browser states. [P65](../2026-09-10-interface-parity-audit/evidence/2026-09-30-encyclopedia-index-catalog.md) adds all 356 source entries, seven English labels and family filters, stable selection, scrolling, and the selected-category no-op through 22 two-faction browser states. [P66A](../2026-09-10-interface-parity-audit/evidence/2026-10-01-encyclopedia-topic-source-bindings.md) strictly extracts 348 topic texts and 191 artwork mappings, binds 346 complete topics per faction, and exposes ten source-empty mission records without fallback. Browser catalog transport, topic composition, interactive navigation, contextual entry, production routing, A0 comparison, and the complete `OBJ-01` matrix remain open. Since 2026-10-23 the topic mode is composed (geometry, previous / next within the category, Left / Right, index / topic switching; `evidence/2026-10-23-encyclopedia-topic-view/`), Since the same day production opens the original index and topic when their source loads, natively and in the browser (`evidence/2026-10-23-encyclopedia-routing/`; placement hyp); the cockpit button and F7 open it too, checked in the browser against reference capture guide-018 (`evidence/2026-10-23-encyclopedia-command/`). The object menu opens it at the object's topic (`FUN_0045d400`; `evidence/2026-10-23-encyclopedia-contextual-entry/`). |
| P36 | Bitmap and interface sweep | Complete every required cell in the [43-family interface ledger](../2026-09-10-interface-parity-audit/2026-10-06-surface-ledger.json), with exact resources, composition, geometry, hotspots, native/browser screenshots, and zero invented or unknown visible elements. |
| P37 | Campaign acceptance | Short smoke runs and long multi-seed campaigns for both factions/difficulties with bounded fleet/event growth, balance, diversity, victory, and full parity reports. |
| P38 | Release artifacts | Fresh native install and deployed browser package, exact artifact contents, startup/storage/media/input tests, and documentation generated from results. |
| P39 | Protected Cloudflare deployment | Preview and production Pages deployments, secret-backed password gate, signed session cookie, logout/expiry/tamper tests, cache/security headers, asset/API access denial before authentication, deployed single-player/multiplayer smoke tests, rollback, and retained browser evidence. |
| P40 | GitHub Pages documentation | Publish the maintained project documentation from `main`; verify Jekyll-safe Markdown, working internal links, current README/audit/roadmap content, successful deployment, and a public smoke test. |

P40 passed on 2026-09-08. Pages run `34306480934` completed from `main` at
`aaf428d286e482471662881124fbad78062fce6f`, and the public landing page,
lowercase audit directory index, and fleet evidence page returned HTTP 200.
See `evidence/2026-09-08-github-pages.md`. This documentation pass does not
imply that the gameplay and release-artifact passes above are complete.

## Bitmap proof protocol

The [interface parity audit](../2026-09-10-interface-parity-audit/) is the
canonical visual-identity protocol. It adds stable surface/state IDs, authority
tiers, exact pixel and hotspot thresholds, navigation-graph coverage, provenance
tracing, a 370-image source corpus, and the hard prohibition on invented visible
UI. The checks below remain the minimum integration subset.

A bitmap passes only when all of the following evidence exists:

1. A resource-ledger entry identifies the screen, state, DLL source, resource ID,
   expected entity or artwork, dimensions, and platform.
2. The manifest entry reconciles with an existing file and the file decodes.
3. Runtime logs show the correct native path or browser HTTP request and cache key.
4. A screenshot shows the correct image, aspect ratio, state, layering, and fallback.
5. The related control or state transition works; rendering alone is insufficient.
6. Original-only, valid-HD, missing-HD, and corrupt-HD cases are exercised.

Required bitmap-bearing surfaces:

- Galaxy cockpit background and both factions' cockpit buttons
- Officer portraits
- Capital-ship and fighter miniatures in fleet panels
- Encyclopedia miniatures and EData art
- Story and generic event artwork
- Tactical task-force panels, gauges, hull/shield detail, and ship art
- Alliance and Imperial droid-advisor frames
- Cutscene display frames where applicable

## Evidence contract

Every feature result must record:

- Feature ID and commit SHA
- Platform, OS/browser version, build profile, data hashes, and asset hashes
- Faction, difficulty, galaxy size, seed, and fixture
- Initial state and exact user inputs
- Expected and observed state mutation
- Command, output, and exit status
- Browser console and network logs when applicable
- Screenshot or recording for visual/input behavior
- Negative and error-path case
- Final disposition: `pass`, `fail`, `blocked`, or explicitly `excluded`

No feature passes merely because a panel opens, code compiles, or a placeholder
or fallback appears.

## Independent browser acceptance loop

Use the browser-acceptance workflow selected in
[`agent-tooling.md`](../../../agent_docs/agent-tooling.md). Reviewer and
debugger runs remain read-only until a fix is explicitly authorized.

After an authorized fix:

1. Rerun the original reproduction.
2. Run adjacent regression cases.
3. Have an independent browser reviewer confirm the evidence.
4. Update the JSON ledger.
5. Close only that feature ID.

## Definition of 100% functional

The claim is permitted only when:

- Every supported feature ID through P40 is `pass` on every claimed platform.
- All P0 and P1 findings are closed.
- Required tests have no failures or unexplained skips.
- Format, warning-free check, strict clippy, native build, and packaged WASM
  build all pass.
- Bitmap/resource reconciliation has zero unexplained misses.
- Multi-seed campaigns satisfy bounded-growth, balance, diversity, parity, and
  victory criteria.
- The same results are reproduced from release artifacts rather than development
  directories.
- Any unsupported platform feature is explicitly documented and excluded from
  the corresponding completion percentage.
