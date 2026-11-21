//! Browser save slots over a key-value store.
//!
//! The browser keeps each slot as two entries, a body and JSON metadata,
//! under keys that carry the save format version:
//!
//! ```text
//! rebellion_save_v<version>_<slot>
//! rebellion_meta_v<version>_<slot>
//! ```
//!
//! The body is the bincode `SaveState`, zlib-compressed, stored as raw
//! bytes ([`StoredValue::Bytes`]). A build before 2026-11-21 stored it as
//! a base64 string ([`StoredValue::Text`]) without compression; such a
//! body still loads.
//!
//! The web page holds them in IndexedDB (`web/gl.js`, `rebellionSaveStore`).
//! IndexedDB answers asynchronously, so the page reads every entry into
//! memory before the game starts and serves reads from there; a write
//! changes that copy at once and commits to IndexedDB in the background.
//! [`SaveStore::commit`] hands out a ticket whose [`WriteStatus`] the game
//! polls. A failed commit puts the copy back as it was, so the slot shows
//! what the disk holds.
//!
//! The slot logic lives here, generic over [`SaveStore`], so it runs under
//! native unit tests with [`MemoryStore`]; `save::wasm_impl` binds it to the
//! page's store.

use serde::{Deserialize, Serialize};

use crate::save::{
    compute_mod_hash, compute_state_fingerprint, decode_body, SaveMeta, SaveState,
    StateFingerprint, MAX_SAVE_SLOTS, MIGRATED_VERSION, SAVE_VERSION, STATE_FINGERPRINT_VERSION,
    V117, V118, V119, V120, V121, V122, V123, V124, V125, V126, V127, V128, V129, V130, V131, V132,
    V133, V134, V135, V136, V137,
};

const BROWSER_META_VERSION: u32 = 1;

/// The zlib level of a stored body (miniz's default; a body of some
/// megabytes compresses in tens of milliseconds).
const BODY_COMPRESSION_LEVEL: u8 = 6;

/// The largest body a stored one decompresses to: a guard against a
/// corrupt entry, far above any campaign's save.
const MAX_BODY_BYTES: usize = 512 << 20;

/// A stored entry's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredValue {
    /// A string: the metadata, or a body a build before 2026-11-21 wrote
    /// (base64 bincode).
    Text(String),
    /// Raw bytes: a zlib-compressed bincode body.
    Bytes(Vec<u8>),
}

impl StoredValue {
    /// The value's size in bytes (a string's UTF-8 length).
    fn size(&self) -> usize {
        match self {
            Self::Text(text) => text.len(),
            Self::Bytes(bytes) => bytes.len(),
        }
    }
}

/// The stored form of a bincode body.
#[must_use]
pub fn encode_body(bincode: &[u8]) -> StoredValue {
    StoredValue::Bytes(miniz_oxide::deflate::compress_to_vec_zlib(
        bincode,
        BODY_COMPRESSION_LEVEL,
    ))
}

/// The bincode body a stored one holds.
///
/// # Errors
/// On bad base64, or bytes that do not decompress (or exceed
/// [`MAX_BODY_BYTES`]).
pub fn decode_stored_body(body: &StoredValue) -> anyhow::Result<Vec<u8>> {
    match body {
        StoredValue::Text(b64) => b64_decode(b64),
        StoredValue::Bytes(compressed) => {
            miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(compressed, MAX_BODY_BYTES)
                .map_err(|error| {
                    anyhow::anyhow!("save body does not decompress: {:?}", error.status)
                })
        }
    }
}

/// The versions whose keys a slot is read from, newest first: a slot holds
/// the current version's save, else the newest older one (migrated on load).
const READ_VERSIONS: [u32; 23] = [
    SAVE_VERSION,
    V137,
    V136,
    V135,
    V134,
    V133,
    V132,
    V131,
    V130,
    V129,
    V128,
    V127,
    V126,
    V125,
    V124,
    V123,
    V122,
    V121,
    V120,
    V119,
    V118,
    V117,
    MIGRATED_VERSION,
];

/// What became of a committed write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteStatus {
    /// The store still writes it.
    Pending,
    /// It is stored.
    Committed,
    /// The browser refused it for lack of space (`QuotaExceededError`).
    QuotaExceeded,
    /// It failed for another reason (access denied, an aborted transaction).
    Failed,
}

impl WriteStatus {
    /// The page bridge's status code (`web/gl.js`, `rebellion_saves_status`).
    #[must_use]
    pub const fn from_code(code: i32) -> Self {
        match code {
            0 => Self::Pending,
            1 => Self::Committed,
            2 => Self::QuotaExceeded,
            _ => Self::Failed,
        }
    }
}

/// A key-value store whose writes are staged and then committed together.
pub trait SaveStore {
    /// The value under `key`; `Ok(None)` when absent.
    ///
    /// # Errors
    /// When the store cannot be read.
    fn get(&self, key: &str) -> anyhow::Result<Option<StoredValue>>;
    /// Stage `value` under `key` for the next commit.
    ///
    /// # Errors
    /// When the store refuses to stage it.
    fn put(&mut self, key: &str, value: StoredValue) -> anyhow::Result<()>;
    /// Stage the removal of `key` for the next commit.
    ///
    /// # Errors
    /// When the store refuses to stage it.
    fn remove(&mut self, key: &str) -> anyhow::Result<()>;
    /// Commit the staged changes as one transaction. Reads see them at once.
    ///
    /// # Errors
    /// When the store refuses the commit outright.
    fn commit(&mut self) -> anyhow::Result<u32>;
    /// What became of the commit `ticket`.
    fn status(&self, ticket: u32) -> WriteStatus;
}

#[derive(Debug, Serialize, Deserialize)]
struct BrowserStateFingerprint {
    version: u16,
    /// Decimal string avoids JavaScript's 53-bit safe-integer limit.
    value: String,
}

impl BrowserStateFingerprint {
    fn from_fingerprint(fingerprint: StateFingerprint) -> Self {
        Self {
            version: fingerprint.version,
            value: fingerprint.value.to_string(),
        }
    }

    fn to_fingerprint(&self) -> anyhow::Result<StateFingerprint> {
        anyhow::ensure!(
            self.version == STATE_FINGERPRINT_VERSION,
            "unsupported state fingerprint version {}",
            self.version
        );
        Ok(StateFingerprint {
            version: self.version,
            value: self.value.parse()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct BrowserSaveMeta {
    schema_version: u32,
    name: String,
    game_tick: u64,
    state_fingerprint: BrowserStateFingerprint,
    /// Absent in metadata written before 2026-10-31.
    #[serde(default)]
    player_is_alliance: Option<bool>,
}

/// Base64 decode (standard alphabet, tolerates missing padding).
///
/// # Errors
/// On a character outside the alphabet.
pub fn b64_decode(s: &str) -> anyhow::Result<Vec<u8>> {
    fn val(c: u8) -> anyhow::Result<u8> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => anyhow::bail!("invalid base64 character: {}", c as char),
        }
    }
    let bytes: Vec<u8> = s.bytes().filter(|b| *b != b'\n' && *b != b'\r').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let a = u32::from(val(chunk[0])?);
        let b = u32::from(val(chunk[1])?);
        let c = if chunk.len() > 2 {
            u32::from(val(chunk[2])?)
        } else {
            0
        };
        let d = if chunk.len() > 3 {
            u32::from(val(chunk[3])?)
        } else {
            0
        };
        let triple = (a << 18) | (b << 12) | (c << 6) | d;
        out.push((triple >> 16) as u8);
        if chunk.len() > 2 && chunk[2] != b'=' {
            out.push((triple >> 8) as u8);
        }
        if chunk.len() > 3 && chunk[3] != b'=' {
            out.push(triple as u8);
        }
    }
    Ok(out)
}

#[must_use]
pub fn versioned_slot_key(version: u32, slot: usize) -> String {
    format!("rebellion_save_v{version}_{slot}")
}

#[must_use]
pub fn versioned_meta_key(version: u32, slot: usize) -> String {
    format!("rebellion_meta_v{version}_{slot}")
}

/// The stored version, body and metadata of a slot, when both are present:
/// the newest version's (see [`READ_VERSIONS`]).
///
/// A body without its metadata (an interrupted write) counts as no save.
fn stored_save(
    store: &impl SaveStore,
    slot: usize,
) -> anyhow::Result<Option<(u32, StoredValue, StoredValue)>> {
    for version in READ_VERSIONS {
        if let (Some(body), Some(meta)) = (
            store.get(&versioned_slot_key(version, slot))?,
            store.get(&versioned_meta_key(version, slot))?,
        ) {
            return Ok(Some((version, body, meta)));
        }
    }
    Ok(None)
}

fn parse_meta(encoded: &StoredValue) -> anyhow::Result<BrowserSaveMeta> {
    let StoredValue::Text(encoded) = encoded else {
        anyhow::bail!("browser save metadata is not text");
    };
    let meta: BrowserSaveMeta = serde_json::from_str(encoded)?;
    anyhow::ensure!(
        meta.schema_version == BROWSER_META_VERSION,
        "unsupported browser save metadata version {}",
        meta.schema_version
    );
    Ok(meta)
}

/// Write `state` to `slot` under the current version's keys, body and
/// metadata in one commit.
///
/// Returns the state's fingerprint and the commit's ticket; the write is
/// done only when [`SaveStore::status`] says so.
///
/// # Errors
/// When the state cannot be encoded or the store refuses the write.
pub fn save_slot(
    store: &mut impl SaveStore,
    slot: usize,
    name: &str,
    state: &SaveState,
) -> anyhow::Result<(StateFingerprint, u32)> {
    let encoded = bincode::serialize(state)?;
    let state_fingerprint = compute_state_fingerprint(state)?;
    // JSON keeps player-provided names lossless and the schema explicit.
    let meta = serde_json::to_string(&BrowserSaveMeta {
        schema_version: BROWSER_META_VERSION,
        name: name.to_string(),
        game_tick: state.clock.tick,
        state_fingerprint: BrowserStateFingerprint::from_fingerprint(state_fingerprint),
        player_is_alliance: Some(state.player_is_alliance),
    })?;
    store.put(
        &versioned_slot_key(SAVE_VERSION, slot),
        encode_body(&encoded),
    )?;
    store.put(
        &versioned_meta_key(SAVE_VERSION, slot),
        StoredValue::Text(meta),
    )?;
    let ticket = store.commit()?;
    Ok((state_fingerprint, ticket))
}

/// Read the save in `slot`.
///
/// # Errors
/// When the slot is empty, the store cannot be read, or the save is
/// corrupt (bad base64 or compression, metadata, body or fingerprint).
pub fn load_slot(store: &impl SaveStore, slot: usize) -> anyhow::Result<(SaveMeta, SaveState)> {
    let Some((version, body, meta_encoded)) = stored_save(store, slot)? else {
        anyhow::bail!("no save in slot {}", slot);
    };
    let bytes = decode_stored_body(&body)?;
    let browser_meta = parse_meta(&meta_encoded)?;
    let expected = browser_meta.state_fingerprint.to_fingerprint()?;
    let (state, state_fingerprint) = decode_body(version, bytes, expected)?;
    let meta = SaveMeta {
        slot,
        name: browser_meta.name,
        timestamp_secs: 0, // no reliable clock in WASM
        game_tick: state.clock.tick,
        mod_names: vec![],
        mod_hash: compute_mod_hash(&[]),
        state_fingerprint,
        player_is_alliance: Some(state.player_is_alliance),
    };
    Ok((meta, state))
}

/// Metadata and payload keys of every readable version count as occupied,
/// including partially written saves. Storage access errors fail closed.
pub fn slot_occupied(store: &impl SaveStore, slot: usize) -> bool {
    READ_VERSIONS.iter().any(|&version| {
        [
            versioned_slot_key(version, slot),
            versioned_meta_key(version, slot),
        ]
        .iter()
        .any(|key| !matches!(store.get(key), Ok(None)))
    })
}

/// Each occupied slot's metadata, in slot order; an unreadable one is an
/// error entry.
pub fn list_saves(store: &impl SaveStore) -> Vec<anyhow::Result<SaveMeta>> {
    (0..MAX_SAVE_SLOTS)
        .filter_map(|slot| match stored_save(store, slot) {
            Ok(None) => None,
            Ok(Some((_, _, encoded))) => Some(parse_meta(&encoded).and_then(|meta| {
                Ok(SaveMeta {
                    slot,
                    name: meta.name,
                    timestamp_secs: 0,
                    game_tick: meta.game_tick,
                    mod_names: vec![],
                    mod_hash: compute_mod_hash(&[]),
                    state_fingerprint: meta.state_fingerprint.to_fingerprint()?,
                    player_is_alliance: meta.player_is_alliance,
                })
            })),
            Err(error) => Some(Err(error)),
        })
        .collect()
}

/// Remove every version's keys of `slot` in one commit.
///
/// # Errors
/// When the store refuses the removal.
pub fn delete_slot(store: &mut impl SaveStore, slot: usize) -> anyhow::Result<u32> {
    for version in READ_VERSIONS {
        store.remove(&versioned_slot_key(version, slot))?;
        store.remove(&versioned_meta_key(version, slot))?;
    }
    store.commit()
}

/// Each changed key and the value it had (`None`: absent), in change order.
type Undo = Vec<(String, Option<StoredValue>)>;

/// An in-memory [`SaveStore`] whose commits settle when told to, for
/// tests: it reads what the page's store would read.
#[derive(Debug, Default)]
pub struct MemoryStore {
    entries: std::collections::BTreeMap<String, StoredValue>,
    staged: Vec<(String, Option<StoredValue>)>,
    /// Per ticket: its status and the entries it replaced, to roll back.
    commits: Vec<(WriteStatus, Undo)>,
    /// When set, a commit whose new size exceeds it fails with a quota
    /// error once settled.
    pub quota_bytes: Option<usize>,
    /// When set, a read fails (an unreadable database).
    pub read_error: bool,
}

impl MemoryStore {
    fn size(&self) -> usize {
        self.entries.iter().map(|(k, v)| k.len() + v.size()).sum()
    }

    /// The stored entries.
    #[must_use]
    pub fn entries(&self) -> &std::collections::BTreeMap<String, StoredValue> {
        &self.entries
    }

    /// Store `value` under `key` directly, as a corrupting writer (or an
    /// older build) would.
    pub fn insert(&mut self, key: &str, value: StoredValue) {
        self.entries.insert(key.to_owned(), value);
    }

    /// Settle every pending commit, oldest first, as the browser's
    /// transactions complete: over the quota one fails and its entries go
    /// back to what they were.
    pub fn settle(&mut self) {
        for index in 0..self.commits.len() {
            if self.commits[index].0 != WriteStatus::Pending {
                continue;
            }
            let over = self.quota_bytes.is_some_and(|quota| self.size() > quota);
            if over {
                let previous = std::mem::take(&mut self.commits[index].1);
                for (key, value) in previous {
                    match value {
                        Some(value) => self.entries.insert(key, value),
                        None => self.entries.remove(&key),
                    };
                }
                self.commits[index].0 = WriteStatus::QuotaExceeded;
            } else {
                self.commits[index].0 = WriteStatus::Committed;
            }
        }
    }
}

impl SaveStore for MemoryStore {
    fn get(&self, key: &str) -> anyhow::Result<Option<StoredValue>> {
        anyhow::ensure!(!self.read_error, "save storage could not be read");
        Ok(self.entries.get(key).cloned())
    }

    fn put(&mut self, key: &str, value: StoredValue) -> anyhow::Result<()> {
        self.staged.push((key.to_owned(), Some(value)));
        Ok(())
    }

    fn remove(&mut self, key: &str) -> anyhow::Result<()> {
        self.staged.push((key.to_owned(), None));
        Ok(())
    }

    fn commit(&mut self) -> anyhow::Result<u32> {
        let mut previous = Vec::new();
        for (key, value) in std::mem::take(&mut self.staged) {
            previous.push((key.clone(), self.entries.get(&key).cloned()));
            match value {
                Some(value) => self.entries.insert(key, value),
                None => self.entries.remove(&key),
            };
        }
        // Undo in reverse so a key staged twice ends at its first value.
        previous.reverse();
        self.commits.push((WriteStatus::Pending, previous));
        Ok(u32::try_from(self.commits.len())?)
    }

    fn status(&self, ticket: u32) -> WriteStatus {
        (ticket as usize)
            .checked_sub(1)
            .and_then(|index| self.commits.get(index))
            .map_or(WriteStatus::Failed, |(status, _)| *status)
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::save::tests::minimal_save_state;

    /// Base64 encode (standard alphabet, no padding).
    #[must_use]
    pub fn b64_encode(data: &[u8]) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
        for chunk in data.chunks(3) {
            let b0 = u32::from(chunk[0]);
            let b1 = chunk.get(1).copied().map_or(0, u32::from);
            let b2 = chunk.get(2).copied().map_or(0, u32::from);
            let triple = (b0 << 16) | (b1 << 8) | b2;
            out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
            out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
            if chunk.len() > 1 {
                out.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
            }
            if chunk.len() > 2 {
                out.push(CHARS[(triple & 0x3F) as usize] as char);
            }
        }
        out
    }

    fn text(value: &str) -> StoredValue {
        StoredValue::Text(value.to_owned())
    }

    fn meta_named(name: &str) -> String {
        serde_json::to_string(&BrowserSaveMeta {
            schema_version: BROWSER_META_VERSION,
            name: name.to_owned(),
            game_tick: 7,
            state_fingerprint: BrowserStateFingerprint {
                version: STATE_FINGERPRINT_VERSION,
                value: "1".to_owned(),
            },
            player_is_alliance: Some(false),
        })
        .unwrap()
    }

    #[test]
    fn a_settled_save_loads_back_with_its_name_and_fingerprint() {
        let mut store = MemoryStore::default();
        let state = minimal_save_state();
        let (fingerprint, ticket) = save_slot(&mut store, 2, "Hoth", &state).unwrap();
        store.settle();
        assert_eq!(store.status(ticket), WriteStatus::Committed);
        let (meta, loaded) = load_slot(&store, 2).unwrap();
        assert_eq!(meta.slot, 2);
        assert_eq!(meta.name, "Hoth");
        assert_eq!(meta.state_fingerprint, fingerprint);
        assert_eq!(compute_state_fingerprint(&loaded).unwrap(), fingerprint);
    }

    #[test]
    fn a_save_stores_its_body_as_compressed_bytes_smaller_than_its_bincode() {
        let mut store = MemoryStore::default();
        let state = minimal_save_state();
        save_slot(&mut store, 0, "Dantooine", &state).unwrap();
        let bincode = bincode::serialize(&state).unwrap();
        let body = &store.entries()[&versioned_slot_key(SAVE_VERSION, 0)];
        let StoredValue::Bytes(compressed) = body else {
            panic!("the body is stored as text: {body:?}");
        };
        assert!(
            compressed.len() < bincode.len(),
            "{} compressed bytes for {} bincode bytes",
            compressed.len(),
            bincode.len()
        );
        assert_eq!(decode_stored_body(body).unwrap(), bincode);
        assert!(matches!(
            store.entries()[&versioned_meta_key(SAVE_VERSION, 0)],
            StoredValue::Text(_)
        ));
    }

    #[test]
    fn a_base64_body_an_older_build_stored_still_loads() {
        let mut store = MemoryStore::default();
        let state = minimal_save_state();
        let (fingerprint, _) = save_slot(&mut store, 1, "Ord Mantell", &state).unwrap();
        let legacy = text(&b64_encode(&bincode::serialize(&state).unwrap()));
        store.insert(&versioned_slot_key(SAVE_VERSION, 1), legacy);
        let (meta, loaded) = load_slot(&store, 1).unwrap();
        assert_eq!(meta.name, "Ord Mantell");
        assert_eq!(meta.state_fingerprint, fingerprint);
        assert_eq!(compute_state_fingerprint(&loaded).unwrap(), fingerprint);
    }

    #[test]
    fn a_compressed_body_that_does_not_decompress_fails_to_load() {
        let mut store = MemoryStore::default();
        save_slot(&mut store, 2, "Kessel", &minimal_save_state()).unwrap();
        let key = versioned_slot_key(SAVE_VERSION, 2);
        let StoredValue::Bytes(mut body) = store.entries()[&key].clone() else {
            panic!("the body is stored as text");
        };
        body.truncate(body.len() / 2);
        store.insert(&key, StoredValue::Bytes(body));
        assert_eq!(list_saves(&store)[0].as_ref().unwrap().name, "Kessel");
        let error = load_slot(&store, 2).unwrap_err().to_string();
        assert!(error.contains("does not decompress"), "{error}");
    }

    #[test]
    fn metadata_stored_as_bytes_lists_as_an_error_entry() {
        let mut store = MemoryStore::default();
        store.insert(&versioned_slot_key(SAVE_VERSION, 0), text("AAAA"));
        store.insert(
            &versioned_meta_key(SAVE_VERSION, 0),
            StoredValue::Bytes(meta_named("Bytes").into_bytes()),
        );
        let listed = list_saves(&store);
        assert_eq!(listed.len(), 1);
        assert!(listed[0].is_err());
    }

    #[test]
    fn a_save_reads_back_while_its_commit_is_pending() {
        let mut store = MemoryStore::default();
        let (_, ticket) = save_slot(&mut store, 0, "Yavin", &minimal_save_state()).unwrap();
        assert_eq!(store.status(ticket), WriteStatus::Pending);
        let listed = list_saves(&store);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].as_ref().unwrap().name, "Yavin");
    }

    #[test]
    fn a_save_writes_body_and_metadata_under_the_current_version_in_one_commit() {
        let mut store = MemoryStore::default();
        let (_, ticket) = save_slot(&mut store, 4, "One", &minimal_save_state()).unwrap();
        assert_eq!(ticket, 1);
        let keys: Vec<_> = store.entries().keys().cloned().collect();
        assert_eq!(
            keys,
            vec![
                versioned_meta_key(SAVE_VERSION, 4),
                versioned_slot_key(SAVE_VERSION, 4)
            ]
        );
    }

    #[test]
    fn a_save_over_the_quota_fails_and_the_slot_keeps_its_previous_save() {
        let mut store = MemoryStore::default();
        let state = minimal_save_state();
        let (first, _) = save_slot(&mut store, 1, "Before", &state).unwrap();
        store.settle();
        let before = store.entries().clone();
        store.quota_bytes = Some(store.size());
        let (_, ticket) =
            save_slot(&mut store, 1, "A much longer name than before", &state).unwrap();
        store.settle();
        assert_eq!(store.status(ticket), WriteStatus::QuotaExceeded);
        assert_eq!(store.entries(), &before);
        let (meta, _) = load_slot(&store, 1).unwrap();
        assert_eq!(meta.name, "Before");
        assert_eq!(meta.state_fingerprint, first);
    }

    #[test]
    fn a_save_that_exactly_fills_the_quota_commits() {
        let state = minimal_save_state();
        let mut sizing = MemoryStore::default();
        save_slot(&mut sizing, 0, "Fits", &state).unwrap();
        let bytes: usize = sizing
            .entries()
            .iter()
            .map(|(k, v)| k.len() + v.size())
            .sum();
        let mut store = MemoryStore {
            quota_bytes: Some(bytes),
            ..MemoryStore::default()
        };
        let (_, ticket) = save_slot(&mut store, 0, "Fits", &state).unwrap();
        store.settle();
        assert_eq!(store.status(ticket), WriteStatus::Committed);
        let (_, ticket) = save_slot(&mut store, 1, "Fits", &state).unwrap();
        store.settle();
        assert_eq!(store.status(ticket), WriteStatus::QuotaExceeded);
    }

    #[test]
    fn a_failed_first_save_leaves_the_slot_empty() {
        let mut store = MemoryStore {
            quota_bytes: Some(0),
            ..MemoryStore::default()
        };
        let (_, ticket) = save_slot(&mut store, 3, "Nothing", &minimal_save_state()).unwrap();
        store.settle();
        assert_eq!(store.status(ticket), WriteStatus::QuotaExceeded);
        assert!(store.entries().is_empty());
        assert!(!slot_occupied(&store, 3));
    }

    #[test]
    fn a_corrupt_body_lists_its_name_but_fails_to_load_and_keeps_its_bytes() {
        let mut store = MemoryStore::default();
        store.insert(
            &versioned_slot_key(SAVE_VERSION, 2),
            text("corrupt fixture"),
        );
        store.insert(
            &versioned_meta_key(SAVE_VERSION, 2),
            text(&meta_named("Bespin")),
        );
        let listed = list_saves(&store);
        assert_eq!(listed[0].as_ref().unwrap().name, "Bespin");
        assert!(load_slot(&store, 2).is_err());
        assert_eq!(
            store.entries()[&versioned_slot_key(SAVE_VERSION, 2)],
            text("corrupt fixture")
        );
    }

    #[test]
    fn unreadable_metadata_lists_as_an_error_entry() {
        let mut store = MemoryStore::default();
        store.insert(&versioned_slot_key(SAVE_VERSION, 1), text(""));
        store.insert(&versioned_meta_key(SAVE_VERSION, 1), text(""));
        let listed = list_saves(&store);
        assert_eq!(listed.len(), 1);
        assert!(listed[0].is_err());
        assert!(load_slot(&store, 1).is_err());
    }

    #[test]
    fn a_body_without_metadata_is_no_save_but_occupies_its_slot() {
        let mut store = MemoryStore::default();
        store.insert(&versioned_slot_key(SAVE_VERSION, 5), text("AAAA"));
        assert!(list_saves(&store).is_empty());
        assert!(slot_occupied(&store, 5));
        assert!(!slot_occupied(&store, 6));
    }

    #[test]
    fn an_older_versions_save_is_read_when_the_current_one_is_absent() {
        let mut store = MemoryStore::default();
        store.insert(&versioned_slot_key(V130, 0), text("AAAA"));
        store.insert(&versioned_meta_key(V130, 0), text(&meta_named("Old")));
        store.insert(&versioned_slot_key(MIGRATED_VERSION, 0), text("AAAA"));
        store.insert(
            &versioned_meta_key(MIGRATED_VERSION, 0),
            text(&meta_named("Oldest")),
        );
        assert_eq!(list_saves(&store)[0].as_ref().unwrap().name, "Old");
        store.insert(&versioned_slot_key(SAVE_VERSION, 0), text("AAAA"));
        store.insert(
            &versioned_meta_key(SAVE_VERSION, 0),
            text(&meta_named("New")),
        );
        assert_eq!(list_saves(&store)[0].as_ref().unwrap().name, "New");
    }

    #[test]
    fn an_unreadable_store_fails_closed() {
        let store = MemoryStore {
            read_error: true,
            ..MemoryStore::default()
        };
        assert!(slot_occupied(&store, 0));
        let listed = list_saves(&store);
        assert_eq!(listed.len(), MAX_SAVE_SLOTS);
        assert!(listed.iter().all(Result::is_err));
        assert!(load_slot(&store, 0).is_err());
    }

    #[test]
    fn delete_removes_every_versions_keys_of_the_slot_only() {
        let mut store = MemoryStore::default();
        for version in READ_VERSIONS {
            store.insert(&versioned_slot_key(version, 3), text("AAAA"));
            store.insert(&versioned_meta_key(version, 3), text("{}"));
        }
        store.insert(&versioned_slot_key(SAVE_VERSION, 4), text("AAAA"));
        let ticket = delete_slot(&mut store, 3).unwrap();
        store.settle();
        assert_eq!(store.status(ticket), WriteStatus::Committed);
        assert!(!slot_occupied(&store, 3));
        assert_eq!(store.entries().len(), 1);
    }

    #[test]
    fn an_unknown_ticket_reads_as_failed() {
        let store = MemoryStore::default();
        assert_eq!(store.status(0), WriteStatus::Failed);
        assert_eq!(store.status(1), WriteStatus::Failed);
    }

    #[test]
    fn bridge_status_codes_map_to_write_statuses() {
        assert_eq!(WriteStatus::from_code(0), WriteStatus::Pending);
        assert_eq!(WriteStatus::from_code(1), WriteStatus::Committed);
        assert_eq!(WriteStatus::from_code(2), WriteStatus::QuotaExceeded);
        assert_eq!(WriteStatus::from_code(3), WriteStatus::Failed);
        assert_eq!(WriteStatus::from_code(-1), WriteStatus::Failed);
    }

    #[test]
    fn base64_round_trips_every_tail_length() {
        let data: Vec<u8> = (0..=255).collect();
        for len in 0..10 {
            let encoded = b64_encode(&data[..len]);
            assert_eq!(b64_decode(&encoded).unwrap(), &data[..len]);
        }
        assert_eq!(b64_encode(b"Man"), "TWFu");
        assert_eq!(b64_encode(b"Ma"), "TWE");
        assert_eq!(b64_decode("TWE=").unwrap(), b"Ma");
        assert!(b64_decode("T*Fu").is_err());
        assert_eq!(b64_decode("TW\r\nFu").unwrap(), b"Man");
    }
}
