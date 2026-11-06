# Mission Reports and Enemy Mission Foiled from TEXTSTRA (2026-11-06)

This covers EVT-01-C010 ("mission") and OBJ-12-C006 ("foiled"). Both cells stay partial, because no A0 capture of a report body exists yet.

## The source

The builders are in `ghidra/notes/message-pictures.md` and `mission-report-question.md`. The decompiles are `FUN_004927c0` (notification `0x1f0`) and `FUN_00491500` (`0x1f1`).

**Mission Report (`FUN_004927c0`).** The builder switches on the mission's family `+0x80`.
- A leader `+0x74` of family `0x30..0x37` (a major character) reports in the first person, with its name as the first substitution. Any other leader gets a report titled by the mission's kind.
- The outcome `param_3 + 0x28` is 3 for a success and 2 for a failure.
- Diplomacy: `0x70d0` or `0x70d4`, then `0x70d1`/`0x70d5` or `0x70d2`/`0x70d6`, then the support side from `+0x84` bits 2..3 (`0x70d9`, `0x70d8` or `0x70da`).
- Espionage: `0x7108` or `0x710b`, then `0x7109`/`0x710c` or `0x710a`/`0x710d`.
- Subdue Uprising: `0x7110` or `0x7113`, then `0x7111`/`0x7114` or `0x7112`/`0x7115`.
- Jedi Training: `0x71c0` or `0x71c3`, then `0x71c1`/`0x71c4` or `0x71c2`/`0x71c5`.
- The question `0x70db` is appended to the same text.

**Mission Foiled (`FUN_00491500`).** A reader that is not the mission's side reads `0x713e` "Enemy Mission Foiled" over `0x713f` "An enemy {1:1} mission to {2:1} has been foiled by our forces.", whatever the end code. The picture is `0x431` or `0x432`, chosen by the reader. The mission's own side reads codes 3..5 as "Mission Foiled" (`0x7148..0x714b`) and codes 0..2 as nothing.

**A0 evidence.** `.artifacts/a0-vm/message-rail/battle-averam-result`: a battle that ended an enemy team filed "Enemy Mission Foiled" on the Mission rail.

## Before

Every Mission Report read the port's own line, for example "Alliance Diplomacy mission at Averam succeeded", and had no title. A foiled enemy mission posted nothing.

## Now

**`story_messages::mission_report_texts`.** It builds the title and body for Diplomacy, Espionage, Subdue Uprising and Jedi Training.
- The app reads the report after the mission's effects, so Diplomacy names the new support side (`economy::support_side`).
- The leader's name and major flag are read before the effects.
- The other kinds keep the port's line. Their reports come next: Recruitment, Rescue, Abduction, Assassination, Sabotage, Incite, Recon and research.

**`story_messages::enemy_mission_foiled_texts` and `foils_the_mission`.** The app's `push_foiled_missions` posts the message to the opponent of each mission that ends with code 3 or above. To support this, `MissionEnded` now carries `target_system`.

**hyp.** `0x1f1` is sent for every end except a run course (1) and an abort (2). Its sender, `FUN_00525070`, is reached through a vtable and has not been traced.

**port.** A diplomacy failure reads "no effect" (`0x70d2`/`0x70d6`), because the port's result does not distinguish a fall in support (`0x70d3`). An espionage report lists no further systems (`FUN_00494df0`).

## Tests

**Unit tests.**
- `a_mission_report_speaks_through_a_major_leader_and_names_the_kind_otherwise` covers the first-person and third-person reports of the four kinds, the support lines, and `None` for an unported kind or missing texts.
- `a_shipped_report_runs_into_its_question_and_the_port_line_takes_a_space` covers how the question is appended.
- `a_foiled_mission_posts_enemy_mission_foiled_to_its_opponent` covers the audience, picture, rail and text, and checks that a code 1 end posts nothing.
- `every_end_but_a_run_course_or_an_abort_foils_a_mission`.
- `a_foiled_enemy_mission_names_the_mission_and_its_target`.
- `a_mission_whose_team_is_gone_ends_with_code_five` now also checks the end's target.

**Mutants.** `mission_report_texts`: 12 caught. `foils_the_mission` and `enemy_mission_foiled_texts`: all caught. `push_foiled_missions`: 3 caught. `report_title_and_text`: 6 caught.

## Open

- An A0 capture of a report body.
- The Mission Foiled report to the mission's own side (`0x7148..0x714b`).
- The remaining kinds.
