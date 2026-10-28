# Ctrl clicks in the original, leaving Escort takes Surround, the launched squadrons' parent (P24)

Date: 2026-10-28. This covers the two points left open after
`../2026-10-28-abstract-lost-carrier-hangar/`:

1. the Ctrl clicks, which had stayed untested in the original;
2. what puts a lost carrier's launched squadrons back aboard after a battle.

The observations come from the original REBEXE.EXE in the Windows 10 VM
(`~/VMs/rebellion-win10`, QMP screendumps at 640x480). The captures are in
the ignored store `.artifacts/a0-vm/2026-10-28-ctrl-squadrons/`. Only
their hashes are kept here.

## 1. Ctrl clicks

The Ctrl clicks reach the game through QMP after all. The key goes down
through `input-send-event` (qcode `ctrl`), the button is first released,
then pressed and released, and only then does the key come up. The earlier
attempts lost the click: the first click after a pointer move is often
swallowed. A plain click on the same spot was used each time to confirm
the hit.

The scenario is `save_res1` (Empire). Imperial Fleet 1 is at Yavin, and
the Alliance's Swift attacks it. The battle was paused for every step.

| Step | Original | Port (`ctrl_click_ship`, `2026-10-27-battle-clicks`) |
|---|---|---|
| Our Victory Destroyer selected (`c2`), Ctrl+left on Swift | the selection stays, Swift gets the white target box, the ship leaves Task Force 1 ("Task Force Unassigned", group 1 gone) (`c4`) | target added; one-unit target ungroups the ship |
| Ctrl+left on Swift again | box gone (`c5`) | target dropped |
| Ctrl+right on Swift, twice | box on (`c6`), then off (`c7`) | the same as Ctrl+left |
| Ctrl+left on empty space | the selection stays (`c8`); a plain left click there clears it (`c9`) | the same |
| nothing selected, Ctrl+left on Swift | Swift is selected (`c10`) | a new selection |
| Red Group selected (`f2`), Ctrl+left on Swift | "Attacking Swift", box on Swift (`f3`) | group target |
| then Ctrl+left on our Victory Destroyer | "Escort Victory Destroyer 2", box on it (`f4`) | Escort |
| Ctrl+left on it again | "No Orders", **"Tactics: Surround"** (it was Stand Off) (`f5`) | escort dropped, order 0, **tactic kept** |

Every row matched except the tactic after the escort was dropped.

The task-force rows of the table (our ship joining or leaving a task
force) were not reached. The fleet had a single capital ship in the
battle, besides the save's Death Star.

### Leaving Escort takes Surround

- `FUN_005a4bd0` (the fighter-group state's slot `+0xc`) drops a held
  escort through `FUN_005a8e80` and the force's slot `+0x64(0)`.
- Slot `+0x64` is `0x005ca530` (SetOrder) for the capital forces
  (`0x66bf70`, built by `FUN_005cbf50` for `FUN_005aded0`), the fighter
  forces (`0x66bee0`, `FUN_005c62b0`) and their base `0x66cd28`.
- When the force's order (`+0x68`) is 1 (Escort), it calls the escort
  executor's slot 4 (`+0x98`) and `FUN_005c9be0(1)`, SetTactic Surround
  (event `0x3d`), before it stores the new order.
- A target (`FUN_005a8fc0` -> `FUN_005cb950`) and a route
  (`FUN_005a8f70`) do not go through SetOrder.

The port:

- `BattleSession::leave_escort` (`crates/rebellion-core/src/tactical.rs`)
  sets Surround on each escorting unit given. For a capital task force it
  posts `TaskForceTactic` (event `0x3d`), once per task force.
- `crates/rebellion-render/src/tactical_view.rs` calls it from the SetOrder
  paths:
  - a Ctrl click that drops an escort, both for the group and for the task
    force;
  - an order from the panel (`assign_selected_command`);
  - a new Escort (`assign_selected_escort`).
- Test: `a_force_leaving_escort_takes_surround`. It fails without the
  change (`left: StandOff, right: Surround`).

## 2. The launched squadrons' parent after a battle

### The earlier VM evidence is void

On 2026-10-27 (`../2026-10-27-squadron-rules/`), a lost carrier's TIE
squadron was found in Fleet 1 after the Battle at Yavin. On that basis the
commit kept launched squadrons in the fleet.

That observation came from an edited save. `save_res1` in the VM's slot 5
is the host-edited save of 2026-10-24 (`~/VMs/rebellion-win10/dspatch.py`):
Imperial Star Destroyer 1 was retyped to the Death Star
(CAPSHPSD 136, family `0x18`). The original `save_res1`
(`saves-orig/SAVEGAME.005`) has no Death Star record. The `resource turn 14`
save it came from has Fleet 1 as ISD 1 with 6 TIE squadrons (6 of 6) and
VD 2 with 1 (1 of 2) (`l5`, `l6`).

The second run today shows how unreliable that save is. After a battle
in which both ships were Operational (`res2`), Fleet 1 held only the
"Death Star". Victory Destroyer 2 was gone, and 4 TIE squadrons were
aboard the Death Star (`p8`, `p9`, `p11`). Two facts explain this:

- The Death Star's tactical unit (`FUN_0040bcd0` -> `FUN_005a9220`, class
  `0x66c3c0`, 0x7c bytes) has no hangar. A retyped ship's squadrons
  therefore have no valid carrier.
- `FUN_005968e0` leaves the record uninitialised when `FUN_005a6340`
  finds no unit (`0x5968f8` jumps over the copy). The commit then reads
  stack data.

### The static rule stands

`FUN_0040d320` gives each squadron of the battle system its parent. Its
entry check, `FUN_004f6df0`, always returns 1.

- With the record's carrier flag set, the parent is the carrier. The flag
  (`FUN_005b8e00`) needs two things:
  - the `+0x164` carrier is found as a capital unit;
  - the state `+0x170` is 1 (in the hangar) or 5 (docked).
  The parent is then that carrier, when it still exists and is on the
  squadron's side.
- Otherwise the parent is the battle's system (`0x40d47c`), that is, the
  planet's surface. Manual p. 129: "Fighters in a system that are not a
  part of a fleet are considered to be in hangars on the ground".
- A dock (`FUN_005b8570`) and a launch (`FUN_005b8c70`) set `+0x164`, so
  the carrier is the one the squadron last docked on.

Nothing static moves a system's squadrons back aboard. `FUN_0040d510`
only moves a ship's squadrons over its capacity out to the system.

Two further VM battles found no case where a launched squadron survived
to the end:

- the Alliance's `test` save, Escort Carrier 1 against Coruscant: the
  X-wing and the Y-wing were destroyed before the withdrawal (`cres2`);
- the second Yavin run: every TIE was destroyed (`zres2`).

### Not changed

The port keeps a launched squadron with its fleet, seated by
`number_ships`. The original would leave it on the battle system's
surface. That would also leave TIEs on the surface after every battle,
since they start deployed (`0x40bf0b`). It changes play visibly, and no
clean capture shows it, so it stays `port:`. The comments in
`tactical_flow.rs` and `squadrons.rs` now say that the earlier VM
evidence came from an edited save.

## Captures

| Capture | SHA-256 |
|---|---|
| `c2.png` | `4ade15820ec7b6ec15d8b2d0e380e502a4a865783005b6c0ecab966e4d992797` |
| `c4.png`, `c6.png` (target box) | `475a0a7f776b8c98280c32206a07fffdb25da86c848b2c2f31c0a49feaa1c6ef` |
| `c5.png`, `c7.png` (box dropped) | `e53464d97fbb0e919cd78b857b2124b96219c7c7be8e7133915770eeda098f39` |
| `c8.png` | `8c4c697b0b2cc2f63eed3db317b1e01c11be11500aef248fe1d5478f8c58d838` |
| `c9.png` | `83b2dad592ce4d06cdd34c879a4b972ae10fd57c25b9581d28893f56a3d00099` |
| `c10.png` | `4deece9380be5555e86c933b3eac75685a573391c949595356da80fc3928cb4b` |
| `f2.png` | `a48fb8746ac8d6403925af6200e6cd575e1cd2b864ce27439d80134625c50658` |
| `f3.png` | `386fdcf51a48d80e75dff361cfa1f7ff4b1501957f4c4bfd37b9f8c92cb7637d` |
| `f4.png` | `3c36b85d317574bf5fdb5ea7bd16b51ced350f0c1ff8635817fc62863dfae27e` |
| `f5.png` | `a4b3b3eff2d081746d63f3995f4e7d86b0d52581a0bc2f65cf0d2e6303dad897` |
| `p8.png` | `b4d925629c54f8ee8ac2a0411e80c78ceb9ecd3f925871325bc510b0f500fa05` |
| `p9.png` | `245cf3d6b983c3578b334a16297231f41d26d7cfe1b1392c9d7b4ec8a119fd65` |
| `p11.png` | `94abb6a7fde9671a8b9158901b23ca311c6c72f689b42132a49060ba17e4d6ed` |
| `l5.png` | `c0019f66b630079737285bd2961eda32961947731b09b5c2ef7d8ea5822379f2` |
| `l6.png` | `f654cfb4a4da2fa4443803e44e8f976df44121eff4347373f9208e9aec8c2b21` |
| `res3.png` | `9d955f2775af856817182ec731c3b8956e24c5d2652b8e45924830892d5da7f2` |
| `cres2.png` | `6602579d7b69f38e8e1cc06ea3c6cbfc5c68d67b5d5d53de3fa56c4bda257a26` |
| `zres2.png` | `f3b2a6610d72da619f56bff02f49228c514ece1375dd8156ca974330b81f295d` |

## Gates

- Browser `--battle --all`: 148/148 passed.
- Workspace: 3013 tests passed (before `only_our_selected_living_squadrons_take_an_order`
  was added; `rebellion-render` then 1025 passed). Provenance scan: no
  uncited items.
- Replay equivalence and the replay manifest passed. The golden is
  unchanged.
- Mutants (41): 40 caught, 1 missed. The miss is `< 8` to `<= 8` in
  `leave_escort`. It is equivalent: a ship of no task force holds 255
  (`TACTICAL_UNGROUPED_TASK_FORCE`), and the slots are 0..7.
- Release WASM SHA-256 `0f69ad9a441ec48ee498a29f945d4ecf7da9c60fc6be365fbaa749dbf210740b`.

**Correction (2026-10-28, s182).** The Death Star's tactical unit has a
hangar: its sub-object at `+0x1c` (`FUN_005b82a0`) is the same hangar a
capital ship keeps at `+0x418`, with the class's fighter capacity (24). The
explanation above for the retyped save is therefore incomplete. See
`../2026-10-28-death-star-hangar/`.
