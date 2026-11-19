# Fog of war: what a side sees of each system (traced 2026-11-19)

Tracker item "Mgła wojny według oryginału". Builds on
`per-side-object-views.md` (the three copies, `FUN_004f9a60`, the seen bits,
`Event319_Handle`) and `headquarters-overlay.md` ("Can the Empire see 904").
Read from `~/Games/rebexe-decompile/decompiled-typed` and the REBEXE.EXE
disassembly (`objdump` at `0x51b990`).

Summary: the original has no fog layer, no sensor range and no partial
reveal. Every window and the GID read the viewer's copy of each system
(`FUN_004f2d10` / `FUN_004f3220`). A copy is live while the side holds the
system or has a usable fleet in it; otherwise it stays as it was last
synced. Copies change only by going live, by a setup push or by an
intelligence push. Nothing decays.

## Where the copies come from: the setup push (`FUN_0051b990`)

Setup stage `0x18` (`FUN_005136d0`) calls `FUN_0051b940`, which runs
`FUN_0051b990` on the galaxy's Alliance copy, then its Empire copy
(`FUN_00539fd0(1)`, `(2)`). Read in full from the disassembly
(`0x51b990..0x51bc2c`).

It builds two update contexts from the caller's (`FUN_004fd450`):

| Context | `+4` (sync mode) | `+8` (state) |
|---|---|---|
| A (`esp+0x88`) | 2 | 2 |
| B (`esp+0x2c`) | 1 | 2 |

- `+8` = 2: only frozen copies take the push (`FUN_005404d0`). A side's own
  systems are already live and skip it.
- `+4` is the sync mode of the copy's vtable `+0x24`. For a system that is
  `FUN_0050e5b0` (through `CoreSystem_vt24` `FUN_00566d00` ->
  `FUN_00566f80`, `RimSystem_vt24` `FUN_0056a530` -> `FUN_0056a7f0`):
  - mode 1 syncs every field: loyalty, energy, raw materials, control kind
    and data, populated, explored, uprising, blockade, battle, the
    incidents (`System_Sync*`), and through `FUN_004f9540` the owner
    (`FUN_004f9ca0`) and the GameObj bits;
  - mode 2 syncs only the container link and the usable, created,
    completed and existing bits (`FUN_004f9d40`, `GameObj_Sync*`). No
    owner, no explored, no values.

The pushes (each through `FUN_004f7e30`, which rebuilds the matching copy
of each child in a type range with vtable `+0x24`):

1. `FUN_004f8340(galaxy, B)`: the galaxy's direct children `0x01..0xff`
   (the sectors), mode 1.
2. For each sector (`FUN_005266a0`, families `0x80..0x8f`):
   - `FUN_0050c8a0(sector, A)`: its systems `0x90..0x97`, mode 2;
   - for each system (`FUN_00512f90`): `FUN_00529a40(system, A)`, its
     `0xa0..0xaf` children (the manufacturing managers, hyp in
     `production-destination.md`), mode 2;
   - `FUN_00566c00(sector, B)`: its core systems `0x90..0x91`, mode 1;
   - for each core system (`FUN_00567340`): `FUN_00539ac0(system, B)`, its
     facilities `0x20..0x2f`, mode 1.
3. `Event319_Handle(10, 2, ..)` on Yavin (`0x92000121`, `FUN_00505f60`):
   the Empire gets an Espionage-level copy of Yavin and all its children.

So at the start of a game each side holds:

- its own systems live;
- a full snapshot of every core system (family `0x90`, `CoreSystem` vtable
  `0x00663698`): side, support, energy, raw materials, explored, populated,
  and its defense, manufacturing and production facilities. No fleets,
  regiments, squadrons, special forces or characters (types `0x08..0x1f`,
  `0x30..0x3f` are not in any range);
- a bare copy of every Rim system (family `0x92`, `RimSystem` vtable
  `0x00664098`): it exists in its sector, but the copy's explored bit, side
  and values are the copy's initial ones (hyp: zero; the copy is created
  frozen by `FUN_0053f760` and the setup setters' syncs
  (`System_SetExplored` `FUN_0050a430` -> `System_SyncExplored`
  `FUN_0050fc80`) fail `FUN_005404d0` on a frozen copy);
- the Empire only: a whole copy of Yavin.

Correction to `headquarters-overlay.md` ("No facility, so no HQ, enters the
Empire's copies at setup"): the core systems' facilities do enter both
sides' copies. The conclusion for the HQ stands, as the HQ starts on a Rim
system (manual p. 69).

## When a copy is live, and what going live does

`FUN_004f9a60` (`per-side-object-views.md`): live while the master's owner
is the side, or while the master has the side's seen bit, is completed and
is not en route. Going from frozen to live rebuilds the copy from the
master with the caller's context (normally mode 1) and `FUN_004f8340` on
its children.

The system's seen bit (`FUN_0050ac80`, read in full): `FUN_004ffef0(system,
mode 1, side)` walks the system's direct children of types `0x08..0x0f`
(fleets) of that side in mode 1, `+0x50` bit 0 usable
(`object-state-flags.md`), or the system's owner is the side. A fleet in
hyperspace is not usable, so it counts toward neither end of its trip until
it arrives (`FUN_00556390` moves its container at once, but the usable bit
is clear in flight). A character, regiment or mission at a system does not
make it seen. Callers: `System_vtc8` `FUN_00508660` (a fleet enters or
leaves) and `System_vtf0` `FUN_00510820` (owner change).

Objects inside a system take the system's seen bit (`FUN_004f7bf0` /
`FUN_004f7c70`, unless their own `+0x50` bit `0x20` is set), so they go live
and frozen with it.

## What the windows read

- GID item (`FUN_0042b330`, read lines `0x42b330..0x42b51d`): `param_3 =
  FUN_004f2d10(view +0x9c, id)`, the viewer's copy. The unexplored mark
  (`+0x88` bit 1), the destroyed gate (`+0x50` bit 3), the holder (`+0x24
  >> 6`), the support (`FUN_00507270`), the populated bit and the
  headquarters burst walk (`FUN_00526cf0(copy, 3)`) are all the copy's.
  Manual p. 70: GRAY "You have no information about this system. It could
  be neutral, unpopulated, or colonized by the enemy."
- Sector window planet (`FUN_0045bbb0`, read `0x45bbb0..` to the overlay):
  `FUN_004f3220(view +0x9c, id)`, the viewer's copy: destroyed (`+0x50`
  bit 3, asteroid 10240 and name `0x2ffff00`), uprising (`+0x88` bit 2,
  905/906), name colour by the side passed in (`0x20000ff`, `0x200ff00`,
  else `0x2ffff00`), the headquarters overlay. The tracks (`FUN_0045b770`)
  read the same copy. The map item's bitmap (the GID mark) is copied into
  the planet, so an unexplored copy shows no mark (A0 sector-windows/04-left).
- Every other window (`FUN_004f2d10..FUN_004f3f10`): the side's copy of the
  object.

## Updates without sight: the intelligence pushes

`Event319_Handle` (`FUN_0050d5a0`) levels and callers are in
`per-side-object-views.md`. All its system levels sync the explored and
populated fields, so a push explores the system for the pushed side only.
A Reconnaissance success (level 8, `FUN_0056bee0`) and an Espionage
success (level 10, `FUN_00573610`, `FUN_00573890`) push for the mission's
side. Explored is a field of each copy; the master's explored bit only
feeds it.

Untraced:
- where the master's explored bit of a never-held Rim system is set. Its
  only setter, `System_SetExplored`, is called only by `System_vtbc`
  `FUN_005081f0` (from `CoreSystem_vtbc` `FUN_00566d40`, `RimSystem_vtbc`
  `FUN_0056a550`), which runs from `FUN_0051bc70` at setup stage `0xc`
  (`FUN_0051a280`, the starting side lists). hyp: the master is explored
  for every system (SYSTEMSD load), since manual p. 107 and p. 121 say a
  Reconnaissance or a fleet's arrival explores a Rim system, and both only
  copy the master's bit;
- whether the copy's explored change (`SystemExploredNotif` `FUN_00512190`,
  notification `0x14a`) reaches the message log;
- the System Finder's tabs (manual p. 74, "Unexplored Systems"): which copy
  it reads.

Nothing decays: no timer clears a seen bit or freezes a live copy except
`FUN_0050ac80` losing its fleet or owner, and nothing clears explored on a
copy (`System_SyncExplored` only copies a master that is never cleared).

## The manual's claims

| Claim | Status |
|---|---|
| Rim systems are unexplored at the start (p. 12) | Confirmed: bare Rim copies (`FUN_0051b990` mode 2) |
| Gray: no information about the system (p. 70) | Confirmed: the GID reads the copy (`FUN_0042b330`) |
| The Empire does not know where the Alliance HQ is (p. 69) | Confirmed: the HQ's Rim system is a bare copy to the Empire |
| Production facilities and resources known at the start are accurate; defenses may be incomplete without Espionage (p. 56, p. 58) | Confirmed for core systems: setup pushes their fields and facilities (`0x20..0x2f`), not regiments or squadrons |
| A fleet arriving at an unexplored system learns what a Recon does (p. 121) | Confirmed in part: the copy goes live (`FUN_0050ac80`, `FUN_004f9a60`). The manual's "except characters, SpecForces and manufacturing" is not in the live rule, which shows all children; unresolved |
| Reconnaissance explores and reports facilities, units, resources, support, control; not characters, SpecForces, manufacturing (p. 107) | Confirmed: level 8 pushes the whole system and `0x20..0x2f`, `0x10..0x1f`, `0xa0..0xaf`, `0x08..0x0f` (no `0x30..0x3f`) |
| Espionage's snapshot "can change" (p. 106) | Confirmed: a pushed frozen copy keeps the pushed values |
| Informants give an Espionage-like, often partial report (p. 104) | Confirmed: `FUN_0050d510` with a level from `FUN_00559db0` |
| Opponent's personnel not accurate until Espionage (p. 39) | Confirmed: characters enter copies only by sight or pushes 5..8, 10 |
| You won't know a fleet's complement until you meet it (p. 142) | Confirmed: fleets are not in the setup push |

## The port (2026-11-19 inventory)

Matches the original:
- `object_views::system_seen` (`object_views.rs:432`): `FUN_0050ac80`.
- `object_views::compute_systems` / `compute` / `advance_extras`: copies go
  live and freeze by `FUN_004f9a60`.
- `queue_push` / `queue_push_both` for every traced `Event319_Handle`
  caller (`per-side-object-views.md`, "The pushes' triggers").
- `viewed_contents::system_sight` (`viewed_contents.rs:82`) except its fog
  fallback.

Port inventions (no original rule):
- `fog.rs:139..229` `FogSystem::advance`: sensor reach
  (`SENSOR_MULTIPLIER` 15 per detection point, every system in the radius,
  forever) and the half-way reveal of a destination
  (`order.progress() >= 0.5`).
- `fog.rs` `FogState`: one monotonic revealed set per side, saved; never
  forgets.
- `viewed_contents.rs:100`: a fog-revealed system with no copy is seen
  live, for every window that goes through `system_sight`,
  `opposing_contents_visible`, `frozen_extras` (system, fleet, defenses,
  manufacturing, missions windows, quadrant icons, finders, GID fleet
  counts, targeting).
- `main.rs:3723..3740`: the "System X revealed" message for each fog
  reveal (unknown in the original; see "Untraced").
- `main.rs:4739..4743`, `integrator.rs:1007`: an informant report also
  reveals the system in the fog (the push already gives the copy).
- `missions.rs:1846`, `integrator.rs:2027`, `main.rs:15206`,
  `final_battle.rs:389`: an intelligence result sets the shared
  `System::exploration_status`, so a Reconnaissance explores the system
  for both sides.
- `seeds.rs:152..172`: the three special systems are explored for both
  sides (the Rebel HQ too).
- `rebellion-data/src/lib.rs:226`: explored is the DAT family (`0x90`
  core = explored, `0x92` Rim = unexplored), one value for both sides.
- New game (`main.rs:5664..5690`): copies are seeded only for systems the
  fog shows (own fleets). There is no setup push and no Yavin push; most
  systems have no copy at all.
- `render/fog.rs` `draw_fog_overlay`: dead code (no caller).

Master reads where the original reads the copy (leaks):
- GID: `lib.rs:734` (support marker), `lib.rs:743..` `gid_marker_for_system`
  (holder, size), `lib.rs:875` `gid_headquarters_burst` (live
  `is_headquarters`: the Empire's GID can show the Alliance HQ's burst),
  `lib.rs:889` `gid_shows_unexplored` (shared explored).
- Sector window: `sector_window.rs:1448` `planet_art` (destroyed),
  `:1481` `system_name_color`, `:1499` `uprising_mark`, `:1805`
  `track_shown`, `:1684` `StatusTrackValues::shown` (live values for a
  system with no copy).
- Explored gates: `defenses_window.rs:967`, `quadrant_icons.rs:326`,
  `fleet_window.rs:2017`, `manufacturing_window.rs:638`, `:666`,
  `system_finder.rs:199`, `missions.rs:1024` (already tagged port).
- AI: `ai_records.rs:1143`, `ai_strategy.rs:634`, `agent_build.rs:195`
  read the shared explored bit (the AI's own copy reads are untraced).

Fixtures and debug, kept: `interface_test_fixture::reveal`
(`main.rs:1934`), `reveal_all_systems`, `see_everything`, replay
`replay.rs:865`.

## Proposed port change

Remove:
1. `FogSystem::advance`'s sensor reach and half-way reveal, and with them
   the three tests that assert them (`advance_in_transit_reveals_destination_at_half_progress`,
   `sensor_radius_reveals_nearby_systems`, `sensor_detection_zero_reveals_only_own_system`):
   the behaviour has no original rule (this note).
2. The fog fallback in `viewed_contents::system_sight`; then `FogState` as a
   source of sight (keep it only for the save migration and the fixtures'
   `reveal`, or replace those with copies).
3. The "System X revealed" message, unless A0 shows a message on
   exploration.
4. The shared `exploration_status` writes on intelligence results and the
   special-system seeding of explored for both sides.
5. `render/fog.rs` (dead).

Add:
1. The setup push at a new game (`FUN_0051b990`): per side, a copy of every
   system; core systems full with their facilities; Rim systems bare
   (unexplored, no side, no values); own systems live; the Empire's
   level-10 push of Yavin.
2. `SystemView.explored`, `is_destroyed`, `uprising` (save version bump;
   migration seeds old saves' copies by the setup rule plus their fog).
3. Copy reads (`system_sight` for every system, frozen or live) in the GID
   marks, unexplored mark, HQ burst and support display; the sector
   window's name colour, tracks, uprising and destroyed art; the windows'
   explored gates; the System Finder tabs once traced.
4. A push explores the pushed side's copy only.

Tests (each written failing first):
- `a_new_game_gives_each_side_a_full_copy_of_every_core_system_and_a_bare_one_of_each_rim_system`;
- `the_empire_starts_with_a_copy_of_yavin_and_its_leaders`;
- `a_moving_fleet_reveals_no_system_before_it_arrives`;
- `a_fleet_reveals_no_neighbouring_system`;
- `a_rim_system_stays_explored_with_its_last_seen_side_after_the_fleet_leaves`;
- `a_reconnaissance_explores_the_system_for_its_side_only`;
- `the_empires_gid_shows_the_headquarters_system_gray_with_no_burst`;
- `a_sector_window_names_a_frozen_system_in_its_copys_colour`;
- the save migration's round trip.

Gates: GID and sector-window interface-parity scenarios per side at day 1
and after each A0 state below, compared with A0 pixel for pixel.

## Ported (2026-11-19)

- Removed: the sensor reach and half-way reveal (`FogSystem::advance` now
  records only where fleets stand), the fog fallback in `system_sight`,
  the "System X revealed" message, the informant's fog reveal, the
  shared `exploration_status` writes of intelligence results and the
  Final Battle, `render/fog.rs`. The special systems' seeding stays: it
  sets the system's own (master) bit, which the hyp above holds explored.
- `object_views::setup` at a new game: core copies with facilities (no
  squadrons, special forces or characters), bare Rim copies, the
  Empire's push of Yavin, taken at once.
- `ObjectViews::system_bits` (explored, destroyed per copy), saved as
  `SaveState::system_copy_bits` (save v127). `side_explored` /
  `side_destroyed` fall back to the system's own bits where a copy holds
  none (a v126 save, tests, fixtures). A live copy takes explored, a
  freezing one keeps its bit, every push sets it; a copy seeded from the
  fog takes the system's own bit or, where the side's fleet stands, explored.
- `viewed_contents::apparent_system`: the GID marks, unexplored mark and
  HQ burst, and the sector window's name colour, destroyed art and tracks
  read the player's copy. port: the facility-count displays still count
  the system's own facilities, and the uprising mark is the system's
  (`SystemView` holds no uprising bit yet).
- The Defenses, Fleet, Manufacturing and Missions windows, the quadrant
  icons, the System Finder tabs, the explored-container creation check
  and the AI system record read the side's explored bit.
- Not changed: the AI's own-system reads (`ai_strategy.rs`,
  `agent_build.rs`), whose tests hold a side's own system unexplored; the
  headless simulation's single fog (`rebellion-data/src/simulation.rs`).

## A0 frames to capture

Each per side (Alliance game, Empire game), saved so the port's fixture can
load the same state:
1. Day 1, GID in its default display and in Popular Support: Rim systems
   gray except own; core systems coloured.
2. Day 1, sector windows: a core sector of the other side (names, tracks,
   marks); a Rim sector with own systems; the Alliance HQ's sector in the
   Empire game (gray, no 904, no tracks); Yavin's sector in the Empire game.
3. Day 1, an enemy core system's Manufacturing window (facilities listed)
   and Defenses window (no regiments, squadrons or personnel).
4. A fleet ordered to a gray Rim system: GID and sector window past half
   the trip (port reveals, original gray), and a gray system near a fleet
   with high detection (port reveals by sensor, original gray).
5. The fleet's arrival: GID, sector window, message log.
6. The fleet gone and the system changed since (garrisoned or support
   moved by the other side): the copy keeps the old values.
7. A Reconnaissance success on a gray Rim system: the GID for that side;
   then the other side's view of the same system (edited save, header
   byte `0x16` flip) still gray.
8. Day 1, the System Finder's Unexplored tab.
