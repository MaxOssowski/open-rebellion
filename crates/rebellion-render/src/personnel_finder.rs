//! The Personnel Finder (window type `0x17`, `FUN_0042a4d0`;
//! `ghidra/notes/personnel-finder.md`).
//!
//! A 470 by 330 window the cockpit's Personnel Finder control or F4
//! opens. Two side tabs (Alliance, Imperial) choose the faction; two view
//! tabs choose between a Characters list and a Special Forces list.
//!
//! Characters view: rows are "Name - Location (state) (rank)"
//! (`FUN_00465bb0`). Special Forces view: rows per system with four
//! count cells by class (`FUN_00465540`). Typing a name picks the best
//! match (`FUN_00609650`); Display, Enter or a double click opens the
//! appropriate window (`FUN_00429440`).

use egui_macroquad::egui;
use rebellion_core::dat::Faction;
use rebellion_core::fog::FogState;
use rebellion_core::ids::{CharacterKey, FleetKey, SpecialForceKey, SystemKey};
use rebellion_core::world::GameWorld;

use crate::bmp_cache::{BmpCache, DllSource};
use crate::cockpit::{CockpitFaction, CockpitLayout};
use crate::fleet_window::paint_native;
use crate::mission_dialog::{button, galaxy_centered_rect};
use crate::system_window::{fleet_label, logical_rect, opposing_contents_visible, rect_contains};

pub const PERSONNEL_FINDER_WIDTH: f32 = 470.0;
pub const PERSONNEL_FINDER_HEIGHT: f32 = 330.0;

// STRATEGY bitmaps by side (FUN_004637a0).
const BACKGROUND: [u32; 2] = [10335, 10336];
const RAIL: [u32; 2] = [10586, 10590];
const RAIL_AT: (f32, f32) = (412.0, 0.0);
const PANEL_AT: (f32, f32) = (12.0, 13.0);

/// Panel bitmaps indexed by `[view][side]`. `FUN_004637a0` lays 10538,
/// 10539, 10536 and 10537 on the window's panels 2, 3, 4 and 5
/// (`0x463b78..0x463bdf`); `FUN_00464e10` shows panel 2 or 3 (Alliance or
/// Empire) for view 2, SpecForces, and 4 or 5 for view 3, Characters. 10538
/// carries the four SpecForces column icons.
const PANELS: [[u32; 2]; 2] = [
    [10536, 10537], // Characters view: Alliance, Imperial
    [10538, 10539], // SpecForces view: Alliance, Imperial
];

/// The side tab (FUN_004637a0's control 100, TEXTSTRA 0x1893/0x1894).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PersonnelTab {
    #[default]
    Alliance,
    Imperial,
}

impl PersonnelTab {
    const ALL: [Self; 2] = [Self::Alliance, Self::Imperial];

    /// The tab's tooltip (`FUN_004637a0`): TEXTSTRA 0x1893 "Alliance
    /// Personnel", 0x1894 "Imperial Personnel".
    const fn tooltip_string(self) -> u16 {
        match self {
            Self::Alliance => 0x1893,
            Self::Imperial => 0x1894,
        }
    }

    fn lists(self, is_alliance: bool) -> bool {
        match self {
            Self::Alliance => is_alliance,
            Self::Imperial => !is_alliance,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Alliance => "Alliance Personnel",
            Self::Imperial => "Imperial Personnel",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Alliance => 0,
            Self::Imperial => 1,
        }
    }
}

/// The view tab (FUN_004637a0's control 0x6e): value 3 is Characters
/// (TEXTSTRA `0x1899`), value 2 SpecForces (`0x1898`); `FUN_00464e10` lists
/// characters for 3 and special forces for 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PersonnelView {
    #[default]
    Characters,
    SpecForces,
}

impl PersonnelView {
    const ALL: [Self; 2] = [Self::SpecForces, Self::Characters];

    /// The tab's tooltip (`FUN_004637a0`): TEXTSTRA 0x1898 "SpecForces",
    /// 0x1899 "Characters".
    const fn tooltip_string(self) -> u16 {
        match self {
            Self::SpecForces => 0x1898,
            Self::Characters => 0x1899,
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Characters => 0,
            Self::SpecForces => 1,
        }
    }

    /// The view button's offset in its group (`FUN_004637a0`): Characters at
    /// the top, SpecForces `0x36` lower.
    const fn button_offset(self) -> f32 {
        match self {
            Self::Characters => 0.0,
            Self::SpecForces => 54.0,
        }
    }

    /// The tab label y (`FUN_00464e10`): `0x77` (119) for view 2,
    /// SpecForces, else `0x73` (115).
    const fn label_y(self) -> f32 {
        match self {
            Self::Characters => 115.0,
            Self::SpecForces => 119.0,
        }
    }
}

/// The side's Close and Display controls (FUN_004637a0).
struct SideControls {
    close: Control,
    display: Control,
    /// View tab group origin and individual button size.
    view_origin: (f32, f32),
    view_button_size: (f32, f32),
}

#[derive(Debug, Clone, Copy)]
struct Control {
    rect: (f32, f32, f32, f32),
    art: (u32, u32),
}

const CONTROLS: [SideControls; 2] = [
    // Alliance (side 1): FUN_004637a0.
    SideControls {
        close: Control {
            rect: (423.0, 25.0, 32.0, 31.0),
            art: (10514, 10515),
        },
        display: Control {
            rect: (423.0, 93.0, 32.0, 31.0),
            art: (10518, 10519),
        },
        view_origin: (423.0, 147.0),
        view_button_size: (32.0, 31.0),
    },
    // Empire (side 2).
    SideControls {
        close: Control {
            rect: (426.0, 21.0, 44.0, 41.0),
            art: (10516, 10517),
        },
        display: Control {
            rect: (426.0, 89.0, 42.0, 41.0),
            art: (10520, 10521),
        },
        view_origin: (426.0, 143.0),
        view_button_size: (42.0, 41.0),
    },
];

/// View tab art: `[side][view]` (FUN_004637a0). Characters (value 3, at the
/// group's top): Alliance 10542/10543, Empire 10544/10545, a face.
/// SpecForces (value 2, `0x36` lower): Alliance 10546/10547, Empire
/// 10548/10549, two soldiers.
const VIEW_TAB_ART: [[(u32, u32); 2]; 2] = [
    // Alliance: [Characters, SpecForces]
    [(10542, 10543), (10546, 10547)],
    // Empire: [Characters, SpecForces]
    [(10544, 10545), (10548, 10549)],
];

/// Side tabs (group 100) at (36, 72): same as troop/fleet finder.
const SIDE_TABS: [Control; 2] = [
    Control {
        rect: (36.0, 72.0, 49.0, 41.0),
        art: (10502, 10503),
    },
    Control {
        rect: (88.0, 72.0, 49.0, 41.0),
        art: (10505, 10506),
    },
];

const TITLE_AT: (f32, f32) = (36.0, 14.0);
/// The list's text x (`+0xe8 = 5`) and unchosen colour (`0x787878`).
const ROW_TEXT_X: f32 = 5.0;
const ROW_COLOR: egui::Color32 = egui::Color32::from_rgb(0x78, 0x78, 0x78);
const NAME_LABEL_AT: (f32, f32) = (36.0, 48.0);
const NAME_BOX: (f32, f32, f32, f32) = (143.0, 45.0, 250.0, 18.0);
const LIST: (f32, f32, f32, f32) = (37.0, 143.0, 349.0, 161.0);
const ROW_HEIGHT: f32 = 20.0;

/// Four Special Forces DatIds per side (FUN_00465540).
const ALLIANCE_SF_CLASSES: [u32; 4] = [0x3c00_0004, 0x3c00_0002, 0x3c00_0001, 0x3c00_0003];
const EMPIRE_SF_CLASSES: [u32; 4] = [0x3c00_0007, 0x3c00_0006, 0x3c00_0008, 0x3c00_0005];

fn sf_classes(tab: PersonnelTab) -> &'static [u32; 4] {
    match tab {
        PersonnelTab::Alliance => &ALLIANCE_SF_CLASSES,
        PersonnelTab::Imperial => &EMPIRE_SF_CLASSES,
    }
}

/// One count cell in a SpecForces row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SfCount {
    pub count: u32,
    /// port: always true (no usable bit in the port).
    pub all_usable: bool,
}

/// Where the player's copy of a character puts it, as `FUN_00429440` reads
/// the copy's container (`FUN_004025b0`) for a Display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyPlace {
    /// Aboard a ship of a copied fleet: that system's Fleet window with the
    /// copy chosen (kind 4).
    Fleet(crate::fleet_window::FleetWindowEntry),
    /// On a copied system: its Defenses window (kind 10).
    System(SystemKey),
    /// On a copied system on a mission that is not hidden: its Missions
    /// window (kind 11). port: the window opens without choosing the
    /// mission, as the copy keeps no mission key.
    Missions(SystemKey),
    /// In no ship or system: nothing opens, though the Finder closes
    /// (`FUN_00429440` leaves its target system null and returns 1;
    /// `FUN_00464890` case `0xc9` closes after it).
    Nowhere,
}

/// One list row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersonnelFinderRow {
    /// A character: "Name - Location (state)" (FUN_00465bb0).
    Character {
        key: CharacterKey,
        name: String,
        location: String,
        state: String,
        system: Option<SystemKey>,
        /// Where the player's copy of an other-side character puts it, when
        /// the row shows that copy; none for a live row.
        copy: Option<CopyPlace>,
    },
    /// A system (or fleet) with SpecForces counts (FUN_00465540).
    SpecForces {
        name: String,
        system: SystemKey,
        fleet: Option<FleetKey>,
        counts: [SfCount; 4],
    },
}

impl PersonnelFinderRow {
    fn display_name(&self) -> &str {
        match self {
            Self::Character { name, .. } => name,
            Self::SpecForces { name, .. } => name,
        }
    }
}

/// The text a chosen row copies into the name box (case `0xca`, `0x29b` of
/// `FUN_00464890`): for a character, its record's name (`FUN_00583c40`,
/// `+0x34`), not the row's place and state words (A0
/// 2026-11-17-edited-saves-6 d-02 bab3f3b8d33e: the box reads "Luke
/// Skywalker" for the row "Luke Skywalker - Dagobah"; d-05 33acd757c002:
/// "Han Solo" for "Han Solo - Sluis Van ( Captured )"); for a SpecForces
/// row, its system or fleet name.
///
/// The case copies a name only for an existing object (`FUN_004f2d10`'s
/// record with `+0x50` bit 6: created and not destroyed, `0x004f7bc0`); a
/// killed or retired character's row is chosen and the box keeps its
/// text: `None` (A0 2026-11-18-edited-saves-9, an unedited Empire game:
/// d-04 5b7a43b193d5 and d-08 2f1d872c5a2e, "Ozzel - Retired" chosen and
/// the box empty).
#[must_use]
pub fn box_name(row: &PersonnelFinderRow, world: &GameWorld) -> Option<String> {
    match row {
        PersonnelFinderRow::Character { key, name, .. } => match world.characters.get(*key) {
            Some(character) if character.is_killed => None,
            Some(character) => Some(character.name.clone()),
            None => Some(name.clone()),
        },
        PersonnelFinderRow::SpecForces { name, .. } => Some(name.clone()),
    }
}

/// A typed name chooses the first row it best begins, or the first row
/// (FUN_00609650, [`crate::list_prefix::best_prefix_row`]).
#[must_use]
pub fn prefix_choice(rows: &[PersonnelFinderRow], text: &str) -> Option<usize> {
    crate::list_prefix::best_prefix_row(rows.iter().map(|row| row.display_name()), text)
}

/// Count special forces of each class among the given keys.
fn count_sf(world: &GameWorld, keys: &[SpecialForceKey], tab: PersonnelTab) -> [SfCount; 4] {
    let classes = sf_classes(tab);
    let mut counts = [SfCount {
        count: 0,
        all_usable: true,
    }; 4];
    for &key in keys {
        let Some(sf) = world.special_forces.get(key) else {
            continue;
        };
        if !tab.lists(sf.is_alliance) {
            continue;
        }
        let raw = sf.class_dat_id.raw();
        for (i, &class_raw) in classes.iter().enumerate() {
            if raw == class_raw {
                counts[i].count += 1;
                // FUN_00465540: +0x50 bit 0 clear. port: a unit moves as a
                // Move mission, so on a mission covers en route too.
                if sf.on_mission {
                    counts[i].all_usable = false;
                }
                break;
            }
        }
    }
    counts
}

/// A character's state word (`FUN_00465bb0`): Captured (`0x8746`) when held
/// (`+0xac` bit 0), else On Mission (`0x8745`), else Injured (`0x8747`,
/// `+0x94`), else Enroute (`0x8744`); the first that applies. port: a
/// character off the map, in no fleet and in no abode is the one travelling.
fn character_state(character: &rebellion_core::world::Character) -> Option<&'static str> {
    if character.is_captive {
        Some("Captured")
    } else if character.on_mission {
        Some("On Mission")
    } else if character.injury != 0 {
        Some("Injured")
    } else if character.current_system.is_none()
        && character.current_fleet.is_none()
        && character.current_abode.is_none()
    {
        Some("Enroute")
    } else {
        None
    }
}

/// A character's command word (`FUN_00465bb0`, `+0x96`): 1 Commander
/// (`0x8802`), 2 Admiral (`0x8801`), 3 General (`0x8800`).
fn command_word(command: rebellion_core::character_command::CommandKind) -> Option<&'static str> {
    use rebellion_core::character_command::CommandKind;
    match command {
        CommandKind::None => None,
        CommandKind::Commander => Some("Commander"),
        CommandKind::Admiral => Some("Admiral"),
        CommandKind::General => Some("General"),
    }
}

/// The window a character's Display opens (`FUN_00429440`, family `0x30`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterTarget {
    /// Aboard: its fleet's Fleet window (kind 4).
    Fleet(FleetKey),
    /// At a system on a mission that is not hidden (`FUN_00520b70` is 0):
    /// the Missions window (kind 11).
    Missions,
    /// At a system otherwise: the Defenses window (kind 10).
    Defenses,
}

/// Which window `character`'s Display opens (`FUN_00429440`): its mission
/// key `+0x68` names a mission (`FUN_004f3000`); a hidden one opens the
/// Defenses window, where its member still lists.
#[must_use]
pub fn character_target(character: &rebellion_core::world::Character) -> CharacterTarget {
    match character.current_fleet {
        Some(fleet) => CharacterTarget::Fleet(fleet),
        None if character.on_mission && !character.on_hidden_mission => CharacterTarget::Missions,
        None => CharacterTarget::Defenses,
    }
}

/// A Characters row's text (`FUN_00465bb0`): the name and " - " (`0x1897`),
/// then:
/// - for a destroyed character (`+0x50` bit 3), "Killed" (`0x8798`), or
///   "Retired" (`0x8799`) when its destruction reason (`+0x40`) is
///   `0x14..0x16`, as a Retire order's (`FUN_00534c00`) is;
/// - in an abode (container family `0x98..0x9f`), the abode's name alone;
/// - with no container, "Location Unknown" (`0x1895`) alone: the function
///   jumps past the words (`0x465cb5` -> `0x46606f`);
/// - otherwise its container's name, the state word, then the command
///   word, each in " ( " and " ) " (`DAT_006a8790`, `DAT_006a878c`).
///
/// A character aboard names the ship it rides (`Character::ship`), as its
/// container (A0 mission-reports/ Empire game day 4: "Darth Vader -
/// Carrack Light Cruiser 1", "Piett - Galleon 1"; personnel-captured/
/// finder-day628: "Jan Dodonna - Punisher ( Captured )").
/// port: one not yet seated names its fleet.
fn character_text(
    world: &GameWorld,
    character: &rebellion_core::world::Character,
    destination: Option<rebellion_core::ids::SystemKey>,
) -> String {
    if character.is_killed {
        let word = if character.is_retired {
            "Retired"
        } else {
            "Killed"
        };
        return format!("{} - {word}", character.name);
    }
    if let Some(abode) = character
        .current_abode
        .and_then(|abode| world.abodes.get(abode))
    {
        return format!("{} - {}", character.name, abode.name);
    }
    let location = character
        .current_fleet
        .and_then(|fleet| {
            let ship = character.ship.and_then(|id| {
                let index = world
                    .fleets
                    .get(fleet)?
                    .capital_ships
                    .iter()
                    .position(|ship| ship.id == id)?;
                world.ship_name(fleet, index)
            });
            ship.map(str::to_owned)
                .or_else(|| fleet_label(world, fleet))
        })
        .or_else(|| {
            character
                .current_system
                // A travelling character's container is already its
                // destination (A0 specforces/00: "Adar Tallon - Mon
                // Calamari ( Enroute )"). port: the trip keeps it apart.
                .or(destination)
                .and_then(|system| world.systems.get(system))
                .map(|system| system.name.clone())
        });
    // No container: 0x1895 alone, past the state and command words
    // (0x465cb5 -> 0x46606f).
    let Some(location) = location else {
        return format!("{} - Location Unknown", character.name);
    };
    let mut text = format!("{} - {location}", character.name);
    for word in [character_state(character), command_word(character.command)]
        .into_iter()
        .flatten()
    {
        text.push_str(&format!(" ( {word} ) "));
    }
    text
}

/// Build the Characters view rows for `tab` (FUN_00464e10 view 3,
/// FUN_00465bb0).
#[must_use]
pub fn character_rows(
    world: &GameWorld,
    fog: &FogState,
    player: Faction,
    tab: PersonnelTab,
) -> Vec<PersonnelFinderRow> {
    character_rows_travelling(world, fog, player, tab, &[])
}

/// [`character_rows`], naming each travelling character's destination.
#[must_use]
///
/// The list holds the player's side's views (`FUN_004f3d50`, `+0x50` bit
/// 1). A0 personnel-other-side/test-day8-imperial-tab-empty (an unedited
/// Alliance game, day 8) lists no Imperial character. The port lists the
/// other side's character where the player sees the other side's objects
/// live (`opposing_contents_visible`), else where the player's frozen copy
/// of a system holds it (`SystemView::characters`, `FUN_004f9a60`), at that
/// system. Where the player holds a copy of the character
/// (`object_views::CharacterCopy`), that copy names its place instead: the
/// copied ship or system, or "Location Unknown".
pub fn character_rows_travelling(
    world: &GameWorld,
    fog: &FogState,
    player: Faction,
    tab: PersonnelTab,
    destinations: &[(rebellion_core::ids::CharacterKey, rebellion_core::ids::SystemKey)],
) -> Vec<PersonnelFinderRow> {
    let player_is_alliance = player == Faction::Alliance;
    // The characters the player's frozen system copies hold, with the
    // copied system.
    let copied: std::collections::HashMap<rebellion_core::ids::DatId, SystemKey> = world
        .systems
        .iter()
        .filter_map(|(system, value)| {
            let view = world.object_views.system(player, value.dat_id)?;
            (!view.live).then_some((system, view))
        })
        .flat_map(|(system, view)| view.characters.iter().map(move |id| (*id, system)))
        .collect();
    let mut rows = Vec::new();
    for (key, character) in &world.characters {
        if !character.recruited {
            continue;
        }
        if !tab.lists(character.is_alliance) {
            continue;
        }
        let mut at = character.current_system;
        // The text and place of the player's copy of the character, when it
        // shows one.
        let mut copy_text = None;
        let mut copy_place = None;
        // The player's own captive carries its captor's side (FUN_004eec60),
        // so the player reads its copy, frozen where the player last saw it
        // (object_views::record_own_captives; A0 2026-11-18-edited-saves-7
        // b-19 e4f53d49b065: "Luke Skywalker - Jabba's Palace" after the
        // Pickup). Without a copy (none taken yet) the row reads it live.
        let own_captive = character.is_alliance == player_is_alliance
            && character.is_captive
            && !character.is_killed;
        let other_unseen = character.is_alliance != player_is_alliance
            && !character
                .current_system
                .or_else(|| {
                    character
                        .current_fleet
                        .and_then(|fleet| world.fleets.get(fleet))
                        .map(|fleet| fleet.location)
                })
                .is_some_and(|system| opposing_contents_visible(world, fog, player, system));
        if own_captive || other_unseen {
            if let Some(copy) = world.object_views.character(player, character.dat_id) {
                // FUN_00465bb0 on the side's copy: its container's name, a
                // copied ship or a system, else 0x1895 "Location Unknown"
                // (A0 2026-11-14-stale-views 1x-personnelfinder-empire-d85
                // d57ed845, d91 1b8818d8).
                at = copy.system.and_then(|id| {
                    world
                        .systems
                        .iter()
                        .find(|(_, value)| value.dat_id == id)
                        .map(|(system, _)| system)
                });
                let place = copy
                    .ship
                    .clone()
                    .or_else(|| at.map(|system| world.systems[system].name.clone()))
                    .unwrap_or_else(|| "Location Unknown".to_owned());
                // The copy's state word, On Mission before Enroute, then the
                // command word its +0x96 held when it last synced
                // (FUN_004ee350 -> FUN_004f04f0), only with a place
                // (FUN_00465bb0 jumps past the words without one; an abode
                // copy has none). port: an other side's copy keeps no
                // Captured or Injured word.
                let state = world.object_views.character_state(player, character.dat_id);
                // An own captive's copy took +0xac bit 0 before it froze
                // (Character_SetCaptured FUN_004ee800 syncs it first).
                let word = if own_captive {
                    Some("Captured")
                } else if state.on_mission {
                    Some("On Mission")
                } else if state.enroute {
                    Some("Enroute")
                } else {
                    None
                };
                let command = command_word(
                    world
                        .object_views
                        .character_command(player, character.dat_id),
                );
                let mut text = format!("{} - {place}", character.name);
                if at.is_some() {
                    for word in [word, command].into_iter().flatten() {
                        text.push_str(&format!(" ( {word} ) "));
                    }
                }
                copy_text = Some(text);
                copy_place = Some(match at {
                    None => CopyPlace::Nowhere,
                    Some(system) => copied_fleet(world, player, system, character.dat_id)
                        .map_or_else(
                            || {
                                // FUN_00429440's system branch: the copy's
                                // mission, unless hidden (FUN_00520b70).
                                if state.on_mission && !state.hidden_mission {
                                    CopyPlace::Missions(system)
                                } else {
                                    CopyPlace::System(system)
                                }
                            },
                            CopyPlace::Fleet,
                        ),
                });
            } else if other_unseen {
                let Some(system) = copied.get(&character.dat_id) else {
                    continue;
                };
                at = Some(*system);
            }
        }
        let location = at
            .and_then(|sys| world.systems.get(sys))
            .map_or_else(String::new, |sys| sys.name.clone());
        let state = if copy_text.is_some() {
            String::new()
        } else {
            character_state(character).unwrap_or_default().to_owned()
        };
        let destination = destinations
            .iter()
            .find(|(traveller, _)| *traveller == key)
            .map(|&(_, system)| system);
        let display = copy_text.unwrap_or_else(|| character_text(world, character, destination));
        rows.push(PersonnelFinderRow::Character {
            key,
            name: display,
            location,
            state,
            system: at,
            copy: copy_place,
        });
    }
    rows.sort_by_cached_key(|row| row.display_name().to_ascii_lowercase());
    rows
}

/// The player's copied fleet at `system` that holds the character `id`, as
/// a Fleet window entry (`FleetWindowEntry::Copy`), while the player sees
/// `system` through its frozen copy.
fn copied_fleet(
    world: &GameWorld,
    player: Faction,
    system: SystemKey,
    id: rebellion_core::ids::DatId,
) -> Option<crate::fleet_window::FleetWindowEntry> {
    let alliance = player == Faction::Alliance;
    let details = world
        .object_views
        .system_details(player, world.systems.get(system)?.dat_id)?;
    let fleet = details
        .fleets
        .iter()
        .filter(|copy| copy.is_alliance != alliance)
        .position(|copy| copy.characters.contains(&id))?;
    Some(crate::fleet_window::FleetWindowEntry::Copy {
        alliance,
        system,
        fleet,
    })
}

/// Build the SpecForces view rows for `tab` (FUN_00464e10 view 2,
/// FUN_00465540).
///
/// The list holds the player's side's views, as the Characters view's
/// (`FUN_004f3d50`). A0 2026-11-13-gid/E2 (an Alliance game, day 244)
/// lists no Imperial special force. As for the characters, the port lists
/// the other side's units where the player sees the other side's objects
/// live (`opposing_contents_visible`), else from its frozen copy of the
/// system (`SystemView::special_forces`).
#[must_use]
pub fn specforces_rows(
    world: &GameWorld,
    fog: &FogState,
    player: Faction,
    tab: PersonnelTab,
) -> Vec<PersonnelFinderRow> {
    let mut rows = Vec::new();
    let others = (player == Faction::Alliance) != (tab == PersonnelTab::Alliance);

    for (system_key, system) in &world.systems {
        if others && !opposing_contents_visible(world, fog, player, system_key) {
            if let crate::viewed_contents::SystemSight::Frozen(view) =
                crate::viewed_contents::system_sight(world, fog, player, system_key)
            {
                let classes = sf_classes(tab);
                let mut counts = [SfCount {
                    count: 0,
                    all_usable: true,
                }; 4];
                let mut any = false;
                for (class, is_alliance) in &view.special_forces {
                    if !tab.lists(*is_alliance) {
                        continue;
                    }
                    any = true;
                    if let Some(i) = classes.iter().position(|raw| *raw == class.raw()) {
                        counts[i].count += 1;
                    }
                }
                if any {
                    rows.push(PersonnelFinderRow::SpecForces {
                        name: system.name.clone(),
                        system: system_key,
                        fleet: None,
                        counts,
                    });
                }
            }
            continue;
        }
        // Direct SF units at this system.
        let ground: Vec<SpecialForceKey> = system
            .special_forces
            .iter()
            .copied()
            .filter(|&key| {
                world
                    .special_forces
                    .get(key)
                    .is_some_and(|sf| tab.lists(sf.is_alliance))
            })
            .collect();
        if !ground.is_empty() {
            let counts = count_sf(world, &ground, tab);
            rows.push(PersonnelFinderRow::SpecForces {
                name: system.name.clone(),
                system: system_key,
                fleet: None,
                counts,
            });
        }

        // FUN_00464e10: then each fleet of the tab's side at the system
        // (FUN_004ffef0) whose ships carry special forces is a row of its
        // own, counted by FUN_00465540 over the fleet.
        for &fleet_key in &system.fleets {
            let Some(fleet) = world.fleets.get(fleet_key) else {
                continue;
            };
            if !tab.lists(fleet.is_alliance) || fleet.special_forces.is_empty() {
                continue;
            }
            rows.push(PersonnelFinderRow::SpecForces {
                name: crate::system_window::fleet_label(world, fleet_key).unwrap_or_default(),
                system: system_key,
                fleet: Some(fleet_key),
                counts: count_sf(world, &fleet.special_forces, tab),
            });
        }
    }

    rows.sort_by_cached_key(|row| row.display_name().to_ascii_lowercase());
    rows
}

/// Build rows for the current view.
#[must_use]
pub fn rows(
    world: &GameWorld,
    fog: &FogState,
    player: Faction,
    tab: PersonnelTab,
    view: PersonnelView,
) -> Vec<PersonnelFinderRow> {
    match view {
        PersonnelView::Characters => character_rows(world, fog, player, tab),
        PersonnelView::SpecForces => specforces_rows(world, fog, player, tab),
    }
}

/// What the Personnel Finder asks of the galaxy view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonnelFinderAction {
    /// Character row: open the character window (FUN_00429440 family 0x30).
    OpenCharacter {
        character: CharacterKey,
        system: Option<SystemKey>,
    },
    /// SpecForces system row: its system's windows for the tab's first
    /// special force there. A0 specforces/02, 03: Display on Mon Calamari
    /// opens its Defenses window, on Tralus the window of the team's running
    /// mission, as `FUN_00429440` routes a special force (family `0x3c`).
    /// hyp: the row's unit is the first of the system's list.
    OpenSystem {
        system: SystemKey,
        unit: Option<SpecialForceKey>,
    },
    /// SpecForces fleet row: open the Fleet window.
    OpenFleet { system: SystemKey, fleet: FleetKey },
    /// A copied character row: what its copy's container opens
    /// ([`CopyPlace`]), never the live character's place.
    OpenCopy(CopyPlace),
}

#[derive(Debug, Clone)]
struct OpenFinder {
    faction: CockpitFaction,
    tab: PersonnelTab,
    view: PersonnelView,
    name: String,
    /// The list's chosen row, drawn white.
    chosen: Option<usize>,
    /// The object Display and Enter open, `+0x178`: set by a typed name
    /// (`0x408`, `FUN_00464c20`), a click or a key step (`0x29b`, case
    /// `0xca` of `FUN_00464890`). A side or view tab rebuilds the list
    /// (`FUN_00464e10`), so no row is chosen, but keeps `+0x178`, and no
    /// prefix match runs there (cases `100` and `0x6e`). port: with the
    /// side tab it was chosen on, which picks a system row's unit.
    target: Option<(PersonnelFinderRow, PersonnelTab)>,
    /// The list's first visible pixel (it scrolls by pixels,
    /// `FUN_0060a490`).
    scroll: usize,
    /// A press held on the list's scroll bar.
    scroll_hold: Option<crate::list_scroll_bar::Hold>,
    focus_name: bool,
    last_clicked: Option<(usize, f64)>,
}

/// The Personnel Finder, one at most (FUN_0042a4d0).
#[derive(Debug, Clone, Default)]
pub struct PersonnelFinderState {
    window: Option<OpenFinder>,
    /// Each travelling character's destination, which names it in its row
    /// ([`PersonnelFinderState::set_destinations`]).
    destinations: Vec<(rebellion_core::ids::CharacterKey, rebellion_core::ids::SystemKey)>,
}

/// The list's height and rows in pixels, its bar's width (RCDATA 10653,
/// `FUN_004637a0`: `FUN_0060a490(.., list, 7, 0x299d, ..)`) and arrow step
/// (`FUN_00607ea0`'s argument 10).
const LIST_PIXELS: usize = 161;
const ROW_PIXELS: usize = 20;
const BAR_WIDTH: f32 = 12.0;
const LIST_STEP: f32 = 10.0;

/// The scroll that shows `row` whole, moving the list as little as
/// possible, within the range of `rows` rows.
fn scrolled_to(scroll: usize, row: usize, rows: usize) -> usize {
    let top = row * ROW_PIXELS;
    let bottom = top + ROW_PIXELS;
    if top < scroll {
        top
    } else if bottom > scroll + LIST_PIXELS {
        (bottom - LIST_PIXELS).min(max_scroll(rows))
    } else {
        scroll
    }
}

/// The list's scroll range for `rows` rows, in pixels
/// ([`crate::list_scroll_bar::max_first`]).
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "A few hundred rows of 20 pixels."
)]
fn max_scroll(rows: usize) -> usize {
    crate::list_scroll_bar::max_first((rows * ROW_PIXELS) as f32, LIST.3) as usize
}

const fn side(faction: CockpitFaction) -> usize {
    match faction {
        CockpitFaction::Alliance => 0,
        CockpitFaction::Empire => 1,
    }
}

const fn player_side(faction: CockpitFaction) -> Faction {
    match faction {
        CockpitFaction::Alliance => Faction::Alliance,
        CockpitFaction::Empire => Faction::Empire,
    }
}

fn window_rect(layout: CockpitLayout) -> egui::Rect {
    galaxy_centered_rect(layout, PERSONNEL_FINDER_WIDTH, PERSONNEL_FINDER_HEIGHT)
}

fn at(frame: egui::Rect, scale: f32, (x, y, w, h): (f32, f32, f32, f32)) -> egui::Rect {
    logical_rect(frame, scale, x, y, w, h)
}

/// Cell colours for SpecForces counts (FUN_00465540): same as the troop
/// finder. Alliance tab → white on red; Imperial tab → black on green.
fn sf_cell_colours(tab: PersonnelTab, all_usable: bool) -> (egui::Color32, egui::Color32) {
    // FUN_00465540: a column holding a unit whose +0x50 bit 0 is clear
    // (en route or on a mission) takes flag 4 and the side's dark brush,
    // as the Troop Finder's (FUN_0046d8d0 brushes 3 and 4). A0
    // specforces/01: Mon Calamari's, Tralus's and Yavin's boxes dark red.
    match (tab, all_usable) {
        (PersonnelTab::Alliance, true) => (egui::Color32::from_rgb(255, 0, 0), egui::Color32::WHITE),
        (PersonnelTab::Alliance, false) => (egui::Color32::from_rgb(128, 0, 0), egui::Color32::WHITE),
        (PersonnelTab::Imperial, true) => (egui::Color32::from_rgb(0, 255, 0), egui::Color32::BLACK),
        (PersonnelTab::Imperial, false) => (egui::Color32::from_rgb(0, 128, 0), egui::Color32::BLACK),
    }
}

impl PersonnelFinderState {
    /// The open window's side tab and the centre of each side tab's button
    /// on the screen, for the interface fixtures.
    #[must_use]
    pub fn shown_side_tabs(&self, layout: CockpitLayout) -> Option<(bool, [egui::Pos2; 2])> {
        let window = self.window.as_ref()?;
        let frame = window_rect(layout);
        Some((
            window.tab == PersonnelTab::Alliance,
            SIDE_TABS.map(|control| at(frame, layout.scale, control.rect).center()),
        ))
    }

    /// The open window's Characters rows as shown, for the interface
    /// fixtures; `None` when closed or on the SpecForces view.
    #[must_use]
    pub fn shown_character_rows(&self, world: &GameWorld, fog: &FogState) -> Option<Vec<String>> {
        let window = self.window.as_ref()?;
        (window.view == PersonnelView::Characters).then(|| {
            character_rows_travelling(
                world,
                fog,
                player_side(window.faction),
                window.tab,
                &self.destinations,
            )
            .iter()
            .map(|row| row.display_name().to_owned())
            .collect()
        })
    }

    /// The open window's shown rows (either view), its chosen row and its
    /// name box's text, for the interface fixtures.
    #[must_use]
    pub fn shown_list(
        &self,
        world: &GameWorld,
        fog: &FogState,
    ) -> Option<(Vec<String>, Option<usize>, String)> {
        let window = self.window.as_ref()?;
        let list = rows(
            world,
            fog,
            player_side(window.faction),
            window.tab,
            window.view,
        );
        Some((
            list.iter()
                .map(|row| row.display_name().to_owned())
                .collect(),
            window.chosen,
            window.name.clone(),
        ))
    }

    /// The travelling characters' destinations (the missions' member
    /// transits), for the rows' location.
    pub fn set_destinations(
        &mut self,
        destinations: Vec<(rebellion_core::ids::CharacterKey, rebellion_core::ids::SystemKey)>,
    ) {
        self.destinations = destinations;
    }

    /// Open the Finder on the player's side tab and the Characters view
    /// (`FUN_004637a0`: `FUN_0060d7e0(side, 1)`, `FUN_0060d7e0(3, 1)`). An
    /// open Finder stays as it is.
    pub fn open(&mut self, faction: CockpitFaction) -> bool {
        if self.window.is_some() {
            return false;
        }
        let tab = match faction {
            CockpitFaction::Alliance => PersonnelTab::Alliance,
            CockpitFaction::Empire => PersonnelTab::Imperial,
        };
        self.window = Some(OpenFinder {
            faction,
            tab,
            view: PersonnelView::Characters,
            name: String::new(),
            chosen: None,
            target: None,
            scroll: 0,
            scroll_hold: None,
            focus_name: true,
            last_clicked: None,
        });
        true
    }

    pub fn close(&mut self) {
        self.window = None;
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.window.is_some()
    }

    #[must_use]
    pub fn contains_screen_point(&self, layout: CockpitLayout, point: (f32, f32)) -> bool {
        self.window.is_some() && rect_contains(window_rect(layout), egui::pos2(point.0, point.1))
    }

    /// Show side `tab` (case `100`, `FUN_00464890`): the list is rebuilt
    /// with no row chosen and no prefix match, but Display and Enter still
    /// open the object chosen before (`+0x178`). The list keeps its first
    /// visible pixel: `FUN_00464e10` swaps its items (`FUN_00608280`) and
    /// lays it out (`FUN_0060a280`), whose `FUN_00609ae0` only clamps it to
    /// the new range; the draw's bar clamps it so.
    pub fn set_tab(&mut self, tab: PersonnelTab) {
        if let Some(window) = &mut self.window {
            if window.tab != tab {
                window.tab = tab;
                window.chosen = None;
                window.last_clicked = None;
            }
        }
    }

    /// Show `view` (case `0x6e`), as [`Self::set_tab`] does.
    pub fn set_view(&mut self, view: PersonnelView) {
        if let Some(window) = &mut self.window {
            if window.view != view {
                window.view = view;
                window.chosen = None;
                window.last_clicked = None;
            }
        }
    }
}

/// Draw the open Personnel Finder, if any, and report an action.
#[expect(
    clippy::too_many_lines,
    reason = "The window paints in the original's control order; splitting it hides that order."
)]
pub fn draw_personnel_finder(
    ctx: &egui::Context,
    world: &GameWorld,
    fog: &FogState,
    state: &mut PersonnelFinderState,
    layout: CockpitLayout,
    cache: &mut BmpCache,
) -> Option<PersonnelFinderAction> {
    let window = state.window.clone()?;
    let side_idx = side(window.faction);
    let controls = &CONTROLS[side_idx];
    let scale = layout.scale;
    let rect = window_rect(layout);
    let list = match window.view {
        PersonnelView::Characters => character_rows_travelling(
            world,
            fog,
            player_side(window.faction),
            window.tab,
            &state.destinations,
        ),
        PersonnelView::SpecForces => {
            specforces_rows(world, fog, player_side(window.faction), window.tab)
        }
    };

    let mut close = false;
    let mut open_action: Option<PersonnelFinderAction> = None;
    let mut new_side_tab = None;
    let mut new_view_tab = None;
    let mut clicked_row = None;
    let mut step = None;
    let mut name = window.name.clone();
    let mut name_changed = false;
    let mut enter = false;
    let mut wheel = 0.0;
    let mut scroll = window.scroll;
    let mut scroll_hold = window.scroll_hold;

    let id = egui::Id::new("original-personnel-finder");
    ctx.move_to_top(egui::LayerId::new(egui::Order::Tooltip, id));
    egui::Area::new(id)
        .fixed_pos(rect.min)
        .order(egui::Order::Tooltip)
        // The original's window shows whole at once.
        .fade_in(false)
        .show(ctx, |ui| {
            let (frame, _) = ui.allocate_exact_size(rect.size(), egui::Sense::click());
            let painter = ui.painter().with_clip_rect(frame);
            let paint = |cache: &mut BmpCache, id: u32, (x, y): (f32, f32)| {
                paint_native(
                    &painter,
                    ctx,
                    cache,
                    DllSource::Strategy,
                    id,
                    frame,
                    scale,
                    x,
                    y,
                );
            };

            painter.rect_filled(frame, 0.0, egui::Color32::BLACK);
            // The panel lies under the base, whose rounded corners, keyed
            // on blue, cover the panel's (A0 System, Fleet, Troop and
            // Personnel Finders).
            paint(
                cache,
                PANELS[window.view.index()][window.tab.index()],
                PANEL_AT,
            );
            paint(cache, BACKGROUND[side_idx], (0.0, 0.0));
            paint(cache, RAIL[side_idx], RAIL_AT);

            // Title (0x1890) centred in its 350 in font 5, label (0x1891
            // "Name") in font 4, the tab's label in font 5; the list in font
            // 10 (A0 2026-10-29), the name box in font 4 (crate::name_box).
            crate::theme::paint_gdi_text(
                &painter,
                at(frame, scale, (TITLE_AT.0, TITLE_AT.1, 350.0, 17.0)),
                scale,
                5,
                "Personnel Finder",
                egui::Color32::WHITE,
                (crate::theme::GdiAlign::Center, crate::theme::GdiVAlign::Top),
            );
            crate::theme::paint_gdi_text(
                &painter,
                at(
                    frame,
                    scale,
                    (NAME_LABEL_AT.0, NAME_LABEL_AT.1, 100.0, 16.0),
                ),
                scale,
                4,
                "Name",
                egui::Color32::WHITE,
                (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
            );
            let label_y = window.view.label_y();
            crate::theme::paint_gdi_text(
                &painter,
                at(frame, scale, (40.0, label_y, 300.0, 16.0)),
                scale,
                5,
                window.tab.label(),
                egui::Color32::WHITE,
                (crate::theme::GdiAlign::Left, crate::theme::GdiVAlign::Top),
            );

            // Side tabs.
            for candidate in PersonnelTab::ALL {
                let control = SIDE_TABS[candidate.index()];
                let down = candidate == window.tab;
                crate::win_tooltip::window_tool(
                    ui,
                    at(frame, scale, control.rect),
                    ("personnel-side-tab", candidate.index()),
                    candidate.tooltip_string(),
                    true,
                );
                if button(
                    ui,
                    cache,
                    at(frame, scale, control.rect),
                    ("personnel-side-tab", candidate.index()),
                    control.art,
                    down,
                    scale,
                ) {
                    new_side_tab = Some(candidate);
                }
            }

            // View tabs (FUN_004637a0 group 0x6e).
            for candidate in PersonnelView::ALL {
                let y_offset = candidate.button_offset();
                let view_rect = (
                    controls.view_origin.0,
                    controls.view_origin.1 + y_offset,
                    controls.view_button_size.0,
                    controls.view_button_size.1,
                );
                let art = VIEW_TAB_ART[side_idx][candidate.index()];
                let down = candidate == window.view;
                // FUN_00464890 case 0x6e disables the chosen view tab
                // (EnableWindow FALSE, drawn pressed), so only the other one
                // shows its tip (A0 tooltip/62, 64, 65).
                crate::win_tooltip::window_tool(
                    ui,
                    at(frame, scale, view_rect),
                    ("personnel-view-tab", candidate.index()),
                    candidate.tooltip_string(),
                    !down,
                );
                if button(
                    ui,
                    cache,
                    at(frame, scale, view_rect),
                    ("personnel-view-tab", candidate.index()),
                    art,
                    down,
                    scale,
                ) {
                    new_view_tab = Some(candidate);
                }
            }

            // Close and Display.
            for (control, id, string) in [
                (
                    controls.close,
                    "personnel-close",
                    crate::system_finder::CLOSE_TOOLTIP,
                ),
                (
                    controls.display,
                    "personnel-display",
                    crate::system_finder::DISPLAY_TOOLTIP,
                ),
            ] {
                crate::win_tooltip::window_tool(
                    ui,
                    at(frame, scale, control.rect),
                    id,
                    string,
                    true,
                );
            }
            if button(
                ui,
                cache,
                at(frame, scale, controls.close.rect),
                "personnel-close",
                controls.close.art,
                false,
                scale,
            ) {
                close = true;
            }
            if button(
                ui,
                cache,
                at(frame, scale, controls.display.rect),
                "personnel-display",
                controls.display.art,
                false,
                scale,
            ) {
                // Case 0xc9: +0x178, which outlives a tab change.
                if let Some((row, tab)) = &window.target {
                    open_action = Some(action_for_row(row, world, *tab));
                }
            }

            // Name box (0xcb).
            let name_rect = at(frame, scale, NAME_BOX);
            // A CoolStringField in the default font 4 (crate::name_box).
            let edit = crate::name_box::show(
                ui,
                name_rect,
                scale,
                egui::Id::new("original-personnel-finder-name"),
                &mut name,
            );
            if window.focus_name {
                edit.request_focus();
            }
            name_changed = edit.changed();
            enter = edit.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            step = crate::list_prefix::key_step(ui, edit.has_focus());

            // The list (0xca).
            let list_rect = at(frame, scale, LIST);
            let list_painter = painter.with_clip_rect(list_rect);
            let cell_font = egui::FontId::proportional((10.0 * scale).max(7.0));

            // The bar, then the rows scrolled by pixels and clipped to the
            // list; a row's click area ends at the bar.
            #[expect(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "A list's pixels."
            )]
            {
                scroll = crate::list_scroll_bar::ui_bar(
                    ui,
                    cache,
                    frame,
                    scale,
                    crate::list_scroll_bar::RESULTS_ART,
                    LIST,
                    (list.len() * ROW_PIXELS) as f32,
                    scroll as f32,
                    &mut scroll_hold,
                    LIST_STEP,
                )
                .max(0.0) as usize;
            }
            for (offset, row) in list.iter().enumerate() {
                #[expect(clippy::cast_precision_loss, reason = "A list's pixels.")]
                let line = offset as f32 - scroll as f32 / ROW_HEIGHT;
                if (line + 1.0) * ROW_HEIGHT <= 0.0 || line * ROW_HEIGHT >= LIST.3 {
                    continue;
                }
                // The text keeps the whole row's place; the click area is
                // the row's part in the list.
                let full_row = at(
                    frame,
                    scale,
                    (LIST.0, LIST.1 + ROW_HEIGHT * line, LIST.2 - BAR_WIDTH, ROW_HEIGHT),
                );
                let row_rect = full_row.intersect(list_rect);
                // FUN_004637a0: the text at x 5 (+0xe8) centred on the row
                // (+0xe4 = 0x24), grey 0x787878 and the chosen row white
                // (FUN_00607ea0's colours; A0 2026-10-29).
                let name_colour = if window.chosen == Some(offset) {
                    egui::Color32::WHITE
                } else {
                    ROW_COLOR
                };

                match row {
                    PersonnelFinderRow::Character { name, .. } => {
                        crate::theme::paint_gdi_text(
                            &list_painter,
                            egui::Rect::from_min_max(
                                full_row.min + egui::vec2(ROW_TEXT_X * scale, 0.0),
                                full_row.max,
                            ),
                            scale,
                            10,
                            name,
                            name_colour,
                            (
                                crate::theme::GdiAlign::Left,
                                crate::theme::GdiVAlign::VCenter,
                            ),
                        );
                    }
                    PersonnelFinderRow::SpecForces { name, counts, .. } => {
                        crate::theme::paint_gdi_text(
                            &list_painter,
                            egui::Rect::from_min_max(
                                full_row.min + egui::vec2(ROW_TEXT_X * scale, 0.0),
                                full_row.max,
                            ),
                            scale,
                            10,
                            name,
                            name_colour,
                            (
                                crate::theme::GdiAlign::Left,
                                crate::theme::GdiVAlign::VCenter,
                            ),
                        );
                        // Four columns at x = 222 + 28*i (FUN_00465540).
                        for (i, cell) in counts.iter().enumerate() {
                            let (box_colour, text_on_box) = sf_cell_colours(window.tab, cell.all_usable);
                            let cell_x = 222.0 + 28.0 * i as f32;
                            if cell.count == 0 {
                                let cell_pos = at(
                                    frame,
                                    scale,
                                    (LIST.0 + cell_x, LIST.1 + ROW_HEIGHT * line + 2.0, 28.0, 0.0),
                                )
                                .min;
                                list_painter.text(
                                    cell_pos + egui::vec2(10.0 * scale, 0.0),
                                    egui::Align2::CENTER_TOP,
                                    "-",
                                    cell_font.clone(),
                                    egui::Color32::WHITE,
                                );
                            } else {
                                let box_rect = at(
                                    frame,
                                    scale,
                                    (
                                        LIST.0 + cell_x,
                                        LIST.1 + ROW_HEIGHT * line + 2.0,
                                        21.0,
                                        16.0,
                                    ),
                                );
                                list_painter.rect_filled(box_rect, 0.0, box_colour);
                                list_painter.text(
                                    box_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    cell.count.to_string(),
                                    cell_font.clone(),
                                    text_on_box,
                                );
                            }
                        }
                    }
                }

                let response = ui.interact(
                    row_rect,
                    ui.id().with(("personnel-row", offset)),
                    egui::Sense::click(),
                );
                if response.clicked() {
                    let (now, delay) = (
                        ui.input(|input| input.time),
                        ctx.options(|options| options.input_options.max_double_click_delay),
                    );
                    if window
                        .last_clicked
                        .is_some_and(|(last, at_time)| last == offset && now - at_time <= delay)
                    {
                        open_action = Some(action_for_row(row, world, window.tab));
                    } else {
                        clicked_row = Some((offset, now));
                    }
                }
            }
            let pointer = ui.input(|input| input.pointer.hover_pos());
            if pointer.is_some_and(|point| rect_contains(list_rect, point)) {
                wheel = ui.input(|input| input.raw_scroll_delta.y);
            }
        });

    let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    let open_window = state.window.as_mut()?;
    open_window.focus_name = false;
    open_window.scroll = scroll;
    open_window.scroll_hold = scroll_hold;
    if name_changed {
        open_window.name.clone_from(&name);
        // FUN_00464c20 0x408: the best row is chosen and +0x178 takes its
        // object, or is emptied with no row.
        let choice = prefix_choice(&list, &name);
        let before = open_window.chosen;
        open_window.chosen = choice;
        open_window.target = choice.map(|index| (list[index].clone(), open_window.tab));
        if let Some(index) = crate::list_prefix::row_to_show(choice, before) {
            open_window.scroll = scrolled_to(open_window.scroll, index, list.len());
        }
    }
    if let Some((row, at_time)) = clicked_row {
        open_window.last_clicked = Some((row, at_time));
        open_window.chosen = Some(row);
        open_window.target = Some((list[row].clone(), open_window.tab));
        // Copy an existing object's name into the name box.
        if let Some(name) = box_name(&list[row], world) {
            open_window.name = name;
        }
    }
    // A key steps the chosen row and copies its name, as a click does (A0
    // finder-keys/, the Fleet Finder's list).
    if let Some(step) = step {
        if let Some(row) = crate::list_prefix::stepped_row(open_window.chosen, list.len(), step) {
            open_window.chosen = Some(row);
            open_window.target = Some((list[row].clone(), open_window.tab));
            if let Some(name) = box_name(&list[row], world) {
                open_window.name = name;
            }
            open_window.scroll = scrolled_to(open_window.scroll, row, list.len());
        }
    }
    if enter {
        // 0x407: +0x178, which outlives a tab change.
        if let Some((row, tab)) = &open_window.target {
            open_action = Some(action_for_row(row, world, *tab));
        }
    }
    // port: the original has no wheel; a notch moves a row.
    if wheel != 0.0 {
        open_window.scroll = if wheel > 0.0 {
            open_window.scroll.saturating_sub(ROW_PIXELS)
        } else {
            (open_window.scroll + ROW_PIXELS).min(max_scroll(list.len()))
        };
    }
    if let Some(tab) = new_side_tab {
        state.set_tab(tab);
    }
    if let Some(view) = new_view_tab {
        state.set_view(view);
    }
    if let Some(action) = open_action {
        state.close();
        return Some(action);
    }
    if close || escape {
        state.close();
    }
    None
}

fn action_for_row(
    row: &PersonnelFinderRow,
    world: &GameWorld,
    tab: PersonnelTab,
) -> PersonnelFinderAction {
    match row {
        PersonnelFinderRow::Character {
            copy: Some(place), ..
        } => PersonnelFinderAction::OpenCopy(*place),
        PersonnelFinderRow::Character { key, system, .. } => PersonnelFinderAction::OpenCharacter {
            character: *key,
            system: *system,
        },
        PersonnelFinderRow::SpecForces { system, fleet, .. } => {
            if let Some(fleet) = fleet {
                PersonnelFinderAction::OpenFleet {
                    system: *system,
                    fleet: *fleet,
                }
            } else {
                let unit = world.systems.get(*system).and_then(|value| {
                    value.special_forces.iter().copied().find(|&key| {
                        world
                            .special_forces
                            .get(key)
                            .is_some_and(|sf| tab.lists(sf.is_alliance))
                    })
                });
                PersonnelFinderAction::OpenSystem {
                    system: *system,
                    unit,
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "personnel_finder_tests.rs"]
mod tests;
