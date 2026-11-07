# No Blockade message for a blockade a battle leaves (2026-11-07)

This covers EVT-01-C008 (blockade).

## A0

The unedited "test" save (Alliance) had two battles that left an Alliance system blockaded:
- **Yavin, day 650:** "Yavin is now under blockade by Imperial forces. The Alliance fleet has withdrawn to Umgul." (`.artifacts/a0-vm/battle-clean/01`). The Conflict list on day 650 held only "Saboteurs Strike at Yavin" (`saboteurs-clean/test-day650-conflict-list`).
- **Umgul, day 718:** "Umgul is now under blockade by Imperial forces. The Alliance fleet has been completely destroyed." (`battle-clean/23`). The All Messages list on days 720 and 793..795 held no Blockade row (`assault-clean/00-index-day720`, `uprising-clean/test-day795-index`).

## The source

`FUN_004960f0` (class `0x65b828`, notification `0x14e`) sets `+0x40` to 0, which means no message, when the system's `+0x88` bit `0x4000` is set.

`FUN_0050a9e0` writes that bit when the blockade bit (`0x20`) changes: it then records whether the battle bit (`0x1000`, both sides present) changed in the same pass (`blockade-bit.md`, item 4). When a battle ends and leaves the loser's system blockaded, both bits change together, so the Blockade class is silent. The battle's own report tells the player instead.

A fleet that arrives at an undefended system changes no battle bit, and its blockade is announced.

## Before

The port keeps no bit `0x4000` (the old note in `blockade_texts` said so). It filed "{system} Under Blockade" or "Fleet Initiates Blockade" for every blockade that started.

## Now

- `main.rs` keeps `battled_systems`, the systems where a battle was resolved since the last blockade pass:
  - the abstract battle adds its system (with `combat_cooldowns`);
  - the tactical battle adds its system as its session ends.
- `blockade_started_message` leaves a blockade at such a system as a log line only. The set is cleared after each blockade pass.
- Other blockades keep `0x70a0..0x70a3` on the Conflict rail, as before.

**Test.** `a_blockade_a_battle_leaves_posts_no_blockade_message`: Umgul held by the Alliance, an Imperial fleet there.
- Without a battle: "Umgul Under Blockade" on the Conflict rail, for the Alliance, at Umgul.
- After a battle: no rail and no title.
