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

## Third follow-up: 10,000 days, and where the fleets come from

The same 25 seeds on `edaeb9ba`, 10,000 days each
(`long-campaigns-10000-days.json`):

- **The Alliance wins too.** Seed 4 ends on day 7713 with Coruscant the
  Alliance's and Vader and the Emperor captive (`0x11003100`). Seven
  more games end in Empire wins (days 2184 to 7651); 17 reach day
  10,000. In the 5,000-day runs the Alliance had come close: in seed 13
  it held Coruscant from day 2283 on, and had Vader (days 2384 to 2700)
  and later the Emperor (4243 to 4915), never both at once.
- **The fleets keep growing; transit falls.** Over the seeds, the median
  fleet count is 12 on day 250, 41 on day 5000 and 79 on day 10,000
  (the most, 201, seed 20); the median share in transit falls from 33%
  to 7%.

A temporary dump (not committed) of seed 20 on day 10,000: 162 of the
Alliance's 176 fleets are one fully repaired ship with nothing aboard,
not moving; 133 of them wait at Kothlis, mostly Bulk Transports (86) and
Bulk Cruisers (32). The chain, as far as it is traced:

1. The side's troop room and the regiments it wants (`Tallies`,
   `FUN_0041ba80`, `FUN_0041b7e0`) both rise for the whole game, from 4 /
   15 to 1062 / 1048 for the Alliance. The regiments wanted are the most
   a system needs to be taken, and garrisons grow with the maintenance
   capacity (900 to 82,650): on day 10,000 Corellia holds 1,107 Imperial
   regiments and Sullust 914 Alliance ones, 5,734 in the galaxy.
2. While troop room is short, module `0xb` builds troop carriers
   (`FUN_004ca5e0`, `choose_role`).
3. The command planners take free ships into fleets of their own (Create
   Fleet `0x270`, `FUN_004782a0`, `FUN_00477a40`): 2,026 such orders in
   seed 20 against 1,409 joins. A ship a plan has done with stays free
   where it is.
4. The original's idle-fleet step (`FUN_004c94c0`) sends a fleet away
   from a system that is no base (`+0x30` `0x40000`) to the nearest
   base, else the headquarters, else other own systems (Move `0x201`).
   The port leaves this move out (`ai_module_b.rs`, `go_home`): of the
   plans that set the bases only kind 3's is ported. That move would
   gather the idle fleets, not join them.

Whether the original's garrisons grow the same way, and so its fleets,
is not settled: no long run of the original exists to compare. Points 2
and 3 stay open with this chain as the lead.

## Fourth follow-up: the facilities behind the growth (commit `4a7d9734`)

The maintenance capacity behind the garrisons was not earned. Another
temporary dump of seed 20 on day 10,000 counted 10,359 mines and
refineries in the galaxy: 1,736 mines and 2,259 refineries at Oetrago
(energy 9, raw materials 6), more than 2,700 at Allyuen (energy 4). From
day 2940 on, a dozen Imperial construction yards set Oetrago as their
destination and kept building for it.

The AI's Build orders are the same order objects as Build Selection's
Confirm, so the original checks them the same way (`FUN_0051ff30` ->
`FUN_005385f0`), the facility room included (`System_vt78`,
`FUN_00507750`), and carries them out the same way (manager slot
`+0x1f0`, `FUN_00529ab0`: the queue is replaced). The port let the AI's
Build skip the room and append to the queue. It now checks the room at
the order and replaces the queue. With the room check alone the 25 runs
came out the same, so the queue change does not move these numbers.

The same 25 seeds, 10,000 days, before (`edaeb9ba`) and after
(`facility-room-10000-days.json`):

| | Before | After |
|---|---|---|
| Mines and refineries built | 184,759 | 4,448 |
| Capital ships built | 16,825 | 2,667 |
| Fleets on day 5,000, median / most | 41 / 74 | 12 / 33 |
| Fleets on day 10,000, median / most | 79 / 201 | 9 / 17 |
| Space battles | 13,807 | 4,996 |
| Games ended (Alliance / Empire) | 8 (1 / 7) | 8 (6 / 2) |

Points 2 and 3 are closed: no fleet growth is left, and the share in
transit falls from about 20% early to near 0 late. The war also slows
late: space battles over the 25 runs fall from 1,187 in the first 1,000
days to 152 in the last, with 17 games still running. Whether the
original's late game is as quiet is not known.

## Fifth follow-up: the quiet late game and the AI's maintenance check

Three AI fixes, each traced:

- `e3e68aff`: the AI's Build orders also pass the order's maintenance
  check (`FUN_005385f0` -> `FUN_0052e970`). The 25 runs barely move: the
  modules' budgets already keep the orders inside the capacity.
- `46c80ced`: the AI's system record (`FUN_00433e40`) walks regiments and
  squadrons in mode 3, Existing, so an unfinished unit ordered there and
  one on its way there count. The port read only the planet's.
- `b37a3d8d`: module `0xa`'s Destination names the fleet its squadrons are
  for (`FUN_004ce350`, `FUN_00537180`). The port named the fleet's system,
  so the squadrons landed on that planet.

What the late game is (`late-game-10000-days.json`, 25 seeds, 10,000
days):

- **The galaxy is divided by about day 3,000.** The median count of
  neutral systems stays at 48 to 50 from day 3,000 on. A temporary dump (not
  committed) of seeds 1 and 2 on day 10,000 found every populated system
  held; the 47 neutral ones were unpopulated and unexplored, which
  Diplomacy cannot take. Diplomacy missions (`e3e68aff`) fall from 676 in
  the first 1,000 days to 3 in the last (17 games running).
- **The forces stop growing.** In four sample seeds the ships, regiments
  and maintenance capacity of each side on day 10,000 are close to day
  2,500's: capacity is fixed by the systems' energy, and the modules spend
  up to their shares of it (80% of each module's percent).
- **A squadron churn.** Module `0xb`, with no role wanted
  (`FUN_004ca5e0` leaves the flags 0), lists every shipyard class and
  draws among equal keys, so it often builds a cheap squadron; on its
  planet that squadron pushes the squadrons' upkeep past module 7's 80%
  line, and module 7 (`FUN_004d46e0` returns 7, `FUN_004d4800`) scraps
  one. About 21,000 planet squadrons are scrapped over the 25 runs. Both
  steps follow the traced functions; whether the original churns the
  same way is not known.

| | `4a7d9734` | `e3e68aff` | `46c80ced` | `b37a3d8d` |
|---|---|---|---|---|
| Games ended | 8 | 8 | 7 | 11 |
| Space battles | 4,996 | 4,949 | 5,204 | 6,582 |
| Space battles, last 1,000 days | 152 | 152 | 346 | 327 |
| Ground battles | 1,069 | 1,069 | 933 | 1,097 |
| Planet squadrons scrapped | 23,193 | 22,933 | 21,246 | 21,294 |

With the last two fixes the war stays busier through the middle game
(space battles 837 in days 4,001 to 5,000, against 359) and more games
end (11: 6 Alliance, 5 Empire). The late game still slows: the front
between the two sides barely moves (ground battles 3 in the last 1,000
days). No long run of the original exists to compare.

Hashes of the committed files are in `SHA256SUMS`.
