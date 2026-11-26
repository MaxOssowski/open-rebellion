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

## Second follow-up: 25 seeds, and squadrons on a lost planet

**Point 1 (no game ends).** Five seeds were too few. Seeds 6 to 25 on the
same build (`0461a7b0`) ended 3 games of 20, all Empire wins by the full
word (headquarters, Luke and Mon Mothma captive: `0x10100031`) on days
2332, 4280 and 4816. Captives come back often: the Rescue missions of
both sides succeed in 216 of 240 tries over seeds 1 to 5 (76% to 100%
per side and seed),
by the combat skill against the MSTB table (`FUN_0056ae30`). The victory
path works; no Alliance win was seen in 25 games, which stays open.

**The Selonia trickle.** A temporary trace (not committed) showed that the
Alliance took Selonia on day 742 by regiments alone, while the Empire's
squadrons stayed on the planet. More kept arriving from Imperial yards,
42 squadrons by day 764, and they stayed for 4,000 days. Every Alliance
ship built at or sent to Selonia met them, retreated on the battle's
first step and withdrew (`withdraw_loser`, `FUN_00516ef0`): 270 battles.
In the original, a side change calls slot `+0xb0` on every object there
(`FUN_004fae40`); a squadron's (`FightSquad_vtb0`, `FUN_00503510`) sets
Autorouting, and the `0x307` handler (`FUN_00515480`) sends another
side's squadron to the nearest unblockaded system of its side, or
destroys it with none (`ghidra/notes/planet-squadrons.md`, "A system that
changes sides"). The port now does this for planet squadrons, squadrons
on their way, and squadron products.

The same 25 seeds before and after (`side-change-25-seeds.json`):

| | Before | After |
|---|---|---|
| Games ended (all Empire wins) | 3 | 4 |
| Space battles, all seeds | 10,511 | 7,574 |
| Busiest system's share, most / median | 52% (Selonia) / 17.7% | 28.6% / 15.5% |
| Seeds whose busiest system takes more than 30% | 5 | 0 |
| Fleets at the last snapshot, both sides | 9 to 66 | 15 to 74 |

Six battles of seed 4 after the change show an Empire side with no ship
or squadron: a fleet holding only the Death Star, which the summary's
ship count leaves out.

Points 2 and 3 (fleet count and transit) stay open: no leak was found.

Hashes of the committed files are in `SHA256SUMS`.
