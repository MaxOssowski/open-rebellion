# Mission lifecycle: phases, timer, per-member outcome

Recovered 2026-09-28 from REBEXE.EXE (Homebrew Ghidra 12.1.3, read-only, no
analysis; full decompile plus vtable dumps). Finding F-019. The detection
phases (decoys, FOILTB, betrayal, exposure) and the phase-10 inputs are in
`decoy-roll.md`. Who fills the team and decoy lists is in
`ai-mission-planning.md`.

"First read" marks a function read once by a tracer and not re-read for this
note. Confirm those before porting the rule that rests on them.

## Phase stepping

A mission's phase `+0x68` runs `0..0xb`. `FUN_005227d0` advances it one step
when the ready bit `+0xa4` bit 0 is set (`FUN_005227d0.c`):

```
if end code +0x64 != 0: phase = 0xb                    // FUN_00520e40
elif ready:
    clear ready                                         // FUN_00522130(this, 0)
    next = phase + 1
    if phase == 10 and record.+0x58 (repeat): next = 8  // FUN_00520b60
    if phase == 0xb: next = 0xb
    set phase next                                      // FUN_00521980 -> slot +0x1f4
```

Setting the ready bit (`FUN_00522130(this, 1)`) calls slot `+0x220`. The
handler `FUN_00524b20` steps at once (`FUN_005227d0`), then runs
`FUN_005229c0`.

After each phase change `FUN_00546ea0` runs:
1. the detection manager (`FUN_00547f60`)
2. `FUN_00548120`
3. `FUN_00548370` on phase 2
4. `FUN_005484d0`
5. `FUN_00548840`
6. `FUN_00522980`

`FUN_00522980` sets the ready bit again unless the phase is 4
(`FUN_00520b20`) or 8 (`FUN_00520b30`). So a mission passes through every
phase in the same tick except two:

- **Phase 4, transit.** Entering it starts every member's transit to the target
  (`FUN_00556430` into `+0x6c`/`+0x78`) and calls `FUN_00522280`. That
  function, and `FUN_00545820` on later arrivals, sets the ready bit once no
  member (`FUN_00525bb0`: team, decoys, and captured) still has its en-route
  active bit (`+0x50` bit 5, `FUN_00556620`) and no end code is set
  (`FUN_00522280.c`).
- **Phase 8, the mission timer.** Entering it sets the result code `+0x60` to 0
  (`FUN_00521880`) and arms timer `0x38b` (`FUN_0053fa60(0x38b, 1, ...)`).
  Leaving it disarms the timer (`FUN_0053fa60(0x38b, 0, ...)`) and runs the
  validator `FUN_00522480`. The timer fires `FUN_00522a10`, which sets the
  ready bit only in phase 8.

The timer's delay record (`+0x54 -> +0x20`) is written at init by
`FUN_005236e0`: minimum = record `+0x50` (`FUN_00520b40`, `FUN_00540230`),
spread = record `+0x54` (`FUN_00520b50`, `FUN_00540270`). A timer fires after
`min + rand(0..=spread)` days (`FUN_00586130`, `timer-scheduler.md`).

### What each phase does (`FUN_00524b70`, entering the new phase)

| Phase | Action | Detection mode (`decoy-roll.md`) |
|-------|--------|---------|
| 2 | `FUN_00522870`: settle the team's location and routing | 2 |
| 3 | none | 1 |
| 4 | start transit to the target; wait for arrival | none |
| 5 | run the validator when no end code is set and the target differs from the current location (`FUN_00520bb0`) | none |
| 6 | land every member at the target (`FUN_004f7640`), validator | none |
| 7 | none | 1 |
| 8 | result code `+0x60` = 0, arm timer `0x38b`; wait | none |
| 9 | none | 4, betrayal |
| 10 | agent classes: the per-member roll (below) | none |
| 0xb | validator; agent classes: slot `+0x284` | none |

Every phase change also raises notification `0x30d` (`FUN_0053f950`).

Phase 2 also runs `FUN_00548370` (first read): it sets a travel-speed
percent of GNPRTB 3083 (shipped 50) when the team holds DatId `0x33000243`,
else 100. `FUN_005484d0` (first read) adds the team's average leadership
divided by GNPRTB 6144 to the target system's Incite or Subdue sum when an
Incite (`0x56`) or Subdue (`0x57`) mission enters or leaves phases 8..10.

## The mission record (MISSNSD)

`record` is the mission's class record (`+0x2c`). Its in-memory offsets sit
`0x28` above the file offsets (`tools/dat-dumper/src/types/missions.rs`):

| Memory | File field | Meaning | Source |
|--------|-----------|---------|--------|
| `+0x50` | `max_officers` (0x28) | timer minimum days | `FUN_005236e0` |
| `+0x54` | `base_duration` (0x2c) | timer spread days | `FUN_005236e0` |
| `+0x58` | `flag_col6` | repeat (phase 10 loops to 8) | `FUN_005227d0` |
| `+0x5c` | `flag_col7` | hidden (OnHiddenMission) | `FUN_00520b70` |
| `+0x60` | `flag_col8` | detection phases on | `FUN_00520b80` |
| `+0x64` | `flag_col9` | members can resign | `FUN_00520b90` |

The dat-dumper's `max_officers`/`base_duration` names are wrong for this
layout. The shipped values read as (min, spread), e.g. `(60, 30)` for record
`0x42`, which matches Jedi Training. The shift is inferred from this fit; the
MISSNSD loader was not traced.

## Phase 10: the per-member roll and outcome

The agent classes override slot `+0x1f4` with `FUN_00592f50`. It runs the
base handler, then on phase 10:

1. slot `+0x278`;
2. for each team character (`FUN_00526090`), roll slot `+0x274`
   (`FUN_00593320`) and collect the result per member. Then apply slot
   `+0x27c(member, result, ctx, 10)` for each, in map order (`+0x1c`);
3. the same for each team special force (`FUN_00525e70`);
4. slot `+0x280` (the world effect), then slot `+0x284` (the end).

On phase `0xb` it calls `+0x284` only. Decoys and captives never roll.

Slot `+0x274` returns the member's success chance; `+0x27c` draws against
it (`FUN_0053e2f0(chance)`, `FUN_00574630.c:9-11`), sets the out flag, and on
a success records the result. DS Sabotage `00574630` then raises the member's
espionage (`+0x5a`) by GNPRTB 6167 (`0x1817`) and combat (`+0x62`; base skills
run diplomacy `+0x58`, espionage `+0x5a`, ship design `+0x5c`, troop training
`+0x5e`, facility design `+0x60`, combat `+0x62`, leadership `+0x64`, loyalty
`+0x66`)
by GNPRTB 6168 (`0x1818`) when slot `+0x1d8` holds (`FUN_0055bfd0.c:164-171`).

`+0x27c` records a per-mission result in `+0x60` (`FUN_00521880`, values
`0..3`; the classes set 3 on a success). `+0x280` reads `+0x60` and applies
the effect once. `+0x60` is distinct from the end code `+0x64`
(`FUN_00521900`).

### Slots per class

The table below comes from vtable dumps; each family code is read from its
class's slot `+4` (`MOV EAX, imm`).

| Family | Mission | Vtable | `+0x274` | `+0x278` | `+0x27c` | `+0x280` | `+0x284` |
|---|---|---|---|---|---|---|---|
| 0x51 | Diplomacy | `0x00667a18` | `573ff0` | `51ebb0` | `5740a0` | `574120` | `592c80` |
| 0x52 | Espionage | `0x00667390` | `573090` | `51ebb0` | `573540` | `573610` | `592c80` |
| 0x53 | Research | `0x00664db8` | `56cb20` | `56cd70` | `56cdb0` | `56cf30` | `592c80` |
| 0x54 | Reconnaissance | `0x00664b28` | `56bea0` | `51ebb0` | `56bec0` | `56bee0` | `592c80` |
| 0x55 | Recruitment | `0x00664890` | `56b7b0` | `56b940` | `56b9a0` | `576700` | `592c80` |
| 0x56 | Incite | `0x00666a70` | `5719d0` | `51ebb0` | `571a60` | `576700` | `592c80` |
| 0x57 | Subdue | `0x00663b78` | `569b90` | `51ebb0` | `569c20` | `576700` | `592c80` |
| 0x58 | Jedi Training | `0x006667d8` | `5710b0` | `51ebb0` | `5712b0` | `576700` | `592c80` |
| 0x61 | Rescue | `0x00664600` | `56ae30` | `51ebb0` | `56ae50` | `576700` | `592c80` |
| 0x62 | Abduction | `0x00668fd0` | `576e50` | `51ebb0` | `576eb0` | `576700` | `592c80` |
| 0x63 | Assassination | `0x00668ac8` | `5765c0` | `51ebb0` | `576620` | `576700` | `592c80` |
| 0x64 | Palace | `0x006652e8` | `56e620` | `51ebb0` | `56e650` | `56e870` | `592c80` |
| 0x65 | Bounty | `0x006685c0` | `575060` | `51ebb0` | `575de0` | `51ebb0` | `592c80` |
| 0x69 | Sabotage | `0x00663e08` | `56a2d0` | `51ebb0` | `56a300` | `5746e0` | `592c80` |
| 0x6a | DS Sabotage | `0x00667ca8` | `574600` | `51ebb0` | `574630` | `5746e0` | `592c80` |
| 0x71 | Dagobah | `0x00667f38` | `575060` | `51ebb0` | `572470` | `572490` | `592c80` |
| 0x72 | Vacation | `0x00666d00` | `575060` | `51ebb0` | `572470` | `572490` | `592c80` |
| 0x73 | Pickup | `0x00665058` | `575060` | `51ebb0` | `56da80` | `576700` | `592c80` |

`51ebb0` is the shared no-op default; `576700` is the shared `+0x280`.

### Outcomes (first read; confirm before porting each)

- Diplomacy (re-read 2026-09-28): `5740a0` draws against the member's
  chance; a success sets `+0x60 = 3` and, when the member's slot `+0x1d8`
  holds (always, for a character: vtable `0x0065ca70` slot `+0x1d8` is
  `FUN_0040f340`, return 1), raises its base diplomacy (`+0x58`) by GNPRTB 6156 (`0x180c`,
  `FUN_00533e20`). `574120` runs once after the members: `+0x60 == 0`
  becomes 2 (failed). On 3 it finds the target system (`FUN_00586720`)
  and adds `FUN_0055cac0(mission side, system side)` support through
  `FUN_0050c9f0`. The gain is GNPRTB 6183 + rand(0..=6184) when the
  system is the mission side's, 6185 + rand(0..=6186) when its side is 3,
  and 0 otherwise (`FUN_0055cac0.c`; ids from `FUN_0055bfd0.c:269-290`).
  One success among the members is enough; more successes add nothing
  beyond their own skill raises.
- Espionage (re-read): `573540` sets 3 on a success and, when the target
  (`FUN_00521070`) belongs to another side, raises the member's espionage by
  GNPRTB 6157 (`0x180d`). `573610`: 0 becomes 2; on 3 and a system target
  (`0x90..0x98`), observation level 10 for the mission side
  (`FUN_0050d5a0`), then the revelation counts (`FUN_0055c940`, first read)
  applied by `FUN_00573170` (not read).
- Recruitment (re-read): on a success `56b9a0` finds the target system
  (`FUN_00586720`), picks a recruitable character there for the side
  (`FUN_0055fc80`, not read), recruits it (`FUN_0056b1a0`), sets 3, and
  raises the member's leadership (`+0x64`) by GNPRTB 6159 (`0x180f`).
  Each successful member recruits one character.
- Rescue (re-read): `56ae50` on a success sets 3, raises combat (`+0x62`)
  by GNPRTB 6162 (`0x1812`), and calls the target character's slot `+0x210`
  with the member's key (the release). The target is `FUN_00586c80`.
- Abduction (re-read): `576eb0` on a success sets 3, raises combat by 6163
  (`0x1813`), and calls the target's slot `+0x20c` with the member's key:
  the capture of `decoy-roll.md` "Exposure", captor = the member.
- Assassination (re-read): `576620` on a success calls the target's slot
  `+0x2ec` (kill). Only when the target is then destroyed (`+0x50` bit 3)
  does it set 3 and raise combat by 6164 (`0x1814`).
- Incite (re-read): `571a60` needs the target system (`FUN_00586720`). On a
  success it starts the uprising (`FUN_0050d030`, `uprising-incident.md`).
  The result is 2 when the system's `+0x84` bits 2..3 already equal the
  mission side's opponent code, or `FUN_00509020(system, holder, 1)` holds;
  else 3. On 3 the member's leadership rises by 6160 (`0x1810`).

The target actions of Recruitment, Rescue, Abduction, Assassination, and
Incite run inside `+0x27c`, once per successful member, in member order;
their shared `+0x280` (`576700`) only turns result 0 into 2 (failed).
- Subdue `569c20`: support gain `FUN_0055cb10` (6187 + rand(6188), contested
  6189 + rand(6190)), then the end-revolt check `FUN_0050c910`.
- Sabotage (re-read): `56a300` sets 3 on a success and raises espionage by
  GNPRTB 6165 (`0x1815`) and combat by 6166 (`0x1816`). DS Sabotage
  `574630` does the same with 6167/6168. Both share `5746e0`: 0 becomes 2;
  on 3 the mission's target object (`FUN_00521030`, not the system) gets
  slot `+0xac(6, ctx)`, the destroy call with cause 6. So a Sabotage mission
  names one target object at creation (order `+0x4c`).
- Palace `56e650`: rescues the target, then frees every prisoner there.
- Reconnaissance `56bec0`: always succeeds, end code 3.
- The missions also raise skills on success; the amounts are not read.

### The end (`FUN_00592c80`, first read)

Slot `+0x284` announces the mission to the target side (`FUN_0050d5a0`,
observation level 6) and sends the members back: to their origin when the
target is a system, else by the target character's location. A repeating
mission never reaches it until an end code is set.

## The validator and the end codes (`FUN_00522480`)

The validator runs on leaving phase 8, on entering 4, 5 (conditionally), 6,
and `0xb`. With no end code set yet:

1. End code 5 (message `0x40`/`0x91`) when every member has a remove or
   resign request (`+0x78 & 0xc`), unless `+0xa4` bit 1 is set.
2. It then builds the mission's rule check. `FUN_005830a0` validates the
   member lists, which is the same call mission creation makes in
   `FUN_005422f0`. After that come slot `+0x1bc` and a checker object
   `FUN_00582b90` (vtable `0x0066a090`) over the side, the record, the
   lists, and the target and its container (`FUN_00521030`/`FUN_00521050`,
   or `FUN_00521160`/`FUN_005211c0`). The checker's callback can set an end
   code.
3. In phase `0xb` with no code, the end code is 1.

A non-zero code goes to slot `+0x1dc` and then `FUN_00521900`, and the next
step jumps to `0xb`. A repeating mission therefore runs until a member
resigns or the creation rules stop holding. The checker's per-kind rules
(`FUN_005830a0`, the `0x0066a090` callback) are not read; the port must
reuse the same rules it applies at creation.

## Still open

- The per-kind legality rules behind `FUN_005830a0` and the `0x0066a090`
  checker, which both refuse a new mission and end a running one.
- `FUN_0055c940`/`FUN_00573170` (what Espionage reveals), `FUN_0055fc80`
  (whom Recruitment picks), and `FUN_0056b1a0`.
- `FUN_00548120` and `FUN_00548840` (run on every phase change).
- What end codes 1, 3, 4, 5, and 7 are shown as.

## Supporting decompiles

`FUN_005227d0`, `FUN_00524b70`, `FUN_00524b20`, `FUN_00522980`,
`FUN_00522130`, `FUN_00522280`, `FUN_00522a10`, `FUN_00522a90`,
`FUN_00520b20`, `FUN_00520b30`, `FUN_00520b40`, `FUN_00520b50`,
`FUN_00521880`, `FUN_005236e0`, `FUN_00540230`, `FUN_00540270`,
`FUN_00546ea0`, `FUN_00592f50`, `FUN_00592c80`, `FUN_00548370`,
`FUN_005484d0`, and the per-class `+0x27c`/`+0x280` files listed above.
