//! The canonical fleet names a side's fleets may take when the game uses
//! [`crate::world::FleetNaming::Canonical`], a port extension (the original
//! only numbers fleets, `ghidra/notes/fleet-names.md`).
//!
//! port: every name is a Galactic Civil War formation of its side, spelled
//! as its Wookieepedia article titles it; sources and the names left out are
//! in `docs/mechanics/fleet-names.md`. The notes are our own paraphrases.

use crate::ids::DatId;

/// One name in a side's bank.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankName {
    pub name: &'static str,
    /// True for a current-canon source, false for Legends.
    pub canon: bool,
    /// A signature name waits for a fleet holding this capital ship class
    /// instead of going to the next new fleet.
    pub flagship: Option<DatId>,
    /// One line on who the formation was.
    pub note: &'static str,
}

/// CAPSHPSD.DAT record `0x86` (TEXTSTRA 10118), the Super Star Destroyer.
pub const SUPER_STAR_DESTROYER: DatId = DatId(0x86);
/// CAPSHPSD.DAT record `0x40` (TEXTSTRA 10048), the Mon Calamari Cruiser.
pub const MON_CALAMARI_CRUISER: DatId = DatId(0x40);

const fn name(name: &'static str, canon: bool, note: &'static str) -> BankName {
    BankName {
        name,
        canon,
        flagship: None,
        note,
    }
}

/// The Empire's names, in the order new fleets take them.
pub const EMPIRE: &[BankName] = &[
    BankName {
        name: "Death Squadron",
        canon: true,
        flagship: Some(SUPER_STAR_DESTROYER),
        note: "Darth Vader's own battle group, led by the Executor at Hoth and Endor.",
    },
    name(
        "Seventh Fleet",
        true,
        "The fleet Grand Admiral Thrawn led against the Lothal rebels.",
    ),
    name(
        "Third Fleet",
        true,
        "Grand Admiral Savit's fleet, flying from the Star Destroyer Firedrake.",
    ),
    name(
        "Eleventh Fleet",
        true,
        "A numbered fleet of the Lothal campaign, promised to Commodore Faro.",
    ),
    name("First Naval Fleet", false, "A fleet of the Imperial Navy."),
    name(
        "96th Task Force",
        true,
        "Admiral Thrawn's task force at Sammun and Batonn.",
    ),
    name(
        "One Oh Third Task Force",
        true,
        "Admiral Durril's task force, flagship the Judicator.",
    ),
    name(
        "One Twenty-Fifth Task Force",
        true,
        "Admiral Kinshara's task force, flagship the Stalwart.",
    ),
    name(
        "Task Force 231",
        true,
        "A task force of the Lothal campaign era.",
    ),
    name(
        "Task Force Admonitor",
        false,
        "Thrawn's task force, named for its Star Destroyer Admonitor.",
    ),
    name(
        "Task Force Vengeance",
        false,
        "Admiral Senn's force, sent to pacify the Airam sector.",
    ),
    name(
        "Corrupter Task Force",
        false,
        "Admiral Holtz's task force, named for his Star Destroyer.",
    ),
    name(
        "Hunter Fleet",
        true,
        "An assault group formed after the Battle of Hoth.",
    ),
    name(
        "Lothal Sector Fleet",
        true,
        "Admiral Konstantine's sector fleet over Lothal.",
    ),
    name("Qeimet Fleet", false, "A unit of the Imperial Navy."),
];

/// The Alliance's names, in the order new fleets take them.
pub const ALLIANCE: &[BankName] = &[
    BankName {
        name: "Rebel Command Fleet",
        canon: false,
        flagship: Some(MON_CALAMARI_CRUISER),
        note: "The Alliance's main force at Endor, backed by at least forty Mon Calamari cruisers.",
    },
    name(
        "Alpha Group",
        true,
        "Admiral Ackbar's battle group at the Mako-Ta Space Docks.",
    ),
    name(
        "Beta Group",
        true,
        "A battle group of the fleet, led by Commander Lajaie.",
    ),
    name("Gamma Group", true, "General Hera Syndulla's battle group."),
    name("Delta Group", true, "General Willard's battle group."),
    name(
        "Fourth Division",
        true,
        "General Organa's division, raised after the fleet scattered from Hoth.",
    ),
    name(
        "Sixth Division",
        true,
        "A naval division that regrouped in the Mid Rim after Hoth.",
    ),
    name(
        "Seventh Division",
        true,
        "A division formed when the fleet scattered after Hoth.",
    ),
    name(
        "Massassi Group",
        true,
        "General Dodonna's cell at the Massassi temples of Yavin 4.",
    ),
    name(
        "Phoenix Cell",
        true,
        "Commander Sato's cell and its carrier, the Phoenix Home.",
    ),
];

/// The bank for a side (`is_alliance` as `Fleet::is_alliance`).
#[must_use]
pub const fn bank(is_alliance: bool) -> &'static [BankName] {
    if is_alliance {
        ALLIANCE
    } else {
        EMPIRE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_name_is_in_both_banks_or_twice_in_one() {
        let mut seen = std::collections::HashSet::new();
        for entry in EMPIRE.iter().chain(ALLIANCE) {
            assert!(seen.insert(entry.name), "{} repeats", entry.name);
        }
    }

    #[test]
    fn each_bank_has_one_signature_name_and_it_leads() {
        for bank in [EMPIRE, ALLIANCE] {
            assert!(bank[0].flagship.is_some());
            assert_eq!(bank.iter().filter(|e| e.flagship.is_some()).count(), 1);
        }
    }
}
