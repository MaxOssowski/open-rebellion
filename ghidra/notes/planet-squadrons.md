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
  `squadrons::move_squadrons` boards a fleet in orbit with room, lands on
  the planet, or passes to another fleet in the same system.
- `AIAction::MoveSquadrons`: the production heuristic standing in for
  module `0xb` boards the side's planet squadrons onto its fleets in orbit,
  up to their room.

port:
- The room is the whole fleet's (living ships' capacity less the squadrons
  aboard), where the original fills one ship at a time.
- A squadron's move to another system is refused.
- A planet's squadrons take no part in a battle: the port's combat is fleet
  against fleet. With the holder's squadrons present an enemy fleet meets
  neither a blockade nor a battle.
- A mission names squadrons aboard fleets only; Scrap and Confirmed Move
  stay disabled for a squadron.
- The port's seeding still puts a seed group's squadrons into a fleet, which
  may hold squadrons alone; such a fleet disbands when its last squadron
  lands.
- The support drift still counts the fleets and their squadrons
  (`economy.rs`), not the planet's squadrons alone.

hyp: a squadron may not land on a planet another side holds (the regiment's
rule, `FUN_0053d430` `1`/`0x28`); the Galaxy display's squadron count
includes the fleets' and the planet's squadrons.
