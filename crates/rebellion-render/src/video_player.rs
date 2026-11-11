use std::fmt;
use std::path::{Path, PathBuf};

use macroquad::prelude::Texture2D;

use crate::movie_error_box::MovieErrorBox;

#[derive(Debug)]
pub enum VideoError {
    NotDecoded {
        path: PathBuf,
    },
    InvalidPath {
        path: PathBuf,
        message: String,
    },
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidMetadata {
        path: PathBuf,
        message: String,
    },
    Decode {
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for VideoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VideoError::NotDecoded { path } => {
                write!(f, "decoded cutscene assets not found at {}", path.display())
            }
            VideoError::InvalidPath { path, message } => {
                write!(f, "invalid cutscene path {}: {message}", path.display())
            }
            VideoError::Io { path, source } => {
                write!(f, "failed to read {}: {source}", path.display())
            }
            VideoError::InvalidMetadata { path, message } => {
                write!(f, "invalid cutscene metadata {}: {message}", path.display())
            }
            VideoError::Decode { path, message } => {
                write!(f, "failed to decode {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for VideoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VideoError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use image::ImageReader;
    use macroquad::prelude::{FilterMode, Texture2D};
    use quad_snd::{AudioContext, Playback, Sound};
    use serde::Deserialize;

    use super::VideoError;

    const FRAME_BUFFER_AHEAD: usize = 3;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct DecodedAssets {
        frames_dir: PathBuf,
        metadata_path: PathBuf,
        audio_path: PathBuf,
    }

    #[derive(Debug, Clone, Deserialize)]
    struct DecodedMetadata {
        fps: f32,
        width: u32,
        height: u32,
        frame_count: usize,
    }

    struct AudioTrack {
        ctx: AudioContext,
        playback: Playback,
        _sound: Sound,
    }

    pub struct VideoPlayer {
        assets: DecodedAssets,
        metadata: DecodedMetadata,
        frame_index: usize,
        accumulated_time: f32,
        texture: Option<Texture2D>,
        frame_cache: BTreeMap<usize, Vec<u8>>,
        finished: bool,
        audio: Option<AudioTrack>,
    }

    impl VideoPlayer {
        ///
        /// # Errors
        /// Returns an error if the video cannot be opened or its first frame cannot be decoded.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "Rendering uses floating pixel coordinates and fixed-width resource IDs; retain existing rounding and narrowing."
        )]
        pub fn open(path: &Path, volume: f32) -> Result<VideoPlayer, VideoError> {
            let assets = resolve_decoded_assets(path)?;
            if !assets.frames_dir.exists() || !assets.metadata_path.exists() {
                return Err(VideoError::NotDecoded {
                    path: assets.frames_dir.clone(),
                });
            }

            let metadata = load_metadata(&assets.metadata_path)?;
            validate_metadata(&assets.metadata_path, &metadata)?;

            let first_frame =
                load_frame_rgba(&assets.frames_dir, 0, metadata.width, metadata.height)?;
            let texture =
                Texture2D::from_rgba8(metadata.width as u16, metadata.height as u16, &first_frame);
            texture.set_filter(FilterMode::Linear);

            let mut player = VideoPlayer {
                assets,
                metadata,
                frame_index: 0,
                accumulated_time: 0.0,
                texture: Some(texture),
                frame_cache: BTreeMap::from([(0, first_frame)]),
                finished: false,
                audio: None,
            };
            player.audio = load_audio_track(&player.assets.audio_path, volume)?;
            player.prime_cache()?;
            Ok(player)
        }

        pub fn advance(&mut self, dt: f32) {
            if self.finished || self.metadata.frame_count <= 1 {
                return;
            }

            self.accumulated_time += dt.max(0.0);
            let frame_duration = 1.0 / self.metadata.fps;

            while self.accumulated_time >= frame_duration {
                self.accumulated_time -= frame_duration;
                let next_index = self.frame_index + 1;
                if next_index >= self.metadata.frame_count {
                    self.stop();
                    break;
                }

                if let Err(error) = self.display_frame(next_index) {
                    eprintln!("[cutscene] {error}");
                    self.stop();
                    break;
                }
            }
        }

        pub fn current_frame(&self) -> Option<&Texture2D> {
            self.texture.as_ref()
        }

        pub fn is_finished(&self) -> bool {
            self.finished
        }

        pub fn stop(&mut self) {
            self.finished = true;
            if let Some(audio) = self.audio.take() {
                audio.playback.stop(&audio.ctx);
            }
        }

        /// The native player opens every file it plays before it starts.
        pub const fn load_failed(&self) -> bool {
            false
        }

        fn display_frame(&mut self, frame_index: usize) -> Result<(), VideoError> {
            if !self.frame_cache.contains_key(&frame_index) {
                let bytes = load_frame_rgba(
                    &self.assets.frames_dir,
                    frame_index,
                    self.metadata.width,
                    self.metadata.height,
                )?;
                self.frame_cache.insert(frame_index, bytes);
            }

            if let Some(bytes) = self.frame_cache.get(&frame_index) {
                if let Some(texture) = &self.texture {
                    texture.update_from_bytes(self.metadata.width, self.metadata.height, bytes);
                }
            }

            self.frame_index = frame_index;
            self.prime_cache()?;
            Ok(())
        }

        fn prime_cache(&mut self) -> Result<(), VideoError> {
            let end = (self.frame_index + FRAME_BUFFER_AHEAD)
                .min(self.metadata.frame_count.saturating_sub(1));

            for index in (self.frame_index + 1)..=end {
                if !self.frame_cache.contains_key(&index) {
                    let bytes = load_frame_rgba(
                        &self.assets.frames_dir,
                        index,
                        self.metadata.width,
                        self.metadata.height,
                    )?;
                    self.frame_cache.insert(index, bytes);
                }
            }

            let keep_from = self.frame_index.saturating_sub(1);
            self.frame_cache
                .retain(|index, _| *index >= keep_from && *index <= end);
            Ok(())
        }
    }

    fn resolve_decoded_assets(path: &Path) -> Result<DecodedAssets, VideoError> {
        let stem = path.file_stem().ok_or_else(|| VideoError::InvalidPath {
            path: path.to_path_buf(),
            message: "missing file stem".to_string(),
        })?;
        let stem = stem.to_string_lossy();

        let references_dir =
            path.parent()
                .and_then(Path::parent)
                .ok_or_else(|| VideoError::InvalidPath {
                    path: path.to_path_buf(),
                    message: "expected assets/references/ref-videos layout".to_string(),
                })?;
        let cutscene_root = references_dir.join("cutscene-frames");
        let frames_dir = cutscene_root.join(stem.as_ref());

        Ok(DecodedAssets {
            metadata_path: frames_dir.join("metadata.json"),
            frames_dir,
            audio_path: cutscene_root.join(format!("{stem}.wav")),
        })
    }

    fn load_metadata(path: &Path) -> Result<DecodedMetadata, VideoError> {
        let bytes = fs::read(path).map_err(|source| VideoError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_slice(&bytes).map_err(|error| VideoError::InvalidMetadata {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
    }

    fn validate_metadata(path: &Path, metadata: &DecodedMetadata) -> Result<(), VideoError> {
        if metadata.fps <= 0.0 {
            return Err(VideoError::InvalidMetadata {
                path: path.to_path_buf(),
                message: "fps must be > 0".to_string(),
            });
        }
        if metadata.width == 0 || metadata.height == 0 {
            return Err(VideoError::InvalidMetadata {
                path: path.to_path_buf(),
                message: "frame dimensions must be > 0".to_string(),
            });
        }
        if metadata.width > u32::from(u16::MAX) || metadata.height > u32::from(u16::MAX) {
            return Err(VideoError::InvalidMetadata {
                path: path.to_path_buf(),
                message: "frame dimensions exceed macroquad texture limits".to_string(),
            });
        }
        if metadata.frame_count == 0 {
            return Err(VideoError::NotDecoded {
                path: path.parent().unwrap_or(path).to_path_buf(),
            });
        }
        Ok(())
    }

    fn frame_path(frames_dir: &Path, frame_index: usize) -> PathBuf {
        frames_dir.join(format!("frame-{:05}.png", frame_index + 1))
    }

    fn load_frame_rgba(
        frames_dir: &Path,
        frame_index: usize,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, VideoError> {
        let path = frame_path(frames_dir, frame_index);
        if !path.exists() {
            return Err(VideoError::NotDecoded { path });
        }

        let reader = ImageReader::open(&path).map_err(|source| VideoError::Io {
            path: path.clone(),
            source,
        })?;
        let image = reader.decode().map_err(|error| VideoError::Decode {
            path: path.clone(),
            message: error.to_string(),
        })?;
        if image.width() != width || image.height() != height {
            return Err(VideoError::InvalidMetadata {
                path,
                message: format!(
                    "frame dimensions {}x{} did not match metadata {}x{}",
                    image.width(),
                    image.height(),
                    width,
                    height
                ),
            });
        }

        Ok(image.to_rgba8().into_raw())
    }

    fn load_audio_track(path: &Path, volume: f32) -> Result<Option<AudioTrack>, VideoError> {
        if !path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(path).map_err(|source| VideoError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let ctx = AudioContext::new();
        let sound = Sound::load(&ctx, &bytes);
        let playback = sound.play(
            &ctx,
            quad_snd::PlaySoundParams {
                looped: false,
                volume,
            },
        );

        Ok(Some(AudioTrack {
            ctx,
            playback,
            _sound: sound,
        }))
    }

    #[cfg(test)]
    mod tests {
        use std::fs;
        use std::path::{Path, PathBuf};
        use std::time::{SystemTime, UNIX_EPOCH};

        use super::{resolve_decoded_assets, VideoPlayer};
        use crate::video_player::VideoError;

        fn temp_root() -> PathBuf {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            std::env::temp_dir().join(format!("open-rebellion-cutscene-test-{nonce}"))
        }

        #[test]
        fn resolves_ref_video_paths_into_decoded_assets() {
            let source = Path::new("assets/references/ref-videos/201.webm");
            let resolved = resolve_decoded_assets(source).expect("path should resolve");

            assert_eq!(
                resolved.frames_dir,
                PathBuf::from("assets/references/cutscene-frames/201")
            );
            assert_eq!(
                resolved.metadata_path,
                PathBuf::from("assets/references/cutscene-frames/201/metadata.json")
            );
            assert_eq!(
                resolved.audio_path,
                PathBuf::from("assets/references/cutscene-frames/201.wav")
            );
        }

        #[test]
        fn open_returns_not_decoded_when_cutscene_directory_is_missing() {
            let root = temp_root();
            let source = root.join("assets/references/ref-videos/000.webm");
            fs::create_dir_all(source.parent().unwrap()).expect("create temp ref-videos dir");

            match VideoPlayer::open(&source, 1.0) {
                Err(VideoError::NotDecoded { path }) => {
                    assert_eq!(path, root.join("assets/references/cutscene-frames/000"));
                }
                Ok(_) => panic!("expected NotDecoded, got an opened player"),
                Err(other) => panic!("expected NotDecoded, got {other:?}"),
            }

            let _ = fs::remove_dir_all(&root);
        }
    }
}

/// The browser's player: the page plays `data/video/<stem>.webm` in a
/// `<video>` over the canvas (`web/index.html`, `open_rebellion_video`),
/// black around it and keeping its aspect, as the native player draws its
/// decoded frames; the game polls its state and stops it on a skip.
#[cfg(target_arch = "wasm32")]
mod web {
    use std::path::{Path, PathBuf};

    use macroquad::prelude::Texture2D;

    use super::VideoError;

    const ENDED: u32 = 2;
    /// The `<video>`'s `error` event: the file could not be loaded.
    const FAILED: u32 = 3;

    extern "C" {
        fn open_rebellion_video_play(ptr: *const u8, len: usize, volume: f32) -> u32;
        fn open_rebellion_video_state() -> u32;
        fn open_rebellion_video_stop();
    }

    pub struct VideoPlayer {
        finished: bool,
    }

    impl VideoPlayer {
        /// Ask the page to play the movie `path` names (its file stem, as in
        /// `assets/references/ref-videos/000.webm`).
        pub fn open(path: &Path, volume: f32) -> Result<VideoPlayer, VideoError> {
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| VideoError::InvalidPath {
                    path: path.to_path_buf(),
                    message: "missing file stem".to_string(),
                })?;
            let started =
                unsafe { open_rebellion_video_play(stem.as_ptr(), stem.len(), volume) } != 0;
            if started {
                Ok(VideoPlayer { finished: false })
            } else {
                Err(VideoError::NotDecoded {
                    path: PathBuf::from(format!("data/video/{stem}.webm")),
                })
            }
        }

        /// The page plays on its own clock; a movie that ended or failed to
        /// load is finished.
        pub fn advance(&mut self, _dt: f32) {
            if unsafe { open_rebellion_video_state() } >= ENDED {
                self.finished = true;
            }
        }

        /// The page draws the movie itself.
        pub fn current_frame(&self) -> Option<&Texture2D> {
            None
        }

        pub fn is_finished(&self) -> bool {
            self.finished || unsafe { open_rebellion_video_state() } >= ENDED
        }

        pub fn stop(&mut self) {
            unsafe { open_rebellion_video_stop() };
            self.finished = true;
        }

        /// Whether the page could not load the movie (FAILED).
        pub fn load_failed(&self) -> bool {
            !self.finished && unsafe { open_rebellion_video_state() } == FAILED
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
use native::VideoPlayer as Backend;
#[cfg(target_arch = "wasm32")]
use web::VideoPlayer as Backend;

/// A movie: playing, or held by the video manager's error box when its
/// file could not be opened (`FUN_005f6920`, `crate::movie_error_box`).
pub struct VideoPlayer {
    playing: Option<Backend>,
    error_box: Option<MovieErrorBox>,
    shown_path: String,
}

impl VideoPlayer {
    /// Play the movie `path` names.
    ///
    /// # Errors
    /// Returns an error if the movie cannot be opened; the caller then shows
    /// the error box with [`VideoPlayer::unopened`].
    pub fn open(path: &Path, volume: f32) -> Result<VideoPlayer, VideoError> {
        Ok(VideoPlayer {
            playing: Some(Backend::open(path, volume)?),
            error_box: None,
            shown_path: shown_path(path),
        })
    }

    /// A movie whose file could not be opened: the box with its path stands
    /// where the movie would play, and the movie ends when it is answered
    /// (`FUN_005f6920`: `FUN_0060a9e0`, then 0x403).
    #[must_use]
    pub fn unopened(path: &Path) -> VideoPlayer {
        let shown = shown_path(path);
        log_box(&shown);
        VideoPlayer {
            playing: None,
            error_box: Some(MovieErrorBox::new(shown.clone())),
            shown_path: shown,
        }
    }

    pub fn advance(&mut self, dt: f32) {
        if self.error_box.is_some() {
            return;
        }
        if let Some(playing) = self.playing.as_mut() {
            // The browser learns only after the start that the file is
            // missing; the box then stands in for the movie as above.
            if playing.load_failed() {
                log_box(&self.shown_path);
                self.playing = None;
                self.error_box = Some(MovieErrorBox::new(self.shown_path.clone()));
                return;
            }
            playing.advance(dt);
        }
    }

    pub fn current_frame(&self) -> Option<&Texture2D> {
        self.playing.as_ref().and_then(Backend::current_frame)
    }

    /// The movie ended, was skipped, or its error box was answered.
    pub fn is_finished(&self) -> bool {
        self.error_box.is_none()
            && self
                .playing
                .as_ref()
                .is_none_or(|playing| playing.is_finished() && !playing.load_failed())
    }

    /// Skip the movie. The error box takes no skip: it waits for its OK.
    pub fn stop(&mut self) {
        if self.error_box.is_some() {
            return;
        }
        if let Some(playing) = self.playing.as_mut() {
            playing.stop();
        }
    }

    /// The open error box, if the movie's file could not be opened.
    pub fn error_box_mut(&mut self) -> Option<&mut MovieErrorBox> {
        self.error_box.as_mut()
    }

    /// Answer the error box: the movie ends as if it had played (0x403).
    pub fn close_error_box(&mut self) {
        if self.error_box.is_some() {
            macroquad::logging::info!("[cutscene] error_box answered path={}", self.shown_path);
        }
        self.error_box = None;
        self.playing = None;
    }
}

/// Log the box: `MessageBeep(0x10)` has no sound in the port (no asset).
fn log_box(path: &str) {
    macroquad::logging::info!(
        "[cutscene] error_box path={path} caption={}",
        crate::movie_error_box::CAPTION
    );
}

/// The path the box shows. port: the port's movie file stands where the
/// original's `<TooBigLocation>\MDATA\MDATA.NNN` stood; the browser plays
/// `data/video/<stem>.webm`.
fn shown_path(path: &Path) -> String {
    if cfg!(target_arch = "wasm32") {
        path.file_stem().and_then(|stem| stem.to_str()).map_or_else(
            || path.display().to_string(),
            |stem| format!("data/video/{stem}.webm"),
        )
    } else {
        path.display().to_string()
    }
}

#[cfg(test)]
mod wrapper_tests {
    use super::*;

    #[test]
    fn an_unopened_movie_waits_for_its_box_then_ends_like_a_played_one() {
        // FUN_005f6920: FUN_0060a9e0's MessageBoxA returns on OK, then 0x403,
        // which FUN_00421790 handles as 0x402 (the natural end).
        let mut movie = VideoPlayer::unopened(Path::new("assets/references/ref-videos/105.webm"));
        assert!(!movie.is_finished());
        movie.advance(10.0);
        movie.stop();
        assert!(!movie.is_finished(), "a skip does not answer the box");
        assert_eq!(
            movie.error_box_mut().map(|dialog| dialog.path().to_owned()),
            Some("assets/references/ref-videos/105.webm".to_owned())
        );
        movie.close_error_box();
        assert!(movie.is_finished());
        assert!(movie.current_frame().is_none());
    }
}
