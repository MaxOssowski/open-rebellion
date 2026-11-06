//! Message log UI panel.
//!
//! Displays a scrollable chronological feed of game events — manufacturing
//! completions, mission results, scripted events, AI actions, and combat
//! outcomes. Each message is color-coded by category and optionally linked
//! to a star system (click to zoom the galaxy map).
//!
//! # Integration
//!
//! Call `draw_message_log` inside the `egui_macroquad::ui` closure alongside
//! the info panel, before the final `egui_macroquad::draw()`. Example from
//! `lib.rs`:
//!
//! ```ignore
//! egui_macroquad::ui(|ctx| {
//!     draw_info_panel_widgets(world, map_state, ctx);
//!     draw_message_log(ctx, &log, &mut log_state);
//! });
//! egui_macroquad::draw();
//! ```

use egui_macroquad::egui::{self, Color32, RichText, ScrollArea};
use rebellion_core::ids::SystemKey;
use serde::Serialize;

// ---------------------------------------------------------------------------
// MessageId
// ---------------------------------------------------------------------------

/// Stable identity for a message, assigned by [`MessageLog::push`].
///
/// The counter is monotonically increasing and never reused, even after
/// deletion or expiry. Selection and delete in the Message
/// Index address messages by this id.
pub type MessageId = u64;

// ---------------------------------------------------------------------------
// MessageCategory
// ---------------------------------------------------------------------------

/// What kind of event produced this message.
///
/// Determines the color used in the UI and which filter toggle controls it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageCategory {
    /// Ship, troop, or facility construction completed.
    Manufacturing,
    /// Mission launched, completed, or failed.
    Mission,
    /// Diplomatic shift or negotiation outcome.
    Diplomacy,
    /// Scripted story event or conditional trigger.
    Event,
    /// AI-driven fleet movement, production order, or officer assignment.
    Ai,
    /// Space or ground combat result.
    Combat,
}

impl MessageCategory {
    /// Egui foreground color for this category.
    #[must_use]
    pub fn color(self) -> Color32 {
        match self {
            MessageCategory::Manufacturing => Color32::from_rgb(100, 220, 100), // green
            MessageCategory::Mission => Color32::from_rgb(100, 160, 255),       // blue
            MessageCategory::Diplomacy => Color32::from_rgb(255, 220, 80),      // yellow
            MessageCategory::Event => Color32::from_rgb(180, 180, 180),         // light gray
            MessageCategory::Ai => Color32::from_rgb(220, 160, 60),             // amber
            MessageCategory::Combat => Color32::from_rgb(240, 80, 80),          // red
        }
    }

    /// Short prefix tag shown before the message text.
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            MessageCategory::Manufacturing => "[BUILD]",
            MessageCategory::Mission => "[MISSION]",
            MessageCategory::Diplomacy => "[DIPLO]",
            MessageCategory::Event => "[EVENT]",
            MessageCategory::Ai => "[AI]",
            MessageCategory::Combat => "[COMBAT]",
        }
    }
}

/// One of the nine Message Index categories on the command-center rail.
///
/// Each message class stores one of these masks at `+0x34`; a posted message
/// lights its category's rail control until that category is read. See
/// `docs/qa/2026-09-10-interface-parity-audit/evidence/2026-09-24-message-index-recovery.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageRail {
    PopularSupport,
    Fleet,
    Mission,
    Resource,
    Manufacturing,
    Defense,
    Conflict,
    Advice,
    Chat,
}

impl MessageRail {
    /// Rail controls top to bottom, carrying commands `0x136..0x13e`.
    pub const RAIL_ORDER: [Self; 9] = [
        Self::PopularSupport,
        Self::Fleet,
        Self::Mission,
        Self::Resource,
        Self::Manufacturing,
        Self::Defense,
        Self::Conflict,
        Self::Advice,
        Self::Chat,
    ];

    /// Category mask from the class getters, as `FUN_0042d8d0` tests it.
    #[must_use]
    pub const fn mask(self) -> u16 {
        match self {
            Self::PopularSupport => 0x001,
            Self::Resource => 0x004,
            Self::Manufacturing => 0x008,
            Self::Mission => 0x010,
            Self::Chat => 0x020,
            Self::Defense => 0x040,
            Self::Fleet => 0x080,
            Self::Conflict => 0x100,
            Self::Advice => 0x200,
        }
    }
}

/// Which faction's Message Index receives a message. The original keeps one
/// list per faction (`+0x78` and `+0x84` on the manager at `DAT_006be3a8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailAudience {
    Alliance,
    Empire,
    Both,
}

impl RailAudience {
    /// The audience of one side.
    #[must_use]
    pub const fn side(is_alliance: bool) -> Self {
        if is_alliance {
            Self::Alliance
        } else {
            Self::Empire
        }
    }

    /// Whether the player's side receives the message.
    #[must_use]
    pub const fn includes(self, player_is_alliance: bool) -> bool {
        match self {
            Self::Alliance => player_is_alliance,
            Self::Empire => !player_is_alliance,
            Self::Both => true,
        }
    }
}

// ---------------------------------------------------------------------------
// GameMessage
// ---------------------------------------------------------------------------

/// A message's picture (`FUN_0046a320`): STRATEGY bitmaps, the background
/// (`+0x2e`) with the foreground (`+0x2c`) blitted keyed and centred on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessagePicture {
    pub background: u32,
    pub foreground: Option<u32>,
}

/// One entry in the message log.
#[derive(Debug, Clone, Serialize)]
pub struct GameMessage {
    /// Stable identity, assigned by [`MessageLog::push`]. Zero before push.
    #[serde(skip)]
    pub id: MessageId,
    /// Game-day on which this message was generated.
    pub tick: u64,
    /// Human-readable message text.
    pub text: String,
    /// Category — controls color and filter visibility.
    pub category: MessageCategory,
    /// If this message is linked to a system, clicking it can zoom the map
    /// to that location. `None` for messages with no spatial context.
    #[serde(skip)]
    pub system: Option<SystemKey>,
    /// Resolved system name for serialization. Populated by `resolve_system_names()`
    /// before JSONL export. `None` if no system is linked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_name: Option<String>,
    /// Message Index category this message is filed under, if it matches a
    /// recovered notification type. Other messages never reach the rail.
    #[serde(skip)]
    pub rail: Option<MessageRail>,
    /// Side whose Message Index receives it.
    #[serde(skip)]
    pub audience: RailAudience,
    /// The unread bit (`0x10` at `+0x24`), set when the message is posted.
    #[serde(skip)]
    pub unread: bool,
    /// The picture shown above the text in the Message window.
    #[serde(skip)]
    pub picture: Option<MessagePicture>,
    /// The original message's title (`+0x44`), where its text comes from
    /// the player's TEXTSTRA message texts; the index lists it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The Assault Summary report a battle-report message (type `0x0f`)
    /// opens in place of its text (`FUN_00469de0` -> `FUN_0042a410`).
    #[serde(skip)]
    pub assault_report: Option<Box<rebellion_core::assault_report::AssaultReport>>,
    /// The event code (`+0x28`) its delivery hands each side's advisor
    /// (`FUN_0048a060` → slot `+0x14`), Alliance then Empire; 0 for none.
    #[serde(skip)]
    pub advisor_event: [u32; 2],
    /// The mission a Mission Report asks to continue (kind 5, `+0x84`; the
    /// Abort order `0x250` it holds at `+0x90`, `FUN_004927c0`), until the
    /// player answers in the Message Index.
    #[serde(skip)]
    pub continue_question: Option<u64>,
    /// The mission a Mission Report names (its object, `+0x78`), which
    /// Display goes to while it runs.
    #[serde(skip)]
    pub mission: Option<u64>,
    /// When the message expires (its class's slot `+0x30`).
    #[serde(skip)]
    pub lifetime: MessageLifetime,
    /// The scheduler's sub-tick when it was posted (`+0x38`, which the base
    /// constructor `FUN_004c4ce0` takes from `FUN_004fcee0`); set by
    /// [`MessageLog::push`].
    #[serde(skip)]
    pub posted: u64,
}

/// Sub-ticks a message stays in the index: the base class's slot `+0x30`
/// (`Class_0065c538_vt30`, `FUN_004c4fe0`) expires it once the scheduler's
/// sub-tick (`FUN_004fcee0`) passes its `+0x38` plus 300. A day has the
/// speed's rate of sub-ticks (600 Very Slow, 60 Slow, 12 Medium, 4 Fast), so
/// that is half a day to 75 days.
pub const MESSAGE_LIFETIME_SUB_TICKS: u64 = 300;

/// When a message expires (its class's slot `+0x30`, which `FUN_0048a2a0`
/// asks of every message). The agent's advice (class `0x0065b188`) and the
/// Assault Summary reports (`0x0065b870`) never expire by class, whatever
/// their lifetime here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MessageLifetime {
    /// The base class's [`MESSAGE_LIFETIME_SUB_TICKS`] sub-ticks.
    #[default]
    Days,
    /// Never: slot `+0x30` is `FUN_006158b0`, which returns 0. Future Jedi
    /// Discovered (`0x0065b520`) is such a class.
    Kept,
    /// Facility Idle (`0x0065b260`, `Class_0065b260_vt30` = `FUN_0048c2b0`):
    /// kept while the system's `area` manager still exists, stands idle
    /// (`+0x58` is 0) and is the message's side.
    WhileIdle {
        system: SystemKey,
        area: rebellion_core::manufacturing::ProductionArea,
    },
}

impl GameMessage {
    /// Construct a message without a system link.
    pub fn new(tick: u64, text: impl Into<String>, category: MessageCategory) -> Self {
        GameMessage {
            id: 0,
            tick,
            text: text.into(),
            category,
            system: None,
            system_name: None,
            rail: None,
            audience: RailAudience::Both,
            unread: false,
            picture: None,
            title: None,
            assault_report: None,
            advisor_event: [0; 2],
            continue_question: None,
            mission: None,
            lifetime: MessageLifetime::Days,
            posted: 0,
        }
    }

    /// Construct a message linked to a specific system.
    pub fn at_system(
        tick: u64,
        text: impl Into<String>,
        category: MessageCategory,
        system: SystemKey,
    ) -> Self {
        GameMessage {
            id: 0,
            tick,
            text: text.into(),
            category,
            system: Some(system),
            system_name: None,
            rail: None,
            audience: RailAudience::Both,
            unread: false,
            picture: None,
            title: None,
            assault_report: None,
            advisor_event: [0; 2],
            continue_question: None,
            mission: None,
            lifetime: MessageLifetime::Days,
            posted: 0,
        }
    }

    /// Give the message its class's lifetime (slot `+0x30`).
    #[must_use]
    pub fn with_lifetime(mut self, lifetime: MessageLifetime) -> Self {
        self.lifetime = lifetime;
        self
    }

    /// Whether the message has expired at sub-tick `now` (`FUN_0048a2a0`
    /// asks slot `+0x30`). `idle` tells whether a system's area manager
    /// still stands idle for the message's side.
    #[must_use]
    pub fn expired(
        &self,
        now: u64,
        idle: &dyn Fn(SystemKey, rebellion_core::manufacturing::ProductionArea) -> bool,
    ) -> bool {
        if self.rail == Some(MessageRail::Advice) || self.assault_report.is_some() {
            return false;
        }
        match self.lifetime {
            MessageLifetime::Days => self.posted + MESSAGE_LIFETIME_SUB_TICKS < now,
            MessageLifetime::Kept => false,
            MessageLifetime::WhileIdle { system, area } => !idle(system, area),
        }
    }

    /// File the message, unread, under a Message Index category for one or
    /// both sides.
    #[must_use]
    pub fn on_rail(mut self, rail: MessageRail, audience: RailAudience) -> Self {
        self.rail = Some(rail);
        self.audience = audience;
        self.unread = true;
        self
    }

    /// Title the message (`+0x44`); the index lists the title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Ask whether `mission` should continue (`FUN_004927c0`: text
    /// `0x70db`, kind 5, an Abort order held at `+0x90`).
    #[must_use]
    pub fn with_continue_question(mut self, mission: u64) -> Self {
        self.continue_question = Some(mission);
        self
    }

    /// Name the mission a Mission Report is about.
    #[must_use]
    pub fn with_mission(mut self, mission: u64) -> Self {
        self.mission = Some(mission);
        self
    }

    /// Show `picture` above the message's text.
    #[must_use]
    pub fn with_picture(mut self, picture: MessagePicture) -> Self {
        self.picture = Some(picture);
        self
    }

    /// Open `report`'s Assault Summary window when the message is shown.
    #[must_use]
    pub fn with_assault_report(
        mut self,
        report: rebellion_core::assault_report::AssaultReport,
    ) -> Self {
        self.assault_report = Some(Box::new(report));
        self
    }

    /// Hand both sides' advisors event `code` on delivery.
    #[must_use]
    pub fn with_advisor_event(self, code: u32) -> Self {
        self.with_advisor_events(code, code)
    }

    /// Hand the Alliance's advisor `alliance` and the Empire's `empire`
    /// (0 for none), for a class whose code depends on the reader, such as
    /// a system joining one side (`FUN_00499760`: 1 for the gaining side,
    /// 2 for the other).
    #[must_use]
    pub fn with_advisor_events(mut self, alliance: u32, empire: u32) -> Self {
        self.advisor_event = [alliance, empire];
        self
    }

    /// Show `picture`, when the message's class has one.
    #[must_use]
    pub fn with_picture_if(mut self, picture: Option<MessagePicture>) -> Self {
        self.picture = picture;
        self
    }
}

// ---------------------------------------------------------------------------
// MessageLog
// ---------------------------------------------------------------------------

/// A story movie: one of the four story messages (System Destroyed by the
/// Death Star, Alliance Headquarters Destroyed, Coruscant Captured, Death
/// Star Sabotaged), whose class's kind 6 `FUN_0048a060` files only in the
/// message manager's movie list (`+0x28`, branch `0x48a149`): no Message
/// Index row, no alert, no advisor event. `movie` is the MDATA number
/// (101..104, from the message's `+0x5c` through `FUN_00421ba0`) and
/// `caption` the message's body (`+0x50`), shown under the movie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryMovie {
    pub movie: u32,
    pub caption: String,
}

/// The message manager's list of game messages (`DAT_006be3a8`).
///
/// Messages leave it when deleted or when they expire (`FUN_0048a2a0`);
/// there is no cap. Lives alongside `GameWorld` and simulation state — not
/// inside either.
#[derive(Debug, Clone)]
pub struct MessageLog {
    messages: Vec<GameMessage>,
    next_id: MessageId,
    /// The manager's `+8` bit `0x10000000`: the Message Index is open, so no
    /// expiry runs (`FUN_0048a610`, from `FUN_004665f0` and `FUN_004664d0`).
    index_open: bool,
    /// A message was posted since the last expiry: `FUN_0048a060` runs
    /// `FUN_0048a2a0` after filing one.
    expiry_due: bool,
    /// The scheduler's sub-tick (`FUN_004fcee0`), which stamps each posted
    /// message and dates expiry.
    now: u64,
    /// Event codes delivered to each side's advisor and not yet taken:
    /// `(is_alliance, code)`.
    advisor_events: Vec<(bool, u32)>,
    /// The categories' posting options (`FUN_0048a1c0`): a default and
    /// per-category values by mask; bit [`SILENT`] posts silently.
    posting: (u32, std::collections::BTreeMap<u16, u32>),
    /// The story movies waiting to play, oldest first (the message
    /// manager's `+0x28`, drained one at a time by `FUN_00488b90`).
    story_movies: std::collections::VecDeque<StoryMovie>,
}

/// A category's posting option bit 4: its messages post silently, without
/// telling the advisor (`FUN_0048a060`, the Message Index's `0x66`).
pub const SILENT: u32 = 4;

impl Default for MessageLog {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageLog {
    /// Create a new empty log.
    #[must_use]
    pub fn new() -> Self {
        MessageLog {
            messages: Vec::new(),
            next_id: 1,
            index_open: false,
            expiry_due: false,
            now: 0,
            advisor_events: Vec::new(),
            posting: (0, std::collections::BTreeMap::new()),
            story_movies: std::collections::VecDeque::new(),
        }
    }

    /// File a story movie to play (`FUN_0048a060`'s kind 6).
    pub fn queue_story_movie(&mut self, movie: StoryMovie) {
        self.story_movies.push_back(movie);
    }

    /// The story movies waiting, oldest first.
    pub fn queued_story_movies(&self) -> impl Iterator<Item = &StoryMovie> {
        self.story_movies.iter()
    }

    /// The oldest story movie waiting, taken off the list.
    pub fn take_story_movie(&mut self) -> Option<StoryMovie> {
        self.story_movies.pop_front()
    }

    /// Append a message, assigning it a stable [`MessageId`].
    ///
    /// The returned id is unique for this log's lifetime. A message filed on
    /// a rail makes an expiry due ([`Self::expire_if_due`]), as
    /// `FUN_0048a060` runs `FUN_0048a2a0` after filing one.
    pub fn push(&mut self, mut msg: GameMessage) -> MessageId {
        let id = self.next_id;
        self.next_id += 1;
        msg.id = id;
        msg.posted = self.now;
        // FUN_0048a060: a delivered message (port: one filed on a side's
        // rail) hands its event code to that side's advisor unless its
        // category posts silently (FUN_0048a1c0 bit 4).
        if msg
            .rail
            .is_some_and(|rail| self.posting_option(Some(rail.mask())) & SILENT == 0)
        {
            for (index, alliance) in [(0, true), (1, false)] {
                let code = msg.advisor_event[index];
                if code != 0 && msg.audience.includes(alliance) {
                    self.advisor_events.push((alliance, code));
                }
            }
        }
        self.expiry_due |= msg.rail.is_some();
        self.messages.push(msg);
        id
    }

    /// Set the scheduler's sub-tick (`FUN_004fcee0`), which stamps the
    /// messages posted from now on and dates their expiry.
    pub fn set_now(&mut self, sub_tick: u64) {
        self.now = sub_tick;
    }

    /// `FUN_0048a2a0`: drop every message that has expired, unless the
    /// Message Index is open.
    pub fn expire(
        &mut self,
        idle: &dyn Fn(SystemKey, rebellion_core::manufacturing::ProductionArea) -> bool,
    ) {
        if self.index_open {
            return;
        }
        self.expiry_due = false;
        let now = self.now;
        self.messages.retain(|msg| !msg.expired(now, idle));
    }

    /// Run [`Self::expire`] when a message was filed since the last run.
    pub fn expire_if_due(
        &mut self,
        idle: &dyn Fn(SystemKey, rebellion_core::manufacturing::ProductionArea) -> bool,
    ) {
        if self.expiry_due {
            self.expire(idle);
        }
    }

    /// `FUN_0048a610`: the Message Index opens (no expiry runs while it is
    /// open) or closes (expiry runs at once).
    pub fn set_index_open(
        &mut self,
        open: bool,
        idle: &dyn Fn(SystemKey, rebellion_core::manufacturing::ProductionArea) -> bool,
    ) {
        let was_open = std::mem::replace(&mut self.index_open, open);
        if was_open && !open {
            self.expire(idle);
        }
    }

    /// The event codes delivered to the side's advisor since the last call,
    /// in order; the other side's are dropped.
    pub fn take_advisor_events(&mut self, player_is_alliance: bool) -> Vec<u32> {
        std::mem::take(&mut self.advisor_events)
            .into_iter()
            .filter(|&(alliance, _)| alliance == player_is_alliance)
            .map(|(_, code)| code)
            .collect()
    }

    /// The posting option of the category `mask`, or the default for
    /// `None` (All Messages): `FUN_0048a1c0` falls back to the default
    /// (`FUN_005f4960`) for a category without its own.
    #[must_use]
    pub fn posting_option(&self, mask: Option<u16>) -> u32 {
        mask.and_then(|mask| self.posting.1.get(&mask).copied())
            .unwrap_or(self.posting.0)
    }

    /// The Message Index's `0x66` (`FUN_00467f10`): flip a category's
    /// [`SILENT`] bit; on All Messages, flip the default and drop every
    /// category's own value (`FUN_0048a1f0`).
    pub fn toggle_silent(&mut self, mask: Option<u16>) {
        match mask {
            Some(mask) => {
                let option = self.posting_option(Some(mask)) ^ SILENT;
                self.posting.1.insert(mask, option);
            }
            None => {
                self.posting.0 ^= SILENT;
                self.posting.1.clear();
            }
        }
    }

    /// Remove all messages.
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// All messages in chronological order (oldest first).
    #[must_use]
    pub fn messages(&self) -> &[GameMessage] {
        &self.messages
    }

    /// Categories holding unread messages for the player's side, recomputed
    /// as `FUN_0048a2a0` rebuilds the mask from every unread message.
    #[must_use]
    pub fn unread_mask(&self, player_is_alliance: bool) -> u16 {
        self.messages
            .iter()
            .filter(|msg| msg.unread && msg.audience.includes(player_is_alliance))
            .filter_map(|msg| msg.rail)
            .fold(0, |mask, rail| mask | rail.mask())
    }

    /// Mark every message in one category read, as closing its Message Index
    /// window does (`FUN_0048a530`).
    pub fn mark_read(&mut self, rail: MessageRail) {
        for msg in &mut self.messages {
            if msg.rail == Some(rail) {
                msg.unread = false;
            }
        }
    }

    /// Mark every message read, as closing the All Messages category does
    /// (`FUN_0048a530` with param 0).
    pub fn mark_all_read(&mut self) {
        for msg in &mut self.messages {
            msg.unread = false;
        }
    }

    /// Mark a single message read by its id.
    ///
    /// port: the original sets the list item's font to 10 (`FUN_00469de0`)
    /// without modifying the message node's `+0x24`; the port collapses
    /// the read/unread distinction into the message's `unread` field.
    pub fn mark_message_read(&mut self, id: MessageId) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == id) {
            msg.unread = false;
        }
    }

    /// Answer a Mission Report's question (`Class_0065b710_vt38` /
    /// `vt3c` drop the held Abort order once used): it asks no more, and
    /// returns the mission it asked about.
    pub fn answer_continue_question(&mut self, id: MessageId) -> Option<u64> {
        self.messages
            .iter_mut()
            .find(|message| message.id == id)
            .and_then(|message| message.continue_question.take())
    }

    /// Remove every message whose id appears in `ids` (`FUN_005f54a0`).
    ///
    /// After deletion the caller should recompute the unread mask
    /// (`FUN_0048a2a0`).
    pub fn delete_by_ids(&mut self, ids: &[MessageId]) {
        self.messages.retain(|msg| !ids.contains(&msg.id));
    }

    /// Number of messages currently stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Resolve `SystemKey` references to human-readable system names.
    ///
    /// Call before `export_jsonl()` to populate `system_name` fields.
    /// Requires a closure that maps `SystemKey` to a name string.
    pub fn resolve_system_names(&mut self, lookup: impl Fn(SystemKey) -> Option<String>) {
        for msg in &mut self.messages {
            if let Some(key) = msg.system {
                msg.system_name = lookup(key);
            }
        }
    }

    /// Export all messages as JSONL (one JSON object per line).
    ///
    /// # Errors
    /// Returns an error if the output file cannot be created or written,
    /// or a message cannot be serialized.
    pub fn export_jsonl(&self, path: &std::path::Path) -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(path)?;
        for msg in &self.messages {
            let json = serde_json::to_string(msg).map_err(std::io::Error::other)?;
            writeln!(file, "{json}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MessageLogState
// ---------------------------------------------------------------------------

/// Mutable UI state for the message log panel.
///
/// Does not own any messages — those live in `MessageLog`.
#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "These independent flags preserve the existing state and serialization model."
)]
pub struct MessageLogState {
    /// Whether each category's messages are shown.
    pub show_manufacturing: bool,
    pub show_mission: bool,
    pub show_diplomacy: bool,
    pub show_event: bool,
    pub show_ai: bool,
    pub show_combat: bool,

    /// Whether the panel is expanded or collapsed.
    pub expanded: bool,

    /// If set, the caller should scroll/zoom the galaxy map to this system.
    /// Cleared on the next `draw_message_log` call after being read.
    pub focus_system: Option<SystemKey>,

    /// Whether to lock scroll to the newest message.
    pub auto_scroll: bool,

    /// Tracks the number of messages seen so far — used to detect new
    /// arrivals and trigger auto-scroll.
    last_seen_count: usize,
}

impl Default for MessageLogState {
    fn default() -> Self {
        MessageLogState {
            show_manufacturing: true,
            show_mission: true,
            show_diplomacy: true,
            show_event: true,
            show_ai: false, // AI messages are verbose; off by default
            show_combat: true,
            expanded: true,
            focus_system: None,
            auto_scroll: true,
            last_seen_count: 0,
        }
    }
}

impl MessageLogState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn is_visible(&self, category: MessageCategory) -> bool {
        match category {
            MessageCategory::Manufacturing => self.show_manufacturing,
            MessageCategory::Mission => self.show_mission,
            MessageCategory::Diplomacy => self.show_diplomacy,
            MessageCategory::Event => self.show_event,
            MessageCategory::Ai => self.show_ai,
            MessageCategory::Combat => self.show_combat,
        }
    }
}

// ---------------------------------------------------------------------------
// draw_message_log
// ---------------------------------------------------------------------------

/// Render the message log as an egui bottom panel.
///
/// Must be called inside an `egui_macroquad::ui` closure. The caller reads
/// `state.focus_system` after this call to zoom the galaxy map if set.
///
/// # Arguments
///
/// - `ctx`: the egui context from `egui_macroquad::ui`
/// - `log`: the message buffer (read-only)
/// - `state`: mutable panel state (scroll, filters, expand/collapse)
pub fn draw_message_log(ctx: &egui::Context, log: &MessageLog, state: &mut MessageLogState) {
    // Detect new messages for auto-scroll trigger.
    let new_messages = log.len() > state.last_seen_count;
    state.last_seen_count = log.len();

    egui::TopBottomPanel::bottom("message_log")
        .resizable(true)
        .min_height(28.0)
        .max_height(240.0)
        .default_height(if state.expanded { 140.0 } else { 28.0 })
        .show(ctx, |ui| {
            // ── Header bar ───────────────────────────────────────────────────
            ui.horizontal(|ui| {
                let toggle_label = if state.expanded {
                    "▼ Message Log"
                } else {
                    "▶ Message Log"
                };
                if ui.small_button(toggle_label).clicked() {
                    state.expanded = !state.expanded;
                }

                if state.expanded {
                    ui.separator();

                    // Category filter toggles.
                    category_toggle(
                        ui,
                        "[BUILD]",
                        MessageCategory::Manufacturing,
                        &mut state.show_manufacturing,
                    );
                    category_toggle(
                        ui,
                        "[MISS]",
                        MessageCategory::Mission,
                        &mut state.show_mission,
                    );
                    category_toggle(
                        ui,
                        "[DIPLO]",
                        MessageCategory::Diplomacy,
                        &mut state.show_diplomacy,
                    );
                    category_toggle(ui, "[EVT]", MessageCategory::Event, &mut state.show_event);
                    category_toggle(ui, "[AI]", MessageCategory::Ai, &mut state.show_ai);
                    category_toggle(ui, "[CMB]", MessageCategory::Combat, &mut state.show_combat);

                    ui.separator();
                    ui.toggle_value(&mut state.auto_scroll, "⬇ Auto");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Clear").clicked() {
                            // We can't clear here (log is &MessageLog), but we
                            // signal intent via a flag the caller checks.
                            // For now clear is a no-op at render time; caller
                            // should expose a method on its own log handle.
                        }
                        ui.label(format!("{} msgs", log.len()));
                    });
                }
            });

            if !state.expanded {
                return;
            }

            ui.separator();

            // ── Scrollable message list ───────────────────────────────────────
            let scroll_id = egui::Id::new("message_log_scroll");
            let mut scroll = ScrollArea::vertical()
                .id_salt(scroll_id)
                .auto_shrink([false, false]);

            // Stick to bottom when auto_scroll is on and new messages arrived.
            if state.auto_scroll && new_messages {
                scroll = scroll.vertical_scroll_offset(f32::MAX);
            }

            scroll.show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                for msg in log.messages() {
                    if !state.is_visible(msg.category) {
                        continue;
                    }

                    ui.horizontal(|ui| {
                        // Tick timestamp.
                        ui.label(
                            RichText::new(format!("Day {:>5}", msg.tick))
                                .color(Color32::from_gray(120))
                                .monospace()
                                .small(),
                        );

                        // Category tag — colored.
                        ui.label(
                            RichText::new(msg.category.tag())
                                .color(msg.category.color())
                                .monospace()
                                .small(),
                        );

                        // Message text. If linked to a system, make it a
                        // clickable label that sets focus_system.
                        let text = RichText::new(&msg.text).small();
                        if let Some(sys_key) = msg.system {
                            let response = ui.add(
                                egui::Label::new(text.color(Color32::from_gray(220)))
                                    .sense(egui::Sense::click()),
                            );
                            if response.clicked() {
                                state.focus_system = Some(sys_key);
                            }
                            if response.hovered() {
                                response.on_hover_text("Click to zoom to system");
                            }
                        } else {
                            ui.label(text.color(Color32::from_gray(200)));
                        }
                    });
                }
            });
        });
}

/// Render a small toggle button for a category filter.
fn category_toggle(ui: &mut egui::Ui, label: &str, category: MessageCategory, enabled: &mut bool) {
    let text = if *enabled {
        RichText::new(label).color(category.color()).small()
    } else {
        RichText::new(label).color(Color32::from_gray(80)).small()
    };
    if ui.add(egui::Button::new(text).frame(false)).clicked() {
        *enabled = !*enabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_takes_a_picture_only_when_its_class_has_one() {
        let message = GameMessage::new(1, "text", MessageCategory::Event);
        assert_eq!(message.clone().with_picture_if(None).picture, None);
        let picture = MessagePicture {
            background: 0x3f2,
            foreground: None,
        };
        assert_eq!(
            message.with_picture_if(Some(picture)).picture,
            Some(picture)
        );
    }

    #[test]
    fn a_filed_message_hands_each_reading_side_its_advisor_event() {
        // FUN_0048a060 calls the reader's advisor (+0x14) with the
        // message's +0x28 when it is not 0.
        let mut log = MessageLog::default();
        log.push(
            GameMessage::new(1, "joins", MessageCategory::Diplomacy)
                .with_advisor_events(1, 2)
                .on_rail(MessageRail::PopularSupport, RailAudience::Both),
        );
        log.push(
            GameMessage::new(2, "yard idle", MessageCategory::Manufacturing)
                .with_advisor_event(3)
                .on_rail(MessageRail::Manufacturing, RailAudience::Empire),
        );
        // Not filed: no delivery. No code: nothing.
        log.push(GameMessage::new(3, "log only", MessageCategory::Event).with_advisor_event(5));
        log.push(
            GameMessage::new(4, "silent", MessageCategory::Event)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );

        assert_eq!(log.take_advisor_events(true), [1]);
        assert!(
            log.take_advisor_events(false).is_empty(),
            "taken with the first call"
        );
        log.push(
            GameMessage::new(5, "joins", MessageCategory::Diplomacy)
                .with_advisor_events(1, 2)
                .on_rail(MessageRail::PopularSupport, RailAudience::Both),
        );
        assert_eq!(log.take_advisor_events(false), [2]);
    }

    #[test]
    fn rail_masks_match_the_recovered_class_getters() {
        // Getters FUN_0040f340 (0x001) through FUN_0048b450 (0x200); rail
        // order follows commands 0x136..0x13e (FUN_0042da10).
        assert_eq!(
            MessageRail::RAIL_ORDER.map(MessageRail::mask),
            [0x001, 0x080, 0x010, 0x004, 0x008, 0x040, 0x100, 0x200, 0x020]
        );
    }

    #[test]
    fn a_silent_category_keeps_its_messages_from_the_advisor() {
        // FUN_0048a060 calls the advisor only when FUN_0048a1c0's option
        // has bit 4 clear; FUN_00467f10 flips a category's own option, and
        // All Messages the default, dropping every category's (FUN_0048a1f0).
        let advice = |code| {
            GameMessage::new(1, "advice", MessageCategory::Event)
                .on_rail(MessageRail::Advice, RailAudience::Alliance)
                .with_advisor_event(code)
        };
        let mut log = MessageLog::default();
        assert_eq!(log.posting_option(None), 0);
        log.toggle_silent(Some(0x200));
        log.push(advice(0x2a));
        assert!(log.take_advisor_events(true).is_empty());
        assert_eq!(log.messages().len(), 1, "a silent message is still filed");
        log.push(
            GameMessage::new(2, "fleet", MessageCategory::Event)
                .on_rail(MessageRail::Fleet, RailAudience::Alliance)
                .with_advisor_event(5),
        );
        assert_eq!(log.take_advisor_events(true), [5]);
        log.toggle_silent(Some(0x200));
        log.push(advice(0x2a));
        assert_eq!(log.take_advisor_events(true), [0x2a]);

        log.toggle_silent(Some(0x200));
        log.toggle_silent(None);
        assert_eq!(log.posting_option(None), SILENT);
        assert_eq!(log.posting_option(Some(0x80)), SILENT);
        log.toggle_silent(None);
        assert_eq!(
            log.posting_option(Some(0x200)),
            0,
            "All drops the category's own"
        );
    }

    #[test]
    fn only_unread_rail_messages_light_the_rail() {
        let mut log = MessageLog::default();
        log.push(GameMessage::new(1, "saved", MessageCategory::Event));
        log.push(
            GameMessage::new(2, "built", MessageCategory::Manufacturing)
                .on_rail(MessageRail::Manufacturing, RailAudience::Both),
        );
        log.push(
            GameMessage::new(3, "arrived", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );
        assert_eq!(log.unread_mask(true), 0x008 | 0x080);
    }

    #[test]
    fn reading_one_category_rests_only_its_rail() {
        let mut log = MessageLog::default();
        log.push(
            GameMessage::new(1, "built", MessageCategory::Manufacturing)
                .on_rail(MessageRail::Manufacturing, RailAudience::Both),
        );
        log.push(
            GameMessage::new(2, "arrived", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );
        log.mark_read(MessageRail::Fleet);
        assert_eq!(log.unread_mask(true), 0x008);
        log.push(
            GameMessage::new(3, "arrived again", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );
        assert_eq!(log.unread_mask(true), 0x008 | 0x080);
    }

    #[test]
    fn two_unread_messages_in_one_category_keep_its_rail_lit() {
        let mut log = MessageLog::default();
        for tick in 1..=2 {
            log.push(
                GameMessage::new(tick, "arrived", MessageCategory::Mission)
                    .on_rail(MessageRail::Fleet, RailAudience::Both),
            );
        }
        assert_eq!(log.unread_mask(true), 0x080);
    }

    #[test]
    fn a_message_lights_only_its_audience_rail() {
        let mut log = MessageLog::default();
        log.push(
            GameMessage::new(1, "arrived", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::side(false)),
        );
        assert_eq!(log.unread_mask(true), 0);
        assert_eq!(log.unread_mask(false), 0x080);
    }

    #[test]
    fn an_expired_message_can_rest_its_rail() {
        // FUN_0048a2a0 rebuilds the unread mask +0x50 from the messages it
        // keeps.
        let mut log = MessageLog::new();
        log.push(
            GameMessage::new(1, "built", MessageCategory::Manufacturing)
                .on_rail(MessageRail::Manufacturing, RailAudience::Both),
        );
        assert_eq!(log.unread_mask(true), 0x008);
        log.set_now(301);
        log.expire(&|_, _| true);
        assert_eq!(log.unread_mask(true), 0);
    }

    #[test]
    fn push_assigns_a_stable_monotonic_id() {
        let mut log = MessageLog::new();
        let a = log.push(GameMessage::new(1, "first", MessageCategory::Event));
        let b = log.push(GameMessage::new(2, "second", MessageCategory::Event));
        let c = log.push(GameMessage::new(3, "third", MessageCategory::Event));

        assert_eq!(a, 1);
        assert_eq!(b, 2);
        assert_eq!(c, 3);
        assert_eq!(log.messages()[0].id, 1);
        assert_eq!(log.messages()[2].id, 3);
    }

    #[test]
    fn ids_survive_expiry_and_never_reuse() {
        let mut log = MessageLog::new();
        let a = log.push(
            GameMessage::new(1, "first", MessageCategory::Event)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );
        log.set_now(301);
        log.expire(&|_, _| true);
        let c = log.push(GameMessage::new(303, "third", MessageCategory::Event));

        // a expired but c's id is still > a's
        assert!(c > a);
        assert_eq!(log.messages().len(), 1);
        assert_eq!(log.messages()[0].id, 2);
    }

    #[test]
    fn a_message_expires_300_sub_ticks_after_it_is_posted_unless_its_class_keeps_it() {
        // FUN_004c4ce0 stamps +0x38 with FUN_004fcee0, the scheduler's
        // sub-tick; Class_0065c538_vt30 (FUN_004c4fe0): +0x38 + 300 < now.
        // The advice class 0x0065b188, Future Jedi Discovered (0x0065b520)
        // and the Assault Summary reports (0x0065b870) take FUN_006158b0
        // (0). The manual (p. 79): "Messages are eventually deleted whether
        // or not you read them, except for agent advice messages."
        let report = rebellion_core::assault_report::AssaultReport {
            system: SystemKey::default(),
            assault: true,
            attacker: rebellion_core::dat::Faction::Empire,
            defender: None,
            taken: false,
            attackers: Default::default(),
            system_forces: Default::default(),
        };
        let mut log = MessageLog::new();
        log.set_now(10);
        for msg in [
            GameMessage::new(1, "fleet", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
            GameMessage::new(1, "advice", MessageCategory::Event)
                .on_rail(MessageRail::Advice, RailAudience::Both),
            GameMessage::new(1, "jedi", MessageCategory::Event)
                .on_rail(MessageRail::Mission, RailAudience::Both)
                .with_lifetime(MessageLifetime::Kept),
            GameMessage::new(1, "assault", MessageCategory::Combat)
                .on_rail(MessageRail::Conflict, RailAudience::Both)
                .with_assault_report(report),
        ] {
            log.push(msg);
        }
        let texts = |log: &MessageLog| -> Vec<String> {
            log.messages().iter().map(|msg| msg.text.clone()).collect()
        };

        assert_eq!(log.messages()[0].posted, 10);
        log.set_now(310);
        log.expire(&|_, _| true);
        assert_eq!(texts(&log), ["fleet", "advice", "jedi", "assault"]);
        log.set_now(311);
        log.expire(&|_, _| true);
        assert_eq!(texts(&log), ["advice", "jedi", "assault"]);
    }

    #[test]
    fn a_facility_idle_message_lasts_while_its_manager_stands_idle() {
        // Class_0065b260_vt30 (FUN_0048c2b0): expired once the manager is
        // gone, no longer idle (+0x58), or another side's.
        use rebellion_core::manufacturing::ProductionArea;
        let system = SystemKey::default();
        let mut log = MessageLog::new();
        log.push(
            GameMessage::new(10, "idle", MessageCategory::Manufacturing)
                .on_rail(MessageRail::Manufacturing, RailAudience::Both)
                .with_lifetime(MessageLifetime::WhileIdle {
                    system,
                    area: ProductionArea::Shipyard,
                }),
        );
        log.set_now(5_000);
        log.expire(&|_, area| area == ProductionArea::Shipyard);
        assert_eq!(log.messages().len(), 1);
        log.expire(&|_, _| false);
        assert!(log.messages().is_empty());
    }

    #[test]
    fn expiry_waits_for_a_posted_message_and_for_the_index_to_close() {
        // FUN_0048a060 runs FUN_0048a2a0 after filing a message;
        // FUN_0048a610 holds it off while the Message Index is open
        // (+8 bit 0x10000000, FUN_004665f0) and runs it on close
        // (FUN_004664d0).
        let mut log = MessageLog::new();
        log.push(GameMessage::new(1, "old", MessageCategory::Event));
        log.set_now(400);
        log.expire_if_due(&|_, _| true);
        assert_eq!(
            log.messages().len(),
            1,
            "a message off the rails posts no expiry"
        );
        log.set_index_open(true, &|_, _| true);
        log.push(
            GameMessage::new(400, "new", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );
        log.expire_if_due(&|_, _| true);
        assert_eq!(log.messages().len(), 2, "the open index holds expiry off");
        log.set_index_open(false, &|_, _| true);
        assert_eq!(log.messages().len(), 1);
        assert_eq!(log.messages()[0].text, "new");
    }

    #[test]
    fn a_filed_message_runs_the_expiry_that_an_idle_frame_does_not() {
        // FUN_0048a060 runs FUN_0048a2a0 after filing; port: main.rs calls
        // set_index_open and expire_if_due every frame, so only the index's
        // closing (FUN_0048a610(0)) or a filing expires.
        let mut log = MessageLog::new();
        log.push(
            GameMessage::new(1, "old", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );
        log.expire_if_due(&|_, _| true);
        log.set_now(400);
        log.set_index_open(false, &|_, _| true);
        log.expire_if_due(&|_, _| true);
        assert_eq!(log.messages().len(), 1, "no filing, no expiry");
        log.push(
            GameMessage::new(2, "new", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );
        log.expire_if_due(&|_, _| true);
        assert_eq!(log.messages().len(), 1);
        assert_eq!(log.messages()[0].text, "new");
    }

    #[test]
    fn delete_by_ids_removes_matching_messages() {
        // FUN_005f54a0: delete a single message by key from the manager's list.
        let mut log = MessageLog::new();
        let a = log.push(
            GameMessage::new(1, "fleet", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );
        let _b = log.push(
            GameMessage::new(2, "built", MessageCategory::Manufacturing)
                .on_rail(MessageRail::Manufacturing, RailAudience::Both),
        );
        let c = log.push(
            GameMessage::new(3, "arrived", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );

        log.delete_by_ids(&[a, c]);

        assert_eq!(log.len(), 1);
        assert_eq!(log.messages()[0].text, "built");
    }

    #[test]
    fn delete_recomputes_the_unread_mask() {
        // FUN_0048a2a0: after deletion the unread mask drops categories
        // that no longer have unread messages.
        let mut log = MessageLog::new();
        let a = log.push(
            GameMessage::new(1, "fleet", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );
        log.push(
            GameMessage::new(2, "built", MessageCategory::Manufacturing)
                .on_rail(MessageRail::Manufacturing, RailAudience::Both),
        );

        assert_eq!(log.unread_mask(true), 0x080 | 0x008);

        log.delete_by_ids(&[a]);

        assert_eq!(log.unread_mask(true), 0x008);
    }

    #[test]
    fn mark_all_read_clears_every_message() {
        // FUN_0048a530 with param 0: clears all unread bits.
        let mut log = MessageLog::new();
        log.push(
            GameMessage::new(1, "fleet", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );
        log.push(
            GameMessage::new(2, "built", MessageCategory::Manufacturing)
                .on_rail(MessageRail::Manufacturing, RailAudience::Both),
        );

        log.mark_all_read();

        assert_eq!(log.unread_mask(true), 0);
    }

    #[test]
    fn mark_message_read_clears_one_message() {
        // FUN_00469de0: displaying a message sets its font to 10 (read).
        // port: the original only changes the list item's font; the port
        // sets the message's unread field.
        let mut log = MessageLog::new();
        let a = log.push(
            GameMessage::new(1, "fleet", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );
        log.push(
            GameMessage::new(2, "arrived", MessageCategory::Mission)
                .on_rail(MessageRail::Fleet, RailAudience::Both),
        );

        log.mark_message_read(a);

        assert!(!log.messages()[0].unread);
        assert!(log.messages()[1].unread);
        // The rail stays lit because the second message is still unread.
        assert_eq!(log.unread_mask(true), 0x080);
    }

    #[test]
    fn export_jsonl_round_trip() {
        let mut log = MessageLog::new();
        log.push(GameMessage::new(1, "Test event", MessageCategory::Event));
        log.push(GameMessage::new(
            2,
            "Combat happened",
            MessageCategory::Combat,
        ));

        let dir = std::env::temp_dir();
        let path = dir.join("test_export.jsonl");
        log.export_jsonl(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);

        // Verify valid JSON
        for line in &lines {
            let _: serde_json::Value = serde_json::from_str(line).unwrap();
        }

        // Clean up
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn story_movies_wait_in_order_and_post_no_message() {
        // FUN_0048a060's kind 6 (0x48a149) adds the message to the movie
        // list +0x28 only; FUN_00488b90 takes the oldest first.
        let mut log = MessageLog::new();
        let movie = |movie: u32| StoryMovie {
            movie,
            caption: format!("caption {movie}"),
        };
        log.queue_story_movie(movie(104));
        log.queue_story_movie(movie(102));
        assert!(log.messages().is_empty());
        assert!(log.take_advisor_events(true).is_empty());
        assert_eq!(log.take_story_movie(), Some(movie(104)));
        assert_eq!(log.take_story_movie(), Some(movie(102)));
        assert_eq!(log.take_story_movie(), None);
    }
}
