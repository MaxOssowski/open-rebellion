# Overnight backlog

Written 2026-10-14 from a read-only survey (roadmap, the 2026-09-08 audit
ledger, `hyp:`/TODO markers, and greps against the current code). Nothing here
has been started: the session could not branch, build, or commit (see
`OVERNIGHT_LOG.md`).

Ordered by dependency, then value. Every item completes an existing system and
has its evidence already in the repo. Audit ids refer to
`docs/qa/2026-09-08-full-functionality-audit/audit-report.json`.

Statuses: `[todo]`, `[in progress]`, `[done]`, `[blocked]`.

1. `[todo]` **Betrayal reads its table with the step lookup (F-028).**
   `betrayal.rs:115` still calls the interpolating `MstbTable::lookup`; the
   original lookup is `FUN_00595090` (`MstbTable::step_lookup`, already ported).
   Done when: betrayal uses `step_lookup`, a test fails under interpolation,
   and any seed-42 golden change is named for this cause.

2. `[todo]` **Escape reads ESCAPETB with the step lookup (F-028).**
   `missions.rs:2533` is the last other interpolating caller.
   Done when: escape uses `step_lookup` with a failing-without test, and
   `MstbTable::lookup` has no production caller left (remove it only if nothing
   else uses it). Depends on 1 only for the shared golden regeneration.

3. `[todo]` **Drop the KDY "production modifier" from the economy (F-023).**
   `economy.rs` (const at :53, step 4 near :478) applies the blockade
   troop-withdraw formula as a production modifier; the audit shows it feeds
   only telemetry and the real rule already lives in
   `BlockadeSystem::withdraw_percent`.
   Done when: the modifier and its constant are gone or re-sourced, telemetry
   consumers still build, and the golden is regenerated only if it moves.

4. `[todo]` **Tag `SupportTier` thresholds and `fog.rs` for provenance (F-031, F-032).**
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

7. `[blocked]` **Stop keying Death Star combat checks on family `0x34` (F-025).** NEEDS YOUR CALL.
   `combat.rs` (:347, :1346) treats family `0x34` as the Death Star; that family
   is the Empire major characters. `CapitalShipClass::is_death_star` already
   accepts CAPSHPSD record 136.
   Done when: `CombatEntityKind::from_family_byte(0x34)` no longer returns
   `DeathStar` and `ShipSnap::is_death_star` uses the class identity, with tests.
   Caveat from F-017: extending shield absorption to record 136 would make the
   seeded Death Star unkillable in auto-resolve, so keep absorption out of this
   slice and say so in the commit.
   Blocked (session B): both halves of "done when" contradict existing
   assertions. `entity_kind_classifies_family_bytes_correctly` asserts
   `from_family_byte(0x34) == DeathStar`; `ds_shield_blocks_hull_damage` builds
   its Death Star with family `0x34`; and
   `the_seeded_death_star_takes_hull_damage_while_nothing_can_drop_its_shield`
   asserts that record 136 is NOT shield-absorbed, which is exactly what
   "`is_death_star` uses the class identity" would undo. Doing this means
   rewriting those expectations, which the overnight rules forbid unattended.
   `CombatEntityKind` has no caller outside `combat.rs` tests, so nothing in
   play depends on the answer. Nothing was changed.

8. `[blocked]` **Resource incident on its recovered timer (F-029, first half).** NEEDS YOUR CALL.
   `economy.rs:1018 evaluate_incident_flags` raises the resource flag from
   overcapped energy or raw materials. Original: event `0x390` every 1 to 500
   ticks (GNPRTB 7719/7720) through `FUN_00556be0` and RESRCTB.
   Done when: the resource incident fires from that timer with a
   failing-without test. Check first whether the timer needs a save field; if
   it does, stop and split (save-format changes need your approval).
   Blocked (session B): the timer needs a persisted "next resource incident
   tick" (like `UprisingState::next_disaster_tick`). The save is a fixed
   layout: any new field bumps `SAVE_VERSION` (34) and rejects older saves.
   Split, nothing implemented:
   - 8a. `[todo]` Approve the save bump (one `Option<u64>`; item 9 would add a
     per-system timer in the same bump).
   - 8b. `[todo]` Port the resolver. Recovered tonight from the `.c` files, so
     this needs no new Ghidra work:
     - `FUN_00556fa0`: delay = GNPRTB 7719 (`DAT_006bb4b8`) + rand(0..=GNPRTB
       7720 (`DAT_006bb4a8`)).
     - `FUN_00556be0`: pick one system uniformly among those with `+0x50`
       bit 6 (existing); none, no incident.
     - `FUN_0050cc70`: raw = `+0x64`, energy = `+0x5c`; has_mine = the
       holder's walk of families `0x2c..0x2d` is not empty (`FUN_0052d690`);
       has_facility = the holder's walk of families `0x20..0x30` is not empty
       (`FUN_00539df0`). Pulse incident bit 19 and store the values only if
       one changed.
     - `FUN_00559ee0`: code = RESRCTB step lookup of a `0..=99` draw
       (`FUN_0053e2e0`). 1: has_mine and raw > G7711, raw - 1. 2: has_facility
       and energy > G7713, energy - 1. 3: raw < G7712 and raw < energy,
       raw + 1. 4: energy < G7714, energy + 1.
     - GNPRTB ids come from the loader `FUN_00558bb0`: `0x1e1f` to
       `DAT_006bb4b4`, `0x1e20` to `DAT_006bb40c`, `0x1e21` to `DAT_006bb3e0`,
       `0x1e22` to `DAT_006bb488`.
     - Not yet read: whether the family ranges are inclusive at the top, and
       what the setters `FUN_00509dc0` (raw) and `FUN_00509cc0` (energy) do
       beyond storing (they may clamp or evict facilities).
   - 8c. `[todo]` Drop the invented `resource` trigger from
     `evaluate_incident_flags`.

9. `[blocked]` **Informant incident on its recovered timer (F-029, second half).** NEEDS YOUR CALL.
   Per-system timer (`+0x40`, GNPRTB 7703/7704) that rolls support in
   `FUN_0050cbe0` before INFORMTB (`FUN_0050d510`).
   Done when: as item 8, and `evaluate_incident_flags` has no invented trigger
   left. Same save-format caution.
   Blocked (session B): needs a per-system timer in the save, so it waits on
   the same approval as 8a. `timer-scheduler.md` also lists this timer's
   record delay as "not traced", so it needs Ghidra work as well. Not started.

10. `[done]` **Load `research_order` for troop and facility classes (F-018).**
    The audit says only capital ships and fighters gate on research; verify
    against `world/mod.rs:1320` first, since the field may already be loaded.
    Done when: both class kinds load the DAT `research_order`, build lists gate
    on the Troop and Facility trees, and three-way tests cover both factions.

11. `[done]` **Re-check F-005 "victory modal is unreachable".**
    `main.rs:2993` now sets `GameMode::VictoryModal`, so the finding may be
    stale (its evidence lines :73 and :1714 no longer match).
    Done when: either the route is shown reachable and the audit JSON, Markdown,
    and roadmap are updated together, or the remaining gap is written down as a
    concrete sub-item. Needs a browser pass to close fully.
    Result: reachable by code reading; audit JSON, Markdown and roadmap updated.
    Remaining gaps, each needing a decision or a browser pass, not started:
    - 11a. `[todo]` Decide the victory/defeat cutscene policy and wire it.
    - 11b. `[todo]` Word the result for the player's side (defeat wording).
    - 11c. `[todo]` Make `ForceVictoryCheck` resolve the game or rename it.
    - 11d. `[todo]` Browser pass: win, modal, Continue, clean New Game.

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
