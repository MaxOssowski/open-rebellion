# Fighter squadrons on a planet

Date: 2026-10-15. Sources: the decompiles in
`/home/retromax/Games/rebexe-decompile/decompiled/` named below.

## Where a built squadron goes

A Build order creates each product at once: `FUN_0052a140` calls
`FUN_004f7d50(class, side, ctx, 1, 0)` with the container `FUN_00504dc0`
resolves, and the product joins that container through its slot `+0xa8`.
Destination (`0x214`, `FUN_0052a380`) moves the queued products to the
target through the same slot. The container is a system, so a finished
squadron stands in the system, not in a fleet. Nothing in the build path
picks a fleet. Delivery (`build-delivery.md`) only delays the arrival.

## The system's squadrons

A system lists its squadrons among its own objects, families `0x1c..0x1f`:

- `FUN_005039d0(owner, mode)` walks the type range `0x1c..0x20` of a
  container. With a ship it gives the squadrons aboard, with a system the
  squadrons on the planet. It does not descend into children, so a system's
  walk does not include the squadrons aboard its fleets.
- `FUN_00503a50(owner, mode, side)` is the same walk filtered by side.

Readers:

| Reader | What it does with them |
|---|---|
| `FUN_00509710` (side presence) | A side is present by an active fleet (`FUN_004ffef0`) or an active squadron (`FUN_00503a50`). |
| `FUN_0050b8e0` (blockade bit) | Both sides present: a battle, no blockade. Otherwise the blockade bit comes from the fleets alone (`FUN_004ffe70`): a squadron on a planet never blockades. |
| `FUN_0050b310` (withdraw percent) | Counts the squadrons aboard each ship and at the system (`FUN_005039d0`) in `FUN_0055a020`. |
| `FUN_0050b230` (support drift) | Passes `FUN_00559c40` the side's squadrons at the system (`FUN_00508fb0` → `FUN_00503a50`), the fleets' capital ships (`FUN_00509090`, `FUN_00502e30` types `0x14..0x1b`), regiments (`FUN_00509020`) and, for the gate, the fleets' Death Stars (`FUN_00509140`, `FUN_00500450` types `0x18..0x1b`). |
| `FUN_00433e40` (AI system record) | `v[23]` all (`+0xa4`), `v[24]` counted (`+0xa8`), `v[25]` active (`+0xac`), `v[26..28]` strengths (`FUN_00473610/620/630`), `v[32]` maintenance (`+0xc8`); an own system sets `+0x28` bit `0x80000`. Mode 3, either side. A squadron counts toward the goal in an own system that is unmarked (`+0x28 & 0x83e00000` clear), or whose class `+0x34` reaches the threshold `+0x40 -> +0x28`. |
| `FUN_0045ce80` (Defenses quadrant icon) | Counts them with defenses, regiments and idle members. |
| `FUN_004a90d0` (Defenses window, page 3) | Lists them. |
| `FUN_00587640` (mission defenders) | With the system flag and the fighter flag, walks the system's squadrons before its regiments. |

## Battle

The system's `+0x88` bit `0x1000` is BattlePending
(`System_SetBattlePending`, `FUN_0050a900`; `ghidra/generated/setters.tsv`),
written by `FUN_0050b8e0` from side presence (`FUN_00509710`): both sides
present, by an active fleet or an active squadron on the planet. The battle
manager `FUN_00516d50` walks the systems (`0x90..0x98`), takes those with
BattlePending, and sets Battle (`+0x88` bit `0x10`, `FUN_0050a580(1)`) on
one. So a fleet entering a system where the other side has only squadrons on
the planet meets a battle (manual p. 141: tactical mode starts "when an
enemy fleet comes into orbit around one of your systems where fleets or
fighter squadrons are stationed").

At the battle's end `FUN_00516ef0` clears Battle and counts each side's
fleets (`FUN_004ffef0`) and planet squadrons (`FUN_00503a50`). With a winner
(`FUN_00509ad0`, the battle object's `+100`), or failing one with both sides
still present the side the system's `+0x78` bits name, it sets the loser's
fleets and planet squadrons autorouting (`FUN_004f78e0`,
`GameObj_SetAutorouting`, object `+0x50` bit `0x1000`) and, when
`FUN_00506e60` allows, passes the winner's planet squadrons to
`FUN_00556710` (a route step; not read further). `FUN_00517270` then gives
the system's battle-won side from the same counts (`FUN_0050a040`).

untraced: how the battle itself takes the planet's squadrons; the battle
object behind system `+0x54`. The "space combat" pipeline in
`space-combat.md` (`FUN_005457f0` → `FUN_00549910`) is reached through
`FUN_005438a0`, which tests story event ids (`0x32000242` and others): it is
the mission manager's, not the battle's.

## Moving a squadron

The squadron vtable is `0x0065def0` (slot `+0x34` is `FUN_00502f80`, the
squadron's speed). Its slot `+0x3c`, `FUN_00502fe0`, gives the order list
`FUN_00557ce0` builds once: `0x201` Move, `0x202` Confirmed Move, `0x204`
(no STRATEGY record, never an item) and `0x200` Scrap.

A move onto a fleet needs the ships' room: `FUN_00500b40` reports a ship's
fighter capacity (`+0x26c`) less the squadrons aboard, and refuses with
status `0x14`/1. A squadron has a speed (`FUN_00502f80`, the FIGHTSD
hyperdrive), so it can also travel between systems on its own.

untraced: which leg builder (`FUN_00552000` and its siblings) fills which
ship; whether a squadron may land on another side's planet; what happens to
squadrons on a planet whose control changes; how the original's battle
setup (`FUN_005442f0` → `FUN_005435f0`) takes the planet's squadrons.

## Port

`crates/rebellion-core/src/squadrons.rs`; `System::fighters` (save v46).

- A completed squadron joins its system's planet
  (`apply_build_completion_inner`).
- Presence, the blockade bit, the withdraw percent, upkeep, the AI's system
  record, the Defenses page and quadrant icon, and the defender walk read
  the planet's squadrons as above.
- Move (`0x201`) of one squadron, from the Defenses window's Squadrons page
  or the Fleet window's Fighters tab, by the pop-up menu or a drag:
  `SquadronTransits::move_squadrons` boards a fleet in orbit with room,
  lands on the planet, or passes to another fleet in the same system at
  once; to another system (since 2026-10-15, for module 7) the squadron
  travels on its own at its FIGHTSD speed and enters its planet or fleet on
  arrival, lost when the planet is destroyed or the fleet gone. A fleet's
  room counts the squadrons on their way to it.
- `AIAction::MoveSquadrons`: module 7's squadron Move
  (`ai-production-modules.md`, "Module 7"). The production heuristic's
  loading that stood in for it before is removed.
- A squadron on a planet takes Scrap (`ScrapTarget::PlanetFighter`), as
  module 7 orders it.

port:
- The room is the whole fleet's (living ships' capacity less the squadrons
  aboard), where the original fills one ship at a time.
- Battle (2026-10-15): a battle starts where both sides are present by a
  fleet or a planet squadron (`squadrons::sides_present`). The port's
  combat is fleet against fleet, so for the battle's length each side's
  planet squadrons join its first fleet in orbit, or a group of their own
  (`squadrons::Sortie`, `scramble`), and the survivors land again
  (`recover`); a group neither bombards nor lands regiments. The original's
  autorouting of the loser's fleets and squadrons is not ported: the port's
  battles end when a side is destroyed or a round changes nothing.
- A mission names squadrons aboard fleets only; the player's Scrap and
  Confirmed Move stay disabled for a squadron.
- The port's seeding still puts a seed group's squadrons into a fleet, which
  may hold squadrons alone; such a fleet disbands when its last squadron
  lands.
- The support drift still counts the fleets and their squadrons
  (`economy.rs`), not the planet's squadrons alone.

hyp: a squadron may not land on a planet another side holds (the regiment's
rule, `FUN_0053d430` `1`/`0x28`); the Galaxy display's squadron count
includes the fleets' and the planet's squadrons.

## A destroyed object's children (2026-10-26)

Source: `~/Games/rebexe-decompile/decompiled-typed/` (outside the
repository); vtable slots read from REBEXE.EXE.

- `GameObj_SetDestroyedReason` (`FUN_004f71d0`) stores the reason in the
  low byte of `+0x40` and calls slot `+0x100` with the old and new reason.
  For capital ships and squadrons (vtables `0x0065dc10`, `0x0065def0`)
  that slot is `0x004f2ad0` -> `FUN_004fb200` -> `FUN_004f7560`. That sets
  `Flags50` bit `0x8` to (reason != 0), syncs both sides (`FUN_004fa580`)
  and calls slot `+0x11c`.
- A capital ship's `+0x11c` is `CapShip_vt11c` (`0x00502840`). It first
  calls the base chain `CombatUnit` -> `Detector` -> `GameObj_0065ce68`
  -> `GameObj_vt11c` (`FUN_004fb520`).
- `GameObj_vt11c` walks every child (`FUN_004fcd00(this, 3)`, families
  1..0xff) and calls the child's slot `+0xb4`, `FUN_004f8710`:
  - a child that is Completed and not EnrouteActive gets
    `FUN_004f7ed0`. While its parent's bit `0x8` is set, that calls the
    child's `+0xac` (SetDestroyedReason) with the parent's reason. So the
    child is destroyed with the same reason, recursively.
  - any other child whose own bit `0x8` is clear gets Autorouting on.
- `CapShip_vt11c` then walks the children of families `0x30..0x3c`
  (characters) through slot `+0x2e8`, for reasons 1..0x10.

Rule: the squadrons docked on a capital ship that is destroyed are
destroyed with it, whatever the reason. Squadrons still being built or en
route reroute.

### The battle's commit, `FUN_0040c780`

The commit walks the system's fleets (`FUN_004ffe70(system, 3)`), their
ships and their squadrons, then the system's own squadrons, in three
passes:

1. `FUN_0040d260` sets each squadron's count from its battle record
   (`FUN_005968e0` -> `FUN_005b8e00`, 12 fighters a squadron).
2. `FUN_0040d320` gives each squadron a new parent:
   - The record's carrier flag is set when the squadron's `+0x164`
     carrier is found among the battle's units as a capital ship
     (`FUN_005a6360`: the unit's type, slot `+0x20`, is 0; it returns
     the hangar `+0x418`), and its state `+0x170` is 1 (in the hangar)
     or 5 (docked).
   - With the flag, the carrier becomes the parent when it still exists
     and is on the squadron's side (`+0x24 & 0xc0`).
   - Otherwise the parent is the battle's system (`[esi+8]` ->
     `FUN_004025b0`, `0x40d47c`).
   - The parent changes through slot `+0xa8` (`FUN_004f6fd0`).
3. `FUN_0040cfc0` applies each ship's hull. `FUN_0040d170` applies the
   fleets' other units. `FUN_0040d510` then moves each ship's squadrons
   over its fighter capacity (`+0x26c`) to the system (`[ebx+8]`,
   `0x40d5f9`).

So after a battle:
- a squadron in a hangar or docked rides on that carrier;
- every other squadron, the launched ones included, stays at the system.

If a carrier lost in the fight is still found by `FUN_005f5500`, its
hangar's squadrons go back aboard and are destroyed with it through the
cascade above. If it is not found, they stay at the system.

Still open:

- whether a scrap or a capture reaches `SetDestroyedReason`;
- whether `FUN_005f5500` still finds a destroyed unit;
- what moves the system's squadrons back onto the fleet afterwards, if
  anything does.
