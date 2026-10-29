# The advisor's reactions: the Agent menu, the toggles and the quips (P34)

Date: 2026-10-29. Tracker item "Droidy-doradcy". This follows the
briefing's schedule (`../2026-10-28-advisor-briefing-playback/`) and save
v108 (`../2026-10-29-translate-counterpart-save-v108/`).

## The original

When the advisor's update (`FUN_004c0a60`) finds nothing due, it calls
`FUN_004c2dd0` (Alliance, C3POACT) or `FUN_004c0d00` (Empire, IMP22ACT).
With the agent active (`+0x10`, set by `FUN_00439320`), these answer one
bit of `DAT_006b28b0` per call. The answer puts the bit's message on the
schedule (`+0x168[id] = today + days`) and clears the bit. The update
stops (`DAT_006b28bc = 0`) until the idle clip next ends.

| Bit | Writer | Alliance | Empire |
|---|---|---|---|
| `0x1` | `FUN_0043a2a0` (the agent's ship move) | `0x23`, 10 days | `0x1c`, 10 |
| `0x10` / `0x20` | `FUN_00439d60`, Garrisons / Production on | `0x24` / `0x26`, 10 | `0x1d` / `0x1f`, 10 |
| `0x100` / `0x200` | the same, off | `0x25` / `0x27`, 50 | `0x1e` / `0x20`, 50 |
| `0x1000000` | toggle counter past 3 | `0x4f`, 5 | no answer |
| `0x2000000` | `FUN_00439ed0`: the Agent menu opened (`FUN_0041d160` → `FUN_00436640` → `FUN_00487c10`), counting `DAT_006b28b8` | `0x4a + n` in `0x4c..0x4e`, 5 | `0x49 + n` in `0x4b..0x4d`, 5 |
| `0x80000000` | `FUN_00439ef0`: the idle clip ended after more than 50 days since the last message (`DAT_006b28d0`) | `FUN_005f5700(12) < 6`: `0x50 + n`, 40000; the update re-arms | `0x4e + n`, 5 |

Each of these messages has one entry, a sprite-DLL record (slot 9): an
op-4 clip with its WAVE.

## The port

- `AdvisorSchedule::add_flags`, `react` and the reaction table per side.
  - `last_played` stands for `DAT_006b28d0`, set when a message plays.
  - The quip flag is set at the idle end.
  - hyp: the quip's roll uses a xorshift32 from a fixed seed in place of
    the interface's generator.
- The app:
  - moves the agent's toggle bits (`PlayerAgent::advisor_flags`, set by
    `FUN_00439d60`) into the schedule each frame;
  - adds `0x2000000` when the Agent menu opens.
- The runtime pack now carries the sprite DLLs' voices
  (`advisor/{alsprite,emsprite}-dll/*.wav`), not only the briefings'.

Not ported:
- `0x1` (the agent's build orders, module `0x17`, are not ported);
- `0x8000000` and `0x4000000` (their writers are untraced);
- the op-`0x13` clips of the quip messages, which the clip player does not
  play: they end at once;
- the mission refusals' reactions (`FUN_004c2940`, `FUN_004c0870`), which
  are the next stage.

## Browser (`tools/interface-parity/agent-objectives.mjs`)

Run `…T13-11-44-947Z` passed for both sides. The WASM hash is
`4c0c0cf0…`. The logs are `{alliance,empire}-advisor.log`.

| Event | Alliance | Empire |
|---|---|---|
| first Agent menu opening | `alsprite-dll` WAVE 1174 (message `0x4c`) | `emsprite-dll` WAVE 1167 (`0x4b`) |
| Alt+G, Manage Garrisons on (`flags=0x10`) | WAVE 1137 (`0x24`) | WAVE 1135 (`0x1d`) |

Further openings while one is pending coalesce into one bit, as in the
original. The Alliance's next answer is WAVE 1175 (`0x4d`).

## Tests

- `cargo test --workspace`: all pass. `scripts/test_build_runtime_pack.py`:
  10 tests.
- New tests:
  - `each_sides_toggle_reactions_follow_its_table_one_per_call`;
  - `the_agent_menus_answers_count_on_within_three_messages`;
  - `after_fifty_quiet_days_an_idle_end_brings_a_quip`;
  - `a_toggles_reaction_plays_after_the_next_idle_end`;
  - `only_the_menus_bit_counts_an_opening_and_answered_bits_clear`;
  - `the_quips_draws_follow_the_ports_generator`.
- Changed: the pack test now expects the sprite DLL's voice to be packed.
- `cargo mutants --in-place --in-diff`: 66 mutants, 15 missed at first.
  After the tests, one is left, equivalent: `message < low` against `<=`
  at the low edge.
