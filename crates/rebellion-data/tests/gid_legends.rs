use std::path::PathBuf;

use rebellion_core::gid_legend::GidLegendItem;

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates directory")
        .parent()
        .expect("repository root")
        .join("data/base")
}

#[test]
#[ignore = "requires original data/base STRATEGY.DLL and TEXTSTRA.DLL"]
fn every_display_legend_loads_with_its_texts() {
    // FUN_004522f0 names 22 RCDATA layouts (0x27c7..0x27d8, 0x2d50..0x2d57).
    let legends = rebellion_data::gid_legends(&data_dir());
    assert_eq!(legends.len(), 22);
    let items = legends.values().flat_map(|legend| &legend.items);
    let texts = items
        .clone()
        .filter(|item| matches!(item, GidLegendItem::Text { .. }))
        .count();
    assert_eq!((texts, items.count()), (186, 372));

    // RCDATA 0x27c7, the Alliance's Popular Support: 180 x 135, its title
    // centred (y 1) in font 4.
    let support = &legends[&0x27c7];
    assert_eq!((support.width, support.height), (180, 135));
    assert_eq!(
        support.items[..2],
        [
            GidLegendItem::Text {
                x: 21,
                y: 1,
                text: "Loyalty to the Alliance".to_string(),
                font: 4,
                rgb: [255, 255, 255],
            },
            GidLegendItem::Bitmap {
                x: 7,
                y: 20,
                id: 10181
            },
        ]
    );
}
