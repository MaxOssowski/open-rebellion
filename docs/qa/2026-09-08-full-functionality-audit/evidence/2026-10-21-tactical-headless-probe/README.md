# Headless tactical probe (F-033)

Date: 2026-10-21. The question: can the port's `BattleSession` resolve a
strategic battle on its own, run without display?

## How

A temporary hook in `resolve_system_space_combat` (`probe.patch`, not
committed) built a `BattleSession` for each strategic battle. It used
the first Alliance fleet against the first Imperial fleet, then called
`step()` until the battle ended or 40,000 steps passed. Each line of
`probe.txt` records:

- the ships and fighter groups on each side;
- the first step that changed any hull or squadron count;
- the steps taken and the winner;
- each side's hull and squadron total before and after.

Command, for seeds 1, 2 and 3:

    OR_TAC_PROBE=1 rebellion-playtest data/base --seed N --ticks 1500 --dual-ai

## Result

67 battles. No damage in any of them, so no winner and the totals did not
move. The port's units move and attack only on a player's order. The
original gives each side a commander object that issues the orders
(`ghidra/notes/abstract-battle.md`, `FUN_005d14d0` -> `FUN_005c9200`).
The port has no equivalent yet.
