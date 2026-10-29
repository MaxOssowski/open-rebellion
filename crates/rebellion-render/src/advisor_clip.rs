//! The agent droid's clip player: the `0xdead` player (`Class_0065a358`,
//! `FUN_004727e0`) of the advisor panel (`FUN_0042adb0`), which plays the
//! clips the advisor's messages queue (`ghidra/notes/advisor-briefing.md`).
//!
//! A queued clip names an op, the DLL slot and RCDATA record of its clip,
//! and an action. `FUN_00403ed0` makes the op's clip class:
//!
//! - op 4 (`Class_00658520`): a clip definition, which starts its voice,
//!   shows its first frame and adds one type-302 frame a tick
//!   (`FUN_00404440`, `FUN_00404360`);
//! - op `0x13` (`Class_006588a8`): the same definition, loaded by a copy
//!   of `FUN_00404440` (`FUN_00412c40`); only its stop differs
//!   (`FUN_00412b80`);
//! - op `0x11` (`Class_00658848`): a frame list, which shows its frame for
//!   one tick (`FUN_00412280`, `FUN_00412260`);
//! - op `0x14` (`Class_006587e8`): the agent holds a frame while the second
//!   droid's player (`0xbeef`) plays a record and then op `0x15`
//!   (`FUN_00411b80`, `FUN_00411b60`);
//! - op `0x15` (`Class_00658818`): a frame for one tick, whose end lets the
//!   held op `0x14` end (`FUN_00411f00`, `FUN_00412080`).
//!
//! When a clip ends, the player hands its action on (`FUN_00412400` ->
//! `FUN_0041d140`) and loads the next queued clip in the same tick
//! (`FUN_004729c0`).

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use crate::advisor::{
    add_type302_deltas, advisor_resource, decode_anchor_bitmap, indexed_frame, AdvisorFaction,
    AdvisorFrameBase, AdvisorResource, DecodedAdvisorFrame,
};

/// The panel's tick: `FUN_0042adb0` creates both apertures with a period
/// of `0x43` ms (`FUN_00441a60`), and `FUN_00441d70` steps the players
/// once per elapsed period.
pub const CLIP_TICK_SECONDS: f32 = 0.067;

/// The DLL slot of the advisor's sprite DLL (`FUN_0042d650`).
pub const SPRITE_SLOT: u16 = 9;
/// The DLL slot of the briefing DLL (`FUN_0042d650`).
pub const BRIEFING_SLOT: u16 = 0xd;

/// The op of a clip definition (`Class_00658520`).
pub const OP_DEFINITION: u16 = 4;
/// The op of a definition whose stop lets its frames run out
/// (`Class_006588a8`): the quips (C3POACT `0x50..0x55`, IMP22ACT
/// `0x4e..0x53`). Its loader `FUN_00412c40` is `FUN_00404440` instruction
/// for instruction, and its step `FUN_00412b80` differs from
/// `FUN_00404360` only once `+0x64` is set (vt `+0x24`), which ends an op-4
/// clip and its sounds at once. Neither class takes an action (vt `+0x14`
/// is the empty `FUN_00403ef0`).
pub const OP_UNSTOPPED_DEFINITION: u16 = 0x13;
/// The op of a frame list (`Class_00658848`).
pub const OP_FRAME_LIST: u16 = 0x11;
/// The op that holds the agent while the second droid plays
/// (`Class_006587e8`).
pub const OP_HOLD_FOR_SECOND: u16 = 0x14;
/// The op whose end releases [`OP_HOLD_FOR_SECOND`] (`Class_00658818`).
pub const OP_SECOND_DONE: u16 = 0x15;
/// The record `FUN_00411b80` queues on the second player after the clip's
/// own: `[0x15, 0x29a]`, the second droid's still frame.
const SECOND_DONE_RECORD: u16 = 0x2d37;

/// What a clip does when it ends: `FUN_0041d140(kind, code)`, which
/// `FUN_00439eb0` stores as the pending map action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClipAction {
    pub kind: u32,
    pub code: u32,
}

/// A clip definition record (op 4), read by `FUN_00404440` as 16-bit
/// words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipDefinition {
    /// A sound in the record's own slot; 0 for none (`FUN_00403f00`).
    pub sound: u16,
    /// A WAVE id; 0 for none (`FUN_00403f70`).
    pub wave: u16,
    /// The DLL slot holding `wave`.
    pub wave_slot: u16,
    /// The number of frames, the first included (`+0x4c`).
    pub frame_count: u16,
    /// The first frame, a bitmap; frames `first + 1 ..` are type-302
    /// deltas (`FUN_0041c590`).
    pub first_frame: u16,
}

impl ClipDefinition {
    /// The definition in `bytes`, or `None` when it is shorter than one
    /// layer's five words.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let word = |index: usize| {
            bytes
                .get(index * 2..index * 2 + 2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        };
        Some(Self {
            sound: word(0)?,
            wave: word(1)?,
            wave_slot: word(2)?,
            frame_count: word(3)?,
            first_frame: word(4)?,
        })
    }
}

/// The staged directory of the DLL in `slot` for `faction`'s advisor, or
/// `None` for a slot the advisor does not load.
#[must_use]
pub fn slot_directory(slot: u16, faction: AdvisorFaction) -> Option<&'static str> {
    match (slot, faction) {
        (SPRITE_SLOT, AdvisorFaction::Alliance) => Some("alsprite-dll"),
        (SPRITE_SLOT, AdvisorFaction::Empire) => Some("emsprite-dll"),
        (BRIEFING_SLOT, AdvisorFaction::Alliance) => Some("albrief-dll"),
        (BRIEFING_SLOT, AdvisorFaction::Empire) => Some("embrief-dll"),
        _ => None,
    }
}

/// One clip waiting for the player (`FUN_00472930`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueuedClip {
    pub op: u16,
    pub slot: u16,
    pub record: u16,
    pub action: ClipAction,
}

/// What the player asks of its owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipEvent {
    /// Start the WAVE `wave` of the staged DLL `directory`. The owner
    /// answers with [`ClipPlayer::voice_started`].
    Voice { directory: &'static str, wave: u16 },
    /// A clip ended; run its action.
    Finished(ClipAction),
    /// A stopped op-4 clip cuts its voice (`FUN_00404360` with `+0x64`:
    /// `FUN_00404020(1)`, `FUN_00404040(1)`).
    StopVoice { directory: &'static str, wave: u16 },
}

#[derive(Debug)]
enum Playing {
    Definition {
        directory: &'static str,
        base: AdvisorFrameBase,
        next_frame: u16,
        last_frame: u16,
        voice_left: f32,
        /// The voice it started, which a stop cuts.
        voice: Option<(&'static str, u16)>,
        /// Op 4 ends at once when stopped and cuts its voice
        /// (`FUN_00404360`); op `0x13` runs out its frames first, then ends
        /// without waiting for its voice (`FUN_00412b80`).
        stops_at_once: bool,
        /// `+0x64`, set by the class's stop (vt `+0x24`, `FUN_00404740`).
        stopped: bool,
    },
    Still {
        ticks_left: u32,
        /// Op `0x15`: its end sets `DAT_006b12b0` (`FUN_00412080`).
        releases_hold: bool,
    },
    /// Op `0x14` (`FUN_00411b60`): done once `DAT_006b12b0` is set.
    Hold,
}

#[derive(Debug)]
struct PlayingClip {
    playing: Playing,
    action: ClipAction,
}

/// The `0xdead` player's queue and its current clip.
#[derive(Debug)]
pub struct ClipPlayer {
    faction: AdvisorFaction,
    asset_root: PathBuf,
    queue: VecDeque<QueuedClip>,
    current: Option<PlayingClip>,
    accumulator: f32,
    image: Option<DecodedAdvisorFrame>,
    image_serial: u64,
    events: Vec<ClipEvent>,
    /// The second droid's player (`0xbeef`, `FUN_0042b290`); only the
    /// agent's player has one.
    second: Option<Box<ClipPlayer>>,
    /// `DAT_006b12b0`: an op `0x15` clip ended.
    second_done: bool,
    /// Whose clip each [`ClipEvent::Voice`] not yet answered belongs to:
    /// `true` for the second player's.
    voice_owners: VecDeque<bool>,
}

impl ClipPlayer {
    #[must_use]
    pub fn new(faction: AdvisorFaction, asset_root: impl Into<PathBuf>) -> Self {
        let asset_root: PathBuf = asset_root.into();
        Self {
            faction,
            asset_root: asset_root.clone(),
            queue: VecDeque::new(),
            current: None,
            accumulator: 0.0,
            image: None,
            image_serial: 0,
            events: Vec::new(),
            second: Some(Box::new(Self::second(faction, asset_root))),
            second_done: false,
            voice_owners: VecDeque::new(),
        }
    }

    /// The second droid's player, which has no player of its own.
    fn second(faction: AdvisorFaction, asset_root: impl Into<PathBuf>) -> Self {
        Self {
            faction,
            asset_root: asset_root.into(),
            queue: VecDeque::new(),
            current: None,
            accumulator: 0.0,
            image: None,
            image_serial: 0,
            events: Vec::new(),
            second: None,
            second_done: false,
            voice_owners: VecDeque::new(),
        }
    }

    /// Point the player at another side's DLLs or staged root; the queue
    /// and the current clip are dropped.
    pub fn reset(&mut self, faction: AdvisorFaction, asset_root: &Path) {
        *self = Self::new(faction, asset_root);
    }

    /// Queue a clip behind those already waiting. As the player's slot 8
    /// does (`FUN_00472930`), a clip still playing is asked to stop (its vt
    /// `+0x24`).
    pub fn queue(&mut self, clip: QueuedClip) {
        self.queue.push_back(clip);
        self.stop_current();
    }

    /// The current clip's stop (vt `+0x24`): op 4 and op `0x13` set `+0x64`
    /// (`FUN_00404740`), a frame list or op `0x15` zeroes its ticks
    /// (`FUN_00412090`), and op `0x14` sets a flag its step never reads
    /// (`FUN_00411d30`).
    fn stop_current(&mut self) {
        let Some(clip) = &mut self.current else {
            return;
        };
        match &mut clip.playing {
            Playing::Definition { stopped, .. } => *stopped = true,
            Playing::Still { ticks_left, .. } => *ticks_left = 0,
            Playing::Hold => {}
        }
    }

    /// Flush the player as a skip does (`FUN_0042d620` -> `FUN_00472ba0`
    /// for both players): the queue empties and the current clip is asked
    /// to stop, so it ends at its next step (running its action, as its end
    /// slot does) or, as op `0x13` does, once its frames run out.
    pub fn flush(&mut self) {
        self.queue.clear();
        self.stop_current();
        if let Some(second) = &mut self.second {
            second.flush();
        }
    }

    /// Queue a message entry's record from DLL `slot` (`FUN_0042b1d0`): the
    /// record is `[op, clip record, a, b]`, and the entry's `a` and `b`
    /// replace its last two words when not 0. A record that cannot be read
    /// queues nothing.
    pub fn queue_entry(&mut self, slot: u16, record: u16, a: u32, b: u32) {
        let Some(directory) = slot_directory(slot, self.faction) else {
            return;
        };
        let Some(words) =
            advisor_resource(&self.asset_root, directory, AdvisorResource::Record, record)
        else {
            return;
        };
        let word = |index: usize| {
            words
                .get(index * 2..index * 2 + 2)
                .map_or(0, |pair| u16::from_le_bytes([pair[0], pair[1]]))
        };
        let kind = if a == 0 { u32::from(word(2)) } else { a };
        let code = if b == 0 { u32::from(word(3)) } else { b };
        self.queue(QueuedClip {
            op: word(0),
            slot,
            record: word(1),
            action: ClipAction { kind, code },
        });
    }

    /// Drop the current clip and every queued one, running no action.
    pub fn clear(&mut self) {
        self.queue.clear();
        self.current = None;
        self.image = None;
        self.second_done = false;
        self.voice_owners.clear();
        if let Some(second) = &mut self.second {
            second.clear();
        }
    }

    /// Whether a clip is showing or waiting.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.current.is_some()
            || !self.queue.is_empty()
            || self.second.as_ref().is_some_and(|second| second.is_busy())
    }

    /// The frame the agent aperture shows while a clip plays, and a serial
    /// that changes with each new frame.
    #[must_use]
    pub fn frame(&self) -> Option<(&DecodedAdvisorFrame, u64)> {
        self.image.as_ref().map(|image| (image, self.image_serial))
    }

    /// The frame the second droid's aperture shows while its player plays.
    #[must_use]
    pub fn second_frame(&self) -> Option<(&DecodedAdvisorFrame, u64)> {
        self.second.as_ref().and_then(|second| second.frame())
    }

    /// The length of the voice the last [`ClipEvent::Voice`] started; the
    /// clip holds its last frame until it ends.
    pub fn voice_started(&mut self, seconds: f32) {
        if self.voice_owners.pop_front() == Some(true) {
            if let Some(second) = &mut self.second {
                second.voice_started(seconds);
            }
            return;
        }
        if let Some(PlayingClip {
            playing: Playing::Definition { voice_left, .. },
            ..
        }) = &mut self.current
        {
            *voice_left = seconds.max(0.0);
        }
    }

    /// Advance by `dt` seconds of panel time and return what the owner must
    /// do.
    pub fn update(&mut self, dt: f32) -> Vec<ClipEvent> {
        if self.is_busy() {
            self.accumulator += dt;
        } else {
            self.accumulator = 0.0;
        }
        while self.accumulator >= CLIP_TICK_SECONDS {
            self.accumulator -= CLIP_TICK_SECONDS;
            self.tick();
        }
        std::mem::take(&mut self.events)
    }

    /// One panel period (`FUN_004729c0`): the current clip steps, and a
    /// clip that ends hands on its action; with no clip, the next queued
    /// one loads.
    fn tick(&mut self) {
        self.tick_own();
        // hyp: the second droid's aperture steps after the agent's in
        // FUN_00441d70.
        if let Some(second) = &mut self.second {
            second.tick();
            if std::mem::take(&mut second.second_done) {
                self.second_done = true;
            }
            for event in std::mem::take(&mut second.events) {
                // Its clips take no action (op 4's +0x14 is empty, and op
                // 0x15's end only sets the flag); its voices play.
                match event {
                    ClipEvent::Voice { .. } => {
                        self.voice_owners.push_back(true);
                        self.events.push(event);
                    }
                    ClipEvent::StopVoice { .. } => self.events.push(event),
                    ClipEvent::Finished(_) => {}
                }
            }
        }
    }

    fn tick_own(&mut self) {
        if let Some(mut clip) = self.current.take() {
            if self.step(&mut clip) {
                self.events.push(ClipEvent::Finished(clip.action));
                self.image = None;
            } else {
                self.current = Some(clip);
            }
        }
        while self.current.is_none() {
            let Some(next) = self.queue.pop_front() else {
                return;
            };
            if let Some(clip) = self.load(next) {
                self.current = Some(clip);
            } else {
                // port: an op the port does not play, or a missing
                // resource, ends at once with its action.
                self.events.push(ClipEvent::Finished(next.action));
            }
        }
    }

    fn step(&mut self, clip: &mut PlayingClip) -> bool {
        match &mut clip.playing {
            // FUN_00404360: add the next frame; with none left the clip
            // ends once its voice has (FUN_00403fe0).
            Playing::Definition {
                directory,
                base,
                next_frame,
                last_frame,
                voice_left,
                voice,
                stops_at_once,
                stopped,
            } => {
                if *stopped && *stops_at_once {
                    if let Some((directory, wave)) = *voice {
                        self.events.push(ClipEvent::StopVoice { directory, wave });
                    }
                    return true;
                }
                *voice_left = (*voice_left - CLIP_TICK_SECONDS).max(0.0);
                if *next_frame <= *last_frame {
                    let id = *next_frame;
                    *next_frame += 1;
                    let applied = advisor_resource(
                        &self.asset_root,
                        directory,
                        AdvisorResource::Frame,
                        id,
                    )
                    .is_some_and(|bytes| {
                        add_type302_deltas(&bytes, base.width, base.height, &mut base.indices)
                            .is_ok()
                    });
                    if !applied {
                        return true;
                    }
                    self.show(base);
                }
                // hyp: the clip waits for its voice (FUN_00403fe0 with
                // flag bit 5, whose writer is unread); a stopped op 0x13
                // does not (FUN_00412b80 returns +0x64).
                *next_frame > *last_frame && (*voice_left <= 0.0 || *stopped)
            }
            // FUN_00412260: one tick (+0x50 = 1, FUN_00412180), then done;
            // op 0x15's end sets DAT_006b12b0 (FUN_00412080).
            Playing::Still {
                ticks_left,
                releases_hold,
            } => {
                if *ticks_left > 0 {
                    *ticks_left -= 1;
                    false
                } else {
                    if *releases_hold {
                        self.second_done = true;
                    }
                    true
                }
            }
            // FUN_00411b60 returns DAT_006b12b0.
            Playing::Hold => std::mem::take(&mut self.second_done),
        }
    }

    fn load(&mut self, clip: QueuedClip) -> Option<PlayingClip> {
        let directory = slot_directory(clip.slot, self.faction)?;
        let record = advisor_resource(
            &self.asset_root,
            directory,
            AdvisorResource::Record,
            clip.record,
        )?;
        let playing = match clip.op {
            // port: the stop (+0x64) is not ported, so both play alike.
            OP_DEFINITION | OP_UNSTOPPED_DEFINITION => {
                let definition = ClipDefinition::parse(&record)?;
                let base = self.bitmap(directory, definition.first_frame)?;
                self.show(&base);
                let mut voice = None;
                if definition.wave != 0 {
                    if let Some(voice_directory) =
                        slot_directory(definition.wave_slot, self.faction)
                    {
                        if self.second.is_some() {
                            self.voice_owners.push_back(false);
                        }
                        self.events.push(ClipEvent::Voice {
                            directory: voice_directory,
                            wave: definition.wave,
                        });
                        voice = Some((voice_directory, definition.wave));
                    }
                }
                Playing::Definition {
                    directory,
                    base,
                    next_frame: definition.first_frame + 1,
                    last_frame: definition.first_frame + definition.frame_count.max(1) - 1,
                    voice_left: 0.0,
                    voice,
                    stops_at_once: clip.op == OP_DEFINITION,
                    stopped: false,
                }
            }
            OP_FRAME_LIST | OP_SECOND_DONE => {
                let first = record.get(0..2)?;
                let base = self.bitmap(directory, u16::from_le_bytes([first[0], first[1]]))?;
                self.show(&base);
                Playing::Still {
                    ticks_left: 1,
                    releases_hold: clip.op == OP_SECOND_DONE,
                }
            }
            // FUN_00411b80: [count, a frame per layer, the second player's
            // record]; it shows its frame, clears DAT_006b12b0, and queues
            // the record and 0x2d37 on the second player (FUN_0041d690 ->
            // FUN_0042b290, from slot 9).
            OP_HOLD_FOR_SECOND => {
                let word = |index: usize| {
                    record
                        .get(index * 2..index * 2 + 2)
                        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                };
                let base = self.bitmap(directory, word(1)?)?;
                let second_record = word(2)?;
                let second = self.second.as_mut()?;
                self.second_done = false;
                second.queue_entry(SPRITE_SLOT, second_record, 0, 0);
                second.queue_entry(SPRITE_SLOT, SECOND_DONE_RECORD, 0, 0);
                self.show(&base);
                Playing::Hold
            }
            _ => return None,
        };
        Some(PlayingClip {
            playing,
            action: clip.action,
        })
    }

    fn bitmap(&self, directory: &str, id: u16) -> Option<AdvisorFrameBase> {
        let bytes = advisor_resource(&self.asset_root, directory, AdvisorResource::Bitmap, id)?;
        decode_anchor_bitmap(&bytes).ok()
    }

    fn show(&mut self, base: &AdvisorFrameBase) {
        if let Ok(frame) = indexed_frame(base.width, base.height, &base.indices, &base.palette) {
            self.image = Some(frame);
            self.image_serial += 1;
        }
    }
}

/// The length in seconds of a RIFF WAVE file, from its format and data
/// chunks; 0 when they are missing.
#[must_use]
pub fn wave_seconds(bytes: &[u8]) -> f32 {
    let word = |slice: &[u8]| slice.try_into().map_or(0, u32::from_le_bytes);
    let (mut byte_rate, mut data_len) = (0u32, 0u32);
    let mut chunks = bytes.get(12..).unwrap_or_default();
    while let (Some(id), Some(size)) = (chunks.get(..4), chunks.get(4..8)) {
        let size = word(size);
        match id {
            b"fmt " => byte_rate = chunks.get(16..20).map_or(0, word),
            b"data" => data_len = size,
            _ => {}
        }
        let next = 8 + size as usize + (size as usize & 1);
        chunks = chunks.get(next..).unwrap_or_default();
    }
    if byte_rate == 0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "WAVE lengths are far below f32's exact integer range."
    )]
    let seconds = data_len as f32 / byte_rate as f32;
    seconds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisor::tests::{indexed_bmp_fixture, type302_fixture};

    /// A scratch directory removed when dropped.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("advisor-clip-{}-{serial}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A staged root with `albrief-dll` and `alsprite-dll`: a definition
    /// 0x103b of `frames` frames from BMP 2101, and a frame list 0x18f
    /// naming BMP 2001.
    fn staged_root(frames: u16) -> TempDir {
        let root = TempDir::new();
        let brief = root.path().join("albrief-dll");
        for kind in ["BMP", "TYPE302", "RCDATA"] {
            std::fs::create_dir_all(brief.join(kind)).unwrap();
        }
        std::fs::write(brief.join("BMP/2101.bmp"), indexed_bmp_fixture()).unwrap();
        for id in 2102..2101 + frames {
            std::fs::write(brief.join(format!("TYPE302/{id}.bin")), type302_fixture()).unwrap();
        }
        let words: [u16; 5] = [0, 1155, BRIEFING_SLOT, frames, 2101];
        let record: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
        std::fs::write(brief.join("RCDATA/4155.bin"), record).unwrap();
        let sprite = root.path().join("alsprite-dll");
        for kind in ["BMP", "RCDATA"] {
            std::fs::create_dir_all(sprite.join(kind)).unwrap();
        }
        std::fs::write(sprite.join("BMP/2001.bmp"), indexed_bmp_fixture()).unwrap();
        std::fs::write(sprite.join("RCDATA/399.bin"), 0x7d1_u16.to_le_bytes()).unwrap();
        root
    }

    fn ticks(player: &mut ClipPlayer, count: u32) -> Vec<ClipEvent> {
        (0..count)
            .flat_map(|_| player.update(CLIP_TICK_SECONDS + 1e-4))
            .collect()
    }

    #[test]
    fn a_definition_record_reads_sound_wave_slot_count_and_first_frame() {
        // FUN_00404440 on ALBRIEF RCDATA 0x103b, the briefing's first clip.
        let record = [0, 0, 0x83, 0x04, 0x0d, 0, 0x89, 0, 0x35, 0x08];
        assert_eq!(
            ClipDefinition::parse(&record),
            Some(ClipDefinition {
                sound: 0,
                wave: 1155,
                wave_slot: 0xd,
                frame_count: 137,
                first_frame: 2101,
            })
        );
        assert_eq!(ClipDefinition::parse(&record[..8]), None);
    }

    #[test]
    fn slots_nine_and_thirteen_are_the_sides_sprite_and_briefing_dlls() {
        // FUN_0042d650 loads alsprite/emsprite into slot 9 and
        // albrief/embrief into slot 0xd.
        assert_eq!(slot_directory(9, AdvisorFaction::Alliance), Some("alsprite-dll"));
        assert_eq!(slot_directory(9, AdvisorFaction::Empire), Some("emsprite-dll"));
        assert_eq!(slot_directory(0xd, AdvisorFaction::Alliance), Some("albrief-dll"));
        assert_eq!(slot_directory(0xd, AdvisorFaction::Empire), Some("embrief-dll"));
        assert_eq!(slot_directory(2, AdvisorFaction::Alliance), None);
    }

    #[test]
    fn a_definition_starts_its_voice_adds_a_frame_a_tick_and_ends_after_its_voice() {
        // FUN_00404440 / FUN_00404360 for op 4, and their copies
        // FUN_00412c40 / FUN_00412b80 for op 0x13.
        for op in [OP_DEFINITION, OP_UNSTOPPED_DEFINITION] {
            a_definition_plays(op);
        }
    }

    fn a_definition_plays(op: u16) {
        let root = staged_root(3);
        let mut player = ClipPlayer::new(AdvisorFaction::Alliance, root.path());
        let action = ClipAction { kind: 2, code: 1 };
        player.queue(QueuedClip {
            op,
            slot: BRIEFING_SLOT,
            record: 0x103b,
            action,
        });

        // The first tick loads the clip: its first frame and its voice.
        assert_eq!(
            ticks(&mut player, 1),
            [ClipEvent::Voice {
                directory: "albrief-dll",
                wave: 1155
            }]
        );
        player.voice_started(4.0 * CLIP_TICK_SECONDS);
        let (first, first_serial) = player.frame().map(|(f, s)| (f.indices.clone(), s)).unwrap();
        // Two deltas, one a tick, each added to the frame before.
        assert!(ticks(&mut player, 1).is_empty());
        let (second, _) = player.frame().unwrap();
        assert_eq!(second.indices[1], first[1].wrapping_add(4));
        assert!(ticks(&mut player, 1).is_empty());
        let (third, third_serial) = player.frame().unwrap();
        assert_eq!(third.indices[1], first[1].wrapping_add(8));
        assert_eq!(third_serial, first_serial + 2);
        // The frames are done; the clip holds until its voice ends.
        assert!(ticks(&mut player, 1).is_empty());
        assert_eq!(ticks(&mut player, 1), [ClipEvent::Finished(action)]);
        assert!(player.frame().is_none());
        assert!(!player.is_busy());
    }

    #[test]
    fn a_frame_list_shows_its_frame_for_one_tick_then_runs_its_action() {
        // FUN_00412260 with +0x50 = 1 (FUN_00412180).
        let root = staged_root(2);
        let mut player = ClipPlayer::new(AdvisorFaction::Alliance, root.path());
        let action = ClipAction { kind: 2, code: 0xc };
        player.queue(QueuedClip {
            op: OP_FRAME_LIST,
            slot: SPRITE_SLOT,
            record: 0x18f,
            action,
        });

        assert!(ticks(&mut player, 1).is_empty());
        assert!(player.frame().is_some());
        assert!(ticks(&mut player, 1).is_empty());
        assert_eq!(ticks(&mut player, 1), [ClipEvent::Finished(action)]);
    }

    #[test]
    fn the_next_clip_loads_in_the_tick_the_last_one_ends() {
        // FUN_004729c0 loads the next queued clip once the current ends.
        let root = staged_root(2);
        let mut player = ClipPlayer::new(AdvisorFaction::Alliance, root.path());
        let first = ClipAction { kind: 2, code: 1 };
        let second = ClipAction { kind: 2, code: 0xe };
        for (op, slot, record, action) in [
            (OP_FRAME_LIST, SPRITE_SLOT, 0x18f, first),
            (OP_DEFINITION, BRIEFING_SLOT, 0x103b, second),
        ] {
            player.queue(QueuedClip {
                op,
                slot,
                record,
                action,
            });
        }

        assert!(ticks(&mut player, 2).is_empty());
        assert_eq!(
            ticks(&mut player, 1),
            [
                ClipEvent::Finished(first),
                ClipEvent::Voice {
                    directory: "albrief-dll",
                    wave: 1155
                }
            ]
        );
    }

    #[test]
    fn a_hold_shows_its_frame_until_the_second_droid_has_played_and_shown_its_still() {
        // FUN_00411b80: op 0x14's record [count, frame, second record]; the
        // second player (FUN_0042b290, slot 9) plays the record, then 0x2d37
        // = [0x15, 0x29a], whose end (FUN_00412080) sets DAT_006b12b0, which
        // ends the hold (FUN_00411b60). Records as ALSPRITE 4412, 10512,
        // 5012 and 0x29a.
        let root = staged_root(2);
        let sprite = root.path().join("alsprite-dll");
        std::fs::create_dir_all(sprite.join("TYPE302")).unwrap();
        let write = |id: u16, words: &[u16]| {
            let bytes: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
            std::fs::write(sprite.join(format!("RCDATA/{id}.bin")), bytes).unwrap();
        };
        write(4412, &[0, 2001, 10512]);
        write(10512, &[OP_DEFINITION, 5012, 0, 0]);
        write(5012, &[0, 1512, SPRITE_SLOT, 3, 3331]);
        write(0x2d37, &[OP_SECOND_DONE, 0x29a, 0, 0]);
        write(0x29a, &[3331]);
        std::fs::write(sprite.join("BMP/3331.bmp"), indexed_bmp_fixture()).unwrap();
        for id in 3332..3334 {
            std::fs::write(sprite.join(format!("TYPE302/{id}.bin")), type302_fixture()).unwrap();
        }
        let mut player = ClipPlayer::new(AdvisorFaction::Alliance, root.path());
        let action = ClipAction { kind: 0, code: 0 };
        player.queue(QueuedClip {
            op: OP_HOLD_FOR_SECOND,
            slot: SPRITE_SLOT,
            record: 4412,
            action,
        });

        // The hold shows its frame; the second player loads its clip and
        // starts its voice, which is the second player's to time.
        assert_eq!(
            ticks(&mut player, 1),
            [ClipEvent::Voice {
                directory: "alsprite-dll",
                wave: 1512
            }]
        );
        player.voice_started(2.0 * CLIP_TICK_SECONDS);
        assert!(player.frame().is_some());
        let (_, second_serial) = player.second_frame().unwrap();
        // Two deltas, then the voice; the agent holds throughout.
        assert!(ticks(&mut player, 2).is_empty());
        assert!(player.second_frame().unwrap().1 > second_serial);
        assert!(player.frame().is_some());
        // The definition ends with its voice, and 0x2d37 shows the still
        // for a tick; its end releases the hold the next period.
        let mut events = Vec::new();
        for _ in 0..6 {
            events.extend(ticks(&mut player, 1));
            if !events.is_empty() {
                break;
            }
        }
        assert_eq!(events, [ClipEvent::Finished(action)]);
        assert!(player.frame().is_none());
        assert!(player.second_frame().is_none());
        assert!(!player.is_busy());
    }

    #[test]
    fn an_unplayable_clip_ends_at_once_with_its_action() {
        let root = staged_root(2);
        let mut player = ClipPlayer::new(AdvisorFaction::Alliance, root.path());
        let action = ClipAction { kind: 3, code: 1 };
        player.queue(QueuedClip {
            op: 0x12,
            slot: SPRITE_SLOT,
            record: 0x18f,
            action,
        });
        player.queue(QueuedClip {
            op: OP_DEFINITION,
            slot: BRIEFING_SLOT,
            record: 0x9999,
            action,
        });

        assert_eq!(
            ticks(&mut player, 1),
            [ClipEvent::Finished(action), ClipEvent::Finished(action)]
        );
        assert!(!player.is_busy());
    }

    #[test]
    #[ignore = "reads the staged original briefing DLLs under data/base/ui"]
    fn every_briefing_clip_plays_each_of_its_frames_to_the_end() {
        // ghidra/notes/advisor-briefing.md, "The briefing": the clips of
        // messages 0x29 (ALBRIEF) and 0x4a (EMBRIEF), queued through their
        // [4, definition] records.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/base/ui");
        let clips: [(AdvisorFaction, &[u16]); 2] = [
            (
                AdvisorFaction::Alliance,
                &[
                    0x27ab, 0x27ec, 0x27ed, 0x27ee, 0x27ef, 0x27f0, 0x27ad, 0x27ae, 0x27af, 0x27b0,
                    0x27b1, 0x27f1, 0x27f2, 0x27b3, 0x27b4, 0x27b8,
                ],
            ),
            (
                AdvisorFaction::Empire,
                &[
                    0x2b8b, 0x2bd4, 0x2bd5, 0x2bd6, 0x2bd7, 0x2bd8, 0x2b8d, 0x2b8e, 0x2b8f, 0x2b90,
                    0x2b91, 0x2bd9, 0x2bda, 0x2b93, 0x2b94, 0x2b98,
                ],
            ),
        ];
        for (faction, entries) in clips {
            let directory = slot_directory(BRIEFING_SLOT, faction).unwrap();
            for &entry in entries {
                // FUN_0042b1d0: the entry record is [op, clip record, a, b].
                let words = advisor_resource(&root, directory, AdvisorResource::Record, entry)
                    .unwrap_or_else(|| panic!("{directory} record {entry:#x}"));
                let op = u16::from_le_bytes([words[0], words[1]]);
                let record = u16::from_le_bytes([words[2], words[3]]);
                assert_eq!(op, OP_DEFINITION, "{directory} {entry:#x}");
                let definition = ClipDefinition::parse(
                    &advisor_resource(&root, directory, AdvisorResource::Record, record).unwrap(),
                )
                .unwrap();
                let mut player = ClipPlayer::new(faction, &root);
                let action = ClipAction { kind: 2, code: u32::from(entry) };
                player.queue(QueuedClip {
                    op,
                    slot: BRIEFING_SLOT,
                    record,
                    action,
                });
                let mut serials = Vec::new();
                let mut events = Vec::new();
                while !events.contains(&ClipEvent::Finished(action)) {
                    events.extend(player.update(CLIP_TICK_SECONDS + 1e-4));
                    if let Some((_, serial)) = player.frame() {
                        serials.push(serial);
                    }
                    assert!(serials.len() <= 1000, "{directory} {entry:#x} never ends");
                }
                serials.dedup();
                assert_eq!(
                    serials.len(),
                    usize::from(definition.frame_count) - 1,
                    "{directory} {entry:#x}: every frame but the last shows"
                );
                assert_eq!(
                    events[0],
                    ClipEvent::Voice {
                        directory,
                        wave: definition.wave
                    }
                );
            }
        }
    }

    #[test]
    #[ignore = "reads the staged original sprite DLLs under data/base/ui"]
    fn every_quip_clip_plays_each_of_its_frames_to_the_end() {
        // The quips' entries (C3POACT 0x50..0x55, IMP22ACT 0x4e..0x53) are
        // [0x13, definition] records in the sprite DLL (slot 9).
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/base/ui");
        for (faction, first_entry) in
            [(AdvisorFaction::Alliance, 10301_u16), (AdvisorFaction::Empire, 11301)]
        {
            let directory = slot_directory(SPRITE_SLOT, faction).unwrap();
            for entry in first_entry..first_entry + 6 {
                let mut player = ClipPlayer::new(faction, &root);
                player.queue_entry(SPRITE_SLOT, entry, 0, 0);
                let words = advisor_resource(&root, directory, AdvisorResource::Record, entry)
                    .unwrap_or_else(|| panic!("{directory} record {entry}"));
                assert_eq!(u16::from_le_bytes([words[0], words[1]]), OP_UNSTOPPED_DEFINITION);
                let record = u16::from_le_bytes([words[2], words[3]]);
                let definition = ClipDefinition::parse(
                    &advisor_resource(&root, directory, AdvisorResource::Record, record).unwrap(),
                )
                .unwrap();
                let mut serials = Vec::new();
                let mut events = Vec::new();
                while !events.iter().any(|event| matches!(event, ClipEvent::Finished(_))) {
                    events.extend(player.update(CLIP_TICK_SECONDS + 1e-4));
                    if let Some((_, serial)) = player.frame() {
                        serials.push(serial);
                    }
                    assert!(serials.len() <= 1000, "{directory} {entry} never ends");
                }
                serials.dedup();
                assert_eq!(
                    serials.len(),
                    usize::from(definition.frame_count) - 1,
                    "{directory} {entry}: every frame but the last shows"
                );
                assert_eq!(
                    events[0],
                    ClipEvent::Voice {
                        directory,
                        wave: definition.wave
                    }
                );
                assert_eq!(events[1], ClipEvent::Finished(ClipAction::default()));
            }
        }
    }

    #[test]
    #[ignore = "reads the staged original sprite DLLs under data/base/ui"]
    fn every_event_messages_hold_ends_after_the_second_droids_clip() {
        // The op-0x14 entries of C3POACT 0x2a..0x44 and IMP22ACT 0x21..0x41
        // (FUN_00411b80 with ALSPRITE / EMSPRITE records).
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/base/ui");
        let entries: [(AdvisorFaction, &[u16]); 2] = [
            (
                AdvisorFaction::Alliance,
                &[
                    9501, 9502, 9503, 9504, 9505, 9506, 9507, 9508, 9509, 9512, 9514, 9515, 9516,
                    9517, 9519, 9521, 9522, 9523, 9524, 9528, 9529, 9531,
                ],
            ),
            (
                AdvisorFaction::Empire,
                &[
                    12501, 12502, 12503, 12504, 12505, 12506, 12507, 12508, 12510, 12512, 12514,
                    12515, 12517, 12519, 12520, 12522, 12523, 12526, 12527, 12529,
                ],
            ),
        ];
        for (faction, entries) in entries {
            for &entry in entries {
                let mut player = ClipPlayer::new(faction, &root);
                player.queue_entry(SPRITE_SLOT, entry, 0, 0);
                let mut events = Vec::new();
                let mut second_frames = Vec::new();
                for _ in 0..1000 {
                    events.extend(player.update(CLIP_TICK_SECONDS + 1e-4));
                    if let Some((_, serial)) = player.second_frame() {
                        second_frames.push(serial);
                        assert!(
                            player.frame().is_some(),
                            "{faction:?} {entry}: the agent holds"
                        );
                    }
                    if events
                        .iter()
                        .any(|event| matches!(event, ClipEvent::Finished(_)))
                    {
                        break;
                    }
                }
                assert!(
                    events.iter().any(|event| matches!(event, ClipEvent::Finished(_))),
                    "{faction:?} {entry} never ends"
                );
                assert!(
                    matches!(events[0], ClipEvent::Voice { .. }),
                    "{faction:?} {entry}: the second droid's voice"
                );
                second_frames.dedup();
                assert!(
                    second_frames.len() > 2,
                    "{faction:?} {entry}: the second droid plays"
                );
            }
        }
    }

    #[test]
    fn a_wave_lasts_its_data_over_its_byte_rate() {
        // An odd-sized chunk before the format chunk takes its pad byte.
        let mut wave = b"RIFF".to_vec();
        wave.extend_from_slice(&60_u32.to_le_bytes());
        wave.extend_from_slice(b"WAVELIST");
        wave.extend_from_slice(&3_u32.to_le_bytes());
        wave.extend_from_slice(b"abc\0fmt ");
        wave.extend_from_slice(&16_u32.to_le_bytes());
        wave.extend_from_slice(&[1, 0, 1, 0]);
        wave.extend_from_slice(&1_u32.to_le_bytes());
        wave.extend_from_slice(&3_u32.to_le_bytes());
        wave.extend_from_slice(&[2, 0, 16, 0]);
        wave.extend_from_slice(b"data");
        wave.extend_from_slice(&6_u32.to_le_bytes());
        wave.extend_from_slice(&[7; 6]);
        assert!((wave_seconds(&wave) - 2.0).abs() < 1e-6);
        assert!(wave_seconds(b"RIFF").abs() < f32::EPSILON);
    }

    #[test]
    fn a_clip_without_a_voice_ends_the_tick_its_last_frame_is_added() {
        // FUN_00404360: with the frame list run out and no sound playing
        // (FUN_00403fe0), the step that adds the last frame ends the clip.
        let root = staged_root(3);
        let mut player = ClipPlayer::new(AdvisorFaction::Alliance, root.path());
        let action = ClipAction { kind: 2, code: 1 };
        player.queue(QueuedClip {
            op: OP_DEFINITION,
            slot: BRIEFING_SLOT,
            record: 0x103b,
            action,
        });

        assert_eq!(ticks(&mut player, 1).len(), 1);
        assert!(ticks(&mut player, 1).is_empty());
        assert_eq!(ticks(&mut player, 1), [ClipEvent::Finished(action)]);
    }

    #[test]
    fn a_new_clip_stops_the_playing_one_each_class_its_own_way() {
        // FUN_00472930 (the player's slot 8) asks the current clip to stop
        // (vt +0x24) when another is queued. Op 4 (FUN_00404740 ->
        // FUN_00404360) ends at its next step and cuts its voice; op 0x13
        // (FUN_00412b80) runs out its frames and ends without its voice; a
        // frame list (FUN_00412090) ends at its next step.
        let root = staged_root(4);
        let action = ClipAction { kind: 2, code: 7 };
        let next = QueuedClip {
            op: OP_FRAME_LIST,
            slot: SPRITE_SLOT,
            record: 399,
            action: ClipAction::default(),
        };

        let mut player = ClipPlayer::new(AdvisorFaction::Alliance, root.path());
        player.queue(QueuedClip {
            op: OP_DEFINITION,
            slot: BRIEFING_SLOT,
            record: 0x103b,
            action,
        });
        ticks(&mut player, 1);
        player.voice_started(10.0);
        player.queue(next);
        assert_eq!(
            ticks(&mut player, 1),
            [
                ClipEvent::StopVoice {
                    directory: "albrief-dll",
                    wave: 1155
                },
                ClipEvent::Finished(action),
            ]
        );

        let mut player = ClipPlayer::new(AdvisorFaction::Alliance, root.path());
        player.queue(QueuedClip {
            op: OP_UNSTOPPED_DEFINITION,
            slot: BRIEFING_SLOT,
            record: 0x103b,
            action,
        });
        ticks(&mut player, 1);
        player.voice_started(10.0);
        player.queue(next);
        // Three deltas still to add; it ends with the last, its voice
        // untouched.
        assert!(ticks(&mut player, 2).is_empty());
        assert_eq!(ticks(&mut player, 1), [ClipEvent::Finished(action)]);

        let mut player = ClipPlayer::new(AdvisorFaction::Alliance, root.path());
        player.queue(QueuedClip {
            op: OP_FRAME_LIST,
            slot: SPRITE_SLOT,
            record: 399,
            action,
        });
        ticks(&mut player, 1);
        player.queue(next);
        assert_eq!(ticks(&mut player, 1), [ClipEvent::Finished(action)]);
    }

    #[test]
    fn a_flush_empties_the_queue_and_stops_the_playing_clip() {
        // FUN_0042d620 -> FUN_00472ba0: the queue empties (FUN_005f5ee0)
        // and the current clip is asked to stop; it ends at its next step,
        // its action run by its end slot.
        let root = staged_root(4);
        let action = ClipAction { kind: 2, code: 7 };
        let mut player = ClipPlayer::new(AdvisorFaction::Alliance, root.path());
        player.queue(QueuedClip {
            op: OP_DEFINITION,
            slot: BRIEFING_SLOT,
            record: 0x103b,
            action,
        });
        player.queue(QueuedClip {
            op: OP_DEFINITION,
            slot: BRIEFING_SLOT,
            record: 0x103b,
            action: ClipAction::default(),
        });
        ticks(&mut player, 1);
        player.voice_started(10.0);
        player.flush();
        assert!(
            player.frame().is_some(),
            "the stopped clip shows until its step"
        );
        assert_eq!(
            ticks(&mut player, 1),
            [
                ClipEvent::StopVoice {
                    directory: "albrief-dll",
                    wave: 1155
                },
                ClipEvent::Finished(action),
            ]
        );
        assert!(!player.is_busy());
        assert!(ticks(&mut player, 2).is_empty());
    }

    #[test]
    fn clearing_drops_the_queue_and_the_shown_frame() {
        let root = staged_root(3);
        let mut player = ClipPlayer::new(AdvisorFaction::Alliance, root.path());
        for _ in 0..2 {
            player.queue(QueuedClip {
                op: OP_DEFINITION,
                slot: BRIEFING_SLOT,
                record: 0x103b,
                action: ClipAction::default(),
            });
        }
        ticks(&mut player, 1);
        assert!(player.frame().is_some());

        player.clear();
        assert!(!player.is_busy());
        assert!(player.frame().is_none());
        assert!(ticks(&mut player, 3).is_empty());
    }

    #[test]
    fn the_advisor_hands_on_its_clip_events_and_drops_its_clips_with_a_new_side() {
        let root = staged_root(3);
        let mut advisor = crate::advisor::AdvisorState::new(AdvisorFaction::Alliance);
        advisor.set_sprite_dir(root.path());
        advisor.clips.queue(QueuedClip {
            op: OP_DEFINITION,
            slot: BRIEFING_SLOT,
            record: 0x103b,
            action: ClipAction::default(),
        });

        advisor.update(CLIP_TICK_SECONDS + 1e-4);
        assert_eq!(
            advisor.take_clip_events(),
            [ClipEvent::Voice {
                directory: "albrief-dll",
                wave: 1155
            }]
        );
        assert!(advisor.take_clip_events().is_empty());
        assert!(advisor.clips.is_busy());

        advisor.set_faction(AdvisorFaction::Empire);
        assert!(!advisor.clips.is_busy());
        advisor.clips.queue(QueuedClip {
            op: OP_DEFINITION,
            slot: BRIEFING_SLOT,
            record: 0x103b,
            action: ClipAction::default(),
        });
        // The Empire's briefing DLL is not staged here: the clip ends at once.
        advisor.update(CLIP_TICK_SECONDS + 1e-4);
        assert_eq!(
            advisor.take_clip_events(),
            [ClipEvent::Finished(ClipAction::default())]
        );

        advisor.set_sprite_dir(root.path().join("elsewhere"));
        advisor.clips.queue(QueuedClip {
            op: OP_DEFINITION,
            slot: BRIEFING_SLOT,
            record: 0x103b,
            action: ClipAction::default(),
        });
        advisor.set_sprite_dir(root.path());
        assert!(!advisor.clips.is_busy());
    }
}
