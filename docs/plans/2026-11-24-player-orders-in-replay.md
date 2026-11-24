# Player orders in the replay (tracker platform 1)

Approved by the user on 2026-11-23 ("tak zapis rozkazów gracza"). Today a
replay holds only engine commands (`AdvanceTicks`, `SetSpeed`, ...), so no
app session can be replayed (`agent_docs/deterministic-replay.md`, "Next
implementation boundary").

## Boundary

The app turns a player's order into a `PanelAction`, and
`panel_actions::apply_panel_action` changes the state. Each state-changing
action moves, in stages, into a shared function in
`rebellion-data::player_command` that takes a `PlayerCommand` and the states
it touches and returns an outcome; the app maps the outcome to its messages
and sounds, and the replay runner applies the same command through
`ReplayCommand::Player`. One function, two callers, so the app and a replay
cannot drift.

## Identity

A replay names objects by ids that survive a fresh run from the same seed,
never by slotmap keys:

- systems and characters by their `DatId`;
- fleets by creation serial (`FleetNames::fleet_serial`, save v111);
- regiments by object id (`ObjectViews::regiment_id`).

## Stages

1. Fleet dispatch (`DispatchFleet`, with the regiments it carries): the
   shared function, the replay command, the app's call, a native test that
   the app's path and a replayed command give the same fingerprint.
2. The app records its commands with their tick and side (a session log),
   and a session can be written as a replay artifact.
3. The other orders, one family a stage: fleet join and split, character,
   regiment, squadron and special-force moves, missions, production
   (build, stop, reserve, destination), bombardment, assault, command and
   retire orders, renames.
4. A browser gate: a short scripted session replayed natively and in WASM
   to the same fingerprints.

Not in scope: a played tactical battle's own command stream (its own
stage later), and the format version stays 2 while commands are only
added (a file with a player command fails closed on an older reader).

## Progress (2026-11-24 night)

- Stage 1: fleet dispatch (99801c59).
- Stage 3a-3e: regiment, squadron, fleet-join and headquarters moves;
  production (Reserved, Stop, Build, Destinations to a system or a
  fleet); missions; renames, Create Fleet, ships into the spare fleet;
  Move of characters, special forces and groups; products' moves;
  command and Retire orders. Each order's app arm calls the same
  function the replay applies, and a seed-42 test per family compares the
  app path's fingerprint with the replay's.
- Stage 2: `session::SessionLog` and the app's logging (each order with
  its day and moment, speed changes) and the palette's "Check Session
  Replay"; `CampaignDayRunner::run_day_with_orders` puts an order between
  a day's AI passes, as the app's frame step does.
- Every order's aftermath (`after_order`: an order's units placed in
  their containers) runs in the app and in the replay.
- Left in stage 3: Planetary Bombardment and Assault. Their app arms use
  app-only parts (`BombardEnv`, `destroy_planet`, `occupy_system`, the
  assault reports) and the day's draws, so they move with those parts.
- Seen on the way: the save-state fingerprint encodes slotmap key
  versions. A product's move stages and drops a fleet; a live world
  reuses the slot with a new version while a state loaded afresh hands
  out a first-version slot, so the same game state can fingerprint
  differently before and after a save and load. Replays run in one live
  runtime, so they are unaffected; a fingerprint across a save and load
  is not a game comparison until keys are normalised.
