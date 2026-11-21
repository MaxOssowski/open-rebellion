//! Audio state shared by the render and app layers: named sound effects,
//! music tracks and the volume and mute state.
//!
//! The actual audio engine (kira `AudioManager`) lives in
//! `rebellion-app/src/audio.rs` to avoid a `links = "alsa"` conflict between
//! `miniquad` (pulled in by `egui-macroquad`) and `cpal` (pulled in by kira).
//! The original sets the volumes only in the Game Options window
//! (`FUN_00406b80` commands 0x8d/0x8e; `game_options.rs`); the engine reads
//! `AudioVolumeState` to set its track volumes.

// ---------------------------------------------------------------------------
// SfxKind — named sound effect identifiers
// ---------------------------------------------------------------------------

/// Named sound effect identifiers.
///
/// Shared between the render crate (for type-safe callbacks) and the app
/// crate (which maps them to file names and `quad-snd` handles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SfxKind {
    MissionSuccess,
    MissionFail,
    BuildComplete,
    FleetDeparture,
    FleetArrival,
    /// Space battle begins at a system.
    CombatStart,
    /// Clicked a button or opened a panel.
    UiClick,
    /// Closed a panel or dismissed a dialog.
    UiClose,
    /// Original COMMON.DLL WAVE 8000, assigned to galaxy-size controls.
    MenuGalaxySize,
    /// Original COMMON.DLL WAVE 8001, assigned to Load/Options.
    MenuLoadOptions,
    /// Original COMMON.DLL WAVE 8002, assigned to Quit.
    MenuQuit,
    /// Original COMMON.DLL WAVE 8004, assigned to the remaining cockpit controls.
    MenuSelect,
}

// ---------------------------------------------------------------------------
// MusicTrack — named music track identifiers
// ---------------------------------------------------------------------------

/// Named music track identifiers.
///
/// Maps 1:1 to WAV files under `data/sounds/music/`.  Files are named
/// `{lowercase_variant}.wav` (e.g. `main_theme.wav`, `battle.wav`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MusicTrack {
    MainTheme,
    Battle,
    Victory,
    Defeat,
    /// Emperor Arrives / Death of Yoda / Obi-Wan Revelation (MDATA.306)
    Imperial,
    /// Battle of Hoth medley (MDATA.312)
    Hoth,
    /// Battle of Endor III medley (MDATA.315)
    Endor,
    /// The original score's track `index` (`FUN_00417520` plays
    /// [`ORIGINAL_MUSIC`]`[index]`), staged as `{mdata}.wav`.
    Original(u8),
}

/// The pointer table `FUN_00417520` plays from (`0x006a8478`, read from
/// REBEXE.EXE): track `index` is `MDATA.{ORIGINAL_MUSIC[index]}`.
pub const ORIGINAL_MUSIC: [u16; 16] = [
    300, 301, 302, 303, 304, 305, 306, 310, 311, 312, 307, 313, 308, 309, 314, 315,
];

impl MusicTrack {
    /// The MDATA file an original track plays, `None` for the port's own
    /// named tracks.
    #[must_use]
    pub fn mdata(self) -> Option<u16> {
        match self {
            Self::Original(index) => ORIGINAL_MUSIC.get(usize::from(index)).copied(),
            _ => None,
        }
    }
}

/// The Battle Alert's track (`FUN_0044f860` -> `FUN_00417610`): 10 for
/// the Alliance's viewer (MDATA.307), 11 for the Empire's (MDATA.313).
#[must_use]
pub const fn battle_alert_music(viewer_alliance: bool) -> MusicTrack {
    MusicTrack::Original(if viewer_alliance { 10 } else { 11 })
}

/// The Battle Summary's track (`FUN_0046f360` sets `+0x18c`, which
/// `FUN_00470e70` plays on the first paint): the viewer's win 12 or 14
/// (MDATA.308, MDATA.314), otherwise (no winner, a loss, a withdrawal) 13
/// or 15 (MDATA.309, MDATA.315); the first of each pair for the Alliance.
#[must_use]
pub const fn battle_summary_music(viewer_alliance: bool, viewer_won: bool) -> MusicTrack {
    MusicTrack::Original(match (viewer_alliance, viewer_won) {
        (true, true) => 12,
        (true, false) => 13,
        (false, true) => 14,
        (false, false) => 15,
    })
}

impl MusicTrack {
    /// Whether the track is one of the galaxy view's picks (indices 1..=9),
    /// which does not repeat. A finished track (`0x4c9` in `FUN_00417780`)
    /// plays again only when `FUN_00417610` started it (bit 2 of
    /// `DAT_006b1490`: the battle windows); otherwise
    /// `FUN_00417520(FUN_0041d3b0())` starts the next: the galaxy view's
    /// next pick in a game, MDATA.300 (index 0) again outside one. So the
    /// menu's track and the battle windows' repeat, the galaxy's do not.
    #[must_use]
    pub const fn is_galaxy_pick(self) -> bool {
        matches!(self, Self::Original(1..=9))
    }
}

/// The advisor's mood byte (`agent+0x170`) when nothing sets it: the
/// agent's constructor (`FUN_00439320`) and `FUN_00439bf0` start from 8.
pub const MOOD_EVEN: u8 = 8;

/// `FUN_00439bf0` (the player's vtable slot `0x54`, `0x4866e0`): the
/// advisor's mood from the agent's galaxy model (`agent+0x3c`).
///
/// - `lead` is `agent+0xc4` minus `agent+0xc8` (model `+0x88`, `+0x8c`):
///   above 1 gives 1, exactly 1 gives 2, -1 gives `0x20`, below -1 `0x40`.
/// - At 0 the systems decide: `own` and `opponent` are `agent+0xd4` and
///   `+0xd8`, the model's sums (`FUN_0049cc80` -> `FUN_0049db10`) of the
///   sector records' counts of systems the side holds (`+0x68`, system
///   bit 4) and the opponent holds (`+0x6c`, bit 8; `FUN_004c6830`,
///   `FUN_004ebe40`). The ratio is `own * 100 / opponent`, or `own * 10`
///   when the opponent holds none: 300 or more gives 2, 200 or more 4,
///   50 or less `0x10`; otherwise the mood stays 8. The asm's later test
///   for 33 or less (`0x20`) cannot be reached and is left out.
#[must_use]
pub const fn advisor_mood(lead: i32, own: i32, opponent: i32) -> u8 {
    if lead > 1 {
        return 1;
    }
    if lead == 1 {
        return 2;
    }
    if lead == -1 {
        return 0x20;
    }
    if lead < -1 {
        return 0x40;
    }
    let ratio = if opponent == 0 {
        own.saturating_mul(10)
    } else {
        own.saturating_mul(100) / opponent
    };
    if ratio >= 300 {
        2
    } else if ratio >= 200 {
        4
    } else if ratio <= 50 {
        0x10
    } else {
        MOOD_EVEN
    }
}

/// The advisor's vtable slot `0x1c`: the track index for a mood. The
/// Alliance's class (`0x0065c4c8`, `FUN_004c48e0`) gives 1 -> 4, 2 -> 5,
/// 4 -> 6, `0x10` -> 7, `0x20` -> 8, `0x40` -> 9; the Empire's
/// (`0x0065c4a0`, `FUN_004c2700`) the reverse, 1 -> 9 ... `0x40` -> 4. Any
/// other mood rolls 1 to 3 (`FUN_0041cd80(3) + 1`). `roll(n)` is in `0..n`.
pub fn mood_track_index(viewer_alliance: bool, mood: u8, roll: &mut impl FnMut(u32) -> u32) -> u8 {
    let rank = match mood {
        1 => 0,
        2 => 1,
        4 => 2,
        0x10 => 3,
        0x20 => 4,
        0x40 => 5,
        _ => return random_track_index(roll(3)),
    };
    if viewer_alliance {
        4 + rank
    } else {
        9 - rank
    }
}

/// A roll's track: `value + 1`, kept within 1..=3.
fn random_track_index(value: u32) -> u8 {
    1 + u8::try_from(value.min(2)).unwrap_or(2)
}

/// The galaxy view's score in a game (`FUN_0041d3b0` -> `FUN_0043a250`
/// on the player's agent): three random picks of MDATA.301..303
/// (`FUN_0041cd80(6) / 2 + 1`, counter `agent+0x190`), then one track
/// from the advisor's mood, and again. A new agent's counter is 0, so a
/// new game's first track is the mood's, from the starting mood 8: one of
/// 301..303 (A0 2026-11-15-audio: the Empire's new game played 303, 301,
/// then 302).
///
/// port: the counter and the mood are not saved (the original saves
/// `+0x170` and `+0x190` with the agent, `FUN_004397a0`); a loaded game
/// starts as a new one. The rolls come from the app, not the game's
/// random stream that `FUN_0041cd80` draws from, so the score does not
/// move the simulation.
///
/// port: the browser decodes a track asynchronously, so the app draws the
/// next pick ahead ([`Self::peek`]) to have it decoded when it is due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GalaxyScore {
    picks_left: i32,
    mood: u8,
    /// The pick drawn ahead, and whether it was the mood's.
    upcoming: Option<(MusicTrack, bool)>,
}

impl Default for GalaxyScore {
    fn default() -> Self {
        Self {
            picks_left: 0,
            mood: MOOD_EVEN,
            upcoming: None,
        }
    }
}

impl GalaxyScore {
    /// The advisor's current mood byte.
    #[must_use]
    pub const fn mood(&self) -> u8 {
        self.mood
    }

    /// Store the mood `FUN_00439bf0` computes ([`advisor_mood`]). A mood
    /// pick drawn ahead under another mood is drawn again when due, as the
    /// original draws it then.
    pub fn refresh_mood(&mut self, lead: i32, own: i32, opponent: i32) {
        let mood = advisor_mood(lead, own, opponent);
        if mood != self.mood && matches!(self.upcoming, Some((_, true))) {
            self.upcoming = None;
            self.picks_left = 0;
        }
        self.mood = mood;
    }

    /// The next track (`FUN_0043a250`). `roll(n)` is in `0..n`.
    pub fn next(&mut self, viewer_alliance: bool, roll: impl FnMut(u32) -> u32) -> MusicTrack {
        match self.upcoming.take() {
            Some((track, _)) => track,
            None => self.draw(viewer_alliance, roll).0,
        }
    }

    /// The track [`Self::next`] will give, drawn now if not yet drawn.
    pub fn peek(&mut self, viewer_alliance: bool, roll: impl FnMut(u32) -> u32) -> MusicTrack {
        if let Some((track, _)) = self.upcoming {
            return track;
        }
        let drawn = self.draw(viewer_alliance, roll);
        self.upcoming = Some(drawn);
        drawn.0
    }

    fn draw(
        &mut self,
        viewer_alliance: bool,
        mut roll: impl FnMut(u32) -> u32,
    ) -> (MusicTrack, bool) {
        if self.picks_left > 0 {
            self.picks_left -= 1;
            return (MusicTrack::Original(random_track_index(roll(6) / 2)), false);
        }
        self.picks_left = 3;
        (
            MusicTrack::Original(mood_track_index(viewer_alliance, self.mood, &mut roll)),
            true,
        )
    }
}

// ---------------------------------------------------------------------------
// MusicContext — high-level game context → track selection
// ---------------------------------------------------------------------------

/// High-level game context used by the app to pick a `MusicTrack`.
///
/// The audio engine maps context to track so callers do not hard-code tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MusicContext {
    /// Title screen and setup screens.
    MainMenu,
    /// Galaxy map during normal play.
    GalaxyMap,
    /// Active space battle (tactical or auto-resolve).
    Combat,
    /// Game won.
    Victory,
    /// Game lost.
    Defeat,
}

// ---------------------------------------------------------------------------
// AudioVolumeState — pure UI state, no kira types
// ---------------------------------------------------------------------------

/// Volume and mute state for the audio system.
///
/// Lives in the render layer (no kira dependency).  The app layer reads these
/// values and applies them to the kira audio manager.
///
/// `dirty` is set to `true` whenever a slider or mute toggle changes so the
/// app layer knows to call `apply_volume` without polling every frame.
#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "These independent flags preserve the existing state and serialization model."
)]
pub struct AudioVolumeState {
    /// Music volume in [0.0, 1.0].
    pub music_volume: f32,
    /// SFX volume in [0.0, 1.0].
    pub sfx_volume: f32,
    /// If true, all audio output is silenced.
    pub muted: bool,
    /// If true, music is silenced while sound effects remain audible.
    pub music_muted: bool,
    /// Set when the user changes a control.  The app layer clears this after
    /// applying the change to the audio engine.
    pub dirty: bool,
    /// Whether the underlying audio backend is available.  Written by the app
    /// layer so the panel can display a "no audio" indicator.
    pub backend_available: bool,
}

impl Default for AudioVolumeState {
    fn default() -> Self {
        AudioVolumeState {
            // FUN_004173c0 / FUN_00413000: an absent MusicVolume or Volume
            // reads 0 (FUN_005f41b0), DirectSound's full volume, which the
            // slider shows at 100 (A0 200-options: the music thumb clipped
            // at the track's right end).
            music_volume: 1.0,
            sfx_volume: 1.0,
            muted: false,
            music_muted: false,
            dirty: false,
            backend_available: true,
        }
    }
}

impl AudioVolumeState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Effective music volume: 0.0 when globally or music-only muted.
    #[must_use]
    pub fn effective_music_volume(&self) -> f64 {
        if self.muted || self.music_muted {
            0.0
        } else {
            f64::from(self.music_volume)
        }
    }

    /// Effective SFX volume: 0.0 when muted, otherwise `sfx_volume`.
    #[must_use]
    pub fn effective_sfx_volume(&self) -> f64 {
        if self.muted {
            0.0
        } else {
            f64::from(self.sfx_volume)
        }
    }

    /// Cutscene audio volume: 0.0 when muted, otherwise full volume.
    #[must_use]
    pub const fn cutscene_volume(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            1.0
        }
    }

    /// Toggle music independently of sound effects.
    pub fn toggle_music(&mut self) {
        self.music_muted = !self.music_muted;
        self.dirty = true;
    }

    /// Whether the dedicated music path is enabled.
    #[must_use]
    pub fn music_enabled(&self) -> bool {
        !self.music_muted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_battle_windows_play_the_original_score_by_side_and_outcome() {
        // 0x006a8478 in REBEXE.EXE; FUN_0044f860 plays 10 / 11, FUN_0046f360
        // picks 12..15.
        assert_eq!(battle_alert_music(true).mdata(), Some(307));
        assert_eq!(battle_alert_music(false).mdata(), Some(313));
        assert_eq!(battle_summary_music(true, true).mdata(), Some(308));
        assert_eq!(battle_summary_music(true, false).mdata(), Some(309));
        assert_eq!(battle_summary_music(false, true).mdata(), Some(314));
        assert_eq!(battle_summary_music(false, false).mdata(), Some(315));
        assert_eq!(MusicTrack::Original(0).mdata(), Some(300));
        assert_eq!(MusicTrack::Original(7).mdata(), Some(310));
        assert_eq!(MusicTrack::Original(16).mdata(), None);
        assert_eq!(MusicTrack::Battle.mdata(), None);
    }

    #[test]
    fn only_the_galaxy_picks_move_on_when_they_end() {
        // FUN_00417780 (0x4c9): bit 2 (FUN_00417610) replays the battle
        // windows' tracks; otherwise FUN_0041d3b0 picks 1..=9 in a game, 0
        // outside one.
        assert!(!MusicTrack::Original(0).is_galaxy_pick());
        for index in 1..=9 {
            assert!(MusicTrack::Original(index).is_galaxy_pick());
        }
        for index in 10..=15 {
            assert!(!MusicTrack::Original(index).is_galaxy_pick());
        }
        assert!(!MusicTrack::MainTheme.is_galaxy_pick());
    }

    #[test]
    fn the_advisor_mood_follows_the_lead_then_the_systems_held() {
        // FUN_00439bf0 (0x439bf0..0x439ca4).
        assert_eq!(advisor_mood(2, 0, 9), 1);
        assert_eq!(advisor_mood(1, 0, 9), 2);
        assert_eq!(advisor_mood(-1, 9, 0), 0x20);
        assert_eq!(advisor_mood(-2, 9, 0), 0x40);
        assert_eq!(advisor_mood(0, 30, 10), 2);
        assert_eq!(advisor_mood(0, 29, 10), 4);
        assert_eq!(advisor_mood(0, 20, 10), 4);
        assert_eq!(advisor_mood(0, 19, 10), MOOD_EVEN);
        assert_eq!(advisor_mood(0, 6, 10), MOOD_EVEN);
        assert_eq!(advisor_mood(0, 5, 10), 0x10);
        assert_eq!(advisor_mood(0, 0, 10), 0x10);
        // No opponent system: own * 10.
        assert_eq!(advisor_mood(0, 30, 0), 2);
        assert_eq!(advisor_mood(0, 20, 0), 4);
        assert_eq!(advisor_mood(0, 6, 0), MOOD_EVEN);
        assert_eq!(advisor_mood(0, 5, 0), 0x10);
    }

    #[test]
    fn each_side_maps_the_moods_onto_the_tracks_in_reverse() {
        // FUN_004c48e0 (Alliance, 0x65c4c8) and FUN_004c2700 (Empire,
        // 0x65c4a0): vtable slot 0x1c.
        let mut never = |_| -> u32 { panic!("a set mood does not roll") };
        let moods = [1, 2, 4, 0x10, 0x20, 0x40];
        let alliance: Vec<u8> = moods
            .iter()
            .map(|&mood| mood_track_index(true, mood, &mut never))
            .collect();
        let empire: Vec<u8> = moods
            .iter()
            .map(|&mood| mood_track_index(false, mood, &mut never))
            .collect();
        assert_eq!(alliance, [4, 5, 6, 7, 8, 9]);
        assert_eq!(empire, [9, 8, 7, 6, 5, 4]);
        // MDATA by ORIGINAL_MUSIC: 304, 305, 306, 310, 311, 312.
        let files: Vec<_> = alliance
            .iter()
            .map(|&index| MusicTrack::Original(index).mdata())
            .collect();
        assert_eq!(
            files,
            [
                Some(304),
                Some(305),
                Some(306),
                Some(310),
                Some(311),
                Some(312)
            ]
        );
        for side in [true, false] {
            let mut asked = Vec::new();
            let index = mood_track_index(side, MOOD_EVEN, &mut |n| {
                asked.push(n);
                2
            });
            assert_eq!((index, asked), (3, vec![3]));
        }
    }

    #[test]
    fn the_galaxy_plays_the_mood_track_then_three_random_ones() {
        // FUN_0043a250: counter +0x190 starts at 0 (FUN_00439320), so the
        // first pick is the mood's; then three of FUN_0041cd80(6) / 2 + 1.
        let mut score = GalaxyScore::default();
        let mut asked = Vec::new();
        let mut rolls = [0, 5, 2, 3, 0, 1].into_iter();
        let mut picks = Vec::new();
        for _ in 0..4 {
            picks.push(score.next(false, |n| {
                asked.push(n);
                rolls.next().unwrap()
            }));
        }
        // Starting mood 8 rolls 0..3: 0 -> track 1.
        assert_eq!(
            picks,
            [
                MusicTrack::Original(1),
                MusicTrack::Original(3),
                MusicTrack::Original(2),
                MusicTrack::Original(2),
            ]
        );
        assert_eq!(asked, [3, 6, 6, 6]);
        // The fifth is the mood's again: the Empire far ahead gets 311.
        score.refresh_mood(0, 30, 10);
        let fifth = score.next(false, |_| panic!("a set mood does not roll"));
        assert_eq!(fifth.mdata(), Some(311));
        let sixth = score.next(false, |_| 5);
        assert_eq!(sixth.mdata(), Some(303));
    }

    #[test]
    fn a_pick_drawn_ahead_is_the_next_unless_its_mood_changed() {
        let mut score = GalaxyScore::default();
        // The first pick is the mood's: drawn ahead under mood 8.
        let ahead = score.peek(true, |_| 1);
        assert_eq!(ahead, MusicTrack::Original(2));
        assert_eq!(score.peek(true, |_| panic!("drawn once")), ahead);
        // The mood changes before it is due: drawn again under the new one
        // (the Alliance far behind: 0x10 -> 7, MDATA.310).
        score.refresh_mood(0, 1, 10);
        let due = score.next(true, |_| panic!("a set mood does not roll"));
        assert_eq!(due.mdata(), Some(310));
        // A random pick drawn ahead stays through a mood change.
        let random = score.peek(true, |_| 4);
        assert_eq!(random, MusicTrack::Original(3));
        score.refresh_mood(0, 30, 10);
        assert_eq!(score.next(true, |_| panic!("drawn ahead")), random);
        // The same mood keeps a mood pick drawn ahead.
        let _ = score.next(true, |_| 0);
        let _ = score.next(true, |_| 0);
        let mood_pick = score.peek(true, |_| panic!("a set mood does not roll"));
        assert_eq!(mood_pick.mdata(), Some(305));
        score.refresh_mood(0, 31, 10);
        assert_eq!(score.next(true, |_| panic!("drawn ahead")), mood_pick);
    }

    #[test]
    fn muting_silences_cutscenes_too() {
        let mut state = AudioVolumeState::default();
        assert!((state.cutscene_volume() - 1.0).abs() < f32::EPSILON);
        state.muted = true;
        assert!(state.cutscene_volume().abs() < f32::EPSILON);
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "These regression checks require exact copied values, endpoints, and pixel coordinates."
    )]
    fn mute_and_gain_produce_exact_effective_levels() {
        let mut state = AudioVolumeState {
            music_volume: 0.35,
            sfx_volume: 0.6,
            ..Default::default()
        };
        assert_eq!(state.effective_music_volume(), f64::from(0.35_f32));
        assert_eq!(state.effective_sfx_volume(), f64::from(0.6_f32));

        state.muted = true;
        assert_eq!(state.effective_music_volume(), 0.0);
        assert_eq!(state.effective_sfx_volume(), 0.0);
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "These regression checks require exact copied values, endpoints, and pixel coordinates."
    )]
    fn music_toggle_preserves_sound_effects() {
        let mut state = AudioVolumeState::default();
        state.toggle_music();

        assert!(state.music_muted);
        assert!(state.dirty);
        assert_eq!(state.effective_music_volume(), 0.0);
        assert_eq!(state.effective_sfx_volume(), 1.0);

        state.toggle_music();
        assert!(state.music_enabled());
        assert_eq!(state.effective_music_volume(), 1.0);
    }

    #[test]
    fn both_volumes_start_full_without_stored_settings() {
        // FUN_004173c0 and FUN_00413000 read MusicVolume and Volume with
        // FUN_005f41b0, which gives 0 (0 dB, full volume) when absent.
        let state = AudioVolumeState::default();
        assert_eq!(
            (state.effective_music_volume(), state.effective_sfx_volume()),
            (1.0, 1.0)
        );
    }
}
