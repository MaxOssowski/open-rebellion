# Long AI-against-AI campaigns after the November AI and seeding ports (F-005, F-007, F-036)

Date: 2026-11-26. This is a check of the release build at commit `0548e1c8`
(code as `d0a70808`), not a code change. It repeats the
[2026-10-19 runs](../2026-10-19-long-campaigns/README.md) after the AI
modules 9 and `0xa`, the shared AI day (F-012), and the original army
seeding (`cdd29b66`).

`rebellion-playtest data/base --dual-ai --ticks 5000 --summary`, seeds 1 to
5 (`long-campaigns.json`). The JSONL logs are not committed.

| Seed | End | Alliance / Empire systems | Fleets now (most, side) | Missions | Space battles (one side shipless) |
|---|---|---|---|---|---|
| 1 | day 5000, no winner | 32 / 30 | 33 (42 Empire) | 5,750 | 206 (48) |
| 2 | day 5000, no winner | 31 / 16 | 64 (50 Empire) | 6,335 | 430 (34) |
| 3 | day 5000, no winner | 19 / 27 | 44 (39 Alliance) | 5,097 | 467 (29) |
| 4 | day 5000, no winner | 32 / 27 | 59 (70 Empire) | 5,130 | 483 (72) |
| 5 | day 5000, no winner | 39 / 22 | 54 (54 Alliance) | 4,902 | 387 (1) |

## What holds

- Every run finishes 5,000 days without a panic, in 20 to 31 seconds.
- Both sides stay alive and hold 16 or more systems at the end.
- Space battles spread over 32 to 37 systems; the busiest system takes 16%
  to 37% of them (seed 3, Kessel, 37%).
- Missions keep flowing all game (4,902 to 6,335); Move is about 60%.

## What changed since 2026-10-19 (open, not yet explained)

1. **No game ends.** On 2026-10-19 seeds 1 and 4 ended in victories (days
   4172 and 4744). Now the victory word only toggles Luke's capture bit
   (`0x10`) and, in seed 1, the Alliance headquarters' loss (`0x1`); the
   Death Star fires 0 to 2 times a game.
2. **More fleets.** The sides start with 12 to 14 fleets together; the most
   one side holds reaches 70 (seed 4), and the two together reach 103 at
   seed 4's day-4250 snapshot. On 2026-10-19 the most was 42. This is the
   roadmap's "one-ship fleets" item, now larger.
3. **Fleets in transit** reach 44% of all fleets at a snapshot (seed 1),
   against the roadmap's 10% guide.
4. **Battles with a shipless side.** 184 space battles over the five runs
   meet a fleet with no capital ships or squadrons. Such a fleet may hold
   unfinished products (the original keeps them in fleets, EVT-02-C008) or
   only cargo; which one, and whether the original fights it, is not yet
   traced.

The playtest summary's "Fleet attack orders" and "Fleet reinforce orders"
read 0: they count the retired heuristic AI's events, not the ported
planners'.

Hashes of the committed files are in `SHA256SUMS`.
