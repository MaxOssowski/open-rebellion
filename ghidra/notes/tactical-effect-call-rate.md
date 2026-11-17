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
| 3 | A hit and then a damage sequence were both running | 3360 (r 0-4), 3375 (r 5) | r 3 or 4 | 6 renders, about 0.12 s, no big ball |

Notes on the table:

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

Pause is not settled. `FUN_0059a530` returns before `FUN_005c1470` on the
`+0x8c4` and `FUN_00401300` paths, so the earlier claim that effects run
while paused needs a recheck.

## Port consequence (not applied)

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
