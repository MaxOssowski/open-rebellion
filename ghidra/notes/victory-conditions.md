# The victory conditions word (galaxy `+0x74`)

Read 2026-10-18 for F-038 (story message rows 20 and 21) and F-005.

## The check (`FUN_005180e0`)

It reads these characters and objects:

- Luke `0x32000242`, Mon Mothma `0x30000240`, Vader `0x35000281`, the
  Emperor `0x34000280` (`FUN_00505190`);
- Coruscant `0x90000109` (`FUN_00505f60`);
- the Alliance headquarters (`FUN_00506110`).

With all of them found, it builds a word:

| Bit | Meaning |
|---|---|
| `0x1` | the Alliance headquarters is gone or destroyed (`+0x50` bit 3) |
| `0x10` | Luke captured (`+0xac` bit 0) |
| `0x20` | Mon Mothma captured |
| `0x100` | Coruscant's side is the Alliance's (`+0x24 & 0xc0 == 0x40`) |
| `0x200` | Coruscant destroyed (`+0x50` bit 3) |
| `0x1000` | Vader captured |
| `0x2000` | the Emperor captured |
| `0x100000` | the Empire wins |
| `0x1000000` | the Alliance wins |
| `0x10000000` | the game is over (either win) |

The wins depend on the game type, galaxy `+0x60`:

- 0, standard:
  - the Empire wins with Luke and Mon Mothma captured and the HQ gone;
  - the Alliance wins with Vader and the Emperor captured and Coruscant
    held or destroyed.
- 1, headquarters:
  - the Empire wins with the HQ gone;
  - the Alliance wins with Coruscant held or destroyed.
- Any other value builds no word.

When both sides would win at once, the win already recorded in the old
word stands: `0x1000000` keeps the Alliance's, `0x100000` the Empire's,
and the Alliance takes it when neither is recorded.

The function stores the word in galaxy `+0x74` and marks it changed
(`FUN_004fd490`).

## Its notification (`0x128`)

- `FUN_00518460` copies galaxy `+0x74` into both sides' VictoryConditions
  (`+0xbc`, `FUN_0052f600`). The two views sync it in `FUN_005322c0`.
- On a change, the side's slot `+0x260` (`FUN_00532f90`, side vtable
  `0x00660af8`) posts `0x128` with the old and new words
  (`FUN_0053fcf0`). Its name is `SideVictoryConditionsNotif`.
- `FUN_00518460` runs from `FUN_0051e050`, which also handles the end of
  a game (not read further).

## The messages

- Row 21, Alliance Headquarters Destroyed (`FUN_0048e5a0`): on `0x128`
  whose new word has bit `0x1`. Both sides read TEXTSTRA `0x7188` /
  `0x7189` "The Empire has destroyed the Alliance Headquarters.". It sets
  speaker `0x11` and `+0x5c` = `0x73` (hyp: a film).
- Row 20, Coruscant Captured (`FUN_0048f790`): on any `0x128` while
  Coruscant's side is the Alliance's. Both sides read `0x7170` / `0x7171`
  "The Alliance has seized the Imperial capital system of Coruscant.". It
  sets the speaker from the side and `+0x5c` = `0x74`.
- Neither builder has a once flag, yet A0 2026-11-23 shows no repeat of
  103 ("Standard games in A0" below); the port films only newly set bits.

## Against the port

- The port's `victory.rs` keeps its own win conditions (`VictoryState`:
  headquarters keys, Death Star, `resolved`). Before 2026-10-18 it had no
  word and no `0x128`; the game type is `VictoryConditions`.
- Ported 2026-10-18 (save v79):
  - `victory::conditions_word` (`FUN_005180e0`) and
    `advance_conditions_word`;
  - the word lives in `VictoryState::conditions_word`;
  - simulation step 14 and the app check it daily before the port's
    victory check, and record `victory_conditions` (old, new);
  - the app posts `story_messages::victory_conditions_messages` (rows 20
    and 21) on each change.
- port:
  - the headquarters is the Alliance HQ system, lost when it is
    destroyed or no longer holds the HQ;
  - the check runs daily.
- Ported 2026-10-18 (F-005): `VictorySystem::check` decides the game by
  the word's game-over and win bits. The Empire's outcome is
  `DeathStarVictory` when the HQ system is destroyed, else `HqDestroyed`.
  The manual's Imperial assault after the bombardment is not needed, and
  a destroyed Coruscant counts for the Alliance, as in the binary.

## From the word to the ending film (read 2026-11-16, END-01-C003)

A0 (Empire, Headquarters Victory, edited save): the Death Star destroys
Tokmia on day 45, MDATA.102 plays, the galaxy runs on to day 46, then
MDATA.108. The ending does not follow the word directly; the advisor's
speech stands between them.

- A changed word reaches each side's advisor through slot `0x10`
  (`Class_0065c4c8_vt10` = `FUN_004c3010`, Alliance;
  `Class_0065c4a0_vt10` = `FUN_004c0f10`, Empire). The Empire's win bit
  `0x100000` is tested first:

  | Side | `0x100000` (Empire wins) | `0x1000000` (Alliance wins) |
  |---|---|---|
  | Alliance | message `0x57` (`+0x15c`) | `0x56` (`+0x158`) |
  | Empire | `0x54` (`+0x150`) | `0x55` (`+0x154`) |

  Each is due for 40000 sub-ticks, and `FUN_0041d810` sets the galaxy
  window's `+0xac` (it then skips `FUN_0042db70`'s `WM_COMMAND 0x15e`).
  hyp: the caller is the `0x128` notification; slot `0x10` is reached
  only through the vtable, and the dispatch was not read.
- The messages (C3POACT.SPT / IMP22ACT.SPT): a speech clip, then the idle
  clip carrying action kind 3, code 1. Alliance `0x56` [`0x276e`,
  `0x2710` (3, 1)], `0x57` [`0x276d`, `0x2710` (3, 1)]; Empire `0x54`
  [`0x2b56`, `0x2af8` (3, 1)], `0x55` [`0x2b55`, `0x2af8` (3, 1)].
- A scheduled message plays only when the advisor's update runs
  (`FUN_004c0a60` / `FUN_004c2b20`, while `DAT_006b28bc` is set), and
  slot `0x10` does not set it: it waits for the droid's idle clip to end
  (`FUN_0041cea0` -> `FUN_00439ef0`, `advisor-briefing.md`).
- The idle clip's end (`Class_00658848` slot `0x20`, `FUN_00412400` ->
  `FUN_0041d140` -> `FUN_00436620` -> `FUN_004878d0` -> `FUN_00439eb0`)
  stores the pending action `DAT_006b28c4 = 3`. The next update runs it
  (`FUN_004c0f60` / `FUN_004c3060`): kind 3 calls `FUN_0041d830` with the
  local side's VictoryConditions (`FUN_004f3dd0`), which calls
  `FUN_00425c90` when the galaxy window exists (`FUN_00422ca0`).
- `FUN_00425c90` sets the galaxy window's `+0xa8`: `0x100000` -> `0x11 +
  (player side +0x9c == 2)`; else `0x1000000` -> `0x10` for side 1,
  `0x13` otherwise; else 0. The game loop (`FUN_0041e500`, `FUN_0041e390`
  -> `FUN_0041db60` -> `FUN_0042db70`) posts `0x482(0, state)` while
  `+0xa4` is 0, and `FUN_00414830` plays the film (`movie-flow.md`).

So the galaxy keeps running while 102 ends, the droid's idle clip ends
and the speech plays: the extra day in A0 is that time, not a rule on
the day tick. The number of days depends on the game speed and the
clips' length (not measured beyond A0's one day at its speed).
Superseded 2026-11-16 and 2026-11-17: the game-over bit holds the clock
(next section), and A0's day 46 is the day the order ran ("The day the
game ends", below).

Ported 2026-11-16: `AdvisorSchedule::victory_word` (slot `0x10`), and
main.rs sets the ending only on a finished clip of kind 3
(`ending_from_conditions_word`, `FUN_00425c90`). port: without the
advisor's messages the ending still follows the victory check at once.

## The clock after the word (read 2026-11-16, END-01-C003)

The port's fixture (Empire win, Fast) showed MDATA.102 on day 1 and 108 on
day 14, with three reaction messages before the speech; A0 shows one day.

- The game-over bit stops the clock. `FUN_0041e390` (state machine case
  0) asks `FUN_0051de80` on the scheduler (`DAT_006b2fcc`, disassembly
  `0x41e3ac`..`0x41e3b3`), which returns `+0xc0` -> `+0x74 & 0x10000000`
  (`0x51de8c`). `+0xc0` is a `FUN_00513240` object (`0x1b0` bytes, built
  by `FUN_0051d6c0`), the galaxy class whose `+0x74` is this word
  (`FUN_00513240` zeroes `+0x74`; `FUN_00513a50` makes the galaxies).
  While the bit is set `FUN_0041e390` skips `FUN_0051dea0` and only runs
  `FUN_0041db60` -> `FUN_0042db70`, and `FUN_0051dea0` itself raises
  neither the sub-ticks `+0xc` nor the day `+0x10`. So the days and the
  advisor's `DAT_006b28cc` (`FUN_004fcee0`) hold while the speech plays.
- `FUN_0042db70` in that state posts `0x482(0, +0xa8)` once `+0xa8` is
  set (the ending), and otherwise `WM_COMMAND 0x15e` (the targeting
  cancel, `move-order.md`) unless `+0xac` is set; `FUN_0041d810`'s
  `+0xac` only silences that cancel. It does not touch the clock.
- The victory message does not pre-empt the queue. `FUN_004c0a60` walks
  the ids from 1 and plays the first due one (zeroing every slot it
  passes), so reactions with lower ids (Empire `0x30`, `0x31`, `0x3d` for
  codes 1, 2, `0xd`) play before `0x54`; nothing clears the table or
  `DAT_006b28bc` on the word (writers: `FUN_00439320`, `FUN_00439550`,
  `FUN_00439ef0`, `FUN_004c0d00`, `FUN_004c2dd0`). With the sub-ticks
  held, their "10 sub-ticks" never run out either.
- The update itself (slot `0x18`, `FUN_00439bb0`) runs from the agent's
  `Class_0065ad70_vt28` (`FUN_00486520`) only while the agent's `+0x5c`
  notification list is empty (`FUN_00487f30`: `add ecx, 0x5c` then the
  count `FUN_005f50e0`); one notification is delivered per call
  (`FUN_004befe0`, `FUN_00488b50`). The port delivers a frame's codes
  before the update; same order of play.

Ported 2026-11-16: main.rs `galaxy_clock_runs` holds the clock while the
word has `0x10000000`. The hyp that followed here (A0's day 46 as a day
step before the scheduler's galaxy got the word through `FUN_005322c0`)
is refuted below: the scheduler reads the master galaxy itself.

## The Alliance's win in A0 and the advisor under the caption film (2026-11-17, END-01-C001)

A0 2026-11-17-edited-saves-5 run 2 (edited save, Alliance "Headquarters
Victory", Coruscant taken on day 94, no input after the summary's Close):
caption film MDATA.103 from 02:30:21.5, ONE strategy frame (h-08, day 94,
C-3PO standing, no balloon), MDATA.105 from 02:37:24.6, crawl, black, main
menu at 02:39:52. The README's "last frame held ~7 minutes" is not a hold:

- Every A0 frame matched against `103.webm` (5 fps, 160x81 grey, mean
  difference 0.5..1.9 per frame): the film plays at speed for its first
  ~12 s (s0050 = 5.6 s in at 5.7 s), then at ~0.116x (1.6 s of film per
  13.8 s, steady) and reaches its last picture (63.2 s) at 02:37:20; its
  black end frames with the caption run to s1864 (427.677 s), then black
  (s1865), the strategy frame (s1866, 428.115 s), black, and 105's first
  picture (s1868, 428.550 s): ~660 ms from 103's window to 105. Run 1
  (seq-caption-film) is as slow (1.3 s of film per 11.2 s).
- 105 ran at speed (s1950 is 17.8 s into the film 17.85 s after its first
  picture); so did the Empire's caption film 102 on 2026-11-16
  (seq-empire-victory: 55.6 s in at 63.9 s, from 8.3 s). The slow 103 is
  the VM's run, not a rule; the port plays 103 in its 65 s.
- hyp (not measured): the slowdown is contention in the VM. Smacker's
  thread (`FUN_005f7020` -> `FUN_005f73f0`) spins on `FUN_005f7240`
  (`SmackWait` with no Sleep, then `GetDC` / `SmackBufferBlit` per frame);
  the app loop (`FUN_00413560`) spins on `PeekMessageA`; once the ending is
  requested, `FUN_00422ce0`'s `0x482` case spins too while the movie holds
  the capture (`FUN_00601340` sends `0x40d` each pass, then `DAT_006b14cc`
  vt+0x2c and `FUN_00401bd0`). Its start ~12 s into the film fits the
  advisor's speech ending then. Or `SmackWait` follows the film's sound,
  and the guest's sound (pw-record flat, README) stalled. No click is
  needed: run 2 had none.

What A0 does settle: the ending's state was set before 103 ended, so the
advisor's speech and its kind-3 clip ran under the caption film.

- `FUN_0041cea0` gates the advisor's update on `DAT_006b14e4`
  (`FUN_00435a40`), the 0x34-byte advisor object `FUN_00435a50` /
  `FUN_00435ad0` build (freed by `FUN_00435b60`), not on the galaxy view.
- A story movie plays on Smacker's thread while the app loop keeps
  calling the frame scheduler (`movie-flow.md`, "Clock, galaxy window,
  return"), so the agent's update and the droid's clips go on.
- The kind-3 action reaches `FUN_00425c90` (`+0xa8`); `FUN_0042db70` posts
  `0x482(0, state)`; the `0x482` case waits while the story movie holds the
  capture (`FUN_00601340`; the story case took it with `FUN_00601080`
  before `FUN_004019a0`), then `FUN_00401040(state)` starts the end at once.
  Hence one strategy frame between the films.

Ported 2026-11-17: main.rs `advisor_runs` (the galaxy view or a story
movie), so the fixture's speech ends under 103 and 105 follows its end.
port: the clock stays held during a story movie (movie-flow.md hyp), and the
speech's voice plays over the film's sound; A0 has no sound to compare.

## The day the game ends (2026-11-17, END-01-C003)

The original's order within one pass of the per-frame state machine
`FUN_0041dff0`:

1. Case 0, `FUN_0041e390`: waits for the 0.25 s timer (`FUN_004010d0`),
   reads the word's `0x10000000` (`0x41e3b3`, `FUN_0051de80`), and only
   while it is clear runs the sub-tick `FUN_0051dea0` (`0x41e3e9`, its
   only call site; no pointer to it in the image). `FUN_0051dea0` is the
   only writer of the day `+0x10`: `+0xc` rises every sub-tick; when the
   previous sub-tick set `+0x24` (in-day count `+0x20` reached the rate
   `+0x1c`), this one clears `+0x20` and raises the day. The case moves
   on only when the sub-tick ran (returns 1), so nothing below runs on a
   held clock, and a pass runs at most one sub-tick.
2. Case 5, `FUN_0041e460` -> `FUN_0051df30`, one item a frame until none
   is due: queue `+0xb0` (events `0x300..0x37f`, posted by `FUN_0053f950`
   or `FUN_0053fbd0` -> `FUN_0053fcd0` -> `FUN_004fd390` ->
   `DAT_006b2af8` -> `FUN_0051d8f0`), then `+0xa8` (timers, due when `+0x40` <= the day,
   `Class_00661fd8_vt1c` = `FUN_0054edc0`), then `+0xac` (the player's
   orders, `FUN_005539f0` -> `DAT_006b2b00` -> `FUN_0051d8d0`), then a
   notification from `+0xa4`. Queues `+0xac` (`0x661ff8`) and `+0xb0`
   (`0x662018`) have slot `+0x1c` = `0x54eed0`, a jump to `0x41c210`
   (`mov eax,[ecx+8]; ret`): their head is always due. So an order runs
   in the first pass after it is queued, after that pass's sub-tick.
   Destroy System is event `0x224` (`Class_00669ec8_vt4c` =
   `FUN_00582200` builds it; `Event224_Fire` = `FUN_005637c0` runs the
   bombardment, `FUN_0058d800`).
3. The destruction's existing changes come back as event `0x300`
   (`Event300_GameObjExistingChanged_Handle` = `FUN_005140c0`; an object
   of family `0x20..0x21`, the headquarters' range in `FUN_00506110`,
   calls `FUN_00517cd0`, which calls
   `FUN_005180e0`) in the same case-5 loop. `FUN_005180e0` writes the word
   to `DAT_006b2bb0 +0x74` (`thunk_FUN_00506e10`).
4. Case 6, `FUN_0041e490` -> `FUN_0051e050` -> `FUN_00518460`: both sides'
   copies and `0x128` (the advisor's slot `0x10`, the caption films).
5. Case `0xd` back to 0: the bit is set, so no further sub-tick.

The word the clock reads is the one `FUN_005180e0` writes: the
scheduler's `+0xc0` is `DAT_006b2bb0`. A new game sets both in
`FUN_005136d0` case 6 (`DAT_006b2bb0 = this`, called on `+0xc0` from
`FUN_0051d800`); a loaded one in `FUN_00513a50`, which `FUN_0051cec0`
calls with `+0xc0` as its out-parameter and which stores the first of
its three galaxies in both. `FUN_005322c0` syncs the side views only.

So the day stands from the event that ends the game to the ending film:
the held day is the day the winning event ran. A0 agrees:

- 2026-11-17-edited-saves-5 run 2 (Alliance, Fast; the player's
  Planetary Assault, an order through `+0xac` as Destroy System is): day
  94 at the assault (h-01 `a7087f65400d`), after the summary (h-02
  `f2153378d456`) and between 103 and 105 (h-08 `99250adbb1bc`). Run 1
  read 90 at the assault and after the summary (g-01, g-04).
- 2026-11-16-edited-saves (Empire, Very Slow: the time bar has no lit
  segment in x-41 `636f8642dcf9` and b0165 `9364c8b58ee7`; Slow lights
  one in speed-timing/slow-0 `50939cd43c00`): x-41 is the bombard
  submenu with Destroy System highlighted on day 45, before the click;
  the next capture is already film 101 (seq-empire-victory/b0000
  `2f5e3844875e`); the galaxy frame between 102 and 108 reads 46 (b0165,
  67.5 s into the burst). No capture shows the galaxy between the click
  and 101. The menu is the game's own window (`FUN_00442380`; the image
  imports no `TrackPopupMenu`), so the clock runs while it is open, as it
  does under Build Selection (2026-11-13-band-title-timing 43/44: 305 ->
  308 at Slow). The day turned 45 -> 46 between x-41 and the order's
  pass, and the Death Star's order ran on day 46. The "one extra day"
  is the capture's timing, not a day after the word.

Against the port (before 2026-11-17): the player's orders run at once
(main.rs `OrderBombardment`, `OrderAssault`), but the word was settled
only in the next day step, which raises the day first. So a win by an
order ended on the order's day + 1 (A0 holds the order's day). The
fixture (the win already in place at load) showed day 1 for the same
reason.

Ported 2026-11-17: `victory::game_over_pending` and
`VictorySystem::outcome`; main.rs settles the word
(`settle_victory_word`) before the clock moves whenever the world would
newly end the game, so `galaxy_clock_runs` holds the order's day.
A follow-up the same day settles the whole word there
(`victory::word_change_pending`, main.rs
`settle_victory_word_between_days`): any bit the world changed since the
word was stored, with its `0x128` caption film (102 for the HQ lost in a
Standard game that goes on), as `FUN_005140c0` -> `FUN_00517cd0` ->
`FUN_005180e0` and `FUN_0051e050` -> `FUN_00518460` do in one pass.
port: the changes a day's own step makes settle after that step, before
the next frame's clock. ending.mjs checks that
the day logged at the game-over word equals the day the ending film
opens: ending-2026-11-17T10-10-38-324Z (result 0174d9865331), all four
cases pass, word and film both on day 0 (the fixtures' win is in place at
load, so it now settles before the first day step).

Aside (not ported, not this cell): by `FUN_0051dea0` a day has rate + 1
sub-ticks (the rollover sub-tick does not count in `+0x20`): Fast 5 x
0.25 s = 1.25 s, Medium 13 = 3.25 s, Slow 61 = 15.25 s, matching A0's
1.26 / 3.22 / ~15 s (game-speed-timing.md, which reads the extra as the
day's own work). The port's day was rate x 0.25 s; ported 2026-11-17
(`GameSpeed::sub_ticks_per_day`, game-speed-timing.md).

## A side that holds no system (checked 2026-11-20)

Nothing ends the game when a side holds no system. `FUN_005180e0` reads
only the four leaders, Coruscant and the Alliance headquarters (its
calls are `FUN_00506e10`, `FUN_00505190` four times, `FUN_00505f60`,
`FUN_00506110`). `FUN_0051e050` compares two list counts at `+0x6c` /
`+0x88` for its `+0x30`/`+0x38`/`+0x40` flags, then calls
`FUN_00518460` and `FUN_0051eaf0` (a message drain). Neither counts
systems. A Standard game therefore goes on until both Imperial leaders
are held at once with Coruscant the Alliance's or destroyed.

Port run (`rebellion-playtest data/base --dual-ai --ticks 5000 --seed 5`,
release, on main 9d20f441): on day 5000 the Empire holds 0 systems,
0 fleets and 0 regiments; the Alliance 53 systems, 51 fleets (94
capital ships) and 713 regiments. The Empire has 10 free characters
(Vader, Piett, Ferrier, Nothos, Rix and Labansat at Chazwa, Daala at
Chalcedon, Lemelisk at Talus, Griff on a mission), 7 held and the rest
killed. Its AI keeps 24 mission planners but gives its last order on day
4910 (days 4500 to 5000: 30 mission orders and one ship move, against
about 1500 to 2000 orders per 500 days before). With no system of its
own, state 6 (`FUN_004bd0a0`) finds no gathering place, and nothing
else has an own system to build at.

The word, from the run's `victory_conditions` events: Vader held from
day 3138 (`0x1000`), Coruscant the Alliance's from day 4065 (`0x100`),
Vader escaped on day 4833 (`captive-escape.md`; word `0x100`), the
Emperor held on day 4972 (`0x2100`). The two leaders were never held
together, so no win, as in the original. The Alliance AI still abducted
at Chazwa (Zuggs 4739, Piett 4804, Menndo 4872, the Emperor 4972), so
the run is not stuck; it ran out of days.

The same command on 60d2e20a and on the Destroyed-hook branch does not
reach the state: seed 5 ends at Empire 27 and 17 systems, seed 4 at 38
systems (60d2e20a) or an Empire win on day 1780 (hook branch). Seeds 0
to 3 and 6 to 9 never drop the Empire below 6 systems.

## Standard games in A0 (2026-11-23, edited saves part 13)

Captures: `.artifacts/a0-vm/2026-11-23-edited-saves-13/` (outside Git;
README, SHA256SUMS, host saves in `~/VMs/rebellion-win10/saves-edit-1123/`).
New Standard games started from the original's main menu, then
host-edited. **Every captive leader there is a partial edit**
(`scripts/captive.py`: master `+0x98` 0 and `+0xac` bit 0 only; a real
capture also flips the side, moves the character to the captor and gives
the other side's view a different record), so these runs show the
presentation, not the full capture rules.

- **Empire wins (b-*, reached):** a Death Star at the Alliance HQ
  (Toprawa), Mon Mothma and Luke captive. Destroy System (day 82) plays the
  destroy film ("The Death Star has destroyed the Alliance system
  Toprawa.", MDATA.101 inferred), the HQ caption film ("The Empire has
  destroyed the Alliance Headquarters.", MDATA.102), one black frame, the
  strategy screen for about 17 s with the clock held at 82 and the Imperial
  advisor speaking, MDATA.108, the crawl, then the main menu. Same order as
  the Headquarters game of part 9 (END-01-C011).
- **Alliance, leaders free (a-25..a-33):** an assault takes Coruscant
  (day 75); the caption film "The Alliance has seized the Imperial capital
  system of Coruscant." plays and the game goes on (days 76, 77). As the
  port: bit `0x100` alone does not end a Standard game.
- **Alliance wins (a-51..a-59, a-62..a-67, reached):** Coruscant the
  Alliance's since day 75, the Emperor and Vader captive (partial edit).
  The game ended only on the day a later event re-ran the check (Leia, and
  in another run Han, captured): the clock held at that day, C-3PO spoke on
  the strategy screen for about 13 s, then MDATA.105, the crawl and the main
  menu. **No caption film MDATA.103 played.**

Two points against the port, both open:

1. The check runs from events (a capture, Coruscant's side or existence),
   as "The check" above says; the port re-reads the word every day step and
   between days. In play without edits a leader's capture or Coruscant's
   change is itself such an event, so the two agree; with edited saves they
   do not.
2. The port posts MDATA.103 on every change of the word while bit `0x100`
   is set, as `FUN_0048f790` reads (0x128, Coruscant `+0x24 & 0xc0 ==
   0x128 on every change). A0 above showed no 103 when the leaders' bits
   ended the game with Coruscant long held. **Settled with real captures
   (part 13b, c-05..c-21):** the take-over on day 75 captured the Emperor
   (aboard a transport there) and played 103; Han's Abduction captured
   Vader on day 181, the clock held, C-3PO spoke ~8 s, then MDATA.105, the
   crawl and the menu, with no 103. Where the original drops the repeat is
   not traced (the builder and `Side_SetVictoryConditions` do not). The
   port now films only the bits a change turns on (e3f4ee61). A run where
   the Emperor escaped before Vader was taken went on to day 388: both
   leaders must be held at once.

## The headquarters destroyed by bombardment in A0 (2026-11-23, part 13c)

Same run directory as above. An Empire "Headquarters Victory" game
(EHQ2-d2), Fleet 1's two ships retyped to Super Star Destroyers, no Death
Star; the battle at the Alliance HQ Tokmia won on day 39 (Destroy System
grey without a Death Star).

- Nine Planetary Bombardments on day 44 with Tokmia's GenCore Level I
  shield up did nothing ("No Casualties" each time). As the port reads
  `FUN_0058df60`: the shots are the bombardment less the shields, and two
  SSDs (`bombardment_modifier` 15 each) do not pass GenCore I's 40.
- With the shield retyped to a KDY-150, bombardment 1 destroyed the
  shipyard and a mine, bombardment 2 the KDY-150; then, with no input and
  the clock held at 44, the HQ caption film ("The Empire has destroyed the
  Alliance Headquarters.", MDATA.102 inferred), black, MDATA.108, the
  crawl and the main menu. No destroy film, no strategy frame between 102
  and 108 (the Death Star route of part 9/13 shows ~17 s of it). Tokmia
  was neither destroyed nor taken: the bombardment alone ends the game, as
  the port has it (bit `0x1`, no assault).
- Coruscant destroyed (bit `0x200`) was not reached: an edited Imperial
  Death Star never took a Move to an Alliance-held Coruscant.
