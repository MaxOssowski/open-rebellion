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

## Follow-up the same day (commits `805f4ec0`, `206fb40f`)

Point 4 was empty fleets. A temporary trace in the runner (not committed)
found four paths:

- A listed fleet whose finished ships left, holding only an unfinished
  one, counted as presence. `FUN_00509710` and `FUN_0050b8e0` walk mode 1
  (Usable fleets only): fixed in `805f4ec0`.
- Scrap of a fleet's last ship left a squadron entry (count 0) on the
  fleet; a fleet whose last ship fell kept the planet's squadrons that
  flew with it until they landed; and a listed fleet whose last product
  went stayed. `FUN_004fe630` destroys a Created fleet with no ship:
  fixed in `206fb40f`.

The same five seeds after both fixes:

| Seed | Alliance / Empire systems | Fleets now (most, side) | Space battles (one side shipless) | Busiest system |
|---|---|---|---|---|
| 1 | 19 / 38 | 37 (33 Empire) | 519 (0) | Selonia 52% |
| 2 | 36 / 12 | 37 (30 Alliance) | 443 (0) | Coruscant 37% |
| 3 | 20 / 23 | 17 (22 Alliance) | 284 (0) | Adega 12% |
| 4 | 29 / 25 | 24 (33 Alliance) | 460 (0) | Denab 9% |
| 5 | 33 / 26 | 45 (46 Alliance) | 196 (0) | Chalcedon 32% |

No empty fleet is left at any day of the five runs. The most fleets one
side holds fell from 26..70 to 15..46 (systems and fleets from the day-5000
snapshot). Still no game ends.

New, open: in seed 1 the Empire holds Selonia with 42 planet squadrons
and no ship, and the Alliance AI sends one or two ships there 270 times;
they survive each battle, which the Empire wins, and come back. Whether the original's planners do this is
not traced.

The playtest summary's "Fleet attack orders" and "Fleet reinforce orders"
read 0: they count the retired heuristic AI's events, not the ported
planners'.

Hashes of the committed files are in `SHA256SUMS`.
