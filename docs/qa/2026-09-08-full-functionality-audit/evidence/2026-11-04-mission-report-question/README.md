# A Mission Report asks whether to continue (2026-11-04)

This covers surface ledger cell OBJ-12 C011, "continue".

The full reading is in `ghidra/notes/mission-report-question.md`. In short:
- The Mission Report (`FUN_004927c0`) of Diplomacy, the design researches and Subdue Uprising (on outcome 2) ends on TEXTSTRA `0x70db`, "Do you wish the mission to continue?".
- Such a report is kind 5 and holds an Abort order (`0x250`).
- The Message Index's mode 2 narrows the text to `0x140` and shows the checkmark `0x98` and the X `0x99` beside it (`FUN_004665f0`, `FUN_00469de0`).
- The checkmark drops the order, so the mission continues.
- The X sends the order with force 1 (`FUN_0041ce20`), so nothing asks first. The order's validator refuses it outside phase 8.

The manual says the same (pp. 1633, 4022): the checkbox continues, the X aborts, and with no answer the mission goes on.

## Before

The port's Mission Report had no question and no answers. A player could abort only from the mission icon's menu.

## Now

The port now covers all of it:
- `story_messages::mission_report_asks_to_continue` decides which reports ask: only those of a mission that goes on after the report.
- `apply_mission_result` (`main.rs`) appends `0x70db` and sets `GameMessage::continue_question`.
- In mode 2, `message_index` draws the two answers and narrows the text to 320.
- The checkmark answers `ContinueMission`.
- The X answers `AbortMission`. The app aborts the mission when `abort_allowed` (phase 8) passes, and logs `message_index answer=abort mission=N result=…`.
- Either answer drops the question, and the list returns.

Tests:
- Unit tests:
  - `diplomacy_research_and_subdue_reports_ask_to_continue_while_the_mission_goes_on`;
  - `a_mission_reports_question_answers_continue_or_abort_then_returns_to_the_list`, which covers the buttons' places and art, both answers, the return to the list, and that an answered report asks no more.
- `message-display.mjs` (EventMessages fixture, Linux, both sides): the fixture's Mission Report asks about a player's Diplomacy mission that has not reached phase 8. The gate checks four things:
  - The report's checkmark and X match 10926 and 10929 exactly at (355, 244) and (355, 281).
  - The X logs `answer=abort mission=0 result=false`, the validator's refusal, and goes back to the list.
  - Shown again, the report has no question.
  - The checkmark's pixels are gone.

Run `message-display-2026-11-04T08-29-37-900Z`: pass.

Full batch b53:
- workspace tests 3351 and fixture tests 116 pass; provenance-scan is clean;
- all 21 Linux gates pass, `message-display` and `tooltip` (the same fixture) among them.

Mutants:
- In b53, five survived in `draw_single_message`, which had no test of its text.
- `a_question_narrows_the_message_text_to_0x140` now checks the text's clip at scale 1.5: 320 wide for a question, 395 otherwise.
- On the rerun of `message_index.rs`, 22 mutants were caught, 2 were unviable and none survived.

## Open

- No A0 capture yet of a report that asks. In the VM's Alliance game the index then held only a Maintenance Shortfall.
- Incite Uprising (case `0x56`) is kind 5 on one branch without `0x70db`; not ported.
- The advisor's reaction to a refused X (`FUN_00487c90`), and the two buttons' tips.
