# Research Mission Reports from TEXTSTRA (2026-11-06)

This covers EVT-01-C010 ("mission"). It follows `2026-11-06-mission-report-uprising-recon/`. The cell stays partial, because no A0 report body has been captured.

## The source

`FUN_004927c0` case `0x53` handles the three design researches:
- **First person.** A major character's report is titled `0x7120` "{1:1} Mission Report". Its body is `0x7121` "My {3:1} Research mission at {2:1} is making progress." or `0x7122` "…has produced valuable results." on a success, and `0x7123` "…has not produced new information." on a failure.
- **Third person.** Any other report is titled `0x7124` "{2:1} Mission Report", with body `0x7125`, `0x7126` or `0x7127`.
- **Substitutions.** The mission's name is `+0x30`, and the system is the substitution before it.
- **Progress or results.** The choice between "making progress" and "valuable results" reads the mission's `+0xac`.
- **The question.** The report always appends `0x70db`. The port asks only while the mission goes on (`mission-report-question.md`).

## Now

- `mission_report_texts` covers the three research kinds.
- The app sets `ReportFacts.valuable` when the result's research points reach a level (`ResearchState::add_points`).

**hyp.**
- `+0xac` is a level reached.
- The mission's name is its kind's TEXTSTRA name, so the first person reads "My Ship Design Research Research mission…". `FUN_00491500` likewise names a mission by its kind ("An enemy {1:1} mission").

Both need an A0 report to settle.

## Tests

`a_research_report_names_the_research_and_tells_progress_from_results` covers progress, results, failure, and both persons. Mutants: `mission_report_texts`, 27 caught.

The app's `valuable` flag sits inside `apply_mission_result`, which has no unit test because it plays audio natively. It is covered by review only.

## Open

- An A0 report body for any kind.
- The `0x70e6` line.
- The Mission Foiled report to the mission's own side.
