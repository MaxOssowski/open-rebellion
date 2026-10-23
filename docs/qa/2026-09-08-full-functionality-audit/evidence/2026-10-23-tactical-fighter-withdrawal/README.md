# Fighters in the player's tactical withdrawal (P24)

Date: 2026-10-23.

## Source

`FUN_005ad530`'s second loop walks the withdrawing side's units: a group
with an attack target drops it (`FUN_005a8d30`, command `0x37`, with its
voice), and each gets order 3 (`FUN_005aded0`, slot `+0x64`).

## Change

- `tactical_view::withdraw_player_fighters`: each of the player's live
  groups drops its attack and escort and starts the recovery to a carrier
  (`begin_original_fighter_recovery`). hyp: order 3 is that recovery.
- When the side's time ends, a group still bound for a carrier that
  escaped goes with it and keeps its craft. hyp: as the abstract battle's
  departure (`abstract_battle::leave`). A group bound for a carrier that
  stays, or only awaiting one, stays.
- A group that docked before the end left the field already
  (`FUN_005b8630`) and keeps its craft.

## Gates

- Workspace 2699 passed, 0 failed.
  `a_withdrawal_sends_the_fighters_to_their_carriers_and_away` fails
  without the escape rule.
- Mutants on the touched core and render code: 10 tested, 10 caught (after
  the tests gained a carrier that stays, an awaiting group and an empty
  group, and a core-side test).
- Provenance scan: 0 uncited items.
- Browser, Arch Chromium 155.0.8059.84 (not the pinned macOS build), the
  whole tactical catalog (TAC-01..TAC-07): 144 of 144.
- Replay equivalence passed.
- Release WASM `976ee992025c10c212c35130d9515d3074ae966e25f915b0cba8f7591920fe81`
  (the tests changed after it; the code it builds did not).
