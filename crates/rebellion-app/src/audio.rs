//! Audio engine — `quad-snd`-backed music, voice lines, and sound effects.
//!
//! # Why not kira?
//!
//! `kira` uses the `cpal` crate for its audio backend.  `cpal` declares
//! `links = "alsa"` in its Cargo metadata.  `miniquad` (the backend for
//! `egui-macroquad`) also declares `links = "alsa"`.  Cargo forbids two
//! packages in the same dependency closure from declaring the same `links`
//! value — even with `optional = true`, the resolver still pulls both in.
//!
//! `quad-snd` uses the same `miniquad` audio subsystem as macroquad, so
//! there is no link conflict.  It supports native (`CoreAudio` on macOS,
//! ALSA/PulseAudio on Linux) and WASM (`WebAudio`) through the same unified
//! API.  Feature parity for this project's needs: looped background music,
//! one-shot SFX, and one-shot tactical voice lines — all fully covered.
//!
//! # File layout convention
//!
//! ```text
//! data/sounds/
//!   music/
//!     main_theme.wav      (MDATA.300 — Return of the Jedi/Battle of Endor cue)
//!     battle.wav          (MDATA.307 — Attacking a Star Destroyer)
//!     victory.wav         (MDATA.201 extraction)
//!     defeat.wav          (MDATA.202 extraction)
//!     imperial.wav        (MDATA.306 — Emperor Arrives / Death of Yoda)
//!     hoth.wav            (MDATA.312 — Battle of Hoth medley)
//!     endor.wav           (MDATA.300 — Battle of Endor medley)
//!   sfx/
//!     mission_success.wav
//!     mission_fail.wav
//!     build_complete.wav
//!     fleet_departure.wav
//!     fleet_arrival.wav
//!     combat_start.wav
//!     ui_click.wav
//!     ui_close.wav
//!     menu_galaxy_size.wav (COMMON.DLL WAVE 8000)
//!     menu_load_options.wav (COMMON.DLL WAVE 8001)
//!     menu_quit.wav        (COMMON.DLL WAVE 8002)
//!     menu_select.wav      (COMMON.DLL WAVE 8004)
//!   voice/
//!     alliance/
//!       14001-voicefxa.wav   (through 15163-voicefxa.wav — from VOICEFXA.DLL)
//!     empire/
//!       15001-voicefxe.wav   (through 15132-voicefxe.wav — from VOICEFXE.DLL)
//! ```
//!
//! All files are optional: missing files are silently skipped and the engine
//! remains silent rather than crashing.
//!
//! # Tactical voice resource IDs
//!
//! The original VOICEFXA.DLL contains resources 14001–15163 (Alliance).
//! VOICEFXE.DLL contains resources 15001–15132 (Empire).  Extracted files
//! are in `assets/references/ref-ui-full/voice-alliance/` and
//! `voice-empire/` under names like `14001-voicefxa.wav`.  The staged
//! copies for runtime use live under `data/sounds/voice/`.
//!
//! # Context-based music
//!
//! Call `play_music_for_context` instead of `play_music` when you only know
//! the game context (galaxy map, combat, etc.).  The engine maps contexts to
//! tracks:
//!
//! | Context | Track |
//! |---------|-------|
//! | `MainMenu` | `MainTheme` |
//! | `GalaxyMap` | `MainTheme` |
//! | Combat | Battle |
//! | Victory | Victory |
//! | Defeat | Defeat |
//!
//! # WASM
//!
//! `quad-snd` provides a `WebAudio` backend for WASM builds with the same API.
//! File loading differs: on WASM you must supply raw bytes (loaded via
//! `macroquad::file::load_file`).  The `load_*_bytes` methods accept raw
//! bytes for this purpose.  The standard `load_*` methods are native-only
//! (`#[cfg(not(target_arch = "wasm32"))]`).
//!
//! # Usage
//!
//! ```rust,ignore
//! let mut audio = AudioEngine::new();
//! audio.load_all(Path::new("data/sounds"));
//!
//! // Context-based: engine picks the right track.
//! audio.play_music_for_context(MusicContext::GalaxyMap, &vol_state);
//!
//! // On game events:
//! audio.play_sfx(SfxKind::BuildComplete, &vol_state);
//!
//! // Each frame after egui controls:
//! if vol_state.dirty {
//!     audio.apply_volume(&vol_state);
//!     vol_state.dirty = false;
//! }
//! ```

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use quad_snd::{AudioContext, Sound};

use rebellion_core::tactical::TacticalAudioCue;
pub use rebellion_render::audio::{AudioVolumeState, MusicContext, MusicTrack, SfxKind};
use rebellion_render::tactical_view::TacticalVoiceFaction;

#[cfg(target_arch = "wasm32")]
const AUDIO_PREFIX: &str = "web/data";
#[cfg(not(target_arch = "wasm32"))]
const AUDIO_PREFIX: &str = "data";

fn audio_base_path(path: &Path) -> PathBuf {
    let prefix = Path::new(AUDIO_PREFIX);

    if path.starts_with(prefix) {
        return path.to_path_buf();
    }

    if let Ok(relative) = path.strip_prefix("data") {
        return prefix.join(relative);
    }

    prefix.join(path)
}

// ---------------------------------------------------------------------------
// File name conventions
// ---------------------------------------------------------------------------

fn sfx_file(kind: SfxKind) -> &'static str {
    match kind {
        SfxKind::MissionSuccess => "mission_success.wav",
        SfxKind::MissionFail => "mission_fail.wav",
        SfxKind::BuildComplete => "build_complete.wav",
        SfxKind::FleetDeparture => "fleet_departure.wav",
        SfxKind::FleetArrival => "fleet_arrival.wav",
        SfxKind::CombatStart => "combat_start.wav",
        SfxKind::UiClick => "ui_click.wav",
        SfxKind::UiClose => "ui_close.wav",
        SfxKind::MenuGalaxySize => "menu_galaxy_size.wav",
        SfxKind::MenuLoadOptions => "menu_load_options.wav",
        SfxKind::MenuQuit => "menu_quit.wav",
        SfxKind::MenuSelect => "menu_select.wav",
    }
}

/// Browser-pack filenames and COMMON.DLL resource identities for cockpit SFX.
pub const MENU_SFX_ASSETS: &[(SfxKind, &str, u32)] = &[
    (SfxKind::MenuGalaxySize, "sfx/menu_galaxy_size.wav", 8000),
    (SfxKind::MenuLoadOptions, "sfx/menu_load_options.wav", 8001),
    (SfxKind::MenuQuit, "sfx/menu_quit.wav", 8002),
    (SfxKind::MenuSelect, "sfx/menu_select.wav", 8004),
];

/// The weapon sounds' variants in `FUN_005bae60`'s table (the fire sounds
/// 0..5 and the shield and hull hits 0x0c..0x0e and 0x11..0x13), then the
/// unit sounds: the superlaser beam `0x14` (`FUN_005ba520`), the hums
/// `0x15`..`0x19` (slot `+0x6c`, `FUN_005b8df0`, `FUN_005b0100`), the launch
/// `0x1b` (`FUN_005a9170`), the arrivals `0x1c` / `0x1e` (`FUN_005a6d60`)
/// and the jumps `0x1d` / `0x1f` (`FUN_005a6b40`). `0x1a` (13060) has no
/// caller in REBEXE. Each row is `(event_id, variant, runtime_path,
/// TACTICAL.DLL WAVE resource)`.
pub const TACTICAL_SFX_ASSETS: &[(u8, u8, &str, u32)] = &[
    (0x00, 0, "sfx/tactical_event_00_0.wav", 13_000),
    (0x00, 1, "sfx/tactical_event_00_1.wav", 13_001),
    (0x00, 2, "sfx/tactical_event_00_2.wav", 13_002),
    (0x00, 3, "sfx/tactical_event_00_3.wav", 13_003),
    (0x01, 0, "sfx/tactical_event_01_0.wav", 13_004),
    (0x01, 1, "sfx/tactical_event_01_1.wav", 13_005),
    (0x01, 2, "sfx/tactical_event_01_2.wav", 13_006),
    (0x01, 3, "sfx/tactical_event_01_3.wav", 13_007),
    (0x02, 0, "sfx/tactical_event_02_0.wav", 13_008),
    (0x02, 1, "sfx/tactical_event_02_1.wav", 13_009),
    (0x02, 2, "sfx/tactical_event_02_2.wav", 13_010),
    (0x02, 3, "sfx/tactical_event_02_3.wav", 13_011),
    (0x03, 0, "sfx/tactical_event_03_0.wav", 13_012),
    (0x03, 1, "sfx/tactical_event_03_1.wav", 13_013),
    (0x04, 0, "sfx/tactical_event_04_0.wav", 13_014),
    (0x04, 1, "sfx/tactical_event_04_1.wav", 13_015),
    (0x05, 0, "sfx/tactical_event_05_0.wav", 13_016),
    (0x05, 1, "sfx/tactical_event_05_1.wav", 13_017),
    (0x0c, 0, "sfx/tactical_event_0c_0.wav", 13_030),
    (0x0c, 1, "sfx/tactical_event_0c_1.wav", 13_031),
    (0x0c, 2, "sfx/tactical_event_0c_2.wav", 13_032),
    (0x0d, 0, "sfx/tactical_event_0d_0.wav", 13_033),
    (0x0d, 1, "sfx/tactical_event_0d_1.wav", 13_034),
    (0x0d, 2, "sfx/tactical_event_0d_2.wav", 13_035),
    (0x0e, 0, "sfx/tactical_event_0e_0.wav", 13_036),
    (0x0e, 1, "sfx/tactical_event_0e_1.wav", 13_037),
    (0x0e, 2, "sfx/tactical_event_0e_2.wav", 13_038),
    (0x11, 0, "sfx/tactical_event_11_0.wav", 13_045),
    (0x11, 1, "sfx/tactical_event_11_1.wav", 13_046),
    (0x11, 2, "sfx/tactical_event_11_2.wav", 13_047),
    (0x12, 0, "sfx/tactical_event_12_0.wav", 13_048),
    (0x12, 1, "sfx/tactical_event_12_1.wav", 13_049),
    (0x12, 2, "sfx/tactical_event_12_2.wav", 13_050),
    (0x13, 0, "sfx/tactical_event_13_0.wav", 13_051),
    (0x13, 1, "sfx/tactical_event_13_1.wav", 13_052),
    (0x13, 2, "sfx/tactical_event_13_2.wav", 13_053),
    (0x14, 0, "sfx/tactical_event_14_0.wav", 13_054),
    (0x15, 0, "sfx/tactical_event_15_0.wav", 13_055),
    (0x16, 0, "sfx/tactical_event_16_0.wav", 13_056),
    (0x17, 0, "sfx/tactical_event_17_0.wav", 13_057),
    (0x18, 0, "sfx/tactical_event_18_0.wav", 13_058),
    (0x19, 0, "sfx/tactical_event_19_0.wav", 13_059),
    (0x1b, 0, "sfx/tactical_event_1b_0.wav", 13_061),
    (0x1c, 0, "sfx/tactical_event_1c_0.wav", 13_062),
    (0x1d, 0, "sfx/tactical_event_1d_0.wav", 13_063),
    (0x1e, 0, "sfx/tactical_event_1e_0.wav", 13_064),
    (0x1f, 0, "sfx/tactical_event_1f_0.wav", 13_065),
];

/// A track's staged file under `music/`: the port's named tracks, or an
/// original track's MDATA number (`307.wav`), as `tools/stage-ui-assets`
/// and `scripts/build-wasm.sh` stage them.
pub fn music_file(track: MusicTrack) -> String {
    match track {
        MusicTrack::MainTheme => "main_theme.wav".into(),
        MusicTrack::Battle => "battle.wav".into(),
        MusicTrack::Victory => "victory.wav".into(),
        MusicTrack::Defeat => "defeat.wav".into(),
        MusicTrack::Imperial => "imperial.wav".into(),
        MusicTrack::Hoth => "hoth.wav".into(),
        MusicTrack::Endor => "endor.wav".into(),
        MusicTrack::Original(_) => track
            .mdata()
            .map_or_else(String::new, |mdata| format!("{mdata}.wav")),
    }
}

/// Select a `MusicTrack` for a given `MusicContext`. The galaxy view's
/// own score is a sequence of picks (`rebellion_render::GalaxyScore`); the
/// app asks for it there instead.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn track_for_context(ctx: MusicContext) -> MusicTrack {
    match ctx {
        MusicContext::MainMenu | MusicContext::GalaxyMap => MusicTrack::MainTheme,
        MusicContext::Combat => MusicTrack::Battle,
        MusicContext::Victory => MusicTrack::Victory,
        MusicContext::Defeat => MusicTrack::Defeat,
    }
}

/// Build the filename for a voice line as found in the extracted DLL output.
///
/// Alliance: `{id}-voicefxa.wav`, Empire: `{id}-voicefxe.wav`.
fn voice_filename(faction: TacticalVoiceFaction, id: u32) -> String {
    let suffix = match faction {
        TacticalVoiceFaction::Alliance => "voicefxa",
        TacticalVoiceFaction::Empire => "voicefxe",
    };
    format!("{id}-{suffix}.wav")
}

fn voice_faction_dir(faction: TacticalVoiceFaction) -> &'static str {
    match faction {
        TacticalVoiceFaction::Alliance => "alliance",
        TacticalVoiceFaction::Empire => "empire",
    }
}

/// Complete source-proven tactical voice banks.
///
/// `VOICEFXA.DLL` stores the Alliance command/result bank at 14001-14122 and
/// the Alliance trench-run bank at 15133-15163. `VOICEFXE.DLL` stores the
/// Imperial command/result/Death Star bank at 15001-15132.
pub const TACTICAL_VOICE_RANGES: &[(TacticalVoiceFaction, u32, u32)] = &[
    (TacticalVoiceFaction::Alliance, 14_001, 14_122),
    (TacticalVoiceFaction::Alliance, 15_133, 15_163),
    (TacticalVoiceFaction::Empire, 15_001, 15_132),
];

/// `FUN_005bae60`: a class-2 request (every tactical event below `0x20`)
/// waits at most this long for a free channel (`+0x44` = `0xe1` ms).
pub const TACTICAL_EFFECT_WAIT_SECONDS: f64 = 0.225;

/// The tactical sound manager's effects class: every event below `0x20` is
/// class 2 (`FUN_005bae60`, `+0x544`), which plays at most
/// `ORIGINAL_TACTICAL_EFFECT_CHANNELS` (5, `+0x24`) at once.
/// `FUN_005baa60` / `FUN_005bad50` start a request while `FUN_005bad00`
/// counts fewer playing, else queue it; each frame `FUN_005bab50` forgets
/// the sounds that ended, drops a request that waited past 225 ms and
/// starts the queued ones, oldest first, while there is room. The looping
/// hum is a request with bit 3 (`FUN_005ba980(event, 0)`): it holds a
/// channel, and with none free it is dropped at once rather than queued.
#[derive(Debug, Default)]
pub struct TacticalEffectChannels {
    /// When each one-shot that holds a channel ends.
    playing: Vec<f64>,
    /// The hum holds a channel.
    loop_held: bool,
    /// Requests waiting for a channel, oldest first, with the time asked.
    waiting: std::collections::VecDeque<(TacticalAudioCue, f64)>,
}

/// One `TacticalEffectChannels::update`: the requests to play now and the
/// ones that waited too long.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TacticalEffectUpdate {
    pub started: Vec<TacticalAudioCue>,
    pub dropped: Vec<TacticalAudioCue>,
}

impl TacticalEffectChannels {
    /// Ask for an effect at `now` seconds (`FUN_005ba980` stamps
    /// `timeGetTime`).
    pub fn request(&mut self, cue: TacticalAudioCue, now: f64) {
        self.waiting.push_back((cue, now));
    }

    fn busy(&self) -> usize {
        self.playing.len() + usize::from(self.loop_held)
    }

    fn forget_ended(&mut self, now: f64) {
        self.playing.retain(|&end| end > now);
    }

    /// The frame's pass (`FUN_005bab50`). `seconds` gives a WAVE's length.
    pub fn update(&mut self, now: f64, seconds: impl Fn(u32) -> f64) -> TacticalEffectUpdate {
        self.forget_ended(now);
        let mut update = TacticalEffectUpdate::default();
        let waiting = std::mem::take(&mut self.waiting);
        for (cue, asked) in waiting {
            if now - asked > TACTICAL_EFFECT_WAIT_SECONDS {
                update.dropped.push(cue);
            } else if self.busy() < rebellion_core::tactical::ORIGINAL_TACTICAL_EFFECT_CHANNELS {
                self.playing.push(now + seconds(cue.resource_id));
                update.started.push(cue);
            } else {
                self.waiting.push_back((cue, asked));
            }
        }
        update
    }

    /// Take a channel for the hum, if one is free.
    pub fn try_hold_loop(&mut self, now: f64) -> bool {
        self.forget_ended(now);
        if self.loop_held {
            return true;
        }
        if self.busy() < rebellion_core::tactical::ORIGINAL_TACTICAL_EFFECT_CHANNELS {
            self.loop_held = true;
        }
        self.loop_held
    }

    /// The hum stopped (`FUN_005bab10`): its channel is free.
    pub fn release_loop(&mut self) {
        self.loop_held = false;
    }

    /// Leaving the battle forgets every channel and request.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

// ---------------------------------------------------------------------------
// The missing-movie box's sound
// ---------------------------------------------------------------------------

/// The stop beep's sample rate (quad-snd mixes at 44100 Hz).
pub const STOP_BEEP_RATE: u32 = 44_100;
/// The stop beep's length in samples: 0.75 s.
pub const STOP_BEEP_SAMPLES: usize = 33_075;
/// The stop beep's loudest sample, well under full scale.
pub const STOP_BEEP_PEAK: f32 = 0.45;
/// The stop beep's two struck tones, highest first: (start in seconds,
/// fundamental in Hz, decay time constant in seconds, weight).
const STOP_BEEP_TONES: [(f32, f32, f32, f32); 2] =
    [(0.0, 880.0, 0.09, 0.8), (0.11, 587.33, 0.17, 1.0)];
/// Each tone's attack and the whole beep's closing fade, in seconds.
const STOP_BEEP_ATTACK: f32 = 0.004;
const STOP_BEEP_FADE: f32 = 0.03;

/// The missing-movie box's sound: `FUN_005f6920` reaches `FUN_0060a9e0`,
/// which calls `MessageBeep(0x10)` (`MB_ICONHAND`, the Windows "Critical
/// Stop" sound) before its `MessageBoxA`. port: the port synthesizes its
/// own stand-in instead of any Windows or game asset: two struck tones a
/// fifth apart falling from A5 (880 Hz) to D5 (587.33 Hz) 110 ms later,
/// each with a quiet octave partial, a 4 ms attack and an exponential
/// decay (90 and 170 ms), 0.75 s long with a 30 ms closing fade, peaking
/// at [`STOP_BEEP_PEAK`].
#[must_use]
pub fn stop_beep_samples() -> Vec<f32> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "Sample indices stay far below 2^24."
    )]
    let raw: Vec<f32> = (0..STOP_BEEP_SAMPLES)
        .map(|index| {
            let t = index as f32 / STOP_BEEP_RATE as f32;
            let end = STOP_BEEP_SAMPLES as f32 / STOP_BEEP_RATE as f32;
            let fade = ((end - t) / STOP_BEEP_FADE).min(1.0);
            let sum: f32 = STOP_BEEP_TONES
                .iter()
                .filter(|&&(start, ..)| t >= start)
                .map(|&(start, hz, decay, weight)| {
                    let local = t - start;
                    let envelope = (local / STOP_BEEP_ATTACK).min(1.0) * (-local / decay).exp();
                    let phase = std::f32::consts::TAU * hz * local;
                    weight * envelope * (phase.sin() + 0.25 * (2.0 * phase).sin())
                })
                .sum();
            sum * fade
        })
        .collect();
    let loudest = raw
        .iter()
        .fold(0.0_f32, |max, sample| max.max(sample.abs()));
    raw.into_iter()
        .map(|sample| sample * STOP_BEEP_PEAK / loudest)
        .collect()
}

/// [`stop_beep_samples`] as a mono 16-bit PCM WAVE, the form `Sound::load`
/// takes on native and in the browser (`decodeAudioData`).
#[must_use]
pub fn stop_beep_wave() -> Vec<u8> {
    let samples = stop_beep_samples();
    let data_len = u32::try_from(samples.len() * 2).unwrap_or(u32::MAX);
    let mut bytes = Vec::with_capacity(44 + samples.len() * 2);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&STOP_BEEP_RATE.to_le_bytes());
    bytes.extend_from_slice(&(STOP_BEEP_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "Samples are bounded by STOP_BEEP_PEAK, inside i16."
        )]
        let value = (sample * f32::from(i16::MAX)).round() as i16;
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

/// A PCM WAVE's length: its `data` chunk over the `fmt ` chunk's byte rate.
#[must_use]
pub fn wave_duration_seconds(bytes: &[u8]) -> Option<f64> {
    let u32_at = |offset: usize| {
        bytes
            .get(offset..offset + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if bytes.get(0..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut offset = 12;
    let mut byte_rate = None;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32_at(offset + 4)? as usize;
        if id == b"fmt " {
            byte_rate = u32_at(offset + 16);
        } else if id == b"data" {
            let rate = f64::from(byte_rate.filter(|&rate| rate > 0)?);
            #[expect(
                clippy::cast_precision_loss,
                reason = "WAVE data sizes are far below 2^52"
            )]
            let size = size as f64;
            return Some(size / rate);
        }
        offset += 8 + size + (size & 1);
    }
    None
}

/// Runtime-pack path and resource identity for every restored tactical voice.
pub fn tactical_voice_assets() -> Vec<(TacticalVoiceFaction, String, u32)> {
    TACTICAL_VOICE_RANGES
        .iter()
        .flat_map(|&(faction, first, last)| {
            (first..=last).map(move |resource_id| {
                (
                    faction,
                    format!(
                        "voice/{}/{}",
                        voice_faction_dir(faction),
                        voice_filename(faction, resource_id)
                    ),
                    resource_id,
                )
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// AudioEngine
// ---------------------------------------------------------------------------

/// Whether a track started at `started` and `seconds` long has ended at
/// `now`. A track of unknown length never ends.
fn track_ended(started: f64, seconds: Option<f64>, now: f64) -> bool {
    seconds.is_some_and(|seconds| now - started >= seconds)
}

/// `quad-snd`-backed audio engine.
///
/// Handles background music (looped), one-shot SFX, and one-shot voice lines.
/// Gracefully no-ops when audio files are absent or the context fails.
pub struct AudioEngine {
    ctx: AudioContext,

    /// Pre-loaded one-shot SFX.
    sfx: HashMap<SfxKind, Sound>,

    /// Original tactical weapon-event variants keyed by WAVE resource ID.
    tactical_sfx: HashMap<u32, Sound>,

    /// Each loaded tactical WAVE's length, for the effect channels.
    tactical_sfx_seconds: HashMap<u32, f64>,

    /// Source-mapped tactical voices keyed by faction and WAVE ID.
    tactical_voice: HashMap<(TacticalVoiceFaction, u32), Sound>,

    /// The advisor's clip voices keyed by staged DLL directory and WAVE ID.
    advisor_voice: HashMap<(String, u16), Sound>,

    /// The selected unit's looping hum: its WAVE and its playback
    /// (`FUN_005ba980(event, 0)`, `DSBPLAY_LOOPING` in `FUN_00613b50`).
    tactical_loop: Option<(u32, quad_snd::Playback)>,

    /// Currently loaded music track + which track it is.
    music: Option<(Sound, MusicTrack)>,

    /// Whether music is currently playing.
    music_playing: bool,

    /// The loaded track's length in seconds, from its WAVE header.
    music_seconds: Option<f64>,

    /// When the track started (`macroquad::time::get_time`).
    music_started: f64,

    /// The browser's next track, decoding ahead of its turn
    /// ([`Self::prepare_music_bytes`]).
    #[cfg(target_arch = "wasm32")]
    prepared: Option<(Sound, MusicTrack)>,

    /// Missing tracks already reported, keeping diagnostics bounded.
    missing_music_logged: HashSet<MusicTrack>,

    /// The missing-movie box's synthesized stop beep ([`stop_beep_wave`]).
    stop_beep: Sound,
}

impl AudioEngine {
    /// Create a new engine and initialise the `quad-snd` audio context.
    pub fn new() -> Self {
        let ctx = AudioContext::new();
        // Loaded at once: the browser decodes it before the first box.
        let stop_beep = Sound::load(&ctx, &stop_beep_wave());
        AudioEngine {
            ctx,
            stop_beep,
            sfx: HashMap::new(),
            tactical_sfx: HashMap::new(),
            tactical_sfx_seconds: HashMap::new(),
            tactical_voice: HashMap::new(),
            advisor_voice: HashMap::new(),
            tactical_loop: None,
            music: None,
            music_playing: false,
            music_seconds: None,
            music_started: 0.0,
            #[cfg(target_arch = "wasm32")]
            prepared: None,
            missing_music_logged: HashSet::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Loading
    // -----------------------------------------------------------------------

    /// Pre-load all SFX from `sounds_dir/sfx/`.  Missing files are silently skipped.
    pub fn load_sfx(&mut self, sounds_dir: &Path) {
        let sfx_dir = audio_base_path(sounds_dir).join("sfx");
        let kinds = [
            SfxKind::MissionSuccess,
            SfxKind::MissionFail,
            SfxKind::BuildComplete,
            SfxKind::FleetDeparture,
            SfxKind::FleetArrival,
            SfxKind::CombatStart,
            SfxKind::UiClick,
            SfxKind::UiClose,
            SfxKind::MenuGalaxySize,
            SfxKind::MenuLoadOptions,
            SfxKind::MenuQuit,
            SfxKind::MenuSelect,
        ];
        for kind in kinds {
            let path = sfx_dir.join(sfx_file(kind));
            if !path.exists() {
                continue;
            }
            match std::fs::read(&path) {
                Ok(bytes) => {
                    let sound = Sound::load(&self.ctx, &bytes);
                    self.sfx.insert(kind, sound);
                }
                Err(e) => eprintln!("[audio] Failed to read SFX {}: {e}", path.display()),
            }
        }
        let audio_root = audio_base_path(sounds_dir);
        for &(_, _, relative_path, resource_id) in TACTICAL_SFX_ASSETS {
            let path = audio_root.join(relative_path);
            if !path.exists() {
                continue;
            }
            match std::fs::read(&path) {
                Ok(bytes) => self.load_tactical_sfx_bytes(resource_id, &bytes),
                Err(error) => {
                    eprintln!(
                        "[audio] Failed to read tactical SFX {}: {error}",
                        path.display()
                    );
                }
            }
        }
    }

    /// Pre-load the complete source-proven tactical voice bank. Missing files
    /// remain silent so an installation without the original DLLs still runs.
    pub fn load_tactical_voice(&mut self, sounds_dir: &Path) {
        let audio_root = audio_base_path(sounds_dir);
        for (faction, relative_path, resource_id) in tactical_voice_assets() {
            let path = audio_root.join(relative_path);
            if !path.exists() {
                continue;
            }
            match std::fs::read(&path) {
                Ok(bytes) => self.load_tactical_voice_bytes(faction, resource_id, &bytes),
                Err(error) => eprintln!(
                    "[audio] Failed to read tactical voice {}: {error}",
                    path.display()
                ),
            }
        }
    }

    /// Load all audio resources from `sounds_dir`.
    ///
    /// SFX and voice lines are pre-loaded.  Music is loaded on demand in
    /// `play_music`.
    pub fn load_all(&mut self, sounds_dir: &Path) {
        self.load_sfx(sounds_dir);
        self.load_tactical_voice(sounds_dir);
    }

    /// Load the binary-mapped cockpit sounds directly from an owned COMMON.DLL.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_original_menu_sfx(&mut self, common_dll: &Path) {
        let resource_ids: Vec<u32> = MENU_SFX_ASSETS
            .iter()
            .map(|(_, _, resource_id)| *resource_id)
            .collect();
        match rebellion_data::load_wave_resources(common_dll, &resource_ids) {
            Ok(waves) => {
                for &(kind, _, resource_id) in MENU_SFX_ASSETS {
                    if let Some(bytes) = waves.get(&resource_id) {
                        self.load_sfx_bytes(kind, bytes);
                    }
                }
                eprintln!(
                    "[audio] loaded {} original cockpit SFX from {}",
                    waves.len(),
                    common_dll.display()
                );
            }
            Err(error) => eprintln!(
                "[audio] original cockpit SFX unavailable path={} error={error}",
                common_dll.display()
            ),
        }
    }

    /// Load recovered tactical event cues from an owned TACTICAL.DLL.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_original_tactical_sfx(&mut self, tactical_dll: &Path) {
        let resource_ids: Vec<u32> = TACTICAL_SFX_ASSETS
            .iter()
            .map(|(_, _, _, resource_id)| *resource_id)
            .collect();
        match rebellion_data::load_wave_resources(tactical_dll, &resource_ids) {
            Ok(waves) => {
                for &(_, _, _, resource_id) in TACTICAL_SFX_ASSETS {
                    if let Some(bytes) = waves.get(&resource_id) {
                        self.load_tactical_sfx_bytes(resource_id, bytes);
                    }
                }
                eprintln!(
                    "[audio] loaded {} original tactical SFX from {}",
                    waves.len(),
                    tactical_dll.display()
                );
            }
            Err(error) => eprintln!(
                "[audio] original tactical SFX unavailable path={} error={error}",
                tactical_dll.display()
            ),
        }
    }

    /// Load the complete source-proven tactical voice banks from owned DLLs.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_original_tactical_voice(&mut self, alliance_dll: &Path, empire_dll: &Path) {
        for (faction, dll_path) in [
            (TacticalVoiceFaction::Alliance, alliance_dll),
            (TacticalVoiceFaction::Empire, empire_dll),
        ] {
            if !dll_path.exists() {
                continue;
            }
            let resource_ids = tactical_voice_assets()
                .into_iter()
                .filter_map(|(candidate, _, resource_id)| {
                    (candidate == faction).then_some(resource_id)
                })
                .collect::<Vec<_>>();
            match rebellion_data::load_wave_resources(dll_path, &resource_ids) {
                Ok(waves) => {
                    for (resource_id, bytes) in waves {
                        self.load_tactical_voice_bytes(faction, resource_id, &bytes);
                    }
                    eprintln!(
                        "[audio] loaded {} original tactical voices from {}",
                        resource_ids.len(),
                        dll_path.display()
                    );
                }
                Err(error) => eprintln!(
                    "[audio] original tactical voice unavailable path={} error={error}",
                    dll_path.display()
                ),
            }
        }
    }

    // -----------------------------------------------------------------------
    // WASM byte-level loaders (for macroquad::file::load_file results)
    // -----------------------------------------------------------------------

    /// Load a single SFX from raw bytes (e.g. from WASM HTTP fetch).
    #[allow(dead_code)]
    pub fn load_sfx_bytes(&mut self, kind: SfxKind, bytes: &[u8]) {
        let sound = Sound::load(&self.ctx, bytes);
        self.sfx.insert(kind, sound);
    }

    /// Load one source-mapped TACTICAL.DLL WAVE resource.
    pub fn load_tactical_sfx_bytes(&mut self, resource_id: u32, bytes: &[u8]) {
        let sound = Sound::load(&self.ctx, bytes);
        self.tactical_sfx.insert(resource_id, sound);
        if let Some(seconds) = wave_duration_seconds(bytes) {
            self.tactical_sfx_seconds.insert(resource_id, seconds);
        }
    }

    /// Whether a tactical WAVE is loaded.
    pub fn has_tactical_sfx(&self, resource_id: u32) -> bool {
        self.tactical_sfx.contains_key(&resource_id)
    }

    /// A loaded tactical WAVE's length; 0 when it is not loaded (nothing
    /// plays, so it holds no channel).
    pub fn tactical_sfx_seconds(&self, resource_id: u32) -> f64 {
        self.tactical_sfx_seconds
            .get(&resource_id)
            .copied()
            .unwrap_or(0.0)
    }

    /// Load one source-mapped tactical voice from raw bytes.
    pub fn load_tactical_voice_bytes(
        &mut self,
        faction: TacticalVoiceFaction,
        resource_id: u32,
        bytes: &[u8],
    ) {
        let sound = Sound::load(&self.ctx, bytes);
        self.tactical_voice.insert((faction, resource_id), sound);
    }

    /// Load one advisor clip voice (a briefing DLL WAVE) from raw bytes.
    pub fn load_advisor_voice_bytes(&mut self, directory: &str, wave: u16, bytes: &[u8]) {
        let sound = Sound::load(&self.ctx, bytes);
        self.advisor_voice.insert((directory.to_owned(), wave), sound);
    }

    /// Play a loaded advisor clip voice at the current SFX volume.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Audio gains are bounded values; the playback API takes f32."
    )]
    pub fn play_advisor_voice(
        &mut self,
        directory: &str,
        wave: u16,
        vol_state: &AudioVolumeState,
    ) -> bool {
        let vol = vol_state.effective_sfx_volume() as f32;
        let Some(sound) = self.advisor_voice.get(&(directory.to_owned(), wave)) else {
            return false;
        };
        if vol > 0.0 {
            sound.play(
                &self.ctx,
                quad_snd::PlaySoundParams {
                    looped: false,
                    volume: vol,
                },
            );
        }
        true
    }

    /// Stop an advisor clip voice that a stopped clip cuts
    /// (`FUN_00404020(1)`, `FUN_00404040(1)`). Returns whether it was loaded.
    pub fn stop_advisor_voice(&mut self, directory: &str, wave: u16) -> bool {
        let Some(sound) = self.advisor_voice.get(&(directory.to_owned(), wave)) else {
            return false;
        };
        sound.stop(&self.ctx);
        true
    }

    /// Begin decoding a music track supplied by the browser runtime pack.
    ///
    /// WebAudio decoding is asynchronous. Call `try_play_loaded_music` on
    /// subsequent frames after a user gesture rather than attempting playback
    /// before the decoded buffer exists.
    #[cfg(target_arch = "wasm32")]
    pub fn load_music_bytes(&mut self, track: MusicTrack, bytes: &[u8]) {
        self.stop_music();
        let sound = match self.prepared.take() {
            Some((sound, prepared)) if prepared == track => sound,
            other => {
                if let Some((sound, _)) = other {
                    sound.delete(&self.ctx);
                }
                Sound::load(&self.ctx, bytes)
            }
        };
        self.music = Some((sound, track));
        self.music_seconds = wave_duration_seconds(bytes);
        self.music_playing = false;
    }

    /// Begin decoding the track due next, so it can start the moment it is
    /// asked for ([`Self::load_music_bytes`]).
    #[cfg(target_arch = "wasm32")]
    pub fn prepare_music_bytes(&mut self, track: MusicTrack, bytes: &[u8]) {
        if self
            .prepared
            .as_ref()
            .is_some_and(|(_, prepared)| *prepared == track)
        {
            return;
        }
        if let Some((sound, _)) = self.prepared.take() {
            sound.delete(&self.ctx);
        }
        self.prepared = Some((Sound::load(&self.ctx, bytes), track));
    }

    /// Whether a next track is decoding or decoded.
    #[cfg(target_arch = "wasm32")]
    pub fn has_prepared_music(&self) -> bool {
        self.prepared.is_some()
    }

    /// Whether the loaded browser music has started.
    #[cfg(target_arch = "wasm32")]
    pub fn music_is_playing(&self) -> bool {
        self.music_playing
    }

    /// Start decoded browser music once, returning whether playback is active.
    #[cfg(target_arch = "wasm32")]
    pub fn try_play_loaded_music(&mut self, vol_state: &AudioVolumeState) -> bool {
        if self.music_playing {
            return true;
        }

        let Some((sound, track)) = &self.music else {
            return false;
        };
        if !sound.is_loaded() {
            return false;
        }

        sound.play(
            &self.ctx,
            quad_snd::PlaySoundParams {
                looped: !track.is_galaxy_pick(),
                volume: vol_state.effective_music_volume() as f32,
            },
        );
        self.music_playing = true;
        self.music_started = macroquad::time::get_time();
        true
    }

    // -----------------------------------------------------------------------
    // Playback
    // -----------------------------------------------------------------------

    /// Play a one-shot SFX at the current SFX volume.
    ///
    /// No-op when the SFX was not loaded or the effective volume is zero.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Audio gains are bounded values; the playback API takes f32."
    )]
    pub fn play_sfx(&mut self, kind: SfxKind, vol_state: &AudioVolumeState) {
        let vol = vol_state.effective_sfx_volume() as f32;
        if vol <= 0.0 {
            return;
        }
        if let Some(sound) = self.sfx.get(&kind) {
            sound.play(
                &self.ctx,
                quad_snd::PlaySoundParams {
                    looped: false,
                    volume: vol,
                },
            );
        }
    }

    /// Play the missing-movie box's stop beep once at the effects volume
    /// (port: the stand-in for `MessageBeep(0x10)` in `FUN_0060a9e0`).
    /// Returns whether it sounded: nothing plays with the effects silent.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Audio gains are bounded values; the playback API takes f32."
    )]
    pub fn play_stop_beep(&mut self, vol_state: &AudioVolumeState) -> bool {
        let vol = vol_state.effective_sfx_volume() as f32;
        if vol <= 0.0 {
            return false;
        }
        self.stop_beep.play(
            &self.ctx,
            quad_snd::PlaySoundParams {
                looped: false,
                volume: vol,
            },
        );
        true
    }

    /// Play one concrete original tactical event variant.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Audio gains are bounded values; the playback API takes f32."
    )]
    pub fn play_tactical_sfx(&mut self, resource_id: u32, vol_state: &AudioVolumeState) -> bool {
        let vol = vol_state.effective_sfx_volume() as f32;
        if let Some(sound) = self.tactical_sfx.get(&resource_id) {
            if vol > 0.0 {
                sound.play(
                    &self.ctx,
                    quad_snd::PlaySoundParams {
                        looped: false,
                        volume: vol,
                    },
                );
            }
            true
        } else {
            false
        }
    }

    /// Start one tactical WAVE looping in place of the running loop (the
    /// selected unit's hum). Returns whether the WAVE was loaded; with the
    /// effects silent nothing plays but the loop is still held.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Audio gains are bounded values; the playback API takes f32."
    )]
    pub fn start_tactical_loop(&mut self, resource_id: u32, vol_state: &AudioVolumeState) -> bool {
        self.stop_tactical_loop();
        let vol = vol_state.effective_sfx_volume() as f32;
        let Some(sound) = self.tactical_sfx.get(&resource_id) else {
            return false;
        };
        if vol > 0.0 {
            let playback = sound.play(
                &self.ctx,
                quad_snd::PlaySoundParams {
                    looped: true,
                    volume: vol,
                },
            );
            self.tactical_loop = Some((resource_id, playback));
        }
        true
    }

    /// Stop the running tactical loop (`FUN_005bab10`), if any.
    pub fn stop_tactical_loop(&mut self) {
        if let Some((_, playback)) = self.tactical_loop.take() {
            playback.stop(&self.ctx);
        }
    }

    /// Play one exact tactical voice at the current SFX volume.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Audio gains are bounded values; the playback API takes f32."
    )]
    pub fn play_tactical_voice(
        &mut self,
        faction: TacticalVoiceFaction,
        resource_id: u32,
        vol_state: &AudioVolumeState,
    ) -> bool {
        let vol = vol_state.effective_sfx_volume() as f32;
        if let Some(sound) = self.tactical_voice.get(&(faction, resource_id)) {
            if vol > 0.0 {
                sound.play(
                    &self.ctx,
                    quad_snd::PlaySoundParams {
                        looped: false,
                        volume: vol,
                    },
                );
            }
            true
        } else {
            false
        }
    }

    /// Start a looping music track, loading it from `sounds_dir/music/` if
    /// not already loaded.
    ///
    /// If the same track is already playing, this is a no-op.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Audio gains are bounded values; the playback API takes f32."
    )]
    pub fn play_music(
        &mut self,
        track: MusicTrack,
        sounds_dir: &Path,
        vol_state: &AudioVolumeState,
    ) {
        let vol = vol_state.effective_music_volume() as f32;

        // Same track already playing — do nothing (a galaxy pick that
        // has ended starts again).
        if let Some((_, current)) = &self.music {
            if *current == track
                && self.music_playing
                && !self.galaxy_pick_finished(macroquad::time::get_time())
            {
                return;
            }
        }

        self.stop_music();

        let staged_path = audio_base_path(sounds_dir)
            .join("music")
            .join(music_file(track));
        let path = if staged_path.exists() {
            staged_path
        } else if track == MusicTrack::MainTheme {
            let configured = std::env::var_os("REBELLION_MDATA_DIR")
                .map(PathBuf::from)
                .map(|directory| directory.join("MDATA.300"));
            configured
                .filter(|candidate| candidate.exists())
                .unwrap_or_else(|| PathBuf::from("../star-wars-rebellion/MDATA/MDATA.300"))
        } else {
            staged_path
        };
        if !path.exists() {
            if self.missing_music_logged.insert(track) {
                eprintln!(
                    "[audio] music unavailable track={track:?} expected_path={}",
                    path.display()
                );
            }
            return;
        }

        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[audio] Failed to read music {}: {e}", path.display());
                return;
            }
        };

        let sound = Sound::load(&self.ctx, &bytes);
        sound.play(
            &self.ctx,
            quad_snd::PlaySoundParams {
                looped: !track.is_galaxy_pick(),
                volume: vol,
            },
        );
        self.music = Some((sound, track));
        self.music_seconds = wave_duration_seconds(&bytes);
        self.music_started = macroquad::time::get_time();
        self.music_playing = true;
    }

    /// Whether the galaxy view's playing track has ended, so the next
    /// pick is due (`FUN_00417780`'s `0x4c9`). The other tracks loop.
    pub fn galaxy_pick_finished(&self, now: f64) -> bool {
        self.music_playing
            && self
                .music
                .as_ref()
                .is_some_and(|(_, track)| track.is_galaxy_pick())
            && track_ended(self.music_started, self.music_seconds, now)
    }

    /// Select and start the appropriate music track for a game context.
    ///
    /// Delegates to `play_music` after mapping context → track.  If the
    /// correct track is already playing, this is a no-op.
    pub fn play_music_for_context(
        &mut self,
        context: MusicContext,
        sounds_dir: &Path,
        vol_state: &AudioVolumeState,
    ) {
        let track = track_for_context(context);
        self.play_music(track, sounds_dir, vol_state);
    }

    /// Stop the browser music but keep its decoded track, so the next
    /// `try_play_loaded_music` starts it again from its beginning (WebAudio
    /// decodes a fresh track asynchronously, which a short screen between
    /// two movies would not wait for).
    #[cfg(target_arch = "wasm32")]
    pub fn halt_music(&mut self) {
        if let Some((sound, _)) = &self.music {
            sound.stop(&self.ctx);
        }
        self.music_playing = false;
    }

    /// Stop the current music track immediately.
    pub fn stop_music(&mut self) {
        if let Some((sound, _)) = self.music.take() {
            // Deleting frees the decoded track (the browser keeps every
            // buffer until then; the galaxy view changes track often).
            sound.stop(&self.ctx);
            sound.delete(&self.ctx);
            self.music_playing = false;
        }
    }

    /// Apply volume changes to the currently playing music.
    ///
    /// Call when `AudioVolumeState::dirty` is true, then clear `dirty`.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Audio gains are bounded values; the playback API takes f32."
    )]
    pub fn apply_volume(&mut self, vol_state: &AudioVolumeState) {
        let vol = vol_state.effective_music_volume() as f32;
        if let Some((sound, _)) = &self.music {
            sound.set_volume(&self.ctx, vol);
        }
    }

    /// Returns the currently playing `MusicTrack`, if any.
    #[allow(dead_code)]
    pub fn current_track(&self) -> Option<MusicTrack> {
        self.music.as_ref().map(|(_, t)| *t)
    }

    /// Always returns `true` — `quad-snd` initialises unconditionally.
    #[expect(
        clippy::unused_self,
        reason = "Keep the same instance API as the audio backend on other platforms."
    )]
    pub fn is_available(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cue(event: rebellion_core::tactical::TacticalAudioEvent) -> TacticalAudioCue {
        TacticalAudioCue::from_variant(event, 0)
    }

    // FUN_005bae60: every event below 0x20 is class 2, at most 5 at once
    // (+0x24), a request waits 225 ms (+0x44 = 0xe1). FUN_005baa60 /
    // FUN_005bad50 start a request when FUN_005bad00 counts fewer than 5
    // playing, else queue it; FUN_005bab50 each frame drops the finished
    // sounds, drops a request that waited longer than 225 ms and starts the
    // queued ones, oldest first, while there is room.
    #[test]
    fn at_most_five_effects_play_and_a_request_waits_225_ms() {
        use rebellion_core::tactical::TacticalAudioEvent as E;
        let mut channels = TacticalEffectChannels::default();
        let seconds = |_wave: u32| 1.0;
        for _ in 0..7 {
            channels.request(cue(E::HullHit), 10.0);
        }
        let first = channels.update(10.0, seconds);
        assert_eq!((first.started.len(), first.dropped.len()), (5, 0));
        // Still full at 10.2 s; at 10.226 s the two waiting are dropped.
        let wait = channels.update(10.2, seconds);
        assert!(wait.started.is_empty() && wait.dropped.is_empty());
        let late = channels.update(10.226, seconds);
        assert_eq!((late.started.len(), late.dropped.len()), (0, 2));
        // Once the five end there is room again.
        channels.request(cue(E::LaserAtCapital), 11.0);
        let freed = channels.update(11.0, seconds);
        assert_eq!(freed.started, vec![cue(E::LaserAtCapital)]);
        // A request that waited exactly 225 ms still waits.
        let mut channels = TacticalEffectChannels::default();
        for _ in 0..6 {
            channels.request(cue(E::HullHit), 0.0);
        }
        assert_eq!(channels.update(0.0, seconds).started.len(), 5);
        assert!(channels.update(0.225, seconds).dropped.is_empty());
        assert_eq!(channels.update(0.25, seconds).dropped.len(), 1);
        // Leaving the battle frees every channel.
        channels.request(cue(E::HullHit), 0.3);
        channels.clear();
        assert!(channels.try_hold_loop(0.3));
        assert_eq!(
            channels.update(0.3, seconds),
            TacticalEffectUpdate::default()
        );
    }

    // The hum (FUN_005ba980(event, 0), bit 3) holds a class-2 channel while
    // it loops; with none free FUN_005baa60 drops it at once (no queue), and
    // the selection state asks again on the next frame.
    #[test]
    fn the_looping_hum_takes_one_of_the_five_channels_or_none() {
        use rebellion_core::tactical::TacticalAudioEvent as E;
        let mut channels = TacticalEffectChannels::default();
        let seconds = |_wave: u32| 2.0;
        assert!(channels.try_hold_loop(0.0));
        for _ in 0..5 {
            channels.request(cue(E::ShieldHit), 0.0);
        }
        let update = channels.update(0.0, seconds);
        assert_eq!(update.started.len(), 4);
        channels.release_loop();
        let update = channels.update(0.1, seconds);
        assert_eq!(update.started.len(), 1);
        assert!(!channels.try_hold_loop(0.1));
        assert!(channels.try_hold_loop(2.0));
    }

    // port: the stand-in for FUN_0060a9e0's MessageBeep(0x10) (reached
    // from FUN_005f6920 when a movie cannot be opened) is our own
    // synthesized sound, not a Windows or game asset.
    #[test]
    fn the_stop_beep_is_a_bounded_audible_three_quarter_second_fall() {
        let samples = stop_beep_samples();
        assert_eq!(samples.len(), STOP_BEEP_SAMPLES);
        assert!(samples.iter().all(|sample| sample.is_finite()));
        let loudest = samples.iter().fold(0.0_f32, |max, s| max.max(s.abs()));
        assert!((loudest - STOP_BEEP_PEAK).abs() < 1e-6, "peak {loudest}");
        // It starts and ends silent: no click at either edge.
        assert!(samples[0].abs() < 1e-6);
        assert!(samples[STOP_BEEP_SAMPLES - 1].abs() < 1e-3);
        // The higher tone sounds first, alone; the lower one takes over
        // 110 ms in: the first 100 ms cross zero more often per second
        // than 200..300 ms (880 Hz against mostly 587 Hz).
        let crossings = |range: std::ops::Range<usize>| {
            samples[range]
                .windows(2)
                .filter(|pair| (pair[0] < 0.0) != (pair[1] < 0.0))
                .count()
        };
        let rate = STOP_BEEP_RATE as usize;
        let high = crossings(0..rate / 10);
        let low = crossings(rate / 5..rate * 3 / 10);
        assert!((170..=190).contains(&high), "first tone crossings {high}");
        assert!((110..=130).contains(&low), "second tone crossings {low}");
        // It decays: the last 100 ms are far quieter than the peak.
        let tail = samples[STOP_BEEP_SAMPLES - rate / 10..]
            .iter()
            .fold(0.0_f32, |max, s| max.max(s.abs()));
        assert!(tail < STOP_BEEP_PEAK / 20.0, "tail {tail}");
        assert!(tail > 0.0);
    }

    #[test]
    fn the_stop_beep_has_no_clicks_and_fades_out_over_its_last_30_ms() {
        let samples = stop_beep_samples();
        let rate = STOP_BEEP_RATE as usize;
        let ms = |count: usize| count * rate / 1000;
        // No jump between neighbours, where the second tone enters too:
        // the steepest 880 Hz slope at this peak is about 0.05 a sample.
        let steepest = samples
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0_f32, f32::max);
        assert!(steepest < 0.08, "steepest step {steepest}");
        let loudest = |range: std::ops::Range<usize>| {
            samples[range]
                .iter()
                .fold(0.0_f32, |max, s| max.max(s.abs()))
        };
        let n = STOP_BEEP_SAMPLES;
        // Before the fade only the 170 ms decay acts (50 ms: about 0.75).
        let before_fade = loudest(n - ms(35)..n - ms(30));
        let earlier = loudest(n - ms(85)..n - ms(80));
        let ratio = before_fade / earlier;
        assert!((0.6..0.9).contains(&ratio), "decay ratio {ratio}");
        // The fade takes the last millisecond close to silence.
        let last = loudest(n - ms(1)..n);
        assert!(last < before_fade / 20.0, "last {last} against {before_fade}");
    }

    #[test]
    fn the_stop_beeps_first_tone_carries_a_quarter_strength_octave() {
        // The first 100 ms are the 880 Hz tone alone; its octave partial
        // is in phase at a quarter of its strength.
        let samples = stop_beep_samples();
        let rate = STOP_BEEP_RATE as usize;
        #[expect(
            clippy::cast_precision_loss,
            reason = "Sample indices stay far below 2^24."
        )]
        let projection = |hz: f32| {
            samples[..rate / 10]
                .iter()
                .enumerate()
                .map(|(index, sample)| {
                    let t = index as f32 / STOP_BEEP_RATE as f32;
                    sample * (std::f32::consts::TAU * hz * t).sin()
                })
                .sum::<f32>()
        };
        let ratio = projection(1760.0) / projection(880.0);
        assert!((0.2..0.3).contains(&ratio), "octave ratio {ratio}");
    }

    #[test]
    fn the_stop_beep_wave_is_mono_16_bit_pcm_of_the_same_length() {
        let bytes = stop_beep_wave();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize,
            bytes.len() - 8
        );
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        // PCM, mono, 44100 Hz, 88200 bytes/s, block 2, 16 bits.
        assert_eq!(&bytes[20..24], &[1, 0, 1, 0]);
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            44_100
        );
        assert_eq!(
            u32::from_le_bytes(bytes[28..32].try_into().unwrap()),
            88_200
        );
        assert_eq!(&bytes[32..36], &[2, 0, 16, 0]);
        assert_eq!(bytes.len(), 44 + STOP_BEEP_SAMPLES * 2);
        assert_eq!(wave_duration_seconds(&bytes), Some(0.75));
        let samples = stop_beep_samples();
        let pcm: Vec<i16> = bytes[44..]
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let peak = pcm.iter().map(|&value| value.unsigned_abs()).max().unwrap();
        assert_eq!(peak, 14_745, "0.45 of full scale");
        for (index, (&value, &sample)) in pcm.iter().zip(&samples).enumerate().step_by(997) {
            let want = (sample * 32_767.0).round();
            assert!((f32::from(value) - want).abs() < 0.5, "sample {index}");
        }
    }

    #[test]
    fn a_track_ends_once_its_length_has_played() {
        assert!(!track_ended(10.0, Some(2.0), 11.9));
        assert!(track_ended(10.0, Some(2.0), 12.0));
        assert!(track_ended(10.0, Some(2.0), 30.0));
        assert!(!track_ended(10.0, None, 1.0e9));
    }

    #[test]
    fn a_wave_lasts_its_data_over_its_byte_rate() {
        // A canonical PCM WAVE: 11025 Hz, mono, 8-bit, 22050 data bytes.
        let mut bytes = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&11_025u32.to_le_bytes());
        bytes.extend_from_slice(&11_025u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&8u16.to_le_bytes());
        bytes.extend_from_slice(b"LIST");
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(b"ab");
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&22_050u32.to_le_bytes());
        bytes.extend(std::iter::repeat_n(0x80, 22_050));
        assert_eq!(wave_duration_seconds(&bytes), Some(2.0));
        assert_eq!(wave_duration_seconds(b"RIFF"), None);
        // An odd chunk is padded to an even size; a short data chunk; no
        // data chunk; a zero byte rate; a header that is not RIFF/WAVE.
        let wave = |rate: u32, extra: &[u8], data: Option<&[u8]>| {
            let mut bytes = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
            bytes.extend_from_slice(&16u32.to_le_bytes());
            bytes.extend_from_slice(&[1, 0, 1, 0]);
            bytes.extend_from_slice(&rate.to_le_bytes());
            bytes.extend_from_slice(&rate.to_le_bytes());
            bytes.extend_from_slice(&[1, 0, 8, 0]);
            bytes.extend_from_slice(extra);
            if let Some(data) = data {
                bytes.extend_from_slice(b"data");
                bytes.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
                bytes.extend_from_slice(data);
            }
            bytes
        };
        let odd = b"junk\x03\0\0\0abc\0";
        assert_eq!(
            wave_duration_seconds(&wave(2, odd, Some(&[0; 4]))),
            Some(2.0)
        );
        assert_eq!(wave_duration_seconds(&wave(2, &[], None)), None);
        assert_eq!(wave_duration_seconds(&wave(0, &[], Some(&[0; 4]))), None);
        let mut not_riff = wave(2, &[], Some(&[0; 4]));
        not_riff[3] = b'X';
        assert_eq!(wave_duration_seconds(&not_riff), None);
        let mut not_wave = wave(2, &[], Some(&[0; 4]));
        not_wave[11] = b'X';
        assert_eq!(wave_duration_seconds(&not_wave), None);
    }

    #[test]
    fn the_tactical_bank_stages_every_unit_sound_but_the_callerless_0x1a() {
        // FUN_005bae60's WAVE table; 0x14..0x1f have one WAVE each from
        // 13054. No FUN_005ba980 call in REBEXE passes 0x1a, and no slot
        // +0x6c returns it (FUN_005b8df0, FUN_005b0100).
        for &(event, variant, path, wave) in TACTICAL_SFX_ASSETS {
            assert_eq!(
                wave,
                rebellion_core::tactical::original_audio_resource_base(event) + u32::from(variant),
                "{path}"
            );
            assert_eq!(
                path,
                format!("sfx/tactical_event_{event:02x}_{variant}.wav")
            );
        }
        let unit_events = TACTICAL_SFX_ASSETS
            .iter()
            .filter(|&&(event, ..)| event >= 0x14)
            .map(|&(event, _, _, wave)| (event, wave))
            .collect::<Vec<_>>();
        assert_eq!(
            unit_events,
            [
                (0x14, 13_054),
                (0x15, 13_055),
                (0x16, 13_056),
                (0x17, 13_057),
                (0x18, 13_058),
                (0x19, 13_059),
                (0x1b, 13_061),
                (0x1c, 13_062),
                (0x1d, 13_063),
                (0x1e, 13_064),
                (0x1f, 13_065),
            ]
        );
    }

    #[test]
    fn an_original_track_plays_its_staged_mdata_file() {
        // 0x006a8478: track 10 is MDATA.307, 11 MDATA.313.
        assert_eq!(music_file(MusicTrack::Original(10)), "307.wav");
        assert_eq!(music_file(MusicTrack::Original(11)), "313.wav");
        assert_eq!(music_file(MusicTrack::Battle), "battle.wav");
        assert_eq!(music_file(MusicTrack::Original(16)), "");
    }

    #[test]
    fn tactical_voice_assets_cover_complete_source_backed_banks() {
        // FUN_005bae60 registers the three continuous faction event ranges.
        let assets = tactical_voice_assets();
        assert_eq!(assets.len(), 285);
        let identities = assets
            .iter()
            .map(|(faction, _, resource_id)| (*faction, *resource_id))
            .collect::<HashSet<_>>();
        assert_eq!(identities.len(), 285);
        assert!(assets.contains(&(
            TacticalVoiceFaction::Alliance,
            "voice/alliance/14001-voicefxa.wav".to_string(),
            14_001,
        )));
        assert!(assets.contains(&(
            TacticalVoiceFaction::Empire,
            "voice/empire/15132-voicefxe.wav".to_string(),
            15_132,
        )));
        assert!(assets.contains(&(
            TacticalVoiceFaction::Alliance,
            "voice/alliance/15163-voicefxa.wav".to_string(),
            15_163,
        )));
        assert!(!identities.contains(&(TacticalVoiceFaction::Alliance, 15_001)));
        assert!(!identities.contains(&(TacticalVoiceFaction::Alliance, 15_132)));
        assert!(!identities.contains(&(TacticalVoiceFaction::Empire, 15_133)));
    }
}
