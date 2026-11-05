# The Agent menu's check marks and Agent Advice against A0 (2026-11-05)

This covers two ledger cells:
- CMD-08-C023, the Agent Advice menu toggle;
- CMD-08-C022, Agent Advice off for Medium and Hard.

## A0

All the captures are in `.artifacts/a0-vm/agent-menu/`.

- **The toggle.**
  - `alliance-agent-menu-day53` (save "test", Easy): Translate Counterpart and Agent Advice are both checked.
  - `alliance-agent-menu-advice-off-day54`: after a click on Agent Advice, its row is bare and Translate Counterpart stays checked.
- **The defaults.** New Alliance games:
  - Easy (`alliance-easy-day1`): Agent Advice is checked, and the Message Index opens on Advice.
  - Intermediate (`alliance-intermediate-day2`, chosen in `main-menu-intermediate`): it is bare.
  - Expert (`alliance-expert-day1`): it is bare.

  The setting does not carry over between games.
- **The check mark.** STRATEGY 11902 has a pure blue (0, 0, 255) square around the mark. A0 shows none of that blue: the menu's fill shows through, here and in `menus3/33-facilities-band-reserved` (a band's Reserved).

## Before

The port drew 11902 opaque. A blue square sat behind every check mark in the Agent menu, the Message Alerts submenu and the object menus.

## Now

- **The check mark.** `bmp_cache` keys 11902 on blue.
- **Unit tests.** `the_menus_check_mark_keys_out_blue` fails without the change. The batch's in-diff mutants: 2 caught.
- **`agent-objectives.mjs`** (Linux, both sides):
  - **The check cells.** Agent Advice's and Translate Counterpart's cells hold the white mark and no pure blue.
  - **The toggle.** A click on Agent Advice logs `command=agent_advice status=off source=menu`. When the menu opens again its cell has no mark, and Translate's keeps its mark. A second click turns advice back on.
  - **The cell.** It is measured from the menu's dotted left border, which the gate finds.
- **`agent-advice.mjs`** (Linux).
  - **A new case.** `alliance-intermediate` chooses Intermediate (control 1) and then the Alliance. The briefing posts no advice and the index stays shut, and the Agent menu's Agent Advice cell is bare (0 white pixels; Translate has 27).
  - **The Easy cases.** Both still post the nine advice items and open the index.
- **The b72 batch.** It ran all 21 Linux gates, and every one passed.
