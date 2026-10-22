//! The galaxy view's legend windows (`FUN_00452090`, painted by
//! `FUN_004522f0` from STRATEGY.DLL's `RT_RCDATA` layouts through
//! `FUN_00452630`; `ghidra/notes/gid-markers.md`).
//!
//! The data crate reads the layouts and resolves their texts from TEXTSTRA;
//! the window draws them.

use std::collections::HashMap;

/// One display's legend window: its size and its bitmaps and texts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GidLegend {
    pub width: u16,
    pub height: u16,
    pub items: Vec<GidLegendItem>,
}

/// One item of a legend window, at its offset from the window's top left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GidLegendItem {
    /// A STRATEGY.DLL bitmap.
    Bitmap { x: u16, y: u16, id: u16 },
    /// A TEXTSTRA string in one of the game's fonts; `y` 1 centres it across
    /// the window's top (`FUN_00452630`).
    Text {
        x: u16,
        y: u16,
        text: String,
        font: u16,
        rgb: [u8; 3],
    },
}

/// The legend windows by their `RT_RCDATA` resource id.
pub type GidLegends = HashMap<u16, GidLegend>;
