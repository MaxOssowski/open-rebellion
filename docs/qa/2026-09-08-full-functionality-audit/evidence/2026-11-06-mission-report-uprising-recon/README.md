# Recon and Incite Uprising reports, and Incite's question (2026-11-06)

This covers EVT-01-C010 ("mission") and OBJ-12-C011 ("continue"). It follows `2026-11-06-mission-report-targets/`. Both cells stay partial, because no A0 report body has been captured.

## The source

`FUN_004927c0`:
- **Recon (case `0x54`).** It writes only when the leader `+0x74` resolves (`local_148`). It is always in the third person: `0x70e8` "Recon Mission Report" over `0x70e9` (success) or `0x70ea` (failure), with the system as the first substitution.
- **Incite Uprising (case `0x56`), first person.** `0x7100` over `0x7101` on a success. On a failure it is `0x7102` while the system is calm, or `0x7107` while it revolts (`+0x88` bit 2).
- **Incite Uprising, third person.** `0x7103` over `0x7104`, `0x7105` or `0x7106`, chosen the same way.
- **The question.** On a failure, both branches jump to `LAB_004941d6`, which appends `0x70db` "Do you wish the mission to continue?". The report stays kind 5. A success sets kind 3 and asks nothing.

## Before

- Recon and Incite reports read the port's own line.
- The continue question skipped Incite Uprising. `mission-report-question.md` had listed case `0x56` as "no `0x70db` (open)".

## Now

- `mission_report_texts` covers Recon and Incite Uprising. `ReportFacts.revolting` comes from the app's `uprising_state.is_uprising(target)`, which is read before the report.
- `mission_report_asks_to_continue` includes Incite Uprising. A failure while the mission goes on asks the question, as Subdue Uprising does.
- The note `mission-report-question.md` is corrected.

## Tests

- `recon_reports_in_the_third_person_and_incite_tells_a_revolt` covers every Recon and Incite text, and checks that Recon without a leader gives none.
- `diplomacy_research_and_uprising_reports_ask_to_continue_while_the_mission_goes_on` now includes Incite Uprising. The expectation changed with the decompile as evidence (case `0x56` → `LAB_004941d6`).
- Mutants: `mission_report_texts` and `mission_report_asks_to_continue` caught all 28.

## Open

- An A0 report body.
- The research reports (`0x53`, which need `+0xac` "valuable results").
- The `0x70e6` line.
