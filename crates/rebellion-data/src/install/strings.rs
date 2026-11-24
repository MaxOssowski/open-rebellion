//! The pack's text tables, decoded from the player's DLLs as
//! `dat-dumper --extract-strings` writes them, through this crate's own PE
//! reader (`install::pe`, which also runs in the browser build) and
//! dat-dumper's byte decoders.
//!
//! port: the JSON files are the port's own; each table's content is the
//! DLL's resources.

use std::collections::{BTreeMap, HashMap};

use dat_dumper::types::{agent_advice, gid_legend, textstra};

use super::pack::{BuildError, Entry, InstallFiles, KIND_GAME_DATA};
use super::pe::{self, ResId};

const RT_STRING: u32 = 6;
const RT_RCDATA: u32 = 10;

/// Each numbered resource of `kind`, its first language's bytes (as
/// dat-dumper reads them), by number.
fn first_language<'a>(resources: &[pe::Resource<'a>], kind: u32) -> BTreeMap<u32, &'a [u8]> {
    let mut out = BTreeMap::new();
    for resource in resources.iter().filter(|r| r.kind == ResId::Id(kind)) {
        if let ResId::Id(id) = resource.name {
            out.entry(id).or_insert(resource.data);
        }
    }
    out
}

fn dll<'a>(install: &'a InstallFiles, file: &str) -> Result<Vec<pe::Resource<'a>>, BuildError> {
    let bytes = install.get(file).ok_or_else(|| BuildError::Missing(file.to_owned()))?;
    pe::resources(bytes).map_err(|error| BuildError::Pe { file: file.to_owned(), error })
}

fn decode_error(file: &str, error: &anyhow::Error) -> BuildError {
    BuildError::BadTable(format!("{file}: {error:#}"))
}

/// A string module's RT_STRING table (`textstra::load_strings`).
fn strings(install: &InstallFiles, file: &str) -> Result<HashMap<u16, String>, BuildError> {
    let mut out = HashMap::new();
    for (bundle, raw) in first_language(&dll(install, file)?, RT_STRING) {
        if bundle == 0 {
            continue;
        }
        textstra::parse_string_bundle(raw, (bundle - 1) * 16, &mut out).map_err(|e| decode_error(file, &e))?;
    }
    Ok(out)
}

/// A value as the pack keeps a text table: keys sorted, no spaces
/// (`build-runtime-pack.py`, `json.dumps(sort_keys=True,
/// separators=(",", ":"), ensure_ascii=False)`).
fn canonical(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for key in keys {
                out.insert(key.clone(), canonical(&map[key]));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => serde_json::Value::Array(items.iter().map(canonical).collect()),
        other => other.clone(),
    }
}

fn table(key: &str, value: &impl serde::Serialize) -> Result<Entry, BuildError> {
    let value = serde_json::to_value(value).map_err(|e| BuildError::BadTable(format!("{key}: {e}")))?;
    let data = serde_json::to_vec(&canonical(&value)).map_err(|e| BuildError::BadTable(format!("{key}: {e}")))?;
    Ok(Entry { kind: KIND_GAME_DATA, key: key.to_owned(), data })
}

/// The seven text tables: TEXTSTRA's strings, message texts and advice
/// list, the other string modules (TEXTCOMM, TEXTTACT, REBDLOG), and
/// STRATEGY.DLL's legend layouts.
///
/// # Errors
/// A missing DLL, or a resource that does not decode.
pub fn text_tables(install: &InstallFiles, out: &mut Vec<Entry>) -> Result<(), BuildError> {
    out.push(table("textstra.json", &strings(install, "TEXTSTRA.DLL")?)?);
    let textstra = dll(install, "TEXTSTRA.DLL")?;
    let records = first_language(&textstra, RT_RCDATA);
    let messages: HashMap<u16, String> = records
        .iter()
        .filter_map(|(&id, raw)| Some((u16::try_from(id).ok()?, textstra::decode_message_text(raw))))
        .collect();
    out.push(table("textstra-messages.json", &messages)?);
    let advice_record = |id: u16| {
        (0x6000..0x7000)
            .contains(&id)
            .then(|| records.get(&u32::from(id)).copied())
            .flatten()
    };
    let advice = agent_advice::AgentAdviceTables {
        alliance: agent_advice::decode_side(agent_advice::COUNT_RECORDS[0], advice_record)
            .map_err(|e| decode_error("TEXTSTRA.DLL", &e))?,
        empire: agent_advice::decode_side(agent_advice::COUNT_RECORDS[1], advice_record)
            .map_err(|e| decode_error("TEXTSTRA.DLL", &e))?,
    };
    out.push(table("textstra-advice.json", &advice)?);
    for (file, key) in [("TEXTCOMM.DLL", "textcomm.json"), ("TEXTTACT.DLL", "texttact.json"), ("REBDLOG.DLL", "rebdlog.json")] {
        out.push(table(key, &strings(install, file)?)?);
    }
    let strategy = first_language(&dll(install, "STRATEGY.DLL")?, RT_RCDATA);
    let mut legends = HashMap::new();
    for &(_, id) in &gid_legend::LEGEND_RECORDS {
        if let Some(raw) = strategy.get(&u32::from(id)) {
            legends.insert(id, gid_legend::decode(raw).map_err(|e| decode_error("STRATEGY.DLL", &e))?);
        }
    }
    out.push(table("gid-legends.json", &legends)?);
    Ok(())
}
