# The score outside the main menu (read 2026-11-21)

Sources: `~/Games/rebexe-decompile/decompiled-typed`, objdump of
`REBEXE.EXE` (GOG), and the A0 audio of 2026-11-15
(`.artifacts/a0-vm/2026-11-15-audio`, README there), matched against
MDATA.300..315 with chromaprint fingerprints (`fpcalc -raw`, 4 kHz mono,
exact-value votes in 10 s windows).

## Who starts a track

`FUN_00417520(index)` plays `MDATA.<n>` from the table at `0x006a8478`
(index 0..15 -> 300, 301, 302, 303, 304, 305, 306, 310, 311, 312, 307,
313, 308, 309, 314, 315). Its callers:

| Caller | Track |
|---|---|
| `FUN_00405050` (main menu window, WM_CREATE), `0x4065e6` (credits' end) | `FUN_0041d3b0()` |
| `FUN_004176e0` (Play Music turned on) | `FUN_0041d3b0()` |
| `FUN_00422cb0` (galaxy window creation, a story movie's end) | `FUN_0041d3b0()` |
| `FUN_004727a0` (Battle Summary's Close) | `FUN_0041d3b0()` |
| `FUN_0044f860` / `0x4504c4` (Battle Alert) | `FUN_00417610(10 or 11)` |
| `FUN_00470e70` (Battle Summary's first paint) | `FUN_00417610(+0x18c)` |
| `FUN_00417780` (message `0x4c9`, the track ended) | see below |

`FUN_00417610` sets bit 2 of `DAT_006b1490`; `FUN_00417640` (stop) clears
it. When a track ends (`0x4c9`), `FUN_00417780` replays the same index if
bit 2 is set (the battle windows), else plays `FUN_0041d3b0()`. So the
menu's MDATA.300 and the battle windows' tracks repeat; the galaxy view
moves on to a new pick.

The Battle Alert's choice stops the music (`0x4505a3`: `FUN_00417640`) and
the tactical battle has none: A0 `tac01-c012-corellia-take-command.wav`
holds MDATA.313 (the Empire's alert) until the tactical view loads, then
no score at all (the "continuous bed" there is effects: it stays with
Music Off).

## FUN_0041d3b0: the galaxy view's pick

`FUN_0041d3b0` returns 0 (MDATA.300) when no game runs
(`thunk_FUN_00435a40` = `DAT_006b14e4` is null). In a game it calls
`FUN_004369a0` -> `FUN_00487fb0` -> `FUN_0043a250` on the player's agent
(`game+0xc` -> `+0xc0`, class `0x00658e10`):

    if (agent+0x190 > 0) { agent+0x190 -= 1; return FUN_0041cd80(6) / 2 + 1; }
    agent+0x190 = 3;
    return agent->vt[0x1c]();

`FUN_0041cd80(n)` is the game's random number in `0..n`, so three picks
of 1..3 (MDATA.301..303), then one by the advisor's class, and again. The
agent's constructor (`FUN_00439320`) sets `+0x190` = 0 and the mood byte
`+0x170` = 8, so a new game's first pick is the class's, from mood 8.
Both are saved with the agent (`FUN_004397a0`, `FUN_00439550`).

Vtable slot `0x1c`:

| Mood `+0x170` | Alliance (`0x0065c4c8`, `FUN_004c48e0`) | Empire (`0x0065c4a0`, `FUN_004c2700`) |
|---|---|---|
| 1 | 4 (MDATA.304) | 9 (312) |
| 2 | 5 (305) | 8 (311) |
| 4 | 6 (306) | 7 (310) |
| `0x10` | 7 (310) | 6 (306) |
| `0x20` | 8 (311) | 5 (305) |
| `0x40` | 9 (312) | 4 (304) |
| other (8) | `FUN_0041cd80(3) + 1` | same |

## FUN_00439bf0: the mood

The player's vtable slot `0x54` (`0x4866e0`) jumps to `FUN_00439bf0` on
the agent (`0x439bf0..0x439ca4`):

- mood = 8; `d` = `agent+0xc4` - `agent+0xc8`: `d > 1` -> 1, `d == 1` -> 2,
  `d == -1` -> `0x20`, `d < -1` -> `0x40`.
- At `d == 0`: `r` = `+0xd4 * 100 / +0xd8`, or `+0xd4 * 10` when `+0xd8` is
  0; `r >= 300` -> 2, `r >= 200` -> 4, `r <= 50` -> `0x10`. The next test
  (`r <= 33` -> `0x20`) follows the `<= 50` return and cannot be reached.

The agent's galaxy model sits at `agent+0x3c` (`FUN_0049c780`, refreshed by
the state machine `FUN_0049cb30`). `FUN_0049cc80` zeroes the model's
`+0x90..` and sums each sector record's `+0x60` block into it
(`FUN_0049db10`), so `agent+0xd4` = model `+0x98` = the sectors' `+0x68`,
`agent+0xd8` = model `+0x9c` = their `+0x6c`. `FUN_004c6830` counts in a
sector record `+0x68` the systems with bit 4 and `+0x6c` those with bit 8;
`FUN_004ebe40` sets bit 4 when the side holds the system and 8 when the
opponent does (agent-build-orders.md). So `r` compares the systems the
viewer's side holds with the opponent's.

Not found: a writer of `agent+0xc4` / `+0xc8` (model `+0x88` / `+0x8c`,
zeroed by the constructor, saved and loaded with the model). The model's
setters write `+0x80` (`FUN_0049cd70`) and `+0x84` (`FUN_0049cd90`,
`SideMaintRequired`) only; no store to `[reg+0x88]`/`[reg+0x8c]` near an
agent or model pointer was found in the disassembly. hyp: they stay 0, so
moods 1, `0x20` and `0x40` (and MDATA.304 and 312) never come up.

Also not read: when slot `0x54` runs (what calls the player's `vt+0x54`).

## A0 (2026-11-15)

- `cmd07-empire-newgame-day2-music-on.wav`: a new Empire game after the
  briefing plays 301 (to 44.7 s), then 302 from its start.
- `pre05-c004-empire-briefing.wav`: the game's first track after the
  side's movie is 303 (from about 166 s), then 301 from its start at
  about 274 s.
- `cmd07-empire-fast-from-d509.wav` (slot 1, day 509): 302 runs out at
  about 60 s and 301 starts from its beginning; the Battle Alert then
  plays 313.
- `pre03-c009-music-volume.wav`: turning Play Music on starts 303 from its
  beginning (`FUN_004176e0` -> `FUN_0041d3b0`).

## Port

`rebellion_render::audio::GalaxyScore` (`next`, `refresh_mood`,
`advisor_mood`, `mood_track_index`, `MusicTrack::is_galaxy_pick`) and the
app's `start_context_score`: the galaxy view's score at a new game, a load,
a story movie's end, the summary's close and the tactical battle's end is
the next pick; a pick that ends starts the next
(`AudioEngine::galaxy_pick_finished`). The browser stages MDATA.301..306
and 310..312 (`scripts/build-wasm.sh`).

- port: the counter and the mood are not saved; a loaded game starts as a
  new one.
- port: the rolls come from macroquad's generator, not the game's stream,
  so the score does not move the simulation.
- port: the mood is set once a game day from the true holders
  (`systems_held_by_viewer`), with the lead taken as 0 (hyp above).
