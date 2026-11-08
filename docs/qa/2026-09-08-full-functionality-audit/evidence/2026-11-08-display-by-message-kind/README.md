# Display lit by the message class's kind

Date: 2026-11-08. Adds to CMD-08-C014. The cell stays partial.

## The original

The Message Index's Display button (0x67) is lit by `FUN_004697b0` from
the chosen message's kind. The kind is what the class's vtable slot `+0x10`
returns:

| Kind | Display |
|---|---|
| 3 | always lit |
| 5 | lit only with a target (`FUN_004ece60`) |
| 4 | lit, opens the Encyclopedia |
| anything else | grey |

A press on kind 3 or 5 goes to the object at `+0x5c` (`FUN_00467f10`). An
empty id goes nowhere.

These are the message vtables' `+0x10` slots, read from REBEXE.EXE for
`0x65b0b0..0x65bc40`:

| `+0x10` | Kind | Classes |
|---|---|---|
| `0x5a6730` | 3 | most, among them Facility Idle `0x65b260`, Future Jedi `0x65b520`, Captured `0x65b638`, Informants `0x65b680` |
| `0x48ad30` | 8 | Research Exhausted `0x65b0b0`, Recruitment Done `0x65b0f8`, Killed by Imperial Assassins `0x65b140`, advice `0x65b188`, Rescue Attempt `0x65b2a8`, Luke's Dagobah messages `0x65b380`, Chat `0x65ba20` |
| `0x48d640`, `0x4953a0` | 6 | the story movies: Death Star Sabotaged, Alliance HQ Destroyed, Coruscant Captured, System Destroyed |
| `0x5a6700` | 4 | Research Complete `0x65bb40` |
| `0x492390` | `+0x84` | mission messages `0x65b6c8`, `0x65b710` |
| `0x497530` | 3 or 7 by `+0x6c` | the assault summary `0x65b870` |

## A0

- `manufacturing-messages/q02` and `q03` (Alliance, day 61 and 83):
  "Construction Yard Idle on Womrik" chosen, Display lit.
- `research-display/08-future-jedi`: Display lit. Kind 3, no object.
- `agent-advice/02-advice-message`: Display grey. Kind 8.

## Port

Before this change, Display was lit only for a message with a system or an
unplaced target. A kind-3 message without a system, such as the fixture's
idle report, showed it grey.

Now `display_kind` works as follows:

- A message is grey if it is `plain` (kind 8) or on the Advice rail.
- A message is lit if it has a system or an unplaced target.
- A message that is not a mission or assault report is lit even without
  either, because it is kind 3.

The kind-8 builders are marked `.plain()`: Research Exhausted, Killed by
Imperial Assassins, advice, Rescue Attempt, and Luke Goes to / Leaves
Dagobah. Recruitment Done and Chat are not posted by the port.

port: mission reports (kind 3 or 5 by `+0x84`) and assault summaries
(kind 3 or 7) keep needing a system. Their `+0x84` and `+0x6c` are not
traced here.

## Tests

- `display_is_lit_for_a_kind_3_message_without_an_object` (message_index).
- Each kind-8 builder's test asserts `plain`.
- `message-display.mjs` (Linux, Alliance) chooses the idle report and
  compares the rail with A0 q02.
