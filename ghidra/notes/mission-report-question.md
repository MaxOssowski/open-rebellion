# The Mission Report's continue question (read 2026-11-04)

The manual (pp. 1633, 4022) says the agent asks whether the mission should continue. The checkbox continues the mission, the X aborts it, and with no answer the mission goes on.

## The report

`Class_0065b710_vt14` (`FUN_004927c0`) builds the Mission Report and switches on the mission's family `+0x80`:

| Case | Family | `+0x84` (the kind vt `+0x10` returns) | Question |
|---|---|---|---|
| `0x51` | Diplomacy | 5 | always: TEXTSTRA `0x70db` ends the body |
| `0x52` | Espionage | 3 | none |
| `0x53` | the design researches | 5 | always (`0x70db`) |
| `0x55` | | 3 | none |
| `0x56` | Incite Uprising | 5, set to 3 on one branch | no `0x70db` (open) |
| `0x57` | Subdue Uprising | 5 | `0x70db` on outcome 2 only (`param_3 + 0x28`); outcome 3 has none |

The texts live in TEXTSTRA's `RT_RCDATA`: `0x70db` reads "Do you wish the mission to continue?". The string 12364 "Continue" is no order record in STRATEGY's RCDATA (Abort's is at `0x96f46e`: `0x250`, sort 2003, 12362), and REBEXE never names it.

Case `0x51` makes an Abort order, `FUN_004f5cd0(0x250)`, keeps it at `+0x90`, and fills in the mission through vt `+0x24` and `+0x2c`.

## The answers

The Message Index shows a viewed kind-5 message (mode 2) in two ways (`FUN_00469de0`):
- the text control is `0x140` wide instead of `0x18b`;
- the checkmark `0x98` takes `0x2aae`/`0x2aaf` and the X `0x99` takes `0x2ab1`/`0x2ab2`.

`FUN_004665f0` places the two buttons at (`0x163`, `0xf4`) and (`0x163`, `0x119`), each 51 by 35. Display `0x67` shows when `FUN_004ece60` is nonzero.

`FUN_00467f10` handles the answers in mode 2 (`+0x46` = 2):
- **`0x98`** calls the message's vt `+0x38`. `Class_0065b710_vt38` destroys the held order: the mission continues.
- **`0x99`** sets the message's `+0x38` to 10 and calls vt `+0x3c`. `Class_0065b710_vt3c` looks the mission up (`FUN_004f3000`, `+0x78`). If the mission is still there, it sends the order with `FUN_0041ce20(order, 1)` → `FUN_00436050` → `FUN_00487740(order, 1)`. With force 1 nothing asks. The validator (`+0x1c`, `FUN_0051ff30` → `FUN_00521600`) refuses any phase but 8, and a refusal goes to `FUN_00487c90` (the advisor). Otherwise it destroys the order.

Both answers then clear `+0x90` and run `0x65`, back to the list.

## Ported (2026-11-04)

Each piece of the port:
- **The rule.** `story_messages::mission_report_asks_to_continue` asks for Diplomacy, the three design researches and Subdue Uprising, and only while the mission goes on after the report. port: that stands in for case `0x57`'s outcome 2. A report of a mission that has ended asks nothing, since its X would find no mission.
- **The report.** It appends `0x70db` and carries `GameMessage::continue_question`, the mission's id.
- **The window.** `message_index::question_button_specs` draws the two answers, and the text narrows to 320.
- **The checkmark.** It answers `ContinueMission`.
- **The X.** It answers `AbortMission`. The app aborts the mission when `abort_allowed` (phase 8) passes, and logs `message_index answer=abort mission=N result=…`.
- **Both answers.** The question is dropped (`MessageLog::answer_continue_question`) and the list returns.

Open:
- case `0x56`'s kind-5 branch;
- the advisor's reaction to a refused X;
- the tips of `0x98` and `0x99`.
