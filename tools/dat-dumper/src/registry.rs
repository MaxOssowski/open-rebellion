use std::collections::HashMap;

use crate::codec::{ByteReader, ByteWriter};
use crate::dat_record::DatRecord;
use crate::types::{
    advisor_script, all_facilities, capital_ships, defense_facilities, entity_table, fighters,
    fleets_seed, general_params, int_table, major_characters, manufacturing_facilities,
    minor_characters, missions, production_facilities, sectors, seed_table, side_params,
    special_forces, syfc_table, systems, troops,
};
use crate::validate::compare_bytes;

pub type ParseFn = fn(&[u8], &str) -> anyhow::Result<String>;

/// Parse a .DAT file, validate round-trip, and return pretty-printed JSON.
fn parse_and_dump<T: DatRecord>(data: &[u8], filename: &str) -> anyhow::Result<String> {
    let mut reader = ByteReader::new(data);
    let record = T::parse(&mut reader)?;
    reader.assert_exhausted(filename)?;

    let mut writer = ByteWriter::with_capacity(data.len());
    record.write_bytes(&mut writer);
    let reserialized = writer.into_bytes();
    compare_bytes(data, &reserialized, filename)?;

    let json = serde_json::to_string_pretty(&record)?;
    Ok(json)
}

#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "Keep this existing ordered routine together; splitting its phases is a separate refactor."
)]
pub fn build_registry() -> HashMap<&'static str, ParseFn> {
    let mut m: HashMap<&'static str, ParseFn> = HashMap::new();

    // Pattern 1 — entity tables (original)
    m.insert(
        "CAPSHPSD.DAT",
        parse_and_dump::<capital_ships::CapitalShipsFile> as ParseFn,
    );
    m.insert(
        "FIGHTSD.DAT",
        parse_and_dump::<fighters::FightersFile> as ParseFn,
    );
    m.insert(
        "TROOPSD.DAT",
        parse_and_dump::<troops::TroopsFile> as ParseFn,
    );
    m.insert(
        "SPECFCSD.DAT",
        parse_and_dump::<special_forces::SpecialForcesFile> as ParseFn,
    );
    m.insert(
        "MJCHARSD.DAT",
        parse_and_dump::<major_characters::MajorCharactersFile> as ParseFn,
    );
    m.insert(
        "MNCHARSD.DAT",
        parse_and_dump::<minor_characters::MinorCharactersFile> as ParseFn,
    );
    m.insert(
        "SYSTEMSD.DAT",
        parse_and_dump::<systems::SystemsFile> as ParseFn,
    );
    m.insert(
        "SECTORSD.DAT",
        parse_and_dump::<sectors::SectorsFile> as ParseFn,
    );
    m.insert(
        "DEFFACSD.DAT",
        parse_and_dump::<defense_facilities::DefenseFacilitiesFile> as ParseFn,
    );
    m.insert(
        "MANFACSD.DAT",
        parse_and_dump::<manufacturing_facilities::ManufacturingFacilitiesFile> as ParseFn,
    );
    m.insert(
        "PROFACSD.DAT",
        parse_and_dump::<production_facilities::ProductionFacilitiesFile> as ParseFn,
    );

    // Pattern 1 — new entity tables
    m.insert(
        "MISSNSD.DAT",
        parse_and_dump::<missions::MissionsFile> as ParseFn,
    );
    m.insert(
        "FLEETSD.DAT",
        parse_and_dump::<fleets_seed::FleetsSeedFile> as ParseFn,
    );
    m.insert(
        "ALLFACSD.DAT",
        parse_and_dump::<all_facilities::AllFacilitiesFile> as ParseFn,
    );

    // Pattern 1 — generic 24-byte entity tables (UNIQUESD, ABODESD, BASICSD, MANMGRSD)
    for name in &["UNIQUESD.DAT", "ABODESD.DAT", "BASICSD.DAT", "MANMGRSD.DAT"] {
        m.insert(
            name,
            parse_and_dump::<entity_table::EntityTableFile> as ParseFn,
        );
    }

    // Pattern 2 — parameter tables (original)
    m.insert(
        "GNPRTB.DAT",
        parse_and_dump::<general_params::GeneralParamsFile> as ParseFn,
    );
    m.insert(
        "SDPRTB.DAT",
        parse_and_dump::<side_params::SideParamsFile> as ParseFn,
    );

    // Pattern 2 — int lookup tables (IntTableEntry, 16 bytes each)
    for name in &[
        "DIPLMSTB.DAT",
        "ESPIMSTB.DAT",
        "ASSNMSTB.DAT",
        "INCTMSTB.DAT",
        "DSSBMSTB.DAT",
        "ABDCMSTB.DAT",
        "RCRTMSTB.DAT",
        "RESCMSTB.DAT",
        "SBTGMSTB.DAT",
        "SUBDMSTB.DAT",
        "ESCAPETB.DAT",
        "FDECOYTB.DAT",
        "FOILTB.DAT",
        "INFORMTB.DAT",
        "CSCRHTTB.DAT",
        "UPRIS1TB.DAT",
        "UPRIS2TB.DAT",
        "RLEVADTB.DAT",
        "RESRCTB.DAT",
        "TDECOYTB.DAT",
    ] {
        m.insert(name, parse_and_dump::<int_table::IntTableFile> as ParseFn);
    }

    // Pattern 2 — system facility seed tables (SeedTableEntry, 16 bytes each)
    for name in &["SYFCCRTB.DAT", "SYFCRMTB.DAT"] {
        m.insert(name, parse_and_dump::<syfc_table::SyfcTableFile> as ParseFn);
    }

    // Pattern 3 — seed tables (all share SeedTableFile)
    for name in &[
        "CMUNAFTB.DAT",
        "CMUNALTB.DAT",
        "CMUNCRTB.DAT",
        "CMUNEFTB.DAT",
        "CMUNEMTB.DAT",
        "CMUNHQTB.DAT",
        "CMUNYVTB.DAT",
        "FACLCRTB.DAT",
        "FACLHQTB.DAT",
    ] {
        m.insert(name, parse_and_dump::<seed_table::SeedTableFile> as ParseFn);
    }

    // The advisor's messages (FUN_004c2c70, FUN_004c0ba0).
    for name in ["C3POACT.SPT", "IMP22ACT.SPT"] {
        m.insert(
            name,
            parse_and_dump::<advisor_script::AdvisorScriptFile> as ParseFn,
        );
    }

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_parses_the_advisor_message_files_and_round_trips_them() {
        // FUN_004c2c70 and FUN_004c0ba0 read C3POACT.SPT and IMP22ACT.SPT.
        let registry = build_registry();
        // One message (id 0x29) with one entry: the idle clip with action
        // kind 2, code 1.
        let mut bytes = Vec::new();
        for value in [1_u32, 0x29, 1, 1, 0] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend(0x2710_u16.to_le_bytes());
        for value in [0_u32, 2, 1] {
            bytes.extend(value.to_le_bytes());
        }
        for name in ["C3POACT.SPT", "IMP22ACT.SPT"] {
            let json = registry[name](&bytes, name).unwrap();
            assert!(json.contains("\"record\": 10000"), "{json}");
        }
        // A trailing byte fails the round trip's full read.
        bytes.push(0);
        assert!(registry["C3POACT.SPT"](&bytes, "C3POACT.SPT").is_err());
    }
}
