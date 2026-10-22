# Replacing the invented `jedi.rs` (inventory, 2026-10-19)

The parity tracker's item "Zastąpienie wymyślonego jedi.rs" (F-038). This
note lists what `rebellion-core/src/jedi.rs` and its fields do, what the
original does instead, and the plan. Each original rule named here is
already traced, in the note cited.

## The original's Force fields (already ported)

| Field | Port | Note |
|---|---|---|
| `ForcePotential`, `ForceAware` | `Character::force_potential`, `force_aware` | `force-awareness.md` |
| `ForceExperience` `+0x8e` | `Character::force_base` | `force-awareness.md` |
| `ForceTraining` `+0x90` | `Character::force_training` | `force-awareness.md` |
| Force `+0x8c` = `+0x8e` + `+0x90` | `Character::force_level()` | `force-awareness.md` |
| discovery by a trainer (`0x362`) | `force_discovery.rs` | tracker item "Odkrywanie Mocy przez trenera" |
| detection across sides | `mission_detection.rs` | tracker item "Wykrywanie Mocy między stronami" |
| training (Jedi Training mission, Dagobah, escapes, encounter) | `missions.rs`, `dagobah.rs` | tracker item "Wzrost Mocy" |
| Force Ranking by `+0x8c` | `status_window.rs` | `status-window.md` |

## What `jedi.rs` adds, with no source

- `ForceTier` (None, Aware, Training, Experienced), on `Character::force_tier`.
  - The module cites "`entity[9] >> 6 & 3`". That is the object's side
    (`+0x24 >> 6 & 3`), not a Force tier.
  - The loader sets Aware from MJCHARSD `is_known_jedi`.
- `force_experience` points: +1 a day in training, with tier thresholds
  `XP_TO_TRAINING` 50 and `XP_TO_EXPERIENCED` 150.
- `JediState::training` and the `JediSystem::advance` day step:
  - `simulation.rs` step 13;
  - `main.rs`;
  - `integrator.rs` `apply_jedi_events`.
- A detection check every 30 days at flat chances (5%, 10%, 20%). It sets
  `Character::is_discovered_jedi`, which nothing else reads.
- These story events in `story_events.rs`:
  - "Force Awakening" and "Force Training Milestone". They are fired on the
    tiers, under ids `0x1e1` and `0x1e5`, which are the original's
    `CharacterForceNotif` and `CharacterForceTrainingNotif`
    (`entity-system.md`), not messages.
  - "Leia Discovers the Force" (`0x363`, no original id), with
    `ModifyForceTier`. Leia's real discovery is the trainer's `0x362`, which
    is already ported.
- The event engine's tier conditions and actions in `events.rs`:
  - `CharacterHasForceLevel`, `CharacterIsForceUser`,
    `CharacterForceExperience`;
  - `ModifyForceTier`, `StartJediTraining`.
- The egui side panels of the port, which have no original counterpart:
  - "Jedi Training" (key J, `panels/jedi.rs`): tiers, XP bars, flat
    detection risk, and start and stop training buttons;
  - "Officers" (key O, `panels/officers.rs`), whose Jedi column shows the
    tier.
- Save fields:
  - `Character::force_tier`, `force_experience`, `is_discovered_jedi`;
  - `SaveState::jedi` (`JediState`).

## Plan

1. **This inventory.**
2. **The simulation.**
   - Remove the `JediSystem` day step, `apply_jedi_events` and
     `StartJediTraining` from the simulation and the app.
   - Remove the three invented story events and the tier conditions and
     actions.
   - The fields stay, unwritten, until stage 4.
   - The campaigns change for this named cause.
   - Done 2026-10-19: the day step (`simulation.rs` step 13, `main.rs`),
     `apply_jedi_events`, the story events' `StartJediTraining` extraction,
     the telemetry `jedi_check`, `jedi_tier_advanced` and
     `jedi_discovered`, and the three story events are gone.
     `define_story_events` now defines none.
   - The event engine's tier variants stay until stage 4, because the save
     keeps event definitions (`EventState::events`).
   - The seed-42 golden is regenerated for this cause, since its state held
     those three definitions.
3. **The panels on the real fields.**
   - The Jedi panel shows each character's Force `+0x8c` and its ranking (the
     Status window's thresholds), `ForceAware` and the discovery.
   - Training is the Jedi Training mission, so the panel gives no training
     order.
   - The Officers panel reads the same fields.
   - Done 2026-10-19. The J panel, now "The Force", lists the side's
     living characters with a Force value, `ForceAware` or `ForcePotential`,
     strongest first: the value `+0x8c`, the Status window's ranking word,
     the two flags and the discoverer (`panels::jedi::force_summary`). It
     gives no order.
   - The Officers panel's badge and detail read the same summary. Its
     "JEDI" badge had followed the class's `jedi_probability` and marked
     many characters with no Force; it is now "FORCE".
   - `PanelAction::StartJediTraining` and `StopJediTraining` are gone.
   - Removed 2026-10-22, at the user's decision on the replacement
     panels: the Officers panel (key O) and "The Force" (key J). The
     original shows a character's Force only in the Status window's
     "Force Ranking:" row (`FUN_004486f0`), which the port has; it shows
     neither `ForceAware`, `ForcePotential` nor the discoverer.
4. **The save.**
   - Remove `force_tier`, `force_experience`, `is_discovered_jedi` and
     `SaveState::jedi`, and delete `jedi.rs`.
   - This is a save format change (v87), which needs the user's approval,
     and it regenerates the golden.
   - Done 2026-10-19 (approved). Removed:
     - `ForceTier`, `force_tier`, `force_experience`, `is_discovered_jedi`;
     - `JediState` and its save field;
     - `jedi.rs`;
     - the event engine's `CharacterHasForceLevel`, `CharacterIsForceUser`,
       `CharacterForceExperience`, `ModifyForceTier`, `StartJediTraining`
       and `AccumulateForceExperience`;
     - `GameEffect::JediTierAdvanced` and `JediDiscovered`;
     - `EVT_LEIA_FORCE` (`0x363`), with the interface fixture's message.
   - The story class for `0x1e1` (Force Growth, TEXTSTRA `0x7180`) stays
     mapped. It is the original's id and texts, though nothing posts it.
