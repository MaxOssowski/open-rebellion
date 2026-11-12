//! Source-faithful Galactic Encyclopedia catalog loading.
//!
//! The original window filters one shared game-object catalog by the family
//! byte stored in each compound object id. This module rebuilds that immutable
//! reference catalog from the shipped DAT tables and `TEXTSTRA.DLL` without
//! adding presentation-only data to [`rebellion_core::world::GameWorld`] or
//! changing the save format.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use dat_dumper::types::capital_ships::CapitalShipsFile;
use dat_dumper::types::defense_facilities::DefenseFacilitiesFile;
use dat_dumper::types::fighters::FightersFile;
use dat_dumper::types::fleets_seed::FleetsSeedFile;
use dat_dumper::types::major_characters::MajorCharactersFile;
use dat_dumper::types::manufacturing_facilities::ManufacturingFacilitiesFile;
use dat_dumper::types::minor_characters::MinorCharactersFile;
use dat_dumper::types::missions::MissionsFile;
use dat_dumper::types::production_facilities::ProductionFacilitiesFile;
use dat_dumper::types::special_forces::SpecialForcesFile;
use dat_dumper::types::systems::SystemsFile;
#[cfg(not(target_arch = "wasm32"))]
use dat_dumper::types::textstra;
use dat_dumper::types::troops::TroopsFile;

use crate::read_dat_file;

/// One original game-object entry available to the Encyclopedia index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncyclopediaCatalogEntry {
    /// Compound game-object identity. The high byte is the source family.
    pub object_id: u32,
    /// TEXTSTRA resource used for the visible name.
    pub text_resource_id: u16,
    /// Localized visible name.
    pub name: String,
}

impl EncyclopediaCatalogEntry {
    #[must_use]
    pub const fn family(&self) -> u8 {
        (self.object_id >> 24) as u8
    }
}

/// One of the seven original index controls (`0x6f..=0x75`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncyclopediaCategory {
    pub command_id: u16,
    pub label_resource_id: u16,
    pub label: String,
    pub(crate) family_range: Option<std::ops::Range<u8>>,
}

impl EncyclopediaCategory {
    #[must_use]
    pub fn contains(&self, entry: &EncyclopediaCatalogEntry) -> bool {
        self.family_range
            .as_ref()
            .is_none_or(|range| range.contains(&entry.family()))
    }
}

/// Immutable localized Encyclopedia index data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncyclopediaCatalog {
    pub title: String,
    pub topic_label: String,
    pub categories: [EncyclopediaCategory; 7],
    /// Global alphabetical order used by the native list control. Filtering a
    /// category preserves this order.
    pub entries: Vec<EncyclopediaCatalogEntry>,
    /// The mission classes whose record `+0x5c` (the hidden flag,
    /// `FUN_00520b70`) is set: `FUN_00422620` leaves them out of the
    /// window's list (A0 encyclopedia/category-4 has no Adrift, Autorouting,
    /// Bounty or Dagobah). The source audit still counts them.
    pub hidden_missions: Vec<u32>,
}

impl EncyclopediaCatalog {
    /// The entries the window lists: all but the hidden missions.
    pub fn listed_entries(&self) -> impl Iterator<Item = &EncyclopediaCatalogEntry> {
        self.entries
            .iter()
            .filter(|entry| !self.hidden_missions.contains(&entry.object_id))
    }

    #[must_use]
    pub fn category(&self, command_id: u16) -> Option<&EncyclopediaCategory> {
        self.categories
            .iter()
            .find(|category| category.command_id == command_id)
    }

    #[must_use]
    pub fn entries_for(&self, command_id: u16) -> Vec<&EncyclopediaCatalogEntry> {
        let Some(category) = self.category(command_id) else {
            return Vec::new();
        };
        self.entries
            .iter()
            .filter(|entry| category.contains(entry))
            .collect()
    }
}

/// Load the complete localized index directly from original game data.
///
/// This reference catalog is deliberately separate from campaign state. It is
/// safe to recreate after loading a save and cannot change bincode layouts.
///
/// # Errors
/// Returns an error if a required DAT table or required TEXTSTRA string is
/// absent or malformed.
pub fn load_encyclopedia_catalog(gdata_path: &Path) -> Result<EncyclopediaCatalog> {
    let strings = load_string_table(gdata_path)?;
    let required = |resource_id: u16| -> Result<String> {
        strings
            .get(&resource_id)
            .cloned()
            .with_context(|| format!("TEXTSTRA resource {resource_id:#06x} is missing"))
    };

    let categories = [
        category(0x6f, 0x1850, required(0x1850)?, None),
        category(0x70, 0x1855, required(0x1855)?, Some(0x90..0x98)),
        category(0x71, 0x1854, required(0x1854)?, Some(0x14..0x20)),
        category(0x72, 0x1852, required(0x1852)?, Some(0x20..0x30)),
        category(0x73, 0x1851, required(0x1851)?, Some(0x40..0x80)),
        category(0x74, 0x1856, required(0x1856)?, Some(0x10..0x14)),
        category(0x75, 0x1853, required(0x1853)?, Some(0x30..0x40)),
    ];

    let mut entries = Vec::new();
    let systems: SystemsFile = read_dat_file(&gdata_path.join("SYSTEMSD.DAT"))?;
    for record in systems.systems {
        push_entry(
            &mut entries,
            record.id,
            record.family_id,
            record.text_stra_dll_id,
            &strings,
        )?;
    }

    let capital_ships: CapitalShipsFile = read_dat_file(&gdata_path.join("CAPSHPSD.DAT"))?;
    for record in capital_ships.ships {
        push_entry(
            &mut entries,
            record.id,
            record.family_id,
            record.text_stra_dll_id,
            &strings,
        )?;
    }
    let fighters: FightersFile = read_dat_file(&gdata_path.join("FIGHTSD.DAT"))?;
    for record in fighters.fighters {
        push_entry(
            &mut entries,
            record.id,
            record.family_id,
            record.text_stra_dll_id,
            &strings,
        )?;
    }

    let defense: DefenseFacilitiesFile = read_dat_file(&gdata_path.join("DEFFACSD.DAT"))?;
    for record in defense.facilities {
        push_entry(
            &mut entries,
            record.id,
            record.family_id,
            record.text_stra_dll_id,
            &strings,
        )?;
    }
    let manufacturing: ManufacturingFacilitiesFile =
        read_dat_file(&gdata_path.join("MANFACSD.DAT"))?;
    for record in manufacturing.facilities {
        push_entry(
            &mut entries,
            record.id,
            record.family_id,
            record.text_stra_dll_id,
            &strings,
        )?;
    }
    let production: ProductionFacilitiesFile = read_dat_file(&gdata_path.join("PROFACSD.DAT"))?;
    for record in production.facilities {
        push_entry(
            &mut entries,
            record.id,
            record.family_id,
            record.text_stra_dll_id,
            &strings,
        )?;
    }

    let missions: MissionsFile = read_dat_file(&gdata_path.join("MISSNSD.DAT"))?;
    let mut hidden_missions = Vec::new();
    for record in missions.missions {
        if record.hidden != 0 {
            let family = u8::try_from(record.family_id)
                .with_context(|| format!("object family {:#x} does not fit one byte", record.family_id))?;
            hidden_missions.push(compound_object_id(record.id, family));
        }
        push_entry(
            &mut entries,
            record.id,
            record.family_id,
            record.text_stra_dll_id,
            &strings,
        )?;
    }

    let troops: TroopsFile = read_dat_file(&gdata_path.join("TROOPSD.DAT"))?;
    for record in troops.troops {
        push_entry(
            &mut entries,
            record.id,
            record.family_id,
            record.text_stra_dll_id,
            &strings,
        )?;
    }

    let major: MajorCharactersFile = read_dat_file(&gdata_path.join("MJCHARSD.DAT"))?;
    for record in major.characters {
        push_entry(
            &mut entries,
            record.id,
            record.family_id,
            record.text_stra_dll_id,
            &strings,
        )?;
    }
    let minor: MinorCharactersFile = read_dat_file(&gdata_path.join("MNCHARSD.DAT"))?;
    for record in minor.characters {
        push_entry(
            &mut entries,
            record.id,
            record.family_id,
            record.text_stra_dll_id,
            &strings,
        )?;
    }
    let special_forces: SpecialForcesFile = read_dat_file(&gdata_path.join("SPECFCSD.DAT"))?;
    for record in special_forces.units {
        push_entry(
            &mut entries,
            record.id,
            record.family_id,
            record.text_stra_dll_id,
            &strings,
        )?;
    }

    let fleets: FleetsSeedFile = read_dat_file(&gdata_path.join("FLEETSD.DAT"))?;
    push_fleet_entries(&mut entries, &fleets, &strings)?;

    entries.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.object_id.cmp(&right.object_id))
    });
    entries.dedup_by_key(|entry| entry.object_id);

    Ok(EncyclopediaCatalog {
        title: required(0x1842)?,
        topic_label: required(0x1843)?,
        categories,
        entries,
        hidden_missions,
    })
}

fn category(
    command_id: u16,
    label_resource_id: u16,
    label: String,
    family_range: Option<std::ops::Range<u8>>,
) -> EncyclopediaCategory {
    EncyclopediaCategory {
        command_id,
        label_resource_id,
        label,
        family_range,
    }
}

fn push_entry(
    entries: &mut Vec<EncyclopediaCatalogEntry>,
    record_id: u32,
    family_id: u32,
    text_resource_id: u16,
    strings: &HashMap<u16, String>,
) -> Result<()> {
    let family = u8::try_from(family_id)
        .with_context(|| format!("object family {family_id:#x} does not fit one byte"))?;
    let object_id = compound_object_id(record_id, family);
    let name = strings
        .get(&text_resource_id)
        .cloned()
        .with_context(|| format!("TEXTSTRA resource {text_resource_id:#06x} is missing"))?;
    entries.push(EncyclopediaCatalogEntry {
        object_id,
        text_resource_id,
        name,
    });
    Ok(())
}

/// `FUN_00422620` keeps a class record of family `0x08..0x0f`, the
/// FLEETSD range (`ghidra/notes/move-order.md`). The shipped file's record
/// 4 (`0x0800_0004`, TEXTSTRA 11523 "Fleet") is the class every fleet is
/// made from; its record 5 has family 0 and stays out. No category but All
/// Databases covers `0x08` (`FUN_0045f100`).
fn push_fleet_entries(
    entries: &mut Vec<EncyclopediaCatalogEntry>,
    fleets: &FleetsSeedFile,
    strings: &HashMap<u16, String>,
) -> Result<()> {
    for record in fleets
        .fleets
        .iter()
        .filter(|record| (0x08..0x10).contains(&record.family_id))
    {
        push_entry(
            entries,
            record.id,
            record.family_id,
            record.text_stra_dll_id,
            strings,
        )?;
    }
    Ok(())
}

pub(crate) const fn compound_object_id(record_id: u32, family: u8) -> u32 {
    if record_id >> 24 == 0 {
        ((family as u32) << 24) | record_id
    } else {
        record_id
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_string_table(gdata_path: &Path) -> Result<HashMap<u16, String>> {
    let path = gdata_path.join("TEXTSTRA.DLL");
    textstra::load_strings(&path)
        .with_context(|| format!("loading Encyclopedia strings from {}", path.display()))
}

#[cfg(target_arch = "wasm32")]
fn load_string_table(_gdata_path: &Path) -> Result<HashMap<u16, String>> {
    Ok(super::WASM_STRING_TABLE.lock().unwrap().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_catalog() -> EncyclopediaCatalog {
        let categories = [
            category(0x6f, 0x1850, "All Databases".into(), None),
            category(0x70, 0x1855, "System Database".into(), Some(0x90..0x98)),
            category(0x71, 0x1854, "Ship Database".into(), Some(0x14..0x20)),
            category(0x72, 0x1852, "Facilities Database".into(), Some(0x20..0x30)),
            category(0x73, 0x1851, "Missions Database".into(), Some(0x40..0x80)),
            category(0x74, 0x1856, "Troop Database".into(), Some(0x10..0x14)),
            category(0x75, 0x1853, "Personnel Database".into(), Some(0x30..0x40)),
        ];
        EncyclopediaCatalog {
            title: "Galactic Encyclopedia".into(),
            topic_label: "Topic".into(),
            categories,
            entries: vec![
                EncyclopediaCatalogEntry {
                    object_id: 0x1000_0001,
                    text_resource_id: 1,
                    name: "Alliance Army Regiment".into(),
                },
                EncyclopediaCatalogEntry {
                    object_id: 0x1400_0001,
                    text_resource_id: 2,
                    name: "Alliance Dreadnaught".into(),
                },
                EncyclopediaCatalogEntry {
                    object_id: 0x9000_0001,
                    text_resource_id: 3,
                    name: "Allyuen".into(),
                },
            ],
            hidden_missions: Vec::new(),
        }
    }

    #[test]
    fn the_window_lists_every_entry_but_the_hidden_missions() {
        // FUN_00422620: a mission class with record +0x5c set stays out
        // (A0 encyclopedia/category-4: no Adrift in Missions Database).
        let mut catalog = fixture_catalog();
        catalog.entries.push(EncyclopediaCatalogEntry {
            object_id: 0x5000_0001,
            text_resource_id: 4,
            name: "Adrift".into(),
        });
        assert_eq!(catalog.listed_entries().count(), 4);
        catalog.hidden_missions.push(0x5000_0001);
        let names: Vec<_> = catalog.listed_entries().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, ["Alliance Army Regiment", "Alliance Dreadnaught", "Allyuen"]);
        assert_eq!(catalog.entries.len(), 4, "the audit still counts it");
    }

    #[test]
    fn category_family_ranges_match_the_recovered_filter_table() {
        let catalog = fixture_catalog();
        assert_eq!(catalog.entries_for(0x6f).len(), 3);
        assert_eq!(catalog.entries_for(0x70)[0].name, "Allyuen");
        assert_eq!(catalog.entries_for(0x71)[0].name, "Alliance Dreadnaught");
        assert_eq!(catalog.entries_for(0x74)[0].name, "Alliance Army Regiment");
        assert!(catalog.entries_for(0x75).is_empty());
        assert!(catalog.entries_for(0xff).is_empty());
    }

    /// The shipped FLEETSD.DAT records: 4 is family `0x08`, TEXTSTRA
    /// 11523; 5 is all zero but its id.
    fn shipped_fleet_records() -> FleetsSeedFile {
        use dat_dumper::types::fleets_seed::FleetSeedEntry;
        let record = |id, family_id, text_stra_dll_id| FleetSeedEntry {
            id,
            field2: 1,
            production_family: 0,
            next_production_family: 0,
            family_id,
            text_stra_dll_id,
            field7: 2,
        };
        FleetsSeedFile {
            field1: 1,
            count: 2,
            family_id: 0x08,
            field4: 0x10,
            fleets: vec![record(4, 0x08, 0x2d03), record(5, 0, 0)],
        }
    }

    #[test]
    fn a_fresh_index_lists_fleet_under_all_databases_and_not_under_ships() {
        // FUN_00422620 adds the FLEETSD class 0x08000004 (TEXTSTRA 11523,
        // key 0x1d03) to the galaxy view's list +0x474 at its first build;
        // FUN_0045f100 shows it only under All Databases (0x6f), between
        // Flax and Fwillsving (A0 menus2/25).
        let mut catalog = fixture_catalog();
        for (object_id, name) in [(0x9000_0002, "Flax"), (0x9000_0003, "Fwillsving")] {
            catalog.entries.push(EncyclopediaCatalogEntry {
                object_id,
                text_resource_id: 5,
                name: name.into(),
            });
        }
        let strings = HashMap::from([(0x2d03, "Fleet".to_owned())]);
        push_fleet_entries(&mut catalog.entries, &shipped_fleet_records(), &strings).unwrap();
        catalog
            .entries
            .sort_by_key(|entry| entry.name.to_lowercase());

        let all: Vec<_> = catalog
            .entries_for(0x6f)
            .into_iter()
            .map(|entry| (entry.object_id, entry.name.as_str()))
            .collect();
        let at = all.iter().position(|entry| entry.1 == "Fleet").unwrap();
        assert_eq!(all[at].0, 0x0800_0004);
        assert_eq!((all[at - 1].1, all[at + 1].1), ("Flax", "Fwillsving"));
        assert_eq!(all.len(), 6, "record 5 (family 0) stays out");
        for command in 0x70..=0x75 {
            assert!(
                catalog
                    .entries_for(command)
                    .iter()
                    .all(|entry| entry.name != "Fleet"),
                "category {command:#x} lists Fleet"
            );
        }
    }

    #[test]
    fn the_installed_index_lists_fleet_under_all_databases_when_present() {
        // FUN_00422620: the shipped FLEETSD gives one topic, "Fleet". The
        // owned data is not in the repository; without it there is
        // nothing to load.
        let gdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/base");
        if !gdata.join("FLEETSD.DAT").exists() || !gdata.join("TEXTSTRA.DLL").exists() {
            return;
        }
        let catalog = load_encyclopedia_catalog(&gdata).unwrap();
        assert_eq!(catalog.entries.len(), 357);
        let fleet = catalog
            .entries_for(0x6f)
            .into_iter()
            .find(|entry| entry.object_id == 0x0800_0004)
            .expect("All Databases lists the FLEETSD class");
        assert_eq!(
            (fleet.name.as_str(), fleet.text_resource_id),
            ("Fleet", 0x2d03)
        );
        assert!(
            catalog
                .entries_for(0x71)
                .iter()
                .all(|entry| entry.object_id != 0x0800_0004)
        );
    }

    #[test]
    fn compound_ids_preserve_existing_families_and_attach_sequential_ones() {
        assert_eq!(compound_object_id(0x0000_0109, 0x90), 0x9000_0109);
        assert_eq!(compound_object_id(0x9200_0109, 0x90), 0x9200_0109);
    }

    #[test]
    fn push_entry_requires_and_preserves_the_source_name() {
        let strings = HashMap::from([(0x2001, "A-wing".to_owned())]);
        let mut entries = Vec::new();

        push_entry(&mut entries, 7, 0x14, 0x2001, &strings).unwrap();
        assert_eq!(
            entries,
            [EncyclopediaCatalogEntry {
                object_id: 0x1400_0007,
                text_resource_id: 0x2001,
                name: "A-wing".to_owned(),
            }]
        );

        let error = push_entry(&mut entries, 8, 0x14, 0x2002, &strings).unwrap_err();
        assert!(error.to_string().contains("0x2002"));
        assert_eq!(entries.len(), 1);
    }
}
