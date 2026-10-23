# The player's tactical withdrawal takes the side's time (P24)

Date: 2026-10-23.

## Source

- The Withdraw confirmation (`FUN_005f1170`) sends command `0x7a`
  (`FUN_00596a70`, `FUN_0059a470`); `FUN_0059fd40` runs
  `FUN_005ad530(side)`.
- `FUN_005ad530`: the side commander's `+0x84` becomes 2; each capital
  ship gets order 3 when its hyperdrive is ready (`FUN_005b1bc0`), else
  order 2; the side's time `+0x7c` becomes at least `DAT_0066c240` =
  30000, or `DAT_0066c234` = 150000 for the Empire while its Death Star is
  operational (constants read from `REBEXE.EXE`).
- `FUN_005c8780`: while `+0x84` is 2, `+0x7c` falls by the step; at 0
  (`DAT_0066cbb4`) the state becomes 3, the withdrawal's end.

## Before

The port moved each withdrawing ship 15 units a step towards the edge
and let it go after 20 steps (5 s), its own rule.

## Change

`BattleSession::begin_player_withdrawal` / `start_withdrawal_time` /
`withdrawal_ms`: the side's time runs down by the 250 ms step and the
ships with order 3 escape when it ends (hyp: together). The port's R key
(the selected ships) starts the same time. port: fighter groups do not
withdraw.

## Gates

- Workspace 2697 passed, 0 failed.
  `the_players_withdrawal_ends_after_the_sides_time` fails with a 5 s
  time.
- Mutants on the touched core code: 26 tested; the first run left 3
  alive (the hyperdrive checks); with a damaged and a missing hyperdrive
  in the test, 25 caught, 1 unviable.
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `f7ee5aa938c7865fb6e197df13e7f5dc74abc93667299d4eba5f9b78510fd7e8`
  (the test changed after it; the code it builds did not).
