//! The GOKRES.DLL miniatures of the capital-ship and squadron classes, which
//! the Fleet, System, Manufacturing and Missions windows draw.

use rebellion_core::ids::DatId;

use crate::bmp_cache::resources::gokres;

// GOKRES.DLL stores fleet mini-icons in faction-specific resource blocks. These
// tables follow CAPSHPSD.DAT and FIGHTSD.DAT record order; they deliberately do
// not derive a resource ID by adding an offset to the compound DatId.
const ALLIANCE_FIGHTER_MINIS: [u32; 4] = [
    gokres::MINI_FIGHTER_A_WING,
    gokres::MINI_FIGHTER_B_WING,
    gokres::MINI_FIGHTER_X_WING,
    gokres::MINI_FIGHTER_Y_WING,
];

const EMPIRE_FIGHTER_MINIS: [u32; 4] = [
    gokres::MINI_FIGHTER_TIE_FIGHTER,
    gokres::MINI_FIGHTER_TIE_INTERCEPTOR,
    gokres::MINI_FIGHTER_TIE_BOMBER,
    gokres::MINI_FIGHTER_TIE_DEFENDER,
];

const ALLIANCE_CAPITAL_SHIP_MINIS: [u32; 15] = [
    gokres::MINI_SHIP_MC80_LIBERTY_CRUISER,
    gokres::MINI_SHIP_BULK_CRUISER,
    gokres::MINI_SHIP_ASSAULT_FRIGATE,
    gokres::MINI_SHIP_NEBULON_B_FRIGATE,
    gokres::MINI_SHIP_ALLIANCE_ESCORT_CARRIER,
    gokres::MINI_SHIP_CORELLIAN_CORVETTE,
    gokres::MINI_SHIP_MEDIUM_TRANSPORT,
    gokres::MINI_SHIP_BULK_TRANSPORT,
    gokres::MINI_SHIP_CORELLIAN_GUNSHIP,
    gokres::MINI_SHIP_ALLIANCE_DREADNAUGHT,
    gokres::MINI_SHIP_CC_7700_FRIGATE,
    gokres::MINI_SHIP_VISCOUNT_STAR_DEFENDER,
    gokres::MINI_SHIP_LIBERATOR_CRUISER,
    gokres::MINI_SHIP_MC30C_FRIGATE,
    gokres::MINI_SHIP_MC80A_HOME_ONE_CRUISER,
];

const EMPIRE_CAPITAL_SHIP_MINIS: [u32; 15] = [
    gokres::MINI_SHIP_STRIKE_CRUISER,
    gokres::MINI_SHIP_LANCER_FRIGATE,
    gokres::MINI_SHIP_INTERDICTOR_CRUISER,
    gokres::MINI_SHIP_CARRACK_LIGHT_CRUISER,
    gokres::MINI_SHIP_VICTORY_I_STAR_DESTROYER,
    gokres::MINI_SHIP_IMPERIAL_I_STAR_DESTROYER,
    gokres::MINI_SHIP_SUPER_STAR_DESTROYER,
    gokres::MINI_SHIP_GLADIATOR_STAR_DESTROYER,
    gokres::MINI_SHIP_DEATH_STAR,
    gokres::MINI_SHIP_ACCLAMATOR_DROP_SHIP,
    gokres::MINI_SHIP_VICTORY_II_STAR_DESTROYER,
    gokres::MINI_SHIP_IMPERIAL_II_STAR_DESTROYER,
    gokres::MINI_SHIP_STAR_GALLEON_FRIGATE,
    gokres::MINI_SHIP_IMPERIAL_ESCORT_CARRIER,
    gokres::MINI_SHIP_IMPERIAL_DREADNOUGHT,
];

pub(crate) fn capital_ship_mini_id(dat_id: DatId) -> Option<u32> {
    match dat_id.index() {
        64..=78 => ALLIANCE_CAPITAL_SHIP_MINIS
            .get((dat_id.index() - 64) as usize)
            .copied(),
        128..=142 => EMPIRE_CAPITAL_SHIP_MINIS
            .get((dat_id.index() - 128) as usize)
            .copied(),
        _ => None,
    }
}

pub(crate) fn fighter_mini_id(dat_id: DatId) -> Option<u32> {
    match dat_id.index() {
        1..=4 => ALLIANCE_FIGHTER_MINIS
            .get((dat_id.index() - 1) as usize)
            .copied(),
        5..=8 => EMPIRE_FIGHTER_MINIS
            .get((dat_id.index() - 5) as usize)
            .copied(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
    )]
    fn maps_every_capital_ship_dat_record_to_its_gokres_miniature() {
        // Source: GOKRES.DLL mini-icon blocks following CAPSHPSD.DAT record order.
        for (offset, &resource_id) in ALLIANCE_CAPITAL_SHIP_MINIS.iter().enumerate() {
            assert_eq!(
                capital_ship_mini_id(DatId::new(0x1400_0040 + offset as u32)),
                Some(resource_id)
            );
        }
        for (offset, &resource_id) in EMPIRE_CAPITAL_SHIP_MINIS.iter().enumerate() {
            assert_eq!(
                capital_ship_mini_id(DatId::new(0x1400_0080 + offset as u32)),
                Some(resource_id)
            );
        }
    }

    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
    )]
    fn maps_every_fighter_dat_record_to_its_gokres_miniature() {
        // Source: GOKRES.DLL mini-icon blocks following FIGHTSD.DAT record order.
        for (offset, &resource_id) in ALLIANCE_FIGHTER_MINIS.iter().enumerate() {
            assert_eq!(
                fighter_mini_id(DatId::new(0x1c00_0001 + offset as u32)),
                Some(resource_id)
            );
        }
        for (offset, &resource_id) in EMPIRE_FIGHTER_MINIS.iter().enumerate() {
            assert_eq!(
                fighter_mini_id(DatId::new(0x1c00_0005 + offset as u32)),
                Some(resource_id)
            );
        }
    }

    #[test]
    fn unknown_class_ids_do_not_request_unrelated_bitmaps() {
        assert_eq!(capital_ship_mini_id(DatId::new(0)), None);
        assert_eq!(capital_ship_mini_id(DatId::new(0x1400_003f)), None);
        assert_eq!(capital_ship_mini_id(DatId::new(0x1400_008f)), None);
        assert_eq!(fighter_mini_id(DatId::new(0)), None);
        assert_eq!(fighter_mini_id(DatId::new(0x1c00_0009)), None);
    }
}
