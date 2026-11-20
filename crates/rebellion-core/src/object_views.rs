//! Each side's view of the other side's objects: the original's per-side
//! copies (`ghidra/notes/per-side-object-views.md`).
//!
//! Every object of the original has a true copy and one copy per side
//! (`FUN_005844e0`: group `+0x24 + side * 4`). The windows read the
//! player's side's copy (`FUN_004f2d10` -> `FUN_0053efd0`). A side's copy is
//! live while the object is the side's own, or while the side's seen bit is
//! set, the object is complete and it is not en route (`FUN_004f9a60`);
//! otherwise it is frozen and keeps its old values (`FUN_005404d0`,
//! `FUN_004f9ba0`). A system's seen bit is set while the side has a fleet in
//! it or holds it (`FUN_0050ac80`); an object inside takes its container's
//! (`FUN_004f7bf0`).
//!
//! port: the port keeps a view only of the other side's objects (a side's
//! own copy is always live, so the windows read the live object), and only
//! of regiments so far. Views are keyed by an object id the port gives each
//! regiment (`DatId`, the class family and a running number), never by a
//! slotmap key. The views are refreshed once a day after movement, control
//! and missions ([`compute`], [`apply`]).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dat::Faction;
use crate::fog::FogState;
use crate::ids::{DatId, SystemKey, TroopKey};
use crate::movement::MovementState;
use crate::troop_transport::{RegimentLeg, TroopTransportState};
use crate::world::GameWorld;

/// Where a regiment's view puts it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewPlace {
    /// On a system's surface (its id).
    Surface(DatId),
    /// In a fleet's cargo at a system: the system's id and the fleet's name
    /// when the view was taken.
    Fleet { system: DatId, fleet_name: String },
}

impl ViewPlace {
    /// The system the place is in.
    #[must_use]
    pub fn system(&self) -> DatId {
        match self {
            Self::Surface(system) | Self::Fleet { system, .. } => *system,
        }
    }
}

/// A side's copy of a regiment (class `0x10..0x13`): what its windows show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegimentView {
    pub class: DatId,
    pub is_alliance: bool,
    pub strength: i16,
    /// Its container. An object in hyperspace is already in its destination
    /// (`FUN_00556390`).
    pub place: ViewPlace,
    /// The arrival day while it was en route (`+0x50` bit 4, `+0x44`).
    pub arrival: Option<u64>,
    /// The copy's sync state (`+0x24` bits 0..3): live (1) or frozen (2).
    pub live: bool,
}

/// One side's view of an object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectView {
    Regiment(RegimentView),
}

impl ObjectView {
    #[must_use]
    pub fn live(&self) -> bool {
        match self {
            Self::Regiment(view) => view.live,
        }
    }

    #[must_use]
    pub fn system(&self) -> DatId {
        match self {
            Self::Regiment(view) => view.place.system(),
        }
    }
}

/// Both sides' views, and the ids that key them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObjectViews {
    /// Each regiment's object id.
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub regiment_ids: HashMap<TroopKey, DatId>,
    /// The last running number given.
    pub last_id: u32,
    /// The Alliance's views of Imperial objects.
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub alliance: HashMap<DatId, ObjectView>,
    /// The Empire's views of Alliance objects.
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub empire: HashMap<DatId, ObjectView>,
    /// The Alliance's views of each system's own fields and of the
    /// facilities, planet squadrons, special forces and characters in it,
    /// by the system's id (v118).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub alliance_systems: HashMap<DatId, SystemView>,
    /// The Empire's views of the systems (v118).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub empire_systems: HashMap<DatId, SystemView>,
    /// The Alliance's copies of each system's fleets, missions and the
    /// units it saw leave for it (v119).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub alliance_extras: HashMap<DatId, SystemExtras>,
    /// The Empire's (v119).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub empire_extras: HashMap<DatId, SystemExtras>,
    /// The Alliance's copies of each system's fleets and missions in full:
    /// names, ships, squadrons, troops and members (v120).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub alliance_details: HashMap<DatId, SystemDetails>,
    /// The Empire's (v120).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub empire_details: HashMap<DatId, SystemDetails>,
    /// The Alliance's copies of Imperial characters, by the character's id
    /// (v121): where the side last held each one ([`CharacterCopy`]).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub alliance_characters: HashMap<DatId, CharacterCopy>,
    /// The Empire's copies of Alliance characters (v121).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub empire_characters: HashMap<DatId, CharacterCopy>,
    /// The Alliance's copies' state bits, by the character's id (v122): a
    /// character absent here has none ([`CharacterCopyState`]).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub alliance_character_states: HashMap<DatId, CharacterCopyState>,
    /// The Empire's copies' state bits (v122).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub empire_character_states: HashMap<DatId, CharacterCopyState>,
    /// The Alliance's copies' command words (`+0x96`), by the character's
    /// id (v123): a character absent here holds no command.
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub alliance_character_commands: HashMap<DatId, crate::character_command::CommandKind>,
    /// The Empire's copies' command words (v123).
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub empire_character_commands: HashMap<DatId, crate::character_command::CommandKind>,
    /// port: the debug "see everything" mode: the windows show every live
    /// object of both sides and ignore the views and the fog. Not saved.
    #[serde(skip)]
    pub see_everything: bool,
    /// Intelligence pushes the day's missions made (`Event319_Handle`),
    /// taken by the next [`advance`] ([`push_system`]). Not saved: the
    /// day's step takes them before a save can be made.
    #[serde(skip)]
    pub pushes: Vec<(Faction, SystemKey)>,
    /// Each side's copies' explored and destroyed bits, by the system's id
    /// (v127). Saved apart (`SaveState::system_copy_bits`, the save's last
    /// field), so the views' own encoding skips it.
    #[serde(skip)]
    pub system_bits: SystemCopyBitsMap,
    /// Each side's copies' uprising bit, by the system's id (v131). Saved
    /// apart (`SaveState::system_copy_uprisings`), so the views' own
    /// encoding skips it.
    #[serde(skip)]
    pub system_uprisings: SystemCopyUprisings,
}

/// Each side's copies' uprising bit (`+0x88` bit 2), which the mode-1 sync
/// (`FUN_0050e5b0`) copies with explored and the sector window's planet
/// (`FUN_0045bbb0`, marks 905/906) and the GID read from the viewer's copy
/// (`ghidra/notes/fog-of-war.md`). A copy absent here is calm.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemCopyUprisings {
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub alliance: HashMap<DatId, bool>,
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub empire: HashMap<DatId, bool>,
}

impl SystemCopyUprisings {
    /// Whether `side`'s copy of the system `id` is in uprising.
    #[must_use]
    pub fn get(&self, side: Faction, id: DatId) -> bool {
        match side {
            Faction::Alliance => self.alliance.get(&id).copied(),
            Faction::Empire => self.empire.get(&id).copied(),
            Faction::Neutral => None,
        }
        .unwrap_or(false)
    }

    fn set(&mut self, side: Faction, id: DatId, uprising: bool) {
        let map = match side {
            Faction::Alliance => &mut self.alliance,
            Faction::Empire => &mut self.empire,
            Faction::Neutral => return,
        };
        if uprising {
            map.insert(id, true);
        } else {
            map.remove(&id);
        }
    }
}

/// A side's copy's explored bit (`+0x88` bit 1, `System_SyncExplored`
/// `FUN_0050fc80`) and destroyed bit (`+0x50` bit 3), which the GID item
/// (`FUN_0042b330`) and the sector window (`FUN_0045bbb0`) read from the
/// viewer's copy (`ghidra/notes/fog-of-war.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SystemCopyBits {
    pub explored: bool,
    pub destroyed: bool,
}

/// Both sides' [`SystemCopyBits`], by the system's id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemCopyBitsMap {
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub alliance: HashMap<DatId, SystemCopyBits>,
    #[serde(
        serialize_with = "crate::serde_ordered::serialize_hash_map",
        deserialize_with = "crate::serde_ordered::deserialize_hash_map"
    )]
    pub empire: HashMap<DatId, SystemCopyBits>,
}

impl SystemCopyBitsMap {
    /// `side`'s bits for the system `id`.
    #[must_use]
    pub fn get(&self, side: Faction, id: DatId) -> Option<SystemCopyBits> {
        match side {
            Faction::Alliance => self.alliance.get(&id).copied(),
            Faction::Empire => self.empire.get(&id).copied(),
            Faction::Neutral => None,
        }
    }

    fn set(&mut self, side: Faction, id: DatId, bits: SystemCopyBits) {
        match side {
            Faction::Alliance => {
                self.alliance.insert(id, bits);
            }
            Faction::Empire => {
                self.empire.insert(id, bits);
            }
            Faction::Neutral => {}
        }
    }
}

impl ObjectViews {
    /// Queues an intelligence push into `side`'s copies at `system`
    /// (`Event319_Handle` `FUN_0050d5a0`): an Espionage (level 10,
    /// `FUN_00573610`) or Reconnaissance (level 8, `FUN_0056bee0`) success,
    /// an encounter or the Final Battle at a system (level 6,
    /// `FUN_0055ff60`, `FUN_0054b040`), a mission's end at its target
    /// (level 6 for its side, `FUN_00592c80`), a bombardment (level 11,
    /// `FUN_005650f0`) or an informant incident (`FUN_0050d510`). port:
    /// every level copies the whole system and its contents.
    pub fn queue_push(&mut self, side: Faction, system: SystemKey) {
        self.pushes.push((side, system));
    }

    /// `side`'s views (Alliance or Empire; none for another side).
    #[must_use]
    pub fn side(&self, side: Faction) -> Option<&HashMap<DatId, ObjectView>> {
        match side {
            Faction::Alliance => Some(&self.alliance),
            Faction::Empire => Some(&self.empire),
            Faction::Neutral => None,
        }
    }

    fn side_mut(&mut self, side: Faction) -> Option<&mut HashMap<DatId, ObjectView>> {
        match side {
            Faction::Alliance => Some(&mut self.alliance),
            Faction::Empire => Some(&mut self.empire),
            Faction::Neutral => None,
        }
    }

    /// `side`'s views of the systems.
    #[must_use]
    pub fn systems(&self, side: Faction) -> Option<&HashMap<DatId, SystemView>> {
        match side {
            Faction::Alliance => Some(&self.alliance_systems),
            Faction::Empire => Some(&self.empire_systems),
            Faction::Neutral => None,
        }
    }

    fn systems_mut(&mut self, side: Faction) -> Option<&mut HashMap<DatId, SystemView>> {
        match side {
            Faction::Alliance => Some(&mut self.alliance_systems),
            Faction::Empire => Some(&mut self.empire_systems),
            Faction::Neutral => None,
        }
    }

    /// `side`'s copies of the systems' fleets, missions and travellers.
    #[must_use]
    pub fn extras(&self, side: Faction) -> Option<&HashMap<DatId, SystemExtras>> {
        match side {
            Faction::Alliance => Some(&self.alliance_extras),
            Faction::Empire => Some(&self.empire_extras),
            Faction::Neutral => None,
        }
    }

    fn extras_mut(&mut self, side: Faction) -> Option<&mut HashMap<DatId, SystemExtras>> {
        match side {
            Faction::Alliance => Some(&mut self.alliance_extras),
            Faction::Empire => Some(&mut self.empire_extras),
            Faction::Neutral => None,
        }
    }

    /// `side`'s full copy of system `id`'s fleets and missions.
    #[must_use]
    pub fn system_details(&self, side: Faction, id: DatId) -> Option<&SystemDetails> {
        match side {
            Faction::Alliance => self.alliance_details.get(&id),
            Faction::Empire => self.empire_details.get(&id),
            Faction::Neutral => None,
        }
    }

    fn system_details_map(&self, side: Faction) -> Option<&HashMap<DatId, SystemDetails>> {
        match side {
            Faction::Alliance => Some(&self.alliance_details),
            Faction::Empire => Some(&self.empire_details),
            Faction::Neutral => None,
        }
    }

    fn details_mut(&mut self, side: Faction) -> Option<&mut HashMap<DatId, SystemDetails>> {
        match side {
            Faction::Alliance => Some(&mut self.alliance_details),
            Faction::Empire => Some(&mut self.empire_details),
            Faction::Neutral => None,
        }
    }

    /// `side`'s copy of the other side's character `id`.
    #[must_use]
    pub fn character(&self, side: Faction, id: DatId) -> Option<&CharacterCopy> {
        match side {
            Faction::Alliance => self.alliance_characters.get(&id),
            Faction::Empire => self.empire_characters.get(&id),
            Faction::Neutral => None,
        }
    }

    /// The state bits of `side`'s copy of the character `id`; none when it
    /// has none.
    #[must_use]
    pub fn character_state(&self, side: Faction, id: DatId) -> CharacterCopyState {
        match side {
            Faction::Alliance => self.alliance_character_states.get(&id),
            Faction::Empire => self.empire_character_states.get(&id),
            Faction::Neutral => None,
        }
        .copied()
        .unwrap_or_default()
    }

    /// The command word (`+0x96`) of `side`'s copy of the character `id`,
    /// as it was when the copy last synced: the master's setter
    /// (`FUN_004ee350`) reaches a copy through `FUN_004f04f0`, which writes
    /// only when `FUN_005404d0` holds, so a frozen copy keeps its old word.
    #[must_use]
    pub fn character_command(
        &self,
        side: Faction,
        id: DatId,
    ) -> crate::character_command::CommandKind {
        match side {
            Faction::Alliance => self.alliance_character_commands.get(&id),
            Faction::Empire => self.empire_character_commands.get(&id),
            Faction::Neutral => None,
        }
        .copied()
        .unwrap_or_default()
    }

    fn character_commands_mut(
        &mut self,
        side: Faction,
    ) -> Option<&mut HashMap<DatId, crate::character_command::CommandKind>> {
        match side {
            Faction::Alliance => Some(&mut self.alliance_character_commands),
            Faction::Empire => Some(&mut self.empire_character_commands),
            Faction::Neutral => None,
        }
    }

    fn character_states_mut(
        &mut self,
        side: Faction,
    ) -> Option<&mut HashMap<DatId, CharacterCopyState>> {
        match side {
            Faction::Alliance => Some(&mut self.alliance_character_states),
            Faction::Empire => Some(&mut self.empire_character_states),
            Faction::Neutral => None,
        }
    }

    fn characters_mut(&mut self, side: Faction) -> Option<&mut HashMap<DatId, CharacterCopy>> {
        match side {
            Faction::Alliance => Some(&mut self.alliance_characters),
            Faction::Empire => Some(&mut self.empire_characters),
            Faction::Neutral => None,
        }
    }

    /// `side`'s copy of system `id`'s fleets, missions and travellers.
    #[must_use]
    pub fn system_extras(&self, side: Faction, id: DatId) -> Option<&SystemExtras> {
        self.extras(side)?.get(&id)
    }

    /// `side`'s view of the system `id`.
    #[must_use]
    pub fn system(&self, side: Faction, id: DatId) -> Option<&SystemView> {
        self.systems(side)?.get(&id)
    }

    /// `side`'s view of the object `id`.
    #[must_use]
    pub fn view(&self, side: Faction, id: DatId) -> Option<&ObjectView> {
        self.side(side)?.get(&id)
    }

    /// A regiment's object id, once numbered.
    #[must_use]
    pub fn regiment_id(&self, key: TroopKey) -> Option<DatId> {
        self.regiment_ids.get(&key).copied()
    }

    /// The live regiment `id` names, if it still exists.
    #[must_use]
    pub fn regiment_key(&self, world: &GameWorld, id: DatId) -> Option<TroopKey> {
        self.regiment_ids
            .iter()
            .find(|(key, value)| **value == id && world.troops.contains_key(**key))
            .map(|(key, _)| *key)
    }
}

/// Gives each regiment without one an object id: its class family and the
/// next running number, as the original's ids carry their family. Drops the
/// ids of regiments that are gone. port: the running number is the port's;
/// the family byte sits in bits 24..31 as in every `DatId`.
pub fn number_regiments(world: &mut GameWorld) {
    let troops = &world.troops;
    let views = &mut world.object_views;
    views
        .regiment_ids
        .retain(|key, _| troops.contains_key(*key));
    let mut keys: Vec<TroopKey> = troops
        .keys()
        .filter(|key| !views.regiment_ids.contains_key(key))
        .collect();
    // Slotmap order is the insertion order of free slots; sort by the
    // key's index so the numbering is deterministic.
    keys.sort_by_key(|key| slotmap::Key::data(key).as_ffi());
    for key in keys {
        views.last_id += 1;
        let family = u32::from(troops[key].class_dat_id.family());
        views.regiment_ids.insert(
            key,
            DatId::new(family << 24 | (views.last_id & 0x00FF_FFFF)),
        );
    }
}

/// Whether `side` sees `system` (`FUN_0050ac80`): the side holds it, or one
/// of its fleets is in it. hyp: a fleet in hyperspace counts toward
/// neither system.
#[must_use]
pub fn system_seen(
    world: &GameWorld,
    movement: &MovementState,
    side: Faction,
    system: SystemKey,
) -> bool {
    let alliance = side == Faction::Alliance;
    world
        .systems
        .get(system)
        .is_some_and(|value| value.control.is_controlled_by(side))
        || world.fleets.iter().any(|(key, fleet)| {
            fleet.is_alliance == alliance
                && fleet.location == system
                && !movement.is_in_transit(key)
                // port: a staged spare (crate::products) watches nothing.
                && !crate::products::is_staged(world, key)
        })
}

/// Where a regiment is now: its place's system key, its view place and its
/// arrival day while en route.
fn regiment_place(
    world: &GameWorld,
    transport: &TroopTransportState,
    key: TroopKey,
) -> Option<(SystemKey, ViewPlace, Option<u64>)> {
    let system_id = |system: SystemKey| world.systems.get(system).map(|value| value.dat_id);
    let fleet_place = |fleet| {
        let value = world.fleets.get(fleet)?;
        Some((
            value.location,
            ViewPlace::Fleet {
                system: system_id(value.location)?,
                fleet_name: world.fleet_name(fleet).unwrap_or_default().to_owned(),
            },
        ))
    };
    if let Some(transit) = transport.transits().iter().find(|value| value.troop == key) {
        let (system, place) = match transit.leg {
            RegimentLeg::Surface(system) => (system, ViewPlace::Surface(system_id(system)?)),
            RegimentLeg::Fleet(fleet) => fleet_place(fleet)?,
        };
        return Some((system, place, Some(transit.arrival_tick)));
    }
    if let Some(fleet) = transport
        .fleet_keys()
        .into_iter()
        .find(|&fleet| transport.cargo(fleet).contains(&key))
    {
        let (system, place) = fleet_place(fleet)?;
        return Some((system, place, None));
    }
    world
        .systems
        .iter()
        .find(|(_, value)| value.ground_units.contains(&key))
        .map(|(system, value)| (system, ViewPlace::Surface(value.dat_id), None))
}

/// A facility as a side's copy holds it: its class and side, and for a
/// production facility whether it is a mine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacilityView {
    pub class: DatId,
    pub side: Faction,
    pub is_mine: bool,
}

/// A side's copy of a system (`FUN_004f9a60` on the system and, through
/// the inherited seen bit `FUN_004f7bf0`, on its stationary contents): its
/// own fields as the System window shows them and the facilities, planet
/// squadrons, special forces and characters in it, in the system's order.
///
/// port: one record per system; a unit on its way in or out is not in it
/// (the port holds travellers outside their systems), and its regiments
/// are copied one by one ([`RegimentView`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemView {
    /// The copy's sync state: live (1) or frozen (2).
    pub live: bool,
    pub control: crate::world::ControlKind,
    pub popularity_alliance: f32,
    pub popularity_empire: f32,
    pub is_populated: bool,
    pub total_energy: u8,
    pub raw_materials: u8,
    pub defense_facilities: Vec<FacilityView>,
    pub manufacturing_facilities: Vec<FacilityView>,
    pub production_facilities: Vec<FacilityView>,
    /// The planet's squadrons: each class's id and count.
    pub fighters: Vec<(DatId, u32)>,
    /// The special forces' class ids and sides.
    pub special_forces: Vec<(DatId, bool)>,
    /// The living characters at the system, by their ids, as the Defenses
    /// window's personnel walk finds them (`current_system`).
    pub characters: Vec<DatId>,
}

impl SystemView {
    /// A Rim system's copy after the setup push (`FUN_0050c8a0`, mode 2):
    /// no side, no values, nothing in it.
    #[must_use]
    pub fn bare() -> Self {
        Self {
            live: false,
            control: crate::world::ControlKind::Uncontrolled,
            popularity_alliance: 0.0,
            popularity_empire: 0.0,
            is_populated: false,
            total_energy: 0,
            raw_materials: 0,
            defense_facilities: Vec::new(),
            manufacturing_facilities: Vec::new(),
            production_facilities: Vec::new(),
            fighters: Vec::new(),
            special_forces: Vec::new(),
            characters: Vec::new(),
        }
    }

    /// The system as `live` copies it now.
    #[must_use]
    pub fn of(world: &GameWorld, system: &crate::world::System, live: bool) -> Self {
        let facility = |class: DatId, side: Faction, is_mine: bool| FacilityView {
            class,
            side,
            is_mine,
        };
        Self {
            live,
            control: system.control,
            popularity_alliance: system.popularity_alliance,
            popularity_empire: system.popularity_empire,
            is_populated: system.is_populated,
            total_energy: system.total_energy,
            raw_materials: system.raw_materials,
            defense_facilities: system
                .defense_facilities
                .iter()
                .filter_map(|key| world.defense_facilities.get(*key))
                .map(|value| facility(value.class_dat_id, value.side, false))
                .collect(),
            manufacturing_facilities: system
                .manufacturing_facilities
                .iter()
                .filter_map(|key| world.manufacturing_facilities.get(*key))
                .map(|value| facility(value.class_dat_id, value.side, false))
                .collect(),
            production_facilities: system
                .production_facilities
                .iter()
                .filter_map(|key| world.production_facilities.get(*key))
                .map(|value| facility(value.class_dat_id, value.side, value.is_mine))
                .collect(),
            fighters: system
                .fighters
                .iter()
                .filter_map(|entry| {
                    Some((world.fighter_classes.get(entry.class)?.dat_id, entry.count))
                })
                .collect(),
            special_forces: system
                .special_forces
                .iter()
                .filter_map(|key| world.special_forces.get(*key))
                .map(|unit| (unit.class_dat_id, unit.is_alliance))
                .collect(),
            characters: world
                .characters
                .values()
                .filter(|character| {
                    !character.is_killed
                        && character.current_system.is_some_and(|at| {
                            world
                                .systems
                                .get(at)
                                .is_some_and(|value| value.dat_id == system.dat_id)
                        })
                })
                .map(|character| character.dat_id)
                .collect(),
        }
    }
}

/// A fleet as a side's copy of its system holds it: its creation serial
/// (`GameWorld::fleet_serial`, the port's object key), name, side and its
/// living ships' classes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetCopy {
    pub serial: u64,
    pub name: String,
    pub is_alliance: bool,
    pub ships: Vec<DatId>,
}

/// A unit the side saw leave for a system it does not see: frozen in
/// hyperspace in its destination's container (`FUN_00556390`;
/// `FUN_004f9a60`, `+0x50` bit 4), until day `arrival`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TravellerCopy {
    Character {
        id: DatId,
        arrival: u64,
    },
    SpecialForce {
        class: DatId,
        is_alliance: bool,
        arrival: u64,
    },
    Squadron {
        class: DatId,
        is_alliance: bool,
        count: u32,
        arrival: u64,
    },
}

impl TravellerCopy {
    #[must_use]
    pub const fn arrival(&self) -> u64 {
        match self {
            Self::Character { arrival, .. }
            | Self::SpecialForce { arrival, .. }
            | Self::Squadron { arrival, .. } => *arrival,
        }
    }
}

/// A side's copy of a system's fleets, the missions there and the units it
/// saw leave for it (v119). It is live and frozen with the system
/// ([`SystemView`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemExtras {
    pub live: bool,
    /// The fleets in orbit, not in hyperspace, in creation order.
    pub fleets: Vec<FleetCopy>,
    /// The missions aimed at the system: each mission's id and its side
    /// (1 or 2). port: a mission's place is its target system.
    pub missions: Vec<(u64, u8)>,
    /// The units the side saw leave for the system while it does not see
    /// it.
    pub travellers: Vec<TravellerCopy>,
}

impl SystemExtras {
    /// `system`'s fleets and missions as `live` copies them now. port: a
    /// mission's side is its faction's side bits, 1 or 2 (`+0x24` bits
    /// 6..7).
    #[must_use]
    pub fn of(
        world: &GameWorld,
        movement: &MovementState,
        missions: &crate::missions::MissionState,
        system: SystemKey,
        live: bool,
    ) -> Self {
        let mut fleets: Vec<(u64, FleetCopy)> = world
            .fleets
            .iter()
            .filter(|(key, fleet)| fleet.location == system && !movement.is_in_transit(*key))
            .map(|(key, fleet)| {
                let serial = world.fleet_serial(key);
                (
                    serial,
                    FleetCopy {
                        serial,
                        name: world.fleet_name(key).unwrap_or_default().to_owned(),
                        is_alliance: fleet.is_alliance,
                        ships: fleet
                            .capital_ships
                            .iter()
                            .filter(|ship| ship.alive)
                            .filter_map(|ship| {
                                world
                                    .capital_ship_classes
                                    .get(ship.class)
                                    .map(|class| class.dat_id)
                            })
                            .collect(),
                    },
                )
            })
            .collect();
        fleets.sort_by_key(|(serial, _)| *serial);
        let mut copied: Vec<(u64, u8)> = missions
            .missions()
            .iter()
            .filter(|mission| mission.target_system == system)
            .map(|mission| {
                let side = match mission.faction {
                    crate::missions::MissionFaction::Alliance => 1,
                    crate::missions::MissionFaction::Empire => 2,
                };
                (mission.id, side)
            })
            .collect();
        copied.sort_unstable();
        Self {
            live,
            fleets: fleets.into_iter().map(|(_, fleet)| fleet).collect(),
            missions: copied,
            travellers: Vec::new(),
        }
    }
}

/// Queues the push both sides take of `system` (`Event319_Fire`
/// `FUN_005650f0`: level 11 for sides 1 and 2, after a bombardment).
pub fn queue_push_both(world: &mut GameWorld, system: SystemKey) {
    for side in [Faction::Alliance, Faction::Empire] {
        world.object_views.queue_push(side, system);
    }
}

/// A capital ship as a side's copy holds it: class, name and hull.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShipCopy {
    pub class: DatId,
    pub name: Option<String>,
    pub hull: i32,
}

/// A fleet as a side's copy of its system holds it in full (`FUN_004f9a60`
/// on the fleet and, by the inherited seen bit `FUN_004f7bf0`, on its
/// ships, squadrons, regiments and characters).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetDetail {
    pub serial: u64,
    pub name: String,
    pub is_alliance: bool,
    /// Its living ships.
    pub ships: Vec<ShipCopy>,
    /// Its squadrons: each class's id and count.
    pub fighters: Vec<(DatId, u32)>,
    /// The regiments aboard, by class.
    pub troops: Vec<DatId>,
    /// The characters aboard, by id.
    pub characters: Vec<DatId>,
}

/// A mission member as a copy holds it: a character by its id, a special
/// force by its class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemberCopy {
    Character(DatId),
    SpecialForce(DatId),
}

/// A mission as a side's copy of its target system holds it in full.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionDetail {
    pub id: u64,
    /// Its side, 1 or 2.
    pub side: u8,
    pub kind: crate::missions::MissionKind,
    pub team: Vec<MemberCopy>,
    pub decoys: Vec<MemberCopy>,
}

/// A side's copy of an other-side character (v121): the system whose copy
/// last held it and, aboard, the copied ship it rode.
///
/// `FUN_00465bb0` starts the row's place as `0x1895` "Location Unknown"
/// (`0x465c21`) and replaces it with the name of the container of the
/// side's copy of the character (vtable `+0xc`, then `FUN_004f62d0(+0x1c)`)
/// only when that container is a system (family `0x90..0x97`) or a capital
/// ship (`0x14..0x1b`), or an abode (`0x98..0x9f`). A0
/// 2026-11-14-stale-views second run: the Alliance's Imperial tab read
/// "Darth Vader - Imperial Star Destroyer 3" while its copy of Yaga Minor
/// held Vader aboard a copied fleet (1x-personnelfinder-empire-d85, sha256
/// d57ed845), then "Darth Vader - Location Unknown" from the day that copy
/// went live without him (1x-personnelfinder-empire-d91 1b8818d8, d97
/// 56d33542). hyp: the live rebuild (`FUN_004f8340`) takes the departed
/// fleet's copy out of the system's copy, which leaves the character's copy
/// in no system or ship; untraced.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterCopy {
    /// The system whose copy holds it; none once that copy was rebuilt
    /// live without it.
    pub system: Option<DatId>,
    /// The name of the ship it rode as copied, when aboard. port: a
    /// character not seated on a ship names its fleet; a side's own
    /// captive copied in an abode names the abode, with no system
    /// ([`record_own_captives`]).
    pub ship: Option<String>,
}

/// The state bits a side's copy of a character keeps (v122), as
/// `FUN_00465bb0` reads them for the row's state word:
/// - on a mission: the copy's mission key `+0x68` (`FUN_0042d170`) names a
///   mission object (family `0x50..0x7f`) while `+0xac` bit 0 (captive) is
///   clear: "On Mission" (`0x8745`); `FUN_00429440` then opens the Missions
///   window (kind 11) unless that mission is hidden (`FUN_00520b70`);
/// - en route: `+0x50` bit 4 (`0x10`): "Enroute" (`0x8744`). A copy seen
///   leaving for a system the side does not see is frozen there with it
///   (`FUN_00556390`, `FUN_004f9a60`).
///
/// port: the bits are taken from the live character when the copy is
/// taken; Captured and Injured, and the command word, are not kept.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterCopyState {
    pub on_mission: bool,
    /// Its mission is hidden (`FUN_00520b70`).
    pub hidden_mission: bool,
    pub enroute: bool,
}

impl CharacterCopyState {
    /// The bits of `character` as a copy takes them now, en route or not.
    #[must_use]
    pub fn of(character: &crate::world::Character, enroute: bool) -> Self {
        Self {
            on_mission: character.on_mission && !character.is_captive,
            hidden_mission: character.on_mission && character.on_hidden_mission,
            enroute,
        }
    }
}

/// The other side's living characters at `system` as `side` copies them
/// now: those aboard its fleets in orbit (with their ships' names) and
/// those on the planet, in id order. The side's own captives count with
/// them: a capture gives the captive its captor's side (`FUN_004eec60`
/// from `Character_SetCaptured` `FUN_004ee800` -> vtable `+0x34c`), so
/// the side's copy of it is live only where the side sees it
/// (`FUN_004f9a60`).
#[must_use]
pub fn character_copies(
    world: &GameWorld,
    movement: &MovementState,
    side: Faction,
    system: SystemKey,
) -> Vec<(DatId, CharacterCopy)> {
    let alliance = side == Faction::Alliance;
    let Some(id) = world.systems.get(system).map(|value| value.dat_id) else {
        return Vec::new();
    };
    let other = |character: &crate::world::Character| {
        (character.is_alliance != alliance || character.is_captive) && !character.is_killed
    };
    let mut copies: Vec<(DatId, CharacterCopy)> = Vec::new();
    for (key, fleet) in &world.fleets {
        if fleet.location != system || movement.is_in_transit(key) {
            continue;
        }
        for character in fleet
            .characters
            .iter()
            .filter_map(|character| world.characters.get(*character))
            .filter(|character| other(character))
        {
            let ship = aboard_name(world, key, character);
            copies.push((
                character.dat_id,
                CharacterCopy {
                    system: Some(id),
                    ship,
                },
            ));
        }
    }
    for character in world.characters.values().filter(|character| {
        other(character)
            && character.current_fleet.is_none()
            && character.current_system == Some(system)
    }) {
        copies.push((
            character.dat_id,
            CharacterCopy {
                system: Some(id),
                ship: None,
            },
        ));
    }
    copies.sort_by_key(|(id, _)| id.raw());
    copies
}

/// The name a copy gives `character` aboard `fleet`: the ship it rides
/// (A0 personnel-captured/finder-day628: "Jan Dodonna - Punisher"). port:
/// one not seated names its fleet.
fn aboard_name(
    world: &GameWorld,
    fleet: crate::ids::FleetKey,
    character: &crate::world::Character,
) -> Option<String> {
    character
        .ship
        .and_then(|seat| {
            let index = world
                .fleets
                .get(fleet)?
                .capital_ships
                .iter()
                .position(|ship| ship.id == seat)?;
            world.ship_name(fleet, index)
        })
        .or_else(|| world.fleet_name(fleet))
        .map(str::to_owned)
}

/// The copy `side` takes of its own captive `character` where it stands
/// now, with the system whose seen bit makes that copy live (none for an
/// abode or a trip). A captive in an abode is copied in it ("Name -
/// Jabba's Palace": `FUN_00465bb0`'s abode rule gives the abode's name
/// alone); one on a trip in its destination, as `FUN_00556390` makes the
/// destination its container. port: the abode's name stands in the
/// copy's `ship`, with no system.
fn own_captive_copy(
    world: &GameWorld,
    movement: &MovementState,
    missions: &crate::missions::MissionState,
    key: crate::ids::CharacterKey,
) -> Option<(Option<SystemKey>, CharacterCopy)> {
    let character = world.characters.get(key)?;
    let system_id = |system: SystemKey| world.systems.get(system).map(|value| value.dat_id);
    let abode = |abode: crate::ids::AbodeKey| {
        world.abodes.get(abode).map(|value| CharacterCopy {
            system: None,
            ship: Some(value.name.clone()),
        })
    };
    if let Some(at) = character.current_abode {
        return Some((None, abode(at)?));
    }
    if let Some(fleet_key) = character.current_fleet {
        let fleet = world.fleets.get(fleet_key)?;
        if !movement.is_in_transit(fleet_key) {
            let ship = aboard_name(world, fleet_key, character);
            return Some((
                Some(fleet.location),
                CharacterCopy {
                    system: Some(system_id(fleet.location)?),
                    ship,
                },
            ));
        }
        return None;
    }
    if let Some(at) = character.current_system {
        return Some((
            Some(at),
            CharacterCopy {
                system: Some(system_id(at)?),
                ship: None,
            },
        ));
    }
    let trip = missions
        .escapees()
        .iter()
        .find(|transit| transit.character == key)
        .map(|transit| (transit.to, transit.abode))
        .or_else(|| {
            missions
                .en_route()
                .iter()
                .find(|transit| transit.member == crate::missions::MissionMember::Character(key))
                .map(|transit| (transit.to, transit.abode))
        })?;
    match trip {
        (_, Some(at)) => Some((None, abode(at)?)),
        (to, None) => Some((
            None,
            CharacterCopy {
                system: Some(system_id(to)?),
                ship: None,
            },
        )),
    }
}

/// `side`'s copies of its own captives (A0 2026-11-18-edited-saves-7 b-19,
/// e4f53d49b065: on day 31 the Alliance's Finder reads "Luke Skywalker -
/// Jabba's Palace" and the like, nine days after Jabba delivered them to
/// Vader; the game's day-34 save holds the Alliance's copies of the four
/// among its copy of the palace's children while their masters stand at
/// Coruscant).
///
/// `Character_SetCaptured` (`FUN_004ee800`) sets `+0xac` bit 0 and syncs
/// both copies while the side's copy is still live (its owner is the
/// side), then gives the captive its captor's side (`FUN_004eec60` ->
/// `FUN_004f6f40`, whose vtable `+0xf0` reaches `FUN_004fae40` ->
/// `FUN_004f9a60`): from then on the side's copy is live only where the
/// side sees the captive. A captive the side does not see keeps the copy
/// it held when it froze, with its Captured bit (A0 personnel-captured/
/// finder-day628: "Jan Dodonna - Punisher ( Captured )").
///
/// port: once a day, before the system copies are rebuilt. A captive with
/// no copy yet (taken since the day before, when its copy was live as the
/// side's own), or whose copy stands in a system the side sees, takes a
/// copy where it stands now; one where the side sees it is left to the
/// rebuilt system copy ([`character_copies`]). hyp: a live copy follows
/// the move that takes the captive out of sight before it freezes, as the
/// four palace copies did. A character that is not (or no longer) a
/// captive has no copy: the side owns it and reads it live.
fn record_own_captives(
    world: &mut GameWorld,
    movement: &MovementState,
    missions: &crate::missions::MissionState,
    side: Faction,
) {
    let alliance = side == Faction::Alliance;
    let seen = |id: DatId| {
        world
            .systems
            .iter()
            .find(|(_, value)| value.dat_id == id)
            .is_some_and(|(system, _)| system_seen(world, movement, side, system))
    };
    let mut dropped = Vec::new();
    let mut taken = Vec::new();
    for (key, character) in &world.characters {
        if character.is_alliance != alliance {
            continue;
        }
        if !character.is_captive || character.is_killed {
            dropped.push(character.dat_id);
            continue;
        }
        let was_live = world
            .object_views
            .character(side, character.dat_id)
            .is_none_or(|copy| copy.system.is_some_and(seen));
        if !was_live {
            continue;
        }
        let Some((at, copy)) = own_captive_copy(world, movement, missions, key) else {
            continue;
        };
        if at.is_some_and(|system| system_seen(world, movement, side, system)) {
            continue;
        }
        taken.push((character.dat_id, copy));
    }
    if let Some(map) = world.object_views.characters_mut(side) {
        for id in &dropped {
            map.remove(id);
        }
        map.extend(taken.iter().cloned());
    }
    // port: an own captive's copy keeps no state bits or command word; its
    // row's word is Captured.
    let cleared: Vec<DatId> = dropped
        .into_iter()
        .chain(taken.into_iter().map(|(id, _)| id))
        .collect();
    if let Some(map) = world.object_views.character_states_mut(side) {
        for id in &cleared {
            map.remove(id);
        }
    }
    if let Some(map) = world.object_views.character_commands_mut(side) {
        for id in &cleared {
            map.remove(id);
        }
    }
}

/// Takes `copies` into `side`'s character copies as each system's copy is
/// rebuilt: a character the side's copy put in one of those systems that
/// is not there now is left in no place ("Location Unknown").
fn record_characters(
    world: &mut GameWorld,
    side: Faction,
    copies: Vec<(DatId, Vec<(DatId, CharacterCopy)>)>,
) {
    use crate::character_command::CommandKind;
    let states: Vec<(DatId, CharacterCopyState, CommandKind)> = copies
        .iter()
        .flat_map(|(_, list)| list.iter())
        .filter_map(|(id, _)| {
            let character = world
                .characters
                .values()
                .find(|value| value.dat_id == *id)?;
            Some((
                *id,
                CharacterCopyState::of(character, false),
                character.command,
            ))
        })
        .collect();
    let Some(map) = world.object_views.characters_mut(side) else {
        return;
    };
    let mut lost = Vec::new();
    for (system, _) in &copies {
        for (id, copy) in map.iter_mut() {
            if copy.system == Some(*system) {
                *copy = CharacterCopy::default();
                lost.push(*id);
            }
        }
    }
    for (_, list) in copies {
        map.extend(list);
    }
    if let Some(map) = world.object_views.character_commands_mut(side) {
        for id in &lost {
            map.remove(id);
        }
        for (id, _, command) in &states {
            map.remove(id);
            if *command != CommandKind::None {
                map.insert(*id, *command);
            }
        }
    }
    if let Some(map) = world.object_views.character_states_mut(side) {
        for id in lost {
            map.remove(&id);
        }
        map.extend(
            states
                .into_iter()
                .map(|(id, state, _)| (id, state))
                .filter(|(_, state)| *state != CharacterCopyState::default()),
        );
    }
}

/// A character `side` saw leave for a system it does not see: its copy is
/// frozen en route in the destination (`FUN_00556390` makes the destination
/// its container; `FUN_004f9a60` freezes it with `+0x50` bit 4 set), so the
/// Personnel Finder reads "Name - Destination ( Enroute ) "
/// (`FUN_00465bb0`, `0x8744`).
fn record_traveller(world: &mut GameWorld, side: Faction, id: DatId, to: DatId) {
    let Some((state, command)) = world
        .characters
        .values()
        .find(|value| value.dat_id == id)
        .map(|character| (CharacterCopyState::of(character, true), character.command))
    else {
        return;
    };
    if let Some(map) = world.object_views.character_commands_mut(side) {
        map.remove(&id);
        if command != crate::character_command::CommandKind::None {
            map.insert(id, command);
        }
    }
    if let Some(map) = world.object_views.characters_mut(side) {
        map.insert(
            id,
            CharacterCopy {
                system: Some(to),
                ship: None,
            },
        );
    }
    if let Some(map) = world.object_views.character_states_mut(side) {
        map.insert(id, state);
    }
}

/// Seeds each side's character copies from its system and fleet copies
/// (the v116 to v120 migrations): each character a copy holds, in that
/// system; one aboard a copied fleet is named by its fleet, as an older
/// copy kept no seat.
pub fn seed_characters_from_copies(world: &mut GameWorld) {
    for side in [Faction::Alliance, Faction::Empire] {
        let views = &world.object_views;
        let mut copies: Vec<(DatId, CharacterCopy)> = Vec::new();
        if let Some(systems) = views.systems(side) {
            for (system, view) in systems {
                for character in &view.characters {
                    copies.push((
                        *character,
                        CharacterCopy {
                            system: Some(*system),
                            ship: None,
                        },
                    ));
                }
            }
        }
        if let Some(details) = views.system_details_map(side) {
            for (system, copy) in details {
                for fleet in copy
                    .fleets
                    .iter()
                    .filter(|fleet| fleet.is_alliance != (side == Faction::Alliance))
                {
                    for character in &fleet.characters {
                        copies.push((
                            *character,
                            CharacterCopy {
                                system: Some(*system),
                                ship: Some(fleet.name.clone()),
                            },
                        ));
                    }
                }
            }
        }
        // port: a character two copies hold takes the one of the system
        // with the highest id, so the seed does not hang on map order.
        copies.sort_by_key(|(id, copy)| (id.raw(), copy.system.map(DatId::raw)));
        if let Some(map) = world.object_views.characters_mut(side) {
            map.extend(copies);
        }
    }
}

/// A side's full copy of a system's fleets and missions (v120), kept with
/// its [`SystemExtras`]: what the side saw then, even after a fleet has
/// moved or died. The original keeps a side's copy of an object until the
/// side has seen it destroyed: `GameObj_SetReadyForDelete` (`FUN_004f82d0`
/// -> `FUN_004f6da0` -> `FUN_004f6c70`) needs `+0x50` bit 3 (destroyed) on
/// all three copies (`FUN_00539fd0(0..2)`), which a frozen copy gets only
/// when it next syncs; only then `Event31A` (`FUN_0057bd00` ->
/// `FUN_004f83b0`) removes the three copies (`FUN_0053f3a0` ->
/// `FUN_00585c70`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemDetails {
    pub fleets: Vec<FleetDetail>,
    pub missions: Vec<MissionDetail>,
    /// The last fleet serial the galaxy had given when the copy was taken:
    /// a fleet made later in the system, while the copy is frozen, takes
    /// an empty copy ([`advance_extras`]).
    pub last_serial: u64,
}

impl SystemDetails {
    /// `system`'s fleets and missions in full, as copied now. port: a
    /// mission's side is its faction's side bits, 1 or 2 (`+0x24` bits
    /// 6..7).
    #[must_use]
    pub fn of(
        world: &GameWorld,
        movement: &MovementState,
        missions: &crate::missions::MissionState,
        transport: &TroopTransportState,
        system: SystemKey,
    ) -> Self {
        let member = |member: &crate::missions::MissionMember| match member {
            crate::missions::MissionMember::Character(key) => world
                .characters
                .get(*key)
                .map(|value| MemberCopy::Character(value.dat_id)),
            crate::missions::MissionMember::SpecialForce(key) => world
                .special_forces
                .get(*key)
                .map(|value| MemberCopy::SpecialForce(value.class_dat_id)),
        };
        let mut fleets: Vec<FleetDetail> = world
            .fleets
            .iter()
            .filter(|(key, fleet)| fleet.location == system && !movement.is_in_transit(*key))
            .map(|(key, fleet)| FleetDetail {
                serial: world.fleet_serial(key),
                name: world.fleet_name(key).unwrap_or_default().to_owned(),
                is_alliance: fleet.is_alliance,
                ships: fleet
                    .capital_ships
                    .iter()
                    .filter(|ship| ship.alive)
                    .filter_map(|ship| {
                        Some(ShipCopy {
                            class: world.capital_ship_classes.get(ship.class)?.dat_id,
                            name: ship.name.clone(),
                            hull: ship.hull_current,
                        })
                    })
                    .collect(),
                fighters: fleet
                    .fighters
                    .iter()
                    .filter_map(|entry| {
                        Some((world.fighter_classes.get(entry.class)?.dat_id, entry.count))
                    })
                    .collect(),
                troops: transport
                    .cargo(key)
                    .iter()
                    .filter_map(|troop| world.troops.get(*troop).map(|value| value.class_dat_id))
                    .collect(),
                characters: fleet
                    .characters
                    .iter()
                    .filter_map(|character| {
                        world.characters.get(*character).map(|value| value.dat_id)
                    })
                    .collect(),
            })
            .collect();
        fleets.sort_by_key(|fleet| fleet.serial);
        let mut copied: Vec<MissionDetail> = missions
            .missions()
            .iter()
            .filter(|mission| mission.target_system == system)
            .map(|mission| MissionDetail {
                id: mission.id,
                side: match mission.faction {
                    crate::missions::MissionFaction::Alliance => 1,
                    crate::missions::MissionFaction::Empire => 2,
                },
                kind: mission.kind,
                team: mission.team.iter().filter_map(&member).collect(),
                decoys: mission.decoys.iter().filter_map(&member).collect(),
            })
            .collect();
        copied.sort_by_key(|mission| mission.id);
        Self {
            fleets,
            missions: copied,
            last_serial: world
                .fleets
                .keys()
                .map(|key| world.fleet_serial(key))
                .max()
                .unwrap_or(0),
        }
    }
}

/// The day's fleet, mission and traveller copies (`FUN_004f9a60`): each
/// system's extras follow its seen bit as [`compute_systems`] does; then
/// each other-side unit in hyperspace that left a system the side sees for
/// one it does not is copied, en route, into that destination's frozen
/// extras (A0 OBJ-02-C010's frozen en route regiment, for squadrons,
/// special forces and characters).
///
/// port: a squadron's transit names its origin; a member's does not, so a
/// character or special force counts as seen leaving when the side's live
/// copy of a system held it the day before (a special force by its class).
pub fn advance_extras(
    world: &mut GameWorld,
    movement: &MovementState,
    missions: &crate::missions::MissionState,
    transport: &TroopTransportState,
) {
    let squadrons = &transport.squadrons;
    for side in [Faction::Alliance, Faction::Empire] {
        let alliance = side == Faction::Alliance;
        let mut updates: Vec<(DatId, SystemExtras)> = Vec::new();
        let Some(old) = world.object_views.extras(side) else {
            continue;
        };
        for (key, system) in &world.systems {
            let live = system_seen(world, movement, side, key);
            let previous = old.get(&system.dat_id);
            if live || previous.is_some_and(|extras| extras.live) {
                let extras = SystemExtras::of(world, movement, missions, key, live);
                if previous != Some(&extras) {
                    updates.push((system.dat_id, extras));
                }
            }
        }
        // The day's pushes copy the system's extras too (Event319_Handle).
        for (pushed, system) in &world.object_views.pushes {
            if *pushed == side {
                if let Some(value) = world.systems.get(*system) {
                    let live = system_seen(world, movement, side, *system);
                    updates.push((
                        value.dat_id,
                        SystemExtras::of(world, movement, missions, *system, live),
                    ));
                }
            }
        }
        // The travellers seen leaving.
        let seen_live: Vec<DatId> = world
            .object_views
            .systems(side)
            .map(|views| {
                views
                    .iter()
                    .filter(|(_, view)| view.live)
                    .map(|(id, _)| *id)
                    .collect()
            })
            .unwrap_or_default();
        let held_before = |is_character: bool, id: DatId| {
            world.object_views.systems(side).is_some_and(|views| {
                seen_live.iter().any(|system| {
                    views.get(system).is_some_and(|view| {
                        if is_character {
                            view.characters.contains(&id)
                        } else {
                            view.special_forces.iter().any(|(class, _)| *class == id)
                        }
                    })
                })
            })
        };
        let mut travellers: Vec<(SystemKey, TravellerCopy)> = Vec::new();
        for transit in squadrons.transits() {
            let crate::squadrons::SquadronPlace::Planet(to) = transit.to else {
                continue;
            };
            let Some(class) = world.fighter_classes.get(transit.class) else {
                continue;
            };
            if class.is_alliance != alliance
                && system_seen(world, movement, side, transit.origin)
                && !system_seen(world, movement, side, to)
            {
                travellers.push((
                    to,
                    TravellerCopy::Squadron {
                        class: class.dat_id,
                        is_alliance: class.is_alliance,
                        count: transit.count,
                        arrival: transit.arrival_tick,
                    },
                ));
            }
        }
        for transit in missions.en_route() {
            if transit.abode.is_some() || system_seen(world, movement, side, transit.to) {
                continue;
            }
            match transit.member {
                crate::missions::MissionMember::Character(key) => {
                    let Some(character) = world.characters.get(key) else {
                        continue;
                    };
                    if character.is_alliance != alliance && held_before(true, character.dat_id) {
                        travellers.push((
                            transit.to,
                            TravellerCopy::Character {
                                id: character.dat_id,
                                arrival: transit.arrival,
                            },
                        ));
                    }
                }
                crate::missions::MissionMember::SpecialForce(key) => {
                    let Some(unit) = world.special_forces.get(key) else {
                        continue;
                    };
                    if unit.is_alliance != alliance && held_before(false, unit.class_dat_id) {
                        travellers.push((
                            transit.to,
                            TravellerCopy::SpecialForce {
                                class: unit.class_dat_id,
                                is_alliance: unit.is_alliance,
                                arrival: transit.arrival,
                            },
                        ));
                    }
                }
            }
        }
        // The full copies follow the extras (FUN_004f9a60 on each fleet and
        // mission there).
        let details: Vec<(DatId, SystemDetails)> = updates
            .iter()
            .filter_map(|(id, _)| {
                let (key, _) = world
                    .systems
                    .iter()
                    .find(|(_, value)| value.dat_id == *id)?;
                Some((
                    *id,
                    SystemDetails::of(world, movement, missions, transport, key),
                ))
            })
            .collect();
        // A0 2026-11-14-stale-views/06-kothlis-fleet16-d374-montage: an
        // Imperial fleet made in Kothlis after the Alliance last saw it
        // shows in the frozen copy with all four of its tabs empty. A new
        // object takes its side copies at once, frozen (FUN_0053f760:
        // FUN_00540630(2)), with its container; its ships, unfinished then
        // (+0x50 bit 2 clear), never sync into it (FUN_005404d0). port: a
        // fleet with a serial above the copy's last, in orbit there, takes
        // a copy with its name only; one that flew in from elsewhere does
        // not (hyp: its container change does not reach a frozen copy).
        let refreshed: std::collections::HashSet<DatId> =
            details.iter().map(|(id, _)| *id).collect();
        record_own_captives(world, movement, missions, side);
        // The character copies follow every rebuilt or live system copy.
        let characters: Vec<(DatId, Vec<(DatId, CharacterCopy)>)> = world
            .systems
            .iter()
            .filter(|(key, value)| {
                refreshed.contains(&value.dat_id) || system_seen(world, movement, side, *key)
            })
            .map(|(key, value)| (value.dat_id, character_copies(world, movement, side, key)))
            .collect();
        record_characters(world, side, characters);
        let mut made: Vec<(DatId, FleetDetail)> = Vec::new();
        if let Some(map) = world.object_views.details_mut(side) {
            map.extend(details);
        }
        if let Some(map) = world.object_views.system_details_map(side) {
            for (key, fleet) in &world.fleets {
                if fleet.is_alliance == alliance || movement.is_in_transit(key) {
                    continue;
                }
                let Some(id) = world.systems.get(fleet.location).map(|value| value.dat_id) else {
                    continue;
                };
                let serial = world.fleet_serial(key);
                let Some(copy) = map.get(&id) else {
                    continue;
                };
                if refreshed.contains(&id)
                    || serial <= copy.last_serial
                    || copy.fleets.iter().any(|known| known.serial == serial)
                {
                    continue;
                }
                made.push((
                    id,
                    FleetDetail {
                        serial,
                        name: world.fleet_name(key).unwrap_or_default().to_owned(),
                        is_alliance: fleet.is_alliance,
                        ships: Vec::new(),
                        fighters: Vec::new(),
                        troops: Vec::new(),
                        characters: Vec::new(),
                    },
                ));
            }
        }
        if let Some(map) = world.object_views.details_mut(side) {
            for (id, fleet) in made {
                if let Some(copy) = map.get_mut(&id) {
                    copy.fleets.push(fleet);
                }
            }
        }
        for (to, traveller) in &travellers {
            if let TravellerCopy::Character { id, .. } = traveller {
                if let Some(to) = world.systems.get(*to).map(|value| value.dat_id) {
                    record_traveller(world, side, *id, to);
                }
            }
        }
        let Some(extras) = world.object_views.extras_mut(side) else {
            continue;
        };
        for (id, value) in updates {
            extras.insert(id, value);
        }
        for (to, traveller) in travellers {
            let Some(id) = world.systems.get(to).map(|value| value.dat_id) else {
                continue;
            };
            let entry = extras.entry(id).or_default();
            if !entry.travellers.contains(&traveller) {
                entry.travellers.push(traveller);
            }
        }
    }
}

/// Seeds each side's fleet and mission copies from the current state for
/// the systems its fog shows or it sees (the v118 migration; a fixture's
/// start), live or frozen by `FUN_004f9a60`.
pub fn seed_extras_from_fog(
    world: &mut GameWorld,
    movement: &MovementState,
    missions: &crate::missions::MissionState,
    fogs: [&FogState; 2],
) {
    seed_extras_only(world, movement, missions, fogs);
}

/// Seeds each side's full fleet and mission copies (`SystemDetails`) for
/// the systems its fog shows or it sees (the v116 to v119 migrations; a new
/// game's and a fixture's start), as copied now.
pub fn seed_details_from_fog(
    world: &mut GameWorld,
    movement: &MovementState,
    missions: &crate::missions::MissionState,
    transport: &TroopTransportState,
    fogs: [&FogState; 2],
) {
    for fog in fogs {
        let side = fog.faction;
        let updates: Vec<(DatId, SystemDetails)> = world
            .systems
            .iter()
            .filter(|(key, _)| system_seen(world, movement, side, *key) || fog.is_visible(*key))
            .map(|(key, system)| {
                (
                    system.dat_id,
                    SystemDetails::of(world, movement, missions, transport, key),
                )
            })
            .collect();
        let characters: Vec<(DatId, Vec<(DatId, CharacterCopy)>)> = world
            .systems
            .iter()
            .filter(|(key, _)| system_seen(world, movement, side, *key) || fog.is_visible(*key))
            .map(|(key, value)| (value.dat_id, character_copies(world, movement, side, key)))
            .collect();
        if let Some(map) = world.object_views.details_mut(side) {
            map.extend(updates);
        }
        record_characters(world, side, characters);
    }
}

fn seed_extras_only(
    world: &mut GameWorld,
    movement: &MovementState,
    missions: &crate::missions::MissionState,
    fogs: [&FogState; 2],
) {
    for fog in fogs {
        let side = fog.faction;
        let updates: Vec<(DatId, SystemExtras)> = world
            .systems
            .iter()
            .filter_map(|(key, system)| {
                let live = system_seen(world, movement, side, key);
                (live || fog.is_visible(key)).then(|| {
                    (
                        system.dat_id,
                        SystemExtras::of(world, movement, missions, key, live),
                    )
                })
            })
            .collect();
        if let Some(extras) = world.object_views.extras_mut(side) {
            extras.extend(updates);
        }
    }
}

/// The day's system view changes (`FUN_004f9a60` on each system): a live
/// copy, or one that was live and freezes now, takes the system's state.
#[must_use]
pub fn compute_systems(
    world: &GameWorld,
    movement: &MovementState,
) -> Vec<(Faction, DatId, SystemView)> {
    let mut out = Vec::new();
    for side in [Faction::Alliance, Faction::Empire] {
        let Some(views) = world.object_views.systems(side) else {
            continue;
        };
        for (key, system) in &world.systems {
            let live = system_seen(world, movement, side, key);
            let old = views.get(&system.dat_id);
            if !live && !old.is_some_and(|view| view.live) {
                continue;
            }
            let view = SystemView::of(world, system, live);
            if old != Some(&view) {
                out.push((side, system.dat_id, view));
            }
        }
    }
    out
}

/// Puts each copy in its side's views. A live copy takes the explored bit
/// (hyp: the master is explored for every system, so a copy synced from it
/// is explored, `ghidra/notes/fog-of-war.md`, "Untraced") and the
/// system's destroyed bit; a copy that freezes keeps its explored bit.
fn apply_systems(world: &mut GameWorld, updates: Vec<(Faction, DatId, SystemView)>) {
    for (side, id, view) in updates {
        let destroyed = world
            .systems
            .values()
            .find(|value| value.dat_id == id)
            .is_some_and(|value| value.is_destroyed);
        // A live copy is synced from the system, explored; one that
        // freezes keeps the bit it had.
        let explored = view.live
            || world
                .object_views
                .system_bits
                .get(side, id)
                .is_some_and(|bits| bits.explored);
        world.object_views.system_bits.set(
            side,
            id,
            SystemCopyBits {
                explored,
                destroyed,
            },
        );
        if let Some(views) = world.object_views.systems_mut(side) {
            views.insert(id, view);
        }
    }
}

/// Whether `side` sees `system` explored: its own system, else its copy's
/// explored bit (`FUN_0042b330` reads `+0x88` bit 1 of the viewer's copy).
/// port: a system the side holds no copy of (a test's or a fixture's
/// world) takes the system's own `exploration_status`, as the port did
/// before the setup copies (`setup`).
#[must_use]
pub fn side_explored(world: &GameWorld, side: Faction, system: SystemKey) -> bool {
    let Some(value) = world.systems.get(system) else {
        return false;
    };
    let own = value.control.is_controlled_by(side);
    if own || world.object_views.see_everything {
        return own || value.exploration_status == crate::dat::ExplorationStatus::Explored;
    }
    match world.object_views.system_bits.get(side, value.dat_id) {
        Some(bits) => bits.explored,
        None => value.exploration_status == crate::dat::ExplorationStatus::Explored,
    }
}

/// Whether `side`'s copy of `system` shows it destroyed (`+0x50` bit 3 of
/// the viewer's copy, `FUN_0045bbb0`). port: a system with no bits takes
/// its own.
#[must_use]
pub fn side_destroyed(world: &GameWorld, side: Faction, system: SystemKey) -> bool {
    let Some(value) = world.systems.get(system) else {
        return false;
    };
    if value.control.is_controlled_by(side) || world.object_views.see_everything {
        return value.is_destroyed;
    }
    world
        .object_views
        .system_bits
        .get(side, value.dat_id)
        .map_or(value.is_destroyed, |bits| bits.destroyed)
}

/// Whether `system` is a core system (SYSTEMSD family `0x90..0x91`,
/// `CoreSystem` vtable `0x00663698`); the rest are Rim systems (`0x92`).
/// port: the loader keeps a system's sequence id alone and its family only
/// as the explored bit, which play changes; every family-`0x90` record of
/// SYSTEMSD lies in a Core sector (SECTORSD group 1), so the sector's group
/// stands for the family.
fn is_core(world: &GameWorld, system: &crate::world::System) -> bool {
    world
        .sectors
        .get(system.sector)
        .is_some_and(|sector| sector.group == crate::dat::SectorGroup::Core)
}

/// Yavin's sequence id (`0x92000121`, `FUN_0051b990` at `0x51bba6`); the
/// loaded galaxy keeps the low 24 bits.
const YAVIN_ID: u32 = 0x121;

/// The setup push (`FUN_0051b990`, setup stage `0x18` through
/// `FUN_0051b940`, once per side), for each system the side holds no copy
/// of yet (its own and those its fleets stand in are live already):
/// - a core system (`FUN_00566c00`, mode 1) takes a full copy of its fields
///   and, through `FUN_00539ac0`, of its facilities (`0x20..0x2f`), but not
///   of its squadrons, special forces or characters, which no setup range
///   holds;
/// - a Rim system (`FUN_0050c8a0`, mode 2) takes a bare copy: it exists,
///   but holds no side, values or explored bit (hyp: the copy's initial
///   fields are zero).
///
/// Then the Empire's level-10 push of Yavin (`Event319_Handle(10, 2)`),
/// queued for the next [`advance`]. See `ghidra/notes/fog-of-war.md`.
pub fn setup(world: &mut GameWorld) {
    let mut copies = Vec::new();
    for side in [Faction::Alliance, Faction::Empire] {
        for system in world.systems.values() {
            if world.object_views.system(side, system.dat_id).is_some() {
                continue;
            }
            let (view, bits) = if is_core(world, system) {
                let mut view = SystemView::of(world, system, false);
                view.fighters.clear();
                view.special_forces.clear();
                view.characters.clear();
                let bits = SystemCopyBits {
                    explored: system.exploration_status == crate::dat::ExplorationStatus::Explored,
                    destroyed: system.is_destroyed,
                };
                (view, bits)
            } else {
                (SystemView::bare(), SystemCopyBits::default())
            };
            copies.push((side, system.dat_id, view, bits));
        }
    }
    for (side, id, view, bits) in copies {
        world.object_views.system_bits.set(side, id, bits);
        if let Some(views) = world.object_views.systems_mut(side) {
            views.insert(id, view);
        }
    }
    if let Some((yavin, _)) = world
        .systems
        .iter()
        .find(|(_, value)| value.dat_id.raw() & 0x00FF_FFFF == YAVIN_ID)
    {
        world.object_views.queue_push(Faction::Empire, yavin);
    }
}

/// A new campaign's first views, in the app and the headless runners
/// alike: the setup push ([`setup`], `FUN_0051b990` at setup stage `0x18`)
/// and the Empire's push of Yavin, taken at once by the day's view step.
pub fn start(
    world: &mut GameWorld,
    movement: &MovementState,
    missions: &crate::missions::MissionState,
    transport: &TroopTransportState,
    uprisings: &crate::uprising::UprisingState,
) {
    setup(world);
    advance_extras(world, movement, missions, transport);
    advance_with_uprisings(world, transport, movement, uprisings);
}

/// A change to one side's views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewUpdate {
    /// The side's copy of `id` takes `view` (a sync or a rebuild).
    Set {
        side: Faction,
        id: DatId,
        view: ObjectView,
    },
    /// The side's copy of `id` is dropped.
    Remove { side: Faction, id: DatId },
}

/// The other side's regiments, with their ids, as `side` would copy them
/// now, and whether that copy is live (`FUN_004f9a60`).
fn regiment_snapshots(
    world: &GameWorld,
    transport: &TroopTransportState,
    side: Faction,
    seen: &dyn Fn(SystemKey) -> bool,
) -> Vec<(DatId, RegimentView)> {
    let alliance = side == Faction::Alliance;
    let mut out = Vec::new();
    for (key, troop) in &world.troops {
        if troop.is_alliance == alliance {
            continue;
        }
        let Some(id) = world.object_views.regiment_id(key) else {
            continue;
        };
        let Some((system, place, arrival)) = regiment_place(world, transport, key) else {
            continue;
        };
        // FUN_004f9a60: live with the seen bit, complete (+0x50 bit 2: a
        // regiment exists only once built) and not en route (bit 4).
        let live = arrival.is_none() && seen(system);
        out.push((
            id,
            RegimentView {
                class: troop.class_dat_id,
                is_alliance: troop.is_alliance,
                strength: troop.regiment_strength,
                place,
                arrival,
                live,
            },
        ));
    }
    out
}

/// The day's view changes for both sides (`FUN_004f9a60` over every
/// object).
///
/// - A live copy takes the object's state.
/// - A copy that was live and is frozen now takes the change that froze it
///   (the master setter syncs the copy before the copy's state changes:
///   hyp, from A0 OBJ-02-C010, where a regiment's frozen copy reads
///   "Enroute, ETA Day 171") and is marked frozen.
/// - A frozen copy keeps its values. A copy that was never live is not
///   made.
/// - A copy of an object that is gone is dropped while the copy is live
///   (the side saw it go); a frozen one stays (hyp).
#[must_use]
pub fn compute(
    world: &GameWorld,
    transport: &TroopTransportState,
    movement: &MovementState,
) -> Vec<ViewUpdate> {
    let mut updates = Vec::new();
    for side in [Faction::Alliance, Faction::Empire] {
        let Some(views) = world.object_views.side(side) else {
            continue;
        };
        let seen = |system| system_seen(world, movement, side, system);
        let snapshots = regiment_snapshots(world, transport, side, &seen);
        let present: std::collections::HashSet<DatId> =
            snapshots.iter().map(|(id, _)| *id).collect();
        for (id, view) in snapshots {
            let was_live = views.get(&id).is_some_and(ObjectView::live);
            let view = ObjectView::Regiment(view);
            if (view.live() || was_live) && views.get(&id) != Some(&view) {
                updates.push(ViewUpdate::Set { side, id, view });
            }
        }
        let mut gone: Vec<DatId> = views
            .iter()
            .filter(|(id, view)| view.live() && !present.contains(id))
            .map(|(id, _)| *id)
            .collect();
        gone.sort_by_key(|id| id.raw());
        updates.extend(gone.into_iter().map(|id| ViewUpdate::Remove { side, id }));
    }
    updates
}

/// An intelligence push into `side`'s copies at `system`
/// (`Event319_Handle` `FUN_0050d5a0`): Espionage's level 10 and
/// Reconnaissance's level 8 both copy the system's regiments (child types
/// `0x01..0xfe`, `0x10..0x1f`). The copies take the object's state and keep
/// their live or frozen state.
#[must_use]
pub fn push_system(
    world: &GameWorld,
    transport: &TroopTransportState,
    movement: &MovementState,
    side: Faction,
    system: SystemKey,
) -> Vec<ViewUpdate> {
    let seen = |at| system_seen(world, movement, side, at);
    let Some(system_id) = world.systems.get(system).map(|value| value.dat_id) else {
        return Vec::new();
    };
    regiment_snapshots(world, transport, side, &seen)
        .into_iter()
        .filter(|(_, view)| view.place.system() == system_id)
        .map(|(id, view)| ViewUpdate::Set {
            side,
            id,
            view: ObjectView::Regiment(view),
        })
        .collect()
}

/// Applies `updates` in order.
pub fn apply(world: &mut GameWorld, updates: &[ViewUpdate]) {
    for update in updates {
        match update {
            ViewUpdate::Set { side, id, view } => {
                if let Some(views) = world.object_views.side_mut(*side) {
                    views.insert(*id, view.clone());
                }
            }
            ViewUpdate::Remove { side, id } => {
                if let Some(views) = world.object_views.side_mut(*side) {
                    views.remove(id);
                }
            }
        }
    }
}

/// The daily step, after movement, control and missions: number new
/// regiments, refresh both sides' views, then take the day's intelligence
/// pushes.
///
/// Returns each system copy the step synced (a live copy that changed, one
/// that froze, each push), for [`sync_uprisings`].
pub fn advance(
    world: &mut GameWorld,
    transport: &TroopTransportState,
    movement: &MovementState,
) -> Vec<(Faction, DatId)> {
    number_regiments(world);
    let mut updates = compute(world, transport, movement);
    let mut systems = compute_systems(world, movement);
    let mut pushed = Vec::new();
    for (side, system) in std::mem::take(&mut world.object_views.pushes) {
        updates.extend(push_system(world, transport, movement, side, system));
        // Event319_Handle levels 8 and 10 copy the whole system.
        if let Some(value) = world.systems.get(system) {
            let live = system_seen(world, movement, side, system);
            systems.push((side, value.dat_id, SystemView::of(world, value, live)));
            pushed.push((side, value.dat_id));
        }
    }
    apply(world, &updates);
    let mut synced: Vec<(Faction, DatId)> =
        systems.iter().map(|(side, id, _)| (*side, *id)).collect();
    apply_systems(world, systems);
    // Every push level syncs explored (System_SyncExplored FUN_0050fc80).
    for (side, id) in pushed {
        if let Some(mut bits) = world.object_views.system_bits.get(side, id) {
            bits.explored = true;
            world.object_views.system_bits.set(side, id, bits);
        }
    }
    synced.dedup();
    synced
}

/// The day's step ([`advance`]) and the uprising bit of each copy it
/// synced ([`sync_uprisings`]).
pub fn advance_with_uprisings(
    world: &mut GameWorld,
    transport: &TroopTransportState,
    movement: &MovementState,
    uprisings: &crate::uprising::UprisingState,
) {
    let synced = advance(world, transport, movement);
    sync_uprisings(world, uprisings, &synced);
}

/// Each of `synced`'s copies takes the system's uprising bit (`+0x88` bit
/// 2): the mode-1 sync (`FUN_0050e5b0` through `CoreSystem_vt24` /
/// `RimSystem_vt24`) copies it with the other fields. A frozen copy keeps
/// the bit it had.
pub fn sync_uprisings(
    world: &mut GameWorld,
    uprisings: &crate::uprising::UprisingState,
    synced: &[(Faction, DatId)],
) {
    for &(side, id) in synced {
        let Some((key, _)) = world.systems.iter().find(|(_, value)| value.dat_id == id) else {
            continue;
        };
        let uprising = uprisings.is_uprising(key);
        world.object_views.system_uprisings.set(side, id, uprising);
    }
}

/// Seeds every copy `side` holds with the system's uprising bit now (the
/// v130 migration: such a save kept no uprising bits). port: a frozen copy
/// of an older save takes today's state.
pub fn seed_uprisings(world: &mut GameWorld, uprisings: &crate::uprising::UprisingState) {
    let mut synced = Vec::new();
    for side in [Faction::Alliance, Faction::Empire] {
        if let Some(views) = world.object_views.systems(side) {
            synced.extend(views.keys().map(|id| (side, *id)));
        }
    }
    sync_uprisings(world, uprisings, &synced);
}

/// Whether `side` sees `system` in uprising (`+0x88` bit 2 of the viewer's
/// copy: `FUN_0045bbb0`'s marks 905/906, `FUN_0042b330`). Its own system, a
/// live copy and the debug mode show the system's own bit; a frozen copy
/// its own. port: a system the side holds no copy of (a test's or a
/// fixture's world) shows the system's own.
#[must_use]
pub fn side_uprising(
    world: &GameWorld,
    side: Faction,
    system: SystemKey,
    uprisings: &crate::uprising::UprisingState,
) -> bool {
    let Some(value) = world.systems.get(system) else {
        return false;
    };
    let own = uprisings.is_uprising(system);
    if value.control.is_controlled_by(side) || world.object_views.see_everything {
        return own;
    }
    match world.object_views.system(side, value.dat_id) {
        Some(view) if !view.live => world.object_views.system_uprisings.get(side, value.dat_id),
        _ => own,
    }
}

/// Seeds the views from the current state for the systems each side's fog
/// shows (the save v116 migration and a fixture's start): each other-side
/// regiment there takes a copy, live or frozen by `FUN_004f9a60`.
/// port: a v116 save kept no copies, so the frozen ones start from today.
pub fn seed_from_fog(
    world: &mut GameWorld,
    transport: &TroopTransportState,
    movement: &MovementState,
    fogs: [&FogState; 2],
) {
    number_regiments(world);
    let mut updates = Vec::new();
    for fog in fogs {
        let side = fog.faction;
        let seen = |at| system_seen(world, movement, side, at);
        for (id, view) in regiment_snapshots(world, transport, side, &seen) {
            let known = world
                .systems
                .iter()
                .find(|(_, value)| value.dat_id == view.place.system())
                .is_some_and(|(system, _)| fog.is_visible(system) || seen(system));
            if known {
                updates.push(ViewUpdate::Set {
                    side,
                    id,
                    view: ObjectView::Regiment(view),
                });
            }
        }
    }
    apply(world, &updates);
    seed_systems_from_fog(world, movement, fogs);
}

/// Seeds each side's system views from the current state for the systems
/// its fog shows or it sees (the v117 migration; part of
/// [`seed_from_fog`]), live or frozen by `FUN_004f9a60`.
pub fn seed_systems_from_fog(
    world: &mut GameWorld,
    movement: &MovementState,
    fogs: [&FogState; 2],
) {
    let mut systems = Vec::new();
    for fog in fogs {
        let side = fog.faction;
        for (key, system) in &world.systems {
            let live = system_seen(world, movement, side, key);
            if live || fog.is_visible(key) {
                systems.push((side, system.dat_id, SystemView::of(world, system, live)));
            }
        }
    }
    let seeded: Vec<(Faction, DatId)> = systems.iter().map(|(side, id, _)| (*side, *id)).collect();
    apply_systems(world, systems);
    // port: a seeded copy (an older save's, a fixture's) is explored as the
    // system is, as the port showed it before the copies held the bit.
    for (side, id) in seeded {
        let explored = world.systems.iter().any(|(key, value)| {
            value.dat_id == id
                && (value.exploration_status == crate::dat::ExplorationStatus::Explored
                    || system_seen(world, movement, side, key))
        });
        if let Some(mut bits) = world.object_views.system_bits.get(side, id) {
            bits.explored = explored;
            world.object_views.system_bits.set(side, id, bits);
        }
    }
}

#[cfg(test)]
#[path = "object_views_tests.rs"]
mod tests;
