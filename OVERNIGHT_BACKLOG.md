# Overnight backlog

Written 2026-10-14 from a read-only survey (roadmap, the 2026-09-08 audit
ledger, `hyp:`/TODO markers, and greps against the current code).

Two sessions ran this prompt at once on 2026-10-14 and split the list: the main
checkout (branch `overnight/2026-10-14`, `OVERNIGHT_LOG.md`) takes items 1 to 6;
the other works items 7 to 12 in `.claude/worktrees/overnight-b` (branch
`overnight/2026-10-14-b`, `OVERNIGHT_LOG_B.md`). Statuses for 7 to 12 are kept
on that branch.

Ordered by dependency, then value. Every item completes an existing system and
has its evidence already in the repo. Audit ids refer to
`docs/qa/2026-09-08-full-functionality-audit/audit-report.json`.

Statuses: `[todo]`, `[in progress]`, `[done]`, `[blocked]`.

1. `[done]` **Betrayal reads its table with the step lookup (F-028).**
   `betrayal.rs:115` still calls the interpolating `MstbTable::lookup`; the
   original lookup is `FUN_00595090` (`MstbTable::step_lookup`, already ported).
   Done when: betrayal uses `step_lookup`, a test fails under interpolation,
   and any seed-42 golden change is named for this cause.

2. `[done]` **Escape reads ESCAPETB with the step lookup (F-028).**
   `missions.rs:2533` is the last other interpolating caller.
   Done when: escape uses `step_lookup` with a failing-without test, and
   `MstbTable::lookup` has no production caller left (remove it only if nothing
   else uses it). Depends on 1 only for the shared golden regeneration.

3. `[blocked]` **Drop the KDY "production modifier" from the economy (F-023).** NEEDS YOUR CALL.
   Blocked 2026-10-14, nothing changed: (a) `SystemEconomy.production_modifier`
   is a positional bincode field of the save, so removing it is a save-format
   break; (b) two tests assert the invented value
   (`kdy_production_modifier_reduces_with_ships`, `..._floors_at_zero`) and
   would have to be deleted. Keeping the field but no longer computing it
   still fails those tests. Both need your sign-off.
   `economy.rs` (const at :53, step 4 near :478) applies the blockade
   troop-withdraw formula as a production modifier; the audit shows it feeds
   only telemetry and the real rule already lives in
   `BlockadeSystem::withdraw_percent`.
   Done when: the modifier and its constant are gone or re-sourced, telemetry
   consumers still build, and the golden is regenerated only if it moves.

4. `[done]` **Tag `SupportTier` thresholds and `fog.rs` for provenance (F-031, F-032).**
   `SupportTier::from_support_int` splits at 20/30/40/60 with no source;
   `fog.rs` is port-owned but untagged.
   Done when: each carries a `port:` or `hyp:` tag with a reason,
   `cargo run -q -p provenance-scan -- check` passes, and
   `scripts/provenance-baseline.json` shrinks.

5. `[todo]` **Correct the false claims in the Ghidra notes and agent docs (F-039).**
   `agent_docs/ghidra-re.md:155` and `agent_docs/systems/ai-parity-tracker.md`
   (:52, :92, :123) call `FUN_004927c0` the master tick and `0x1f0` the daily AI
   trigger; `FUN_00508250` is described as per tick.
   Done when: each claim is corrected at its source, citing
   `ghidra/notes/timer-scheduler.md`. Docs only, no build impact.

6. `[todo]` **Remove the Emperor's 1.5x battle bonus (F-024).** NEEDS YOUR CALL.
   `combat.rs:237-261` multiplies Empire damage by 1.5 and the code itself says
   the bonus has no recovered source.
   Done when: the bonus is removed and the golden change is named.
   Why it needs you: removal means deleting or rewriting the R12 tests that
   assert the bonus. The overnight rules forbid deleting tests; CLAUDE.md allows
   changing an expectation with evidence it was wrong. I would not do this
   unattended without your sign-off.

7. `[todo]` **Stop keying Death Star combat checks on family `0x34` (F-025).**
   `combat.rs` (:347, :1346) treats family `0x34` as the Death Star; that family
   is the Empire major characters. `CapitalShipClass::is_death_star` already
   accepts CAPSHPSD record 136.
   Done when: `CombatEntityKind::from_family_byte(0x34)` no longer returns
   `DeathStar` and `ShipSnap::is_death_star` uses the class identity, with tests.
   Caveat from F-017: extending shield absorption to record 136 would make the
   seeded Death Star unkillable in auto-resolve, so keep absorption out of this
   slice and say so in the commit.

8. `[todo]` **Resource incident on its recovered timer (F-029, first half).**
   `economy.rs:1018 evaluate_incident_flags` raises the resource flag from
   overcapped energy or raw materials. Original: event `0x390` every 1 to 500
   ticks (GNPRTB 7719/7720) through `FUN_00556be0` and RESRCTB.
   Done when: the resource incident fires from that timer with a
   failing-without test. Check first whether the timer needs a save field; if
   it does, stop and split (save-format changes need your approval).

9. `[todo]` **Informant incident on its recovered timer (F-029, second half).**
   Per-system timer (`+0x40`, GNPRTB 7703/7704) that rolls support in
   `FUN_0050cbe0` before INFORMTB (`FUN_0050d510`).
   Done when: as item 8, and `evaluate_incident_flags` has no invented trigger
   left. Same save-format caution.

10. `[todo]` **Load `research_order` for troop and facility classes (F-018).**
    The audit says only capital ships and fighters gate on research; verify
    against `world/mod.rs:1320` first, since the field may already be loaded.
    Done when: both class kinds load the DAT `research_order`, build lists gate
    on the Troop and Facility trees, and three-way tests cover both factions.

11. `[todo]` **Re-check F-005 "victory modal is unreachable".**
    `main.rs:2993` now sets `GameMode::VictoryModal`, so the finding may be
    stale (its evidence lines :73 and :1714 no longer match).
    Done when: either the route is shown reachable and the audit JSON, Markdown,
    and roadmap are updated together, or the remaining gap is written down as a
    concrete sub-item. Needs a browser pass to close fully.

12. `[done]` **Remove `PanelAction::AdvanceTicks` or make it run the real tick (M2).**
    `main.rs:7240`.
    Done when: the action either calls the authoritative simulation step or is
    deleted with its dev-command caller, with a test for whichever is kept.

13. `[todo]` **Injury recovery timer `0x388` (F-038, first slice).** NEEDS YOUR CALL.
    Character `+0x94`, GNPRTB 2563/2564; the uprising incident already reports
    injuries it cannot apply (F-026 "Open").
    Done when: an injured character carries the injury, recovers on the timer,
    and the incident applies it, with failing-without tests.
    Why it needs you: it adds a persisted character field, so it is a
    save-format bump (v27), which CLAUDE.md says to ask about first.

## Deliberately left out

Too large or not possible unattended: the F-033 combat slot handlers and the
real bombardment function (need new Ghidra decompiles), F-035 per-facility
manufacturing managers, F-036 AI cadence, F-040 missing mission kinds, the
native GUI checks (need `cua-driver` on macOS), and every browser acceptance
gate.
