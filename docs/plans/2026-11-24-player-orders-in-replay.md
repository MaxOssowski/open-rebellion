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
