# Tactical impact effect: how often the counter steps per render

This is for TAC-03-C004 (the destroyed hull's fireball). It adds to
`tactical-impact-effect-path.md`. The functions are in
`~/Games/rebexe-decompile/decompiled-typed/`. The addresses were read from
`REBEXE.EXE` (sha256 `b3fe3997…`) with `objdump`.

## The question

YouTube footage of the original shows two timings for a capital ship's
death. See the evidence in the 2026-11-17 run notes:
`.artifacts/youtube/2026-11-17/` and `fireball/`.

- **Long:** 3360, 3365/3366, 3370/3372, gone. This lasts 0.30-0.32 s.
  Seen in rFNBJUJUROE at 41:01.4 and 42:32.4.
- **Short:** 3360 (about 0.1 s), then 3369/3370 (about 0.07 s), gone. This
  lasts 0.17-0.20 s. Seen in RdDI7YURucY at 29:23.2, in five deaths in
  Cm9OrouqAX0, and probably in A0 (3369, 0.17 s or less).

Both recordings present about 50 fps. So the short case is not a faster
render.

## The trace

1. The render runs once per idle pass, and only from there.
   - `FUN_0059a530` (the tactical idle step) steps the battle once with
     `FUN_0059a6a0`.
   - It then busy-waits on `FUN_005c1490` → `FUN_005c1cf0`. That function
     sets the scene flag `+0x658` once the scene timer at `+0x660` passes
     0.02 s (`0x5c1cfd..0x5c1d15`, float `0x66c954` = 0.02).
   - Then it calls `FUN_005c1470` → scene vtable 0x24, `FUN_005c25b0`. That
     function resets the timer (`FUN_005fbbb0`) and calls `FUN_005c2c80`
     once.
   - The callers were checked in the corpus. `FUN_005c1470` is called only
     by `FUN_0059a530`. `FUN_005c2c80` is called only by `FUN_005c25b0`.
     `FUN_005d41a0` is called only by `FUN_005c2c80`.
   - So the 0.02 s wait gates the whole render: at most one render, and one
     battle step, every 20 ms or more.
   - The timer is `FUN_005fbbe0`, which uses QueryPerformanceCounter or
     timeGetTime.
2. `FUN_005c2c80` makes one `FUN_005d41a0` call for each **list node**,
   not for each effect.
   - It walks the list whose head is at scene `+0x5ec` (the list object is
     at `+0x5e4`).
   - For each node it calls the node's vtable 0x14 and passes the result
     as ECX to `FUN_005d41a0` (`0x5c2ccd..0x5c2cd6`).
   - The node class is `0x66dae8`. Its vtable 0x14 is `0x5b7e30`
     (`mov eax,[ecx+0x10]`), which is the effect object.
   - When `FUN_005d41a0` returns nonzero, `FUN_005c2c80` removes one node
     by key (`vt 0xc` = `0x5ee550`, the key at `+0xc`; `FUN_005f5970` →
     `FUN_005f5920` finds the first node with that key).
3. Every accepted state change adds a new node, and the old node stays.
   - `FUN_005d3e90` (effect vtable 0x28) runs after the priority and
     mutual-exclusion checks.
   - It allocates a 0x14-byte node, `FUN_005ee4f0(effect)`. The node
     stores the effect at `+0x10` and the effect's `+0x18` id as its key
     at `+0xc`.
   - It adds the node with `FUN_005f58d0` on `DAT_006bcbd4 + 0x5e4`
     (`0x5d3ffe..0x5d4013`), then sets the counter `+0x140` to 0.
   - `FUN_005f59f0` rejects only the same node pointer, so a second node
     for the same effect is always added.
   - Nothing removes the earlier node when the state is replaced.
   - `DAT_006bcbd4` (`FUN_005c1c10`) and `DAT_006bc700` (`FUN_005c1060`)
     are both set to the battle scene that `FUN_0059ab40` built.
4. So one effect object can sit in the list k times.
   - k is the number of accepted `FUN_005d3e90` calls since that object's
     last sequence ended.
   - Each render then calls `FUN_005d41a0` k times on the same object, so
     the counter `+0x140` steps k per render.
   - The swap test (`+0xf0` timer past 0.1 s, `0x5d422d`) fires only on the
     first of those calls. That call resets the timer, so the later calls
     in the same render do not swap.
   - When the counter reaches the count, the call clears the state and
     sets counter = -1+1 = 0 and count = 0.
   - Every remaining node then fails `0 < 0` and returns 1, so it is
     removed in the same render or the next one.
5. Where the states come from. `FUN_005c13d0(id, code, stage)` calls the
   effect's vtable 0x28 when scene `+0x684` is set.
   - Stage 2 (destroyed) comes from the hit handlers (for example the
     turbolaser handler `FUN_005b5750`) through the ship's vtable 0xb8, on
     the killing hit. It is the only call for that hit.
   - Earlier hits on the same ship add their own hit (0x04/0x08) or damage
     (0x10/0x20/0x40) node, if accepted.
   - Those sequences last 6 counts (hit), 7 counts (damage), or 16 counts
     (ion damage).

## The model

At the cap (P ≈ 20 ms a render), with k nodes on the dying ship, the first
call of render r has the counter at k·r:

| k | When it happens | Images (renders) | Hull goes | Lasts |
| --- | --- | --- | --- | --- |
| 1 | No effect was running on the ship when the killing hit landed | 3360 (r 0-4/5), 3365/3366, 3370/3372, 3375 | r 11, under 3370 | 16 renders, 0.30-0.33 s |
| 2 | One earlier hit or damage sequence was still running | 3360 (r 0-4), 3370 (r 5-7) | r 5, as 3370 appears | 8 renders, about 0.16-0.18 s |
| 3 | A hit and then a damage sequence were both running | 3360 (r 0-4) | r 3 | 5 renders, about 0.1 s, no big ball |

Notes on the table:

- k = 3: render 5's first call swaps to 3375, but its second call finds
  the count reached and deletes the visual (`FUN_005d41a0` vtable 0xa4)
  before the scene draws, so 3375 is never seen.

- The image after 3360 is 3370 at r = 5 and 3372 at r = 6. The swap goes
  to r = 6 when five renders do not quite pass 0.1 s.
- With slower renders, for example about 25 ms near a death in
  RdDI7YURucY, k = 2 still swaps to 3370. It ends at render 8, about
  0.2 s, which matches the footage (0.133 s + 0.067 s).
- A0's 3369 is an odd counter. That fits k = 3 at P of 34 ms or more:
  3360 for 3 renders, then 3369, then gone at render 5. It does not fit
  k = 1 within 0.17 s. This is not settled, because the VM's render
  period during the death was not measured.

The model predicts both footage cases at the same 50 fps. The k = 1 deaths
in rFNBJUJUROE were single-bolt kills, with nothing visible on the hull
before the bolt. Whether the k = 2 deaths had an earlier sequence running
is not checked. Hit and damage sprites are 32 px at most and can be hidden
by the hull.

Game speed scales only the battle step (`_DAT_0066bae0`), not the render
rate. So it changes k only through how often hits land. The number of
views does not matter: there is one scene and one list.

## Pause

The Pause Battle button does not stop the render, so effects run on while
the battle is paused.

- Command 0xad22 (`FUN_0059b130`) calls `FUN_00596a10` or `FUN_00596a30`,
  which are battle vtable 0xc and 8 (`Class_0066c2d0_vtc`/`vt8`). They
  forward to vt0/vt4 (`FUN_005aac30`/`FUN_005aac60`), which count `+0x8c8`
  up or down and set the button state (`FUN_005c0f80` → `FUN_0059cbe0`).
- `FUN_0059a530` reads `+0x8c8` only on the `FUN_00401300` = 1 path.
  `FUN_00401300` → `FUN_00416270` tests the root object's `+200`/`+0xcc`
  and `FUN_00401c30` (`FUN_00409270`/`FUN_00409390`), the autosave state
  that `FUN_004161b0` starts. That path returns before the render whether
  or not the battle is paused.
- The `+0x8c4` path is the battle's end (`battle-flow.md`).
- The scene's own stop flag `+0x678` (`FUN_005c25b0` returns at once) is
  set only by `FUN_0059cd60`, which also pauses the battle. It is not the
  button's path.
- `FUN_005c2c80` and `FUN_005d41a0` do not test `+0x8c8`.

So on the normal path a paused battle still renders every 0.02 s and the
impact counters still step.

### What else the paused render steps (read 2026-11-18)

`FUN_005c25b0` (`Class_0066c960_vt24`) runs, on each render that passes
its `+0x678` test, `FUN_005c2c80`, then moves the scene
(`(+0x138)->vtbl[0x94]`, `IDirect3DRMFrame::Move`, "Moving scene
failed"), then draws it (`FUN_005da080`). Nothing on that path reads
`+0x8c8`. `FUN_005c2c80` steps three lists and one slot:

- `+0x5fc` objects (vtbl `+0x14`, `+0x18`);
- `+0x60c` fields: `FUN_005d3cc0`, eight frames at 0.1 s on its own
  timer (`FUN_005fbbe0` on `+0x108`);
- `+0x5ec` impact sequences: `FUN_005d41a0`, removed through
  `FUN_005f5970` when they end;
- `+0x654` the superlaser beam: `0x005da830` grows it on its own timer
  (`+0x40`) for 1 s (`DAT_0066d4bc`), then clears `+0x28`.

The projectiles (`FUN_005ee590`) hang `LAB_005eeb90` on their frame with
`AddMoveCallback` (frame vtbl `+0x34`). The scene Move runs it on every
render: it reads its own timer `+0x40`, places the projectile at
t / `+0x58` between the source point and the target's current point, and
clears `+0x28` once t passes `+0x58` (the one- or two-second life).

`FUN_005fbbe0` is a bare QueryPerformanceCounter or timeGetTime delta
with no pause state. So while paused, fields cycle, impact sequences run
out, projectiles fly on and expire, and a fired beam grows and ends. The
combat steps stop (this read did not place the `+0x8c8` test in the
manager update; the port keeps the steps gated on `paused`). A beam's
kill is the type-3 turn event, so a beam that ends while paused leaves
its target alive until the battle resumes.

Ships and fighter groups: A0 run 3 and run 4 (below) show no unit moving
while paused, so whatever the scene Move advances, it does not move them.
The port keeps them where the last step left them.

### Port (applied 2026-11-18)

`tactical_view.rs` `advance_tactical_frame` runs
`advance_presentational_effects` (projectiles, impact sequences, fields,
beam) on every Combat frame, paused or not; the combat steps still need
`!session.paused`. Tests:
`a_paused_battle_keeps_its_effects_running_but_not_its_combat`,
`the_still_fixture_switch_holds_a_paused_battles_effects`,
`an_unpaused_frame_steps_combat_four_times_a_second_per_speed_unit`.

The still fixtures compare paused frames byte for byte and inspect
effects at chosen frames, so every tactical fixture
(`tactical_test_fixture::apply`) calls
`TacticalState::freeze_paused_effects_for_fixture` (feature
`interface-test-fixtures` only, never in a production build). The ones
that hold effects while paused: `configure_effect_presentation_fixture`
(six impact sequences), `configure_detail_escort_fixture` (a destroyed
hull's sequence), `configure_projectile_field_fixture` (projectiles at
0.5 s, fields at frames 3 and 5), `configure_subsystem_field_command_fixture`,
`configure_live_subsystem_damage_fixture` and
`configure_subsystem_repair_mobility_fixture` (fields), the Death Star
laser journey (it pauses to inspect the growing beam), and the
battle-entry probe's re-pause stability check after a resumed stretch.

Turn clock (applied 2026-11-18, after the A0 above): while paused,
`advance_tactical_frame` adds the frame time to
`BattleSession::paused_turn_seconds` once the clock has started
(`turn_clock_started`: a step or a resumed frame has run, port of the
first-update `+0x89c`). The first resumed frame calls
`resume_paused_turn_clock`, which applies the `e % turn + turn` cut to the
turn's phase in steps (`ORIGINAL_TACTICAL_TURN_STEPS`), so an overdue
turn's steps run on that frame. The still fixtures' switch holds this
clock too. The drawn interpolation fraction (`step_accumulator`) does not
change while paused, so ships and fighter groups stay put. Tests:
`a_beam_paused_mid_flight_ends_while_paused_and_kills_on_resume`,
`projectiles_in_flight_finish_while_paused_and_ships_hold_still`,
`a_long_pause_owes_one_turn_and_keeps_the_remainder_as_its_phase`,
`a_pause_before_the_first_combat_frame_owes_no_turn`.

port: the original runs a turn's updates at its start and interpolates the
units by wall time (slot `+0x98`); the port's four 250 ms steps per turn
mean the overdue turn's remaining steps (up to three) and the new phase's
steps run together on the resumed frame.

### The turn clock across a pause (read 2026-11-18)

Where the combat steps stop: `FUN_005aacb0` (battle vt, called by
`FUN_0059a6a0` from `FUN_0059a530` paused or not) runs the arrival states,
then `FUN_005ada10`, then tests `+0x8c8` (`0x005aad2e`). Set, it calls the
manager's slot `+0x7c` (`FUN_005ae1e0`, empty, in `0x66c7c8`) and returns;
clear, it calls slot `+0x9c` (`0x005aad71`), the manager update
`FUN_005bea30` (`FUN_005c0500` in `0x66c880` has the same clock).

That update times turns with `timeGetTime`. Its first call (`+0x8c0` set)
stores the start `+0x89c` (`0x005bea8a`) and the length `+0x8a8` = 1000.
Each call takes `e = now - +0x89c`; if `e >= +0x8a8` it is cut to
`e % 1000 + 1000` (`0x005beb48`-`0x005beb52`). With the turn's work done
(`+0x8b0` = 5) a due turn advances `+0x878`, sets `+0x89c = now - (e -
1000)` and runs the events stamped `+0x878 - 1` (`FUN_005a96b0`). The pause
does not stop `timeGetTime`, so on the first update after a pause of a turn
or more one overdue turn begins at once, and the next keeps the remainder
`e % 1000` as its phase. Only one turn is caught up.

So a superlaser kill that falls due during a pause runs on the first frame
after Resume, not a turn's rest later.

### A0 (2026-11-18, `.artifacts/a0-vm/2026-11-18-edited-saves-8`)

Edited save (Death Star retyped ISD), genuine Empire player, Battle at
Tokmia. Ignored store; nothing committed.

- Run 3, `seq-p1-paused-projectiles` (p-01 `c529da2b0183`): Pause Battle
  0.12 s after a laser shot was seen. Only a 10x17 px area changes, through
  0.89 s after the press (the projectiles fly on and expire); then nothing
  changes for 4 s. No ship moves.
- Run 4, `seq-p2-paused-beam` (p-02..p-07): the beam started 0.20 s before
  the press; paused, it grows (p-04 dt 0.527 s, p-05 full at 0.810 s) and
  vanishes with its star at 0.841 s (life about 1.04 s); the target is
  intact and nothing changes for 6 s (p-06, p-07).
- Run 4, `seq-p3-resume-kill` (p-08 `f148a69ce41b`), times from the start
  of the Resume hold-click: the button turns green at k0048 (0.246 s);
  the "Battle Paused" text goes and the first fire pixel at the target
  shows at k0076 (0.374 s); the fireball grows by k0097 (0.467 s). So the
  kill came within about 0.13 s of the resume. The unpaused run 2 shows
  the fire 0.04-0.17 s after the beam ends, so this is the overdue turn
  beginning on the first update, as the clock above says.
- No hull fireball ran during a pause in these runs.

## Port consequence (applied)
## Port consequence (applied)

The port fix belongs in `crates/rebellion-core/src/tactical.rs`.

- **`TacticalImpactEffect`:** add `nodes: u8`, the `+0x5e4` list nodes
  for this object.
- **`queue_original_tactical_impact`:** on an accepted replacement, carry
  the old `nodes + 1` instead of building a fresh effect with 1. A new
  effect for an idle target starts at 1.
- **`advance_presentational_effects`:** once per 0.02 s render, run
  `original_render_call` `nodes` times.
- **`original_render_call`:** the swap test must count **renders**, not
  calls. Swap on the first call of a render once five renders have passed
  since the last swap (`ORIGINAL_TACTICAL_EFFECT_SWAP_CALLS`), never on
  the later calls of the same render.
- **Hull:** keep the count-11 check per call. `hull_in_scene` follows
  `frame`.
- **Tests** (cite `FUN_005d3e90` 0x5d4013 and `FUN_005c2c80` 0x5c2cd6):
  - a k = 2 death shows 3360 for 5 renders, then 3370 for 3, and is gone
    at render 8, with the hull gone at render 5;
  - a k = 1 death keeps 3360, 3365, 3370, 3375 and 16 renders.
