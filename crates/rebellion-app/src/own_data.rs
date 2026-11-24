//! Bring your own data: the browser builds the runtime pack from the
//! player's copy of the game (`docs/plans/2026-11-24-bring-your-own-data.md`).
//!
//! When the server has no `data/runtime.orpk` (or the address asks for
//! `?data=own`, which makes that request fail), the game asks the page for
//! `own-data/pack` (index.html, "own data"), which answers with the pack it
//! kept from an earlier visit, or with a list
//! of the files the player picked, each then served as
//! `own-data/file/<index>`. From those this module builds the pack with
//! `rebellion_data::install`, hands it to the page to keep, and returns it.
//!
//! port: not part of the original; the files are the player's own.

use macroquad::file::load_file;
use rebellion_data::install::pack::{build_entries, write_pack, InstallFiles};
use rebellion_data::install::smacker::Smacker;
use rebellion_render::video_player::{set_movie_opener, MovieDecoder};

/// Bumped when the builder's output changes, so a kept pack is rebuilt.
pub const BUILDER_VERSION: u32 = 1;

extern "C" {
    fn open_rebellion_own_data_keep(ptr: *const u8, len: usize, builder: u32);
    fn open_rebellion_own_data_failed(ptr: *const u8, len: usize);
    fn open_rebellion_own_data_offered() -> u32;
}

/// Whether the page offers the player's own files (not with `?data=server`).
pub fn offered() -> bool {
    unsafe { open_rebellion_own_data_offered() != 0 }
}

/// The page asks which builder made a kept pack before serving it.
#[no_mangle]
pub extern "C" fn open_rebellion_own_data_builder() -> u32 {
    BUILDER_VERSION
}

/// Tell the page the pack could not be built; it shows the reason.
pub fn report_failure(reason: &str) {
    unsafe { open_rebellion_own_data_failed(reason.as_ptr(), reason.len()) };
}

/// A Smacker movie from the player's MDATA files, for the video player.
struct SmackerMovie(Smacker);

impl MovieDecoder for SmackerMovie {
    fn size(&self) -> (u32, u32) {
        (self.0.width() as u32, self.0.height() as u32)
    }

    fn frame_duration_us(&self) -> u64 {
        self.0.frame_duration_us()
    }

    fn next_frame_rgba(&mut self, out: &mut Vec<u8>) -> Option<Result<(), String>> {
        self.0.next_frame_rgba(out).map(|done| done.map_err(|error| error.to_string()))
    }

    fn audio_wav(&self) -> Option<Result<Vec<u8>, String>> {
        self.0.audio_wav(0).map(|wav| wav.map_err(|error| error.to_string()))
    }
}

fn open_smacker(bytes: Vec<u8>) -> Result<Box<dyn MovieDecoder>, String> {
    Smacker::open(bytes)
        .map(|movie| Box::new(SmackerMovie(movie)) as Box<dyn MovieDecoder>)
        .map_err(|error| error.to_string())
}

/// The runtime pack from the player's own files, when the page is in that
/// mode: `None` when it is not, else the pack or why it could not be built.
/// The movies then play from the player's files too (the page serves them
/// when the video player asks).
pub async fn pack() -> Option<Result<Vec<u8>, String>> {
    set_movie_opener(open_smacker);
    let answer = load_file("own-data/pack").await.ok()?;
    if answer.starts_with(b"ORPK") {
        return Some(Ok(answer));
    }
    Some(build(&answer).await)
}

async fn build(listing: &[u8]) -> Result<Vec<u8>, String> {
    let listing = std::str::from_utf8(listing).map_err(|_| "the file list is not UTF-8".to_string())?;
    let mut install = InstallFiles::default();
    for (index, path) in listing.lines().filter(|line| !line.is_empty()).enumerate() {
        let bytes = load_file(&format!("own-data/file/{index}"))
            .await
            .map_err(|error| format!("{path} could not be read: {error:?}"))?;
        install.insert(path, bytes);
    }
    let entries = build_entries(&install).map_err(|error| error.to_string())?;
    drop(install);
    let pack = write_pack(&entries);
    drop(entries);
    unsafe { open_rebellion_own_data_keep(pack.as_ptr(), pack.len(), BUILDER_VERSION) };
    macroquad::logging::info!("own_data built the runtime pack from the player's files: {} bytes", pack.len());
    Ok(pack)
}
