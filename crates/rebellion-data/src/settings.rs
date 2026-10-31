//! The player's Game Options settings, kept between runs.
//!
//! The original keeps them in `HKLM\SOFTWARE\LucasArts Entertainment ...`:
//! `MusicVolume` and `Volume` (effects) as negated DirectSound hundredths of
//! a decibel, `MusicSwitch` (1 when the music is off) and `DisplayOpts`, the
//! Game Options window's flag word (`FUN_00417670`, `FUN_00416360`,
//! `FUN_004176e0`, `FUN_004163a0`; read back by `FUN_004173c0` and
//! `FUN_00413000`). The port keeps the same four values in
//! `<saves>/settings.json` natively and in `localStorage` in the browser.

use serde::{Deserialize, Serialize};

/// `DisplayOpts` bits (`FUN_004082b0`).
pub mod display {
    /// Play Music.
    pub const MUSIC: u32 = 0x100;
    /// Show Starfield.
    pub const STARFIELD: u32 = 0x1000_0000;
    /// Show Planet.
    pub const PLANET: u32 = 0x0100_0000;
    /// Show Pyrotechnics.
    pub const PYROTECHNICS: u32 = 0x0010_0000;
    /// Use High Detail Models.
    pub const HIGH_DETAIL: u32 = 0x0001_0000;
    /// Display Holocube.
    pub const HOLOCUBE: u32 = 0x0200_0000;
}

/// The persisted Game Options.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Music volume, 0..1 (the slider's percent over 100).
    pub music_volume: f32,
    /// Sound-effect volume, 0..1.
    pub effects_volume: f32,
    /// The window's flag word, in the original's `DisplayOpts` layout.
    pub display_options: u32,
}

impl Settings {
    /// Whether `bit` of [`display`] is set.
    #[must_use]
    pub const fn shows(&self, bit: u32) -> bool {
        self.display_options & bit != 0
    }
}

/// Parse stored settings; `None` for anything unreadable.
#[must_use]
pub fn decode(text: &str) -> Option<Settings> {
    let settings: Settings = serde_json::from_str(text).ok()?;
    (settings.music_volume.is_finite() && settings.effects_volume.is_finite()).then(|| Settings {
        music_volume: settings.music_volume.clamp(0.0, 1.0),
        effects_volume: settings.effects_volume.clamp(0.0, 1.0),
        ..settings
    })
}

/// Serialize settings for storage.
#[must_use]
pub fn encode(settings: &Settings) -> String {
    serde_json::to_string(settings).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
mod store {
    use super::{decode, encode, Settings};
    use std::path::Path;

    const FILE: &str = "settings.json";

    /// The stored settings, when there are any.
    #[must_use]
    pub fn load(saves_dir: &Path) -> Option<Settings> {
        decode(&std::fs::read_to_string(saves_dir.join(FILE)).ok()?)
    }

    /// Store the settings.
    ///
    /// # Errors
    /// When the directory or the file cannot be written.
    pub fn store(saves_dir: &Path, settings: &Settings) -> std::io::Result<()> {
        std::fs::create_dir_all(saves_dir)?;
        std::fs::write(saves_dir.join(FILE), encode(settings))
    }
}

#[cfg(target_arch = "wasm32")]
mod store {
    use super::{decode, encode, Settings};
    use std::path::Path;

    const KEY: &str = "rebellion_settings_v1";

    /// The stored settings, when there are any.
    #[must_use]
    pub fn load(_saves_dir: &Path) -> Option<Settings> {
        decode(&crate::save::wasm_impl::storage_get(KEY).ok()??)
    }

    /// Store the settings.
    ///
    /// # Errors
    /// When `localStorage` refuses the write.
    pub fn store(_saves_dir: &Path, settings: &Settings) -> std::io::Result<()> {
        crate::save::wasm_impl::storage_set(KEY, &encode(settings))
            .map_err(|error| std::io::Error::other(error.to_string()))
    }
}

pub use store::{load, store};

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Settings {
        Settings {
            music_volume: 0.49,
            effects_volume: 1.0,
            display_options: display::MUSIC | display::PLANET | display::HOLOCUBE,
        }
    }

    #[test]
    fn settings_round_trip_through_their_text() {
        assert_eq!(decode(&encode(&sample())), Some(sample()));
    }

    #[test]
    fn unreadable_or_out_of_range_settings_are_rejected_or_clamped() {
        assert_eq!(decode("not json"), None);
        assert_eq!(decode(r#"{"music_volume":0.5}"#), None);
        for (music, effects) in [("1e39", "0.5"), ("0.5", "1e39")] {
            let text = format!(
                r#"{{"music_volume":{music},"effects_volume":{effects},"display_options":0}}"#
            );
            assert_eq!(decode(&text), None, "{text}");
        }
        let clamped =
            decode(r#"{"music_volume":2.0,"effects_volume":-1.0,"display_options":256}"#).unwrap();
        assert_eq!((clamped.music_volume, clamped.effects_volume), (1.0, 0.0));
        assert!(clamped.shows(display::MUSIC));
        assert!(!clamped.shows(display::STARFIELD));
    }

    #[test]
    fn settings_are_kept_in_the_saves_directory() {
        let dir = std::env::temp_dir().join(format!("settings-{}", std::process::id()));
        assert_eq!(load(&dir), None);
        store(&dir, &sample()).unwrap();
        assert_eq!(load(&dir), Some(sample()));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_display_bits_are_the_originals() {
        // FUN_004082b0: masks for commands 0x99, 0xa1..0xa4, 0xa6.
        assert_eq!(
            [
                display::MUSIC,
                display::STARFIELD,
                display::PLANET,
                display::PYROTECHNICS,
                display::HIGH_DETAIL,
                display::HOLOCUBE
            ],
            [
                0x100,
                0x1000_0000,
                0x0100_0000,
                0x0010_0000,
                0x0001_0000,
                0x0200_0000
            ]
        );
    }
}
