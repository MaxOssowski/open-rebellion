//! The port's remaining panels: game setup and mods.
//!
//! Each panel is a standalone function that accepts the egui context, read-only
//! world data, and mutable panel-local state.  Every panel returns
//! `Option<PanelAction>` — the caller applies the action to game state rather
//! than letting the panel borrow mutable world references, which would conflict
//! with egui's `FnMut` closure requirements.
//!
//! # Integration
//!
//! Call panel draw functions inside the `egui_macroquad::ui` closure:
//!
//! ```ignore
//! egui_macroquad::ui(|ctx| {
//!     if let Some(action) = panels::draw_mod_manager(ctx, &mut mods) {
//!         apply_panel_action(&mut world, &mut missions, action);
//!     }
//! });
//! egui_macroquad::draw();
//! ```

pub mod game_setup;
pub mod mod_manager;

#[cfg(any(debug_assertions, not(target_arch = "wasm32")))]
pub mod command_palette;

pub use mod_manager::{draw_mod_manager, ModInfo, ModManagerAction, ModManagerState};

use rebellion_core::fleet_join::FleetMover;
use rebellion_core::ids::{CharacterKey, FleetKey, SystemKey, TroopKey};
use rebellion_core::manufacturing::BuildableKind;
use rebellion_core::missions::{MissionFaction, MissionKind, MissionMember};
use rebellion_core::troop_transport::RegimentTarget;

/// Player-initiated actions returned by War Room panels.
///
/// The caller applies these to `GameWorld`, `ManufacturingState`, and
/// `MissionState` rather than the panels borrowing mutable world refs.
#[derive(Debug, Clone)]
pub enum PanelAction {
    // ── Faction Selection ─────────────────────────────────────────────────────
    /// Player has chosen a starting faction.  Gates all other panels.
    SelectFaction(MissionFaction),

    // ── Characters ────────────────────────────────────────────────────────────
    /// A confirmed Retire (`0x242`) on a character or a special force: the
    /// order validates again and the member retires
    /// (`rebellion_core::character_retire`).
    Retire(MissionMember),

    // ── Fleets ────────────────────────────────────────────────────────────────
    /// Select a system on the galaxy map (the Encyclopedia's system entry).
    FocusFleetSystem(SystemKey),
    /// A fleet's or capital ships' Move (`0x201`) released on a Fleet window
    /// (`fleet_join::join_fleet`).
    JoinFleet { mover: FleetMover, target: FleetKey },
    /// Destination (`0x214`): `system`'s production areas, or the one
    /// given, deliver to `destination` (`ManufacturingState::set_destination`).
    SetDestination {
        system: SystemKey,
        area: Option<rebellion_core::manufacturing::ProductionArea>,
        destination: SystemKey,
    },
    /// Destination (`0x214`) released on a fleet: `system`'s `area`
    /// (a shipyard or training manager) delivers aboard `fleet`, standing at
    /// `at` (`ManufacturingState::set_fleet_destination`).
    SetFleetDestination {
        system: SystemKey,
        area: rebellion_core::manufacturing::ProductionArea,
        fleet: FleetKey,
        at: SystemKey,
    },
    /// Rename (`0x203`) issued from its edit: `FUN_004f6e60` sets a fleet's
    /// or, with `ship`, one of its capital ships' name.
    Rename {
        fleet: FleetKey,
        ship: Option<usize>,
        name: String,
    },
    /// Rename (`0x203`) of an unfinished ship, issued from its edit
    /// (`rebellion_core::products::rename`).
    RenameProduct { product: u64, name: String },
    /// A capital ship's Create Fleet (`0x270`, `fleet_join::create_fleet`).
    CreateFleet {
        fleet: FleetKey,
        ships: Vec<usize>,
        roster: u64,
    },
    /// Capital ships' Move (`0x201`) released on a system: into the
    /// side's bit-1 spare there (`fleet_join::move_ships_into_spare`).
    MoveShips {
        fleet: FleetKey,
        ships: Vec<usize>,
        roster: u64,
        system: SystemKey,
    },
    /// Dispatch one player-controlled fleet to a selected destination.
    DispatchFleet {
        fleet: FleetKey,
        destination: SystemKey,
        troops: Vec<TroopKey>,
    },
    /// A regiment's Move (`0x201`) released on a system or fleet
    /// (`TroopTransportState::move_regiment`).
    MoveRegiment {
        troop: TroopKey,
        target: RegimentTarget,
    },
    /// The headquarters' Move (`0x201`) to a system
    /// (`rebellion_core::headquarters_move::move_headquarters`).
    MoveHeadquarters { destination: SystemKey },
    /// A character's Move (`0x201` -> `0x241`) released on a system, or on
    /// a fleet there to board: the Move mission `0x41000001`
    /// (`ghidra/notes/move-order.md`, "A character's Move").
    MoveCharacter {
        character: rebellion_core::ids::CharacterKey,
        system: rebellion_core::ids::SystemKey,
        fleet: Option<rebellion_core::ids::FleetKey>,
    },
    /// The Move (`0x201`) of unfinished units (`rebellion_core::products`)
    /// released on a system, or on a fleet there: each changes container
    /// at once, with no trip (`FUN_00556390`: not Deployed, so not mobile;
    /// `products::move_to_target`).
    MoveProducts {
        products: Vec<u64>,
        system: rebellion_core::ids::SystemKey,
        fleet: Option<rebellion_core::ids::FleetKey>,
    },
    /// A special force's Move (`0x201` -> `0x241`) released on a system,
    /// or on a fleet there to board: the Move mission with the unit as its
    /// team, as a character's.
    MoveSpecialForce {
        unit: rebellion_core::ids::SpecialForceKey,
        system: rebellion_core::ids::SystemKey,
        fleet: Option<rebellion_core::ids::FleetKey>,
    },
    /// The one `0x241` sub-order a Move builds for its characters and
    /// special forces together (`FUN_0053c810`): one Move mission with them
    /// all as its team, to a system or aboard a fleet there.
    MoveMembers {
        members: Vec<rebellion_core::missions::MissionMember>,
        system: rebellion_core::ids::SystemKey,
        fleet: Option<rebellion_core::ids::FleetKey>,
    },
    /// A squadron's Move (`0x201`) released on a system or fleet
    /// (`squadrons::move_squadrons`).
    MoveSquadron {
        class: rebellion_core::ids::FighterKey,
        from: rebellion_core::squadrons::SquadronPlace,
        to: rebellion_core::squadrons::SquadronPlace,
    },

    // ── Manufacturing ─────────────────────────────────────────────────────────
    /// Stop (`0x213`): `system`'s `area` drops every unit it was building
    /// (`ManufacturingState::stop`).
    StopProduction {
        system: SystemKey,
        area: rebellion_core::manufacturing::ProductionArea,
    },
    /// Reserved (`0x216`): toggle `system`'s `area` manager's Reserved
    /// (`ManuMgr_SetReserved`); with no `area`, a facility icon's, every
    /// manager of the system (`ManufacturingState::toggle_system_reserved`).
    ToggleReserved {
        system: SystemKey,
        area: Option<rebellion_core::manufacturing::ProductionArea>,
    },
    /// Build Selection's Confirm (`FUN_00438980`): `count` units of `kind`
    /// replace what `system`'s `area` was building.
    BuildProduction {
        system: SystemKey,
        area: rebellion_core::manufacturing::ProductionArea,
        kind: BuildableKind,
        count: u32,
    },

    // ── Missions ──────────────────────────────────────────────────────────────
    /// Dispatch a mission ordered on day `tick` with its agents and decoys
    /// (the dialog's "Begin Mission", `FUN_0046c3c0` case `0x66`).
    DispatchMission {
        kind: MissionKind,
        faction: MissionFaction,
        team: Vec<MissionMember>,
        decoys: Vec<MissionMember>,
        target: SystemKey,
        target_character: Option<CharacterKey>,
        /// The object the release targeted (`+0x68`), if not a character.
        target_object: Option<rebellion_core::missions::MissionTarget>,
        tick: u64,
    },

    // ── Save / Load (the Game Options window's rows) ─────────────────────────
    /// Save to the given slot with the given name.
    SaveGame { slot: usize, name: String },
    /// Load the given slot.
    LoadGame { slot: usize },

    // ── Mod Manager ─────────────────────────────────────────────────────
    /// Open/close the mod manager panel.
    OpenModManager,
    /// Toggle a mod's enabled state.
    ToggleMod { name: String },
    /// Reload all mods from disk.
    ReloadMods,

    // ── Bombardment ──────────────────────────────────────────────────
    /// Order Planetary Bombardment `kind` (`0x220..0x223`) of `system` by
    /// the player's `fleets` (`FUN_00486fb0` -> `FUN_00487740`).
    OrderBombardment {
        fleets: Vec<FleetKey>,
        system: SystemKey,
        kind: u32,
    },
    /// Order Planetary Assault (`0x234`) of `system` by the player's
    /// `fleets` (`FUN_00486fb0` -> `FUN_00487740` -> `FUN_00563390`).
    /// A Command order (`0x260..0x263`, `character-command.md`) on a
    /// character of the player's.
    OrderCommand {
        character: rebellion_core::ids::CharacterKey,
        order: u16,
    },
    OrderAssault {
        fleets: Vec<FleetKey>,
        system: SystemKey,
    },

    // ── Play-testing (command palette) ────────────────────────────────
    /// Advance simulation by N ticks immediately.
    AdvanceTicks(u64),
    /// Set one of the original Game Speed menu's choices.
    SetGameSpeed(rebellion_core::tick::GameSpeed),
    /// Toggle AI control for both factions.
    ToggleDualAI,
    /// Immediately evaluate victory conditions.
    ForceVictoryCheck,
    /// Remove fog of war from all systems.
    RevealAllFog,
    /// port: the debug "see everything" mode: every window shows both
    /// sides' live objects, past the fog and the side's views
    /// (`ObjectViews::see_everything`). Not saved.
    ToggleSeeEverything,
    /// Export message log to file.
    ExportGameLog,
    /// Display current game statistics overlay.
    ShowGameStats,
    /// List all currently active missions.
    ListActiveMissions,
    /// List all fleet positions and compositions.
    ListActiveFleets,
    /// Show count of triggered events.
    ShowEventCount,
}

impl PanelAction {
    /// Whether this is a player's game order (the `0x2xx` order objects
    /// `FUN_00487740` submits through `FUN_0048aa90` -> `FUN_0041cee0`),
    /// which acts only in the frame step, not when given. Window, save and
    /// play-test actions act at once.
    #[must_use]
    pub const fn is_order(&self) -> bool {
        matches!(
            self,
            Self::Retire(_)
                | Self::JoinFleet { .. }
                | Self::SetDestination { .. }
                | Self::SetFleetDestination { .. }
                | Self::Rename { .. }
                | Self::RenameProduct { .. }
                | Self::CreateFleet { .. }
                | Self::MoveShips { .. }
                | Self::DispatchFleet { .. }
                | Self::MoveRegiment { .. }
                | Self::MoveHeadquarters { .. }
                | Self::MoveCharacter { .. }
                | Self::MoveProducts { .. }
                | Self::MoveSpecialForce { .. }
                | Self::MoveMembers { .. }
                | Self::MoveSquadron { .. }
                | Self::StopProduction { .. }
                | Self::ToggleReserved { .. }
                | Self::BuildProduction { .. }
                | Self::DispatchMission { .. }
                | Self::OrderBombardment { .. }
                | Self::OrderCommand { .. }
                | Self::OrderAssault { .. }
        )
    }
}

/// The actions to apply this frame. While a pause is `holding` the clock,
/// the orders among `actions` wait in `held`; once it runs, the held orders
/// come first, in the order given.
///
/// A player's order is queued (`FUN_004360f0` -> `FUN_00536fe0`), passed to
/// the scheduler's list `+0xa4` in phase 3 (`Class_0065b070_vt2c` ->
/// `FUN_0041db80` -> `FUN_0051d880`) and executed in phase 5
/// (`FUN_0051df30`, `0x51dfee`..`0x51e011`: `FUN_0051dd20` -> its slot
/// `+0x20`). A held pause stops the frame step at phase 2 (`FUN_0041e300`),
/// so nothing given meanwhile acts, or posts its notices (`FUN_0051eaf0`),
/// until the game runs (`ghidra/notes/pause-order-hold.md`).
pub fn admit_orders(
    held: &mut Vec<PanelAction>,
    holding: bool,
    actions: Vec<PanelAction>,
) -> Vec<PanelAction> {
    if holding {
        let (orders, now): (Vec<_>, Vec<_>) = actions.into_iter().partition(PanelAction::is_order);
        held.extend(orders);
        return now;
    }
    let mut now = std::mem::take(held);
    now.extend(actions);
    now
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hq() -> PanelAction {
        PanelAction::MoveHeadquarters {
            destination: SystemKey::default(),
        }
    }

    fn stop() -> PanelAction {
        PanelAction::ToggleReserved {
            system: SystemKey::default(),
            area: None,
        }
    }

    fn names(actions: &[PanelAction]) -> Vec<&'static str> {
        actions
            .iter()
            .map(|action| match action {
                PanelAction::MoveHeadquarters { .. } => "hq",
                PanelAction::ToggleReserved { .. } => "reserved",
                PanelAction::ShowEventCount => "events",
                PanelAction::ReloadMods => "mods",
                _ => "other",
            })
            .collect()
    }

    #[test]
    fn orders_given_while_a_pause_holds_wait_until_the_game_runs() {
        // FUN_0041e300 holds FUN_0041dff0 at phase 2; orders act in phase 5
        // (FUN_0051df30 -> FUN_0051dd20).
        let mut held = Vec::new();
        let now = admit_orders(
            &mut held,
            true,
            vec![hq(), PanelAction::ShowEventCount, stop()],
        );
        assert_eq!(names(&now), vec!["events"]);
        assert_eq!(names(&held), vec!["hq", "reserved"]);

        // Still held: nothing more acts.
        assert!(admit_orders(&mut held, true, Vec::new()).is_empty());
        assert_eq!(held.len(), 2);

        let now = admit_orders(&mut held, false, vec![PanelAction::ReloadMods]);
        assert_eq!(names(&now), vec!["hq", "reserved", "mods"]);
        assert!(held.is_empty());
    }

    #[test]
    fn orders_given_while_the_game_runs_act_at_once() {
        let mut held = Vec::new();
        let now = admit_orders(&mut held, false, vec![hq(), stop()]);
        assert_eq!(names(&now), vec!["hq", "reserved"]);
        assert!(held.is_empty());
    }

    #[test]
    fn window_save_and_speed_actions_are_not_orders() {
        assert!(hq().is_order());
        assert!(stop().is_order());
        assert!(!PanelAction::ShowEventCount.is_order());
        assert!(!PanelAction::FocusFleetSystem(SystemKey::default()).is_order());
        assert!(!PanelAction::SaveGame {
            slot: 0,
            name: String::new()
        }
        .is_order());
        assert!(!PanelAction::SetGameSpeed(rebellion_core::tick::GameSpeed::Fast).is_order());
    }
}
