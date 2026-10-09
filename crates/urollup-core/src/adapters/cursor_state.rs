//! Cursor composer state-store adapter (`cursor-state`).
//!
//! The usage owner is `composerData` / `bubbleId` in `state.vscdb`, or a synthetic
//! `urollup-cursor-state/v1` JSON snapshot used by fixtures. JSONL transcripts are the
//! same `composerId` when present and are not a second session. Discovery is opt-in.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use jiff::Timestamp;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, Row};
use serde::Deserialize;
use serde_json::Value;

use super::{AdapterError, Ingested};
use crate::ledger::capacity::ObservationCapacity;
use crate::ledger::diagnostics::{Diagnostic, DiagnosticCode};
use crate::ledger::entities::{
    Basis, CompactTimestamp, Confidence, ModelBasis, ModelName, Relationship, RelationshipKind,
    SourceArtifact, SourceCapability, Thread,
};
use crate::ledger::identity::{IdPrefix, KeyComponent, StoredIdentity};
use crate::ledger::names::Name;
use crate::ledger::reconcile::{
    LatestRevision, ObservationRole, OwnerEvidence, ReconcileInput, RequestObservation,
    reconcile_with_capacity,
};
use crate::ledger::scope::{
    ComponentRole, ComponentSlot, IdScope, IdentityBasis, KeySpec, artifact_local_key,
};
use crate::ledger::tokens::TokenMeasures;
use crate::selection::{Agent, agent_thread_identity};
use crate::sources::evidence::{EvidenceRef, SourceTable};
use crate::sources::manifest::{
    Cutoff, FileIdentity, Fingerprint, ManifestEntry, RecordCounters, Representation,
    SOURCE_STABLE_LOCATOR, SnapshotManifest,
};

const DIALECT: &str = "cursor-state";
const FIXTURE_FORMAT: &str = "urollup-cursor-state/v1";

const REQUEST_SLOTS: &[ComponentSlot] = &[
    ComponentSlot::required("thread", ComponentRole::Parent),
    ComponentSlot::required("native_id", ComponentRole::NativeId),
];

const REQUEST_KEY: KeySpec = KeySpec {
    prefix: IdPrefix::Request,
    kind: "cursor-request",
    precedence: 0,
    basis: IdentityBasis::Native,
    scope: IdScope::Thread,
    slots: REQUEST_SLOTS,
};

const USAGE_KEY: KeySpec = KeySpec {
    prefix: IdPrefix::Request,
    kind: "cursor-usage-uuid",
    precedence: 1,
    basis: IdentityBasis::Native,
    scope: IdScope::Thread,
    slots: REQUEST_SLOTS,
};

/// Whether `path` is a Cursor state store or a synthetic snapshot.
pub fn is_cursor_source(path: &Path) -> bool {
    if path.is_file() {
        return is_state_db_name(path) || is_cursor_fixture(path);
    }
    path.join("state.vscdb").is_file() || path.join("cursor-state.json").is_file()
}

fn is_state_db_name(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("vscdb"))
}

fn is_cursor_fixture(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    peek_fixture(&bytes)
}

fn peek_fixture(bytes: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return false;
    };
    value.get("format").and_then(Value::as_str) == Some(FIXTURE_FORMAT)
}

/// Reads one Cursor store path. A directory may contain `state.vscdb` or `cursor-state.json`.
pub fn ingest_root(root: &Path) -> Result<Ingested, AdapterError> {
    ingest_roots(&[root.to_owned()], true, &ObservationCapacity::default(), None)
}

/// Reads each opted-in Cursor store.
pub fn ingest_roots(
    roots: &[PathBuf],
    missing_is_error: bool,
    capacity: &ObservationCapacity,
    composer_ids: Option<&[String]>,
) -> Result<Ingested, AdapterError> {
    let mut composers = Vec::new();
    let mut first_store: Option<(PathBuf, Vec<u8>)> = None;
    for root in roots {
        let store = resolve_store(root, missing_is_error)?;
        let Some(store) = store else {
            continue;
        };
        let snapshot = if is_state_db_name(&store) {
            read_sqlite(&store, composer_ids)?
        } else {
            let bytes = fs::read(&store).map_err(|error| AdapterError::Store {
                path: store.clone(),
                message: error.to_string(),
            })?;
            let mut snapshot = parse_fixture(&bytes)
                .map_err(|error| AdapterError::Store { path: store.clone(), message: error })?;
            if let Some(ids) = composer_ids {
                snapshot.composers = retain_composer_family(snapshot.composers, ids);
            }
            snapshot
        };
        if first_store.is_none() {
            first_store = Some((store.clone(), store_fingerprint_prefix(&store)?));
        }
        composers.extend(snapshot.composers);
    }
    let Some((path, bytes)) = first_store else {
        return Ok(Ingested::default());
    };
    normalize(&path, &bytes, StoreSnapshot { composers }, capacity)
}

fn store_fingerprint_prefix(path: &Path) -> Result<Vec<u8>, AdapterError> {
    let mut file = File::open(path).map_err(|error| AdapterError::Store {
        path: path.to_owned(),
        message: error.to_string(),
    })?;
    let mut prefix = vec![0_u8; 256];
    let read = file.read(&mut prefix).map_err(|error| AdapterError::Store {
        path: path.to_owned(),
        message: error.to_string(),
    })?;
    prefix.truncate(read);
    Ok(prefix)
}

fn resolve_store(root: &Path, missing_is_error: bool) -> Result<Option<PathBuf>, AdapterError> {
    if root.is_file() {
        return Ok(Some(root.to_owned()));
    }
    if root.is_dir() {
        if root.join("state.vscdb").is_file() {
            return Ok(Some(root.join("state.vscdb")));
        }
        if root.join("cursor-state.json").is_file() {
            return Ok(Some(root.join("cursor-state.json")));
        }
        return Err(AdapterError::Store {
            path: root.to_owned(),
            message: "directory has no state.vscdb or cursor-state.json".to_owned(),
        });
    }
    if missing_is_error { Err(AdapterError::MissingRoot(root.to_owned())) } else { Ok(None) }
}

fn store_error(path: &Path, message: impl Into<String>) -> AdapterError {
    AdapterError::Store { path: path.to_owned(), message: message.into() }
}

#[derive(Clone, Debug, Default, Deserialize)]
struct FixtureFile {
    format: String,
    #[serde(default)]
    composers: Vec<ComposerRecord>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComposerRecord {
    composer_id: String,
    #[serde(default)]
    created_at: Option<i64>,
    #[serde(default)]
    last_updated_at: Option<i64>,
    #[serde(default)]
    unified_mode: Option<String>,
    #[serde(default)]
    workspace_id: Option<String>,
    #[serde(default)]
    is_subagent: bool,
    #[serde(default)]
    parent_composer_id: Option<String>,
    #[serde(default)]
    sub_composer_ids: Vec<String>,
    #[serde(default)]
    subagent_composer_ids: Vec<String>,
    /// Recorded on Best-of-N sibling composers. Not an ownership edge: `Other`
    /// does not define descendants, and whether Cursor bills siblings separately is
    /// still open. Independently recorded tokens still count on each composer.
    #[serde(default)]
    #[allow(dead_code)]
    is_best_of_n_subcomposer: bool,
    #[serde(default)]
    usage_data: BTreeMap<String, UsagePayload>,
    #[serde(default)]
    model_name: Option<String>,
    #[serde(default)]
    bubbles: Vec<BubbleRecord>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsagePayload {
    #[serde(default)]
    cost_in_cents: Option<i64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BubbleRecord {
    #[serde(default)]
    bubble_id: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    usage_uuid: Option<String>,
    #[serde(default)]
    created_at: Option<i64>,
    #[serde(default)]
    model_name: Option<String>,
    #[serde(default)]
    thinking_style: Option<i64>,
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
}

#[derive(Clone, Debug, Default)]
struct StoreSnapshot {
    composers: Vec<ComposerRecord>,
}

fn parse_fixture(bytes: &[u8]) -> Result<StoreSnapshot, String> {
    let file: FixtureFile =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid fixture JSON: {error}"))?;
    if file.format != FIXTURE_FORMAT {
        return Err(format!("unsupported Cursor fixture format {:?}", file.format));
    }
    Ok(StoreSnapshot { composers: file.composers })
}

fn read_sqlite(
    path: &Path,
    composer_ids: Option<&[String]>,
) -> Result<StoreSnapshot, AdapterError> {
    let snapshot = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| store_error(path, error.to_string()))?;
    snapshot
        .pragma_update(None, "query_only", true)
        .map_err(|error| store_error(path, error.to_string()))?;
    let mut composers = if let Some(ids) = composer_ids {
        load_composer_family(&snapshot, ids).map_err(|error| store_error(path, error))?
    } else {
        load_composers(&snapshot).map_err(|error| store_error(path, error))?
    };
    let headers = load_headers(&snapshot).map_err(|error| store_error(path, error))?;
    for composer in &mut composers {
        if let Some(header) = headers.get(&composer.composer_id) {
            composer.is_subagent |= header.is_subagent;
            if composer.parent_composer_id.is_none() {
                composer.parent_composer_id = header.parent_composer_id.clone();
            }
            if composer.workspace_id.is_none() {
                composer.workspace_id = header.workspace_id.clone();
            }
        }
    }
    let wanted: BTreeSet<&str> =
        composers.iter().map(|composer| composer.composer_id.as_str()).collect();
    let bubbles = load_bubbles(&snapshot, composer_ids.is_some().then_some(&wanted))
        .map_err(|error| store_error(path, error))?;
    let mut by_composer: BTreeMap<String, Vec<BubbleRecord>> = BTreeMap::new();
    for (composer_id, bubble) in bubbles {
        by_composer.entry(composer_id).or_default().push(bubble);
    }
    for composer in &mut composers {
        composer.bubbles = by_composer.remove(&composer.composer_id).unwrap_or_default();
    }
    Ok(StoreSnapshot { composers })
}

struct HeaderRow {
    is_subagent: bool,
    parent_composer_id: Option<String>,
    workspace_id: Option<String>,
}

const COMPOSER_COLUMNS: &str = "SELECT key,
                    json_extract(value, '$.composerId'),
                    json_extract(value, '$.createdAt'),
                    json_extract(value, '$.lastUpdatedAt'),
                    json_extract(value, '$.unifiedMode'),
                    json_extract(value, '$.usageData'),
                    json_extract(value, '$.subComposerIds'),
                    json_extract(value, '$.subagentComposerIds'),
                    json_extract(value, '$.isBestOfNSubcomposer'),
                    json_extract(value, '$.subagentInfo.parentComposerId'),
                    json_extract(value, '$.modelConfig.modelName'),
                    json_extract(value, '$.workspaceId')
             FROM cursorDiskKV";

fn load_composers(connection: &Connection) -> Result<Vec<ComposerRecord>, String> {
    let mut statement = connection
        .prepare(&format!("{COMPOSER_COLUMNS} WHERE key LIKE 'composerData:%'"))
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                optional_text(row, 1)?,
                optional_i64(row, 2)?,
                optional_i64(row, 3)?,
                optional_text(row, 4)?,
                optional_text(row, 5)?,
                optional_text(row, 6)?,
                optional_text(row, 7)?,
                optional_i64(row, 8)?,
                optional_text(row, 9)?,
                optional_text(row, 10)?,
                optional_text(row, 11)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut composers = Vec::new();
    for row in rows {
        let (
            key,
            composer_id,
            created_at,
            last_updated_at,
            unified_mode,
            usage_data,
            sub_composer_ids,
            subagent_composer_ids,
            is_best_of_n,
            parent_composer_id,
            model_name,
            workspace_id,
        ) = row.map_err(|error| error.to_string())?;
        let composer_id = composer_id
            .or_else(|| key.strip_prefix("composerData:").map(str::to_owned))
            .filter(|id| !id.is_empty());
        let Some(composer_id) = composer_id else {
            continue;
        };
        composers.push(ComposerRecord {
            composer_id,
            created_at,
            last_updated_at,
            unified_mode,
            workspace_id,
            parent_composer_id,
            sub_composer_ids: parse_id_list(sub_composer_ids.as_deref()),
            subagent_composer_ids: parse_id_list(subagent_composer_ids.as_deref()),
            is_best_of_n_subcomposer: is_best_of_n == Some(1),
            usage_data: parse_usage_data(usage_data.as_deref()),
            model_name,
            ..ComposerRecord::default()
        });
    }
    Ok(composers)
}

fn load_composer_family(
    connection: &Connection,
    seeds: &[String],
) -> Result<Vec<ComposerRecord>, String> {
    let mut statement = connection
        .prepare(&format!("{COMPOSER_COLUMNS} WHERE key = ?1"))
        .map_err(|error| error.to_string())?;
    let mut attempted = BTreeSet::new();
    let mut stack: Vec<String> = seeds.to_vec();
    let mut composers = Vec::new();
    while let Some(id) = stack.pop() {
        if !attempted.insert(id.clone()) {
            continue;
        }
        let key = format!("composerData:{id}");
        let mut rows = statement
            .query_map([&key], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    optional_text(row, 1)?,
                    optional_i64(row, 2)?,
                    optional_i64(row, 3)?,
                    optional_text(row, 4)?,
                    optional_text(row, 5)?,
                    optional_text(row, 6)?,
                    optional_text(row, 7)?,
                    optional_i64(row, 8)?,
                    optional_text(row, 9)?,
                    optional_text(row, 10)?,
                    optional_text(row, 11)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        let Some(row) = rows.next() else {
            continue;
        };
        let (
            key,
            composer_id,
            created_at,
            last_updated_at,
            unified_mode,
            usage_data,
            sub_composer_ids,
            subagent_composer_ids,
            is_best_of_n,
            parent_composer_id,
            model_name,
            workspace_id,
        ) = row.map_err(|error| error.to_string())?;
        let composer_id = composer_id
            .or_else(|| key.strip_prefix("composerData:").map(str::to_owned))
            .filter(|id| !id.is_empty());
        let Some(composer_id) = composer_id else {
            continue;
        };
        let composer = ComposerRecord {
            composer_id,
            created_at,
            last_updated_at,
            unified_mode,
            workspace_id,
            parent_composer_id,
            sub_composer_ids: parse_id_list(sub_composer_ids.as_deref()),
            subagent_composer_ids: parse_id_list(subagent_composer_ids.as_deref()),
            is_best_of_n_subcomposer: is_best_of_n == Some(1),
            usage_data: parse_usage_data(usage_data.as_deref()),
            model_name,
            ..ComposerRecord::default()
        };
        stack.extend(composer.sub_composer_ids.iter().cloned());
        stack.extend(composer.subagent_composer_ids.iter().cloned());
        composers.push(composer);
    }
    Ok(composers)
}

fn load_headers(connection: &Connection) -> Result<BTreeMap<String, HeaderRow>, String> {
    let mut headers = BTreeMap::new();
    let Ok(mut statement) = connection.prepare(
        "SELECT composerId, isSubagent,
                json_extract(value, '$.parentComposerId'),
                json_extract(value, '$.workspaceId')
         FROM composerHeaders",
    ) else {
        return Ok(headers);
    };
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                optional_i64(row, 1)?.unwrap_or(0),
                optional_text(row, 2)?,
                optional_text(row, 3)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    for row in rows {
        let (composer_id, is_subagent, parent, workspace_id) =
            row.map_err(|error| error.to_string())?;
        headers.insert(
            composer_id,
            HeaderRow { is_subagent: is_subagent != 0, parent_composer_id: parent, workspace_id },
        );
    }
    if let Ok(mut statement) =
        connection.prepare("SELECT composerId, workspaceId FROM composerHeaders")
    {
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, optional_text(row, 1)?)))
            .map_err(|error| error.to_string())?;
        for row in rows {
            let (composer_id, workspace_id) = row.map_err(|error| error.to_string())?;
            if let Some(header) = headers.get_mut(&composer_id) {
                if header.workspace_id.is_none() {
                    header.workspace_id = workspace_id;
                }
            }
        }
    }
    Ok(headers)
}

fn load_bubbles(
    connection: &Connection,
    only: Option<&BTreeSet<&str>>,
) -> Result<Vec<(String, BubbleRecord)>, String> {
    const BUBBLE_SELECT: &str = "SELECT key,
                    json_extract(value, '$.requestId'),
                    json_extract(value, '$.usageUuid'),
                    json_extract(value, '$.createdAt'),
                    coalesce(
                        json_extract(value, '$.modelInfo.modelName'),
                        json_extract(value, '$.modelName')
                    ),
                    json_extract(value, '$.thinkingStyle'),
                    json_extract(value, '$.tokenCount.inputTokens'),
                    json_extract(value, '$.tokenCount.outputTokens')
             FROM cursorDiskKV
             WHERE key LIKE ?1";
    let patterns: Vec<String> = match only {
        Some(ids) if !ids.is_empty() => ids.iter().map(|id| format!("bubbleId:{id}:%")).collect(),
        Some(_) => return Ok(Vec::new()),
        None => vec!["bubbleId:%".to_owned()],
    };
    let mut bubbles = Vec::new();
    let mut statement = connection.prepare(BUBBLE_SELECT).map_err(|error| error.to_string())?;
    for pattern in patterns {
        let rows = statement
            .query_map([&pattern], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    optional_text(row, 1)?,
                    optional_text(row, 2)?,
                    optional_i64(row, 3)?,
                    optional_text(row, 4)?,
                    optional_i64(row, 5)?,
                    optional_i64(row, 6)?,
                    optional_i64(row, 7)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        for row in rows {
            let (
                key,
                request_id,
                usage_uuid,
                created_at,
                model_name,
                thinking_style,
                input,
                output,
            ) = row.map_err(|error| error.to_string())?;
            let rest = key.strip_prefix("bubbleId:").unwrap_or(&key);
            let Some((composer_id, bubble_id)) = rest.split_once(':') else {
                continue;
            };
            bubbles.push((
                composer_id.to_owned(),
                BubbleRecord {
                    bubble_id: Some(bubble_id.to_owned()),
                    request_id,
                    usage_uuid,
                    created_at,
                    model_name,
                    thinking_style,
                    input_tokens: input.and_then(|value| u64::try_from(value).ok()),
                    output_tokens: output.and_then(|value| u64::try_from(value).ok()),
                },
            ));
        }
    }
    Ok(bubbles)
}

fn retain_composer_family(composers: Vec<ComposerRecord>, seeds: &[String]) -> Vec<ComposerRecord> {
    let by_id: BTreeMap<String, ComposerRecord> =
        composers.into_iter().map(|composer| (composer.composer_id.clone(), composer)).collect();
    let mut keep = BTreeSet::new();
    let mut stack: Vec<String> = seeds.to_vec();
    while let Some(id) = stack.pop() {
        if !keep.insert(id.clone()) {
            continue;
        }
        if let Some(composer) = by_id.get(&id) {
            stack.extend(composer.sub_composer_ids.iter().cloned());
            stack.extend(composer.subagent_composer_ids.iter().cloned());
        }
    }
    by_id.into_values().filter(|composer| keep.contains(&composer.composer_id)).collect()
}

fn optional_text(row: &Row<'_>, idx: usize) -> Result<Option<String>, rusqlite::Error> {
    match row.get_ref(idx)? {
        ValueRef::Text(bytes) => {
            let text = std::str::from_utf8(bytes).unwrap_or_default();
            if text.is_empty() || text == "null" { Ok(None) } else { Ok(Some(text.to_owned())) }
        }
        ValueRef::Integer(value) => Ok(Some(value.to_string())),
        ValueRef::Real(value) => Ok(Some(value.to_string())),
        ValueRef::Null | ValueRef::Blob(_) => Ok(None),
    }
}

fn optional_i64(row: &Row<'_>, idx: usize) -> Result<Option<i64>, rusqlite::Error> {
    match row.get_ref(idx)? {
        ValueRef::Integer(value) => Ok(Some(value)),
        ValueRef::Text(bytes) => {
            let text = std::str::from_utf8(bytes).unwrap_or_default().trim();
            if text.is_empty() || text == "null" {
                return Ok(None);
            }
            if let Ok(parsed) = text.parse::<i64>() {
                return Ok(Some(parsed));
            }
            if text.eq_ignore_ascii_case("true") {
                return Ok(Some(1));
            }
            if text.eq_ignore_ascii_case("false") {
                return Ok(Some(0));
            }
            Ok(text.parse::<Timestamp>().ok().map(Timestamp::as_millisecond))
        }
        ValueRef::Null | ValueRef::Real(_) | ValueRef::Blob(_) => Ok(None),
    }
}

fn parse_id_list(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw.filter(|value| !value.is_empty() && *value != "null") else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

fn parse_usage_data(raw: Option<&str>) -> BTreeMap<String, UsagePayload> {
    let Some(raw) = raw.filter(|value| !value.is_empty() && *value != "null" && *value != "{}")
    else {
        return BTreeMap::new();
    };
    serde_json::from_str(raw).unwrap_or_default()
}

fn normalize(
    path: &Path,
    bytes: &[u8],
    snapshot: StoreSnapshot,
    capacity: &ObservationCapacity,
) -> Result<Ingested, AdapterError> {
    let fingerprint = Fingerprint::of(bytes.get(..256).unwrap_or(bytes));
    let source = StoredIdentity::derive(
        SOURCE_STABLE_LOCATOR
            .key(vec![
                KeyComponent::text("local"),
                KeyComponent::text(DIALECT),
                KeyComponent::text(
                    path.file_name().and_then(|name| name.to_str()).unwrap_or("cursor"),
                ),
                KeyComponent::text(fingerprint.to_base32()),
            ])?
            .key,
    )?;
    let metadata = fs::metadata(path).ok();
    let file_len = metadata.as_ref().map_or(0, fs::Metadata::len);
    let entry = ManifestEntry {
        source: Some(source.clone()),
        environment: "local".to_owned(),
        dialect: DIALECT.to_owned(),
        locator: path.to_string_lossy().into_owned(),
        file: FileIdentity { path: path.to_owned(), device: None, inode: None },
        representation: Representation::Plain,
        twins: Vec::new(),
        file_len,
        modified: metadata.and_then(|meta| meta.modified().ok()),
        fingerprint: Some(fingerprint),
        cutoff: Cutoff {
            complete_through: file_len,
            pending_tail: None,
            captured_at: SystemTime::now(),
        },
        counters: RecordCounters {
            records: u64::try_from(snapshot.composers.len()).unwrap_or(u64::MAX),
            decoded: u64::try_from(snapshot.composers.len()).unwrap_or(u64::MAX),
            ..RecordCounters::default()
        },
        first_malformed: None,
        failures: Vec::new(),
        changes: Vec::new(),
    };
    let source_table = SourceTable::from_ids([source.id.clone()]);
    let source_index = source_table.index_of(&source.id).unwrap_or(0);

    let mut threads = BTreeMap::new();
    let mut relationships = Vec::new();
    let mut requests = Vec::new();
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();
    let mut offset = 0_u64;

    for composer in snapshot.composers {
        if !seen.insert(composer.composer_id.clone()) {
            continue;
        }
        let identity = agent_thread_identity(Agent::Cursor, &composer.composer_id)?;
        let thread_id = identity.id.clone();
        let mut native_key = BTreeMap::new();
        native_key.insert("composer_id".to_owned(), composer.composer_id.clone());
        threads.insert(
            thread_id.clone(),
            Thread {
                identity,
                basis: IdentityBasis::Native,
                aliases: Vec::new(),
                native_key,
                source: Basis::Observed("cursor".to_owned()),
                initiator: Basis::Unknown,
                purpose: composer.unified_mode.clone().map_or(Basis::Unknown, Basis::Observed),
                execution_environment: Basis::Observed("local".to_owned()),
                project: composer.workspace_id.clone().map_or(Basis::Unknown, Basis::Observed),
                account: Basis::Unknown,
                evidence: vec![EvidenceRef::new(source_index, offset, 1)],
            },
        );
        for child in composer.sub_composer_ids.iter().chain(composer.subagent_composer_ids.iter()) {
            let child_identity = agent_thread_identity(Agent::Cursor, child)?;
            relationships.push(Relationship {
                kind: RelationshipKind::Spawn,
                from: thread_id.clone(),
                to: child_identity.id,
                confidence: Confidence::Proven,
                evidence: vec![EvidenceRef::new(source_index, offset, 1)],
            });
        }
        if let Some(parent) = &composer.parent_composer_id {
            let parent_identity = agent_thread_identity(Agent::Cursor, parent)?;
            relationships.push(Relationship {
                kind: RelationshipKind::Spawn,
                from: parent_identity.id,
                to: thread_id.clone(),
                confidence: Confidence::Proven,
                evidence: vec![EvidenceRef::new(source_index, offset, 1)],
            });
        }

        let cost_estimate = composer
            .usage_data
            .values()
            .any(|payload| payload.cost_in_cents.is_some_and(|cost| cost > 0));
        if cost_estimate {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::CursorEstimateCost,
                Some(thread_id.clone()),
                [EvidenceRef::new(source_index, offset, 1)],
                "Cursor reported a session cost estimate",
            ));
        }
        for bubble in composer.bubbles {
            if !has_tokens(&bubble) {
                offset = offset.saturating_add(1);
                continue;
            }
            let evidence = EvidenceRef::new(source_index, offset, 1);
            let mut observation = RequestObservation::new(evidence);
            if let Some(request_id) = bubble.request_id.as_deref().filter(|id| !id.is_empty()) {
                observation.keys.push(
                    REQUEST_KEY
                        .key(vec![
                            KeyComponent::text(thread_id.to_string()),
                            KeyComponent::text(request_id),
                        ])?
                        .derive()?,
                );
            }
            if let Some(usage_uuid) = bubble.usage_uuid.as_deref().filter(|id| !id.is_empty()) {
                observation.keys.push(
                    USAGE_KEY
                        .key(vec![
                            KeyComponent::text(thread_id.to_string()),
                            KeyComponent::text(usage_uuid),
                        ])?
                        .derive()?,
                );
            }
            if observation.keys.is_empty() {
                if let Some(bubble_id) = bubble.bubble_id.as_deref().filter(|id| !id.is_empty()) {
                    observation.keys.push(
                        REQUEST_KEY
                            .key(vec![
                                KeyComponent::text(thread_id.to_string()),
                                KeyComponent::text(bubble_id),
                            ])?
                            .derive()?,
                    );
                } else {
                    let local = artifact_local_key(IdPrefix::Request, &source.id, offset)
                        .ok_or_else(|| {
                            store_error(path, "cannot assign an artifact-local request key")
                        })?;
                    observation.keys.push(local.derive()?);
                }
            }
            observation.owner = OwnerEvidence::Proven(thread_id.clone());
            observation.role = ObservationRole::Original;
            observation.usage = Some(
                TokenMeasures {
                    uncached_input: bubble.input_tokens,
                    output: bubble.output_tokens,
                    ..TokenMeasures::default()
                }
                .into(),
            );
            observation.model = attributed_model_name(
                bubble.model_name.as_deref().or(composer.model_name.as_deref()),
            )
            .map(|name| ModelName { name: Name::from(name), basis: ModelBasis::Requested });
            observation.effort =
                bubble.thinking_style.map(|style| Name::from(format!("style-{style}")));
            observation.timestamp = millis_timestamp(
                bubble.created_at.or(composer.last_updated_at).or(composer.created_at),
            );
            if let Some(model) = observation.model.as_ref() {
                observation.invariants.push(("model", model.name));
            }
            requests.push(observation);
            offset = offset.saturating_add(1);
        }
        offset = offset.saturating_add(1);
    }

    let ledger = reconcile_with_capacity(
        ReconcileInput {
            threads: threads.values().cloned().collect(),
            relationships: relationships.clone(),
            requests,
            tool_actions: Vec::new(),
            limit_observations: Vec::new(),
            links: Vec::new(),
            gaps: Vec::new(),
            diagnostics,
            source_table,
        },
        &LatestRevision,
        capacity,
    )?;

    Ok(Ingested {
        manifest: SnapshotManifest { entries: vec![entry.clone()], skipped_links: Vec::new() },
        sources: vec![SourceArtifact {
            dialect_version: Basis::Unknown,
            capability: SourceCapability::Supported,
            snapshot: entry,
        }],
        threads,
        relationships,
        ledger,
        limit_observations: Vec::new(),
    })
}

fn has_tokens(bubble: &BubbleRecord) -> bool {
    bubble.input_tokens.unwrap_or(0) > 0 || bubble.output_tokens.unwrap_or(0) > 0
}

/// Historical model attached to a counted bubble.
///
/// `default` is Auto. A comma-separated picker list is not one catalog id: if every
/// part is the same name, keep that name; otherwise leave the model unknown rather
/// than invent a joined label or a token split.
fn attributed_model_name(name: Option<&str>) -> Option<String> {
    let name = name?.trim();
    if name.is_empty() {
        return None;
    }
    let mut unique = BTreeSet::new();
    for part in name.split(',') {
        let part = part.trim();
        if !part.is_empty() {
            unique.insert(part);
        }
    }
    let single = match unique.len() {
        1 => unique.into_iter().next()?,
        _ => return None,
    };
    if single == "default" { Some("auto".to_owned()) } else { Some(single.to_owned()) }
}

fn millis_timestamp(value: Option<i64>) -> Option<CompactTimestamp> {
    value.and_then(|ms| Timestamp::from_millisecond(ms).ok()).map(CompactTimestamp::from)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{ingest_root, ingest_roots, is_cursor_source};
    use crate::ledger::provider::provider_for;
    use crate::query::{GroupBy, QueryMetadata, QuerySource, ResolvedTimeZone, report};
    use crate::selection::{Agent, SelectionQuery, SessionIndex};
    use std::collections::BTreeSet;
    use std::fs;
    use tempfile::tempdir;

    fn write_fixture(body: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("cursor-state.json");
        fs::write(&path, body).expect("write fixture");
        (dir, path)
    }

    fn fixture_json() -> String {
        r#"{
          "format": "urollup-cursor-state/v1",
          "composers": [
            {
              "composerId": "11111111-1111-4111-8111-111111111111",
              "createdAt": 1726700000000,
              "lastUpdatedAt": 1726700005000,
              "unifiedMode": "agent",
              "bubbles": [
                {
                  "bubbleId": "b1",
                  "requestId": "req-opus",
                  "createdAt": 1726700001000,
                  "modelName": "claude-4.5-opus-high-thinking",
                  "thinkingStyle": 2,
                  "inputTokens": 20,
                  "outputTokens": 5
                }
              ]
            },
            {
              "composerId": "22222222-2222-4222-8222-222222222222",
              "createdAt": 1726701000000,
              "lastUpdatedAt": 1726701005000,
              "unifiedMode": "agent",
              "bubbles": [
                {
                  "bubbleId": "b2",
                  "requestId": "req-grok",
                  "createdAt": 1726701001000,
                  "modelName": "cursor-grok-4.6-xhigh-fast",
                  "inputTokens": 10,
                  "outputTokens": 3
                }
              ]
            },
            {
              "composerId": "33333333-3333-4333-8333-333333333333",
              "createdAt": 1726702000000,
              "unifiedMode": "chat",
              "usageData": { "default": { "costInCents": 4 } },
              "bubbles": [
                { "bubbleId": "b3", "inputTokens": 0, "outputTokens": 0 }
              ]
            }
          ]
        }"#
        .to_owned()
    }

    #[test]
    fn fixture_path_is_recognized() {
        let (_dir, path) = write_fixture(&fixture_json());
        assert!(is_cursor_source(&path));
    }

    #[test]
    fn ingest_counts_only_nonzero_tokens_and_keeps_zero_usage_sessions() {
        let committed = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/cursor-state/basic/cursor-state.json");
        let ingested = ingest_root(&committed).expect("committed fixture ingests");
        assert_eq!(ingested.threads.len(), 3);
        assert_eq!(ingested.ledger.requests.len(), 2);
        assert!(
            ingested.ledger.diagnostics.iter().any(|diagnostic| diagnostic.code
                == crate::ledger::diagnostics::DiagnosticCode::CursorEstimateCost),
            "cost-only Auto chat is a source estimate: {:?}",
            ingested.ledger.diagnostics
        );
        let models: BTreeSet<_> = ingested
            .ledger
            .requests
            .values()
            .filter_map(|request| request.model.as_ref().map(|model| model.name.as_str()))
            .collect();
        assert!(models.contains("claude-4.5-opus-high-thinking"));
        assert!(models.contains("cursor-grok-4.6-xhigh-fast"));
    }

    #[test]
    fn group_by_provider_splits_cursor_vendors() {
        let (_dir, path) = write_fixture(&fixture_json());
        let ingested = ingest_root(&path).expect("fixture ingests");
        let mut index = SessionIndex::default();
        index.add(Agent::Cursor, &ingested).expect("index");
        let selected = index
            .select(&SelectionQuery { all: true, ..SelectionQuery::default() })
            .expect("select");
        let timezone = ResolvedTimeZone::resolve(Some("UTC")).expect("utc");
        let document = report(
            &[QuerySource { agent: Agent::Cursor, ingested: &ingested }],
            &index,
            &selected,
            true,
            QueryMetadata::new("report", "all", crate::selection::Scope::SelfOnly, &timezone),
            &BTreeSet::from([GroupBy::Provider, GroupBy::Agent, GroupBy::Model, GroupBy::Purpose]),
        )
        .expect("report");
        let providers: BTreeSet<_> = document
            .breakdowns
            .get("provider")
            .expect("provider breakdown")
            .iter()
            .map(|row| row.value.as_str())
            .collect();
        assert!(providers.contains("anthropic"));
        assert!(providers.contains("cursor"));
        let agents: BTreeSet<_> = document
            .breakdowns
            .get("agent")
            .expect("agent breakdown")
            .iter()
            .map(|row| row.value.as_str())
            .collect();
        assert_eq!(agents, BTreeSet::from(["cursor"]));
        let purposes: BTreeSet<_> = document
            .breakdowns
            .get("purpose")
            .expect("purpose breakdown")
            .iter()
            .map(|row| row.value.as_str())
            .collect();
        assert_eq!(purposes, BTreeSet::from(["agent"]));
        assert_eq!(
            provider_for(Agent::Cursor, Some("cursor-grok-4.6-xhigh-fast")).value(),
            Some(&"cursor")
        );
    }

    #[test]
    fn purpose_effort_provider_and_model_are_independent_facets() {
        let body = r#"{
          "format": "urollup-cursor-state/v1",
          "composers": [
            {
              "composerId": "11111111-1111-4111-8111-111111111111",
              "unifiedMode": "agent",
              "bubbles": [{
                "bubbleId": "b-agent",
                "requestId": "req-claude",
                "modelName": "claude-4.6-opus-high-thinking",
                "thinkingStyle": 2,
                "inputTokens": 8,
                "outputTokens": 2
              }]
            },
            {
              "composerId": "22222222-2222-4222-8222-222222222222",
              "unifiedMode": "chat",
              "bubbles": [{
                "bubbleId": "b-chat",
                "requestId": "req-gemini",
                "modelName": "gemini-3-pro",
                "thinkingStyle": 1,
                "inputTokens": 5,
                "outputTokens": 1
              }]
            },
            {
              "composerId": "33333333-3333-4333-8333-333333333333",
              "unifiedMode": "plan",
              "bubbles": [{
                "bubbleId": "b-plan",
                "requestId": "req-gpt",
                "modelName": "gpt-5.1-codex-high",
                "inputTokens": 4,
                "outputTokens": 1
              }]
            },
            {
              "composerId": "44444444-4444-4444-8444-444444444444",
              "unifiedMode": "multitask",
              "bubbles": [{
                "bubbleId": "b-multi",
                "requestId": "req-composer",
                "modelName": "composer-2.5-fast",
                "inputTokens": 3,
                "outputTokens": 1
              }]
            },
            {
              "composerId": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
              "unifiedMode": "background",
              "bubbles": [{
                "bubbleId": "b-bg",
                "requestId": "req-kimi",
                "modelName": "kimi-k2-instruct",
                "inputTokens": 2,
                "outputTokens": 1
              }]
            }
          ]
        }"#;
        let (_dir, path) = write_fixture(body);
        let ingested = ingest_root(&path).expect("facet fixture ingests");
        let mut index = SessionIndex::default();
        index.add(Agent::Cursor, &ingested).expect("index");
        let selected = index
            .select(&SelectionQuery { all: true, ..SelectionQuery::default() })
            .expect("select");
        let timezone = ResolvedTimeZone::resolve(Some("UTC")).expect("utc");
        let document = report(
            &[QuerySource { agent: Agent::Cursor, ingested: &ingested }],
            &index,
            &selected,
            true,
            QueryMetadata::new("report", "all", crate::selection::Scope::SelfOnly, &timezone),
            &BTreeSet::from([
                GroupBy::Provider,
                GroupBy::Purpose,
                GroupBy::Effort,
                GroupBy::Model,
                GroupBy::Agent,
            ]),
        )
        .expect("report");
        let values = |name: &str| -> BTreeSet<&str> {
            document
                .breakdowns
                .get(name)
                .expect(name)
                .iter()
                .map(|row| row.value.as_str())
                .collect()
        };
        assert_eq!(values("agent"), BTreeSet::from(["cursor"]));
        assert_eq!(
            values("purpose"),
            BTreeSet::from(["agent", "background", "chat", "multitask", "plan"])
        );
        assert_eq!(
            values("provider"),
            BTreeSet::from(["anthropic", "cursor", "google", "moonshot", "openai"])
        );
        assert_eq!(
            values("model"),
            BTreeSet::from([
                "claude-4.6-opus-high-thinking",
                "composer-2.5-fast",
                "gemini-3-pro",
                "gpt-5.1-codex-high",
                "kimi-k2-instruct",
            ])
        );
        assert_eq!(values("effort"), BTreeSet::from(["style-1", "style-2", "unknown"]));
        assert_eq!(document.totals.requests.owned, 5);
    }

    #[test]
    fn shared_request_ids_stay_per_composer() {
        let body = r#"{
          "format": "urollup-cursor-state/v1",
          "composers": [
            {
              "composerId": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
              "bubbles": [
                {
                  "bubbleId": "b1",
                  "requestId": "shared-request",
                  "modelName": "claude-4.5-opus-high-thinking",
                  "inputTokens": 2,
                  "outputTokens": 1
                }
              ]
            },
            {
              "composerId": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
              "bubbles": [
                {
                  "bubbleId": "b2",
                  "requestId": "shared-request",
                  "modelName": "gemini-3-pro",
                  "inputTokens": 3,
                  "outputTokens": 1
                }
              ]
            }
          ]
        }"#;
        let (_dir, path) = write_fixture(body);
        let ingested = ingest_root(&path).expect("shared request ids ingest");
        assert_eq!(ingested.ledger.requests.len(), 2);
        assert!(ingested.ledger.diagnostics.is_empty());
    }

    #[test]
    fn composer_id_is_the_native_session_selector() {
        let (_dir, path) = write_fixture(&fixture_json());
        let ingested = ingest_root(&path).expect("fixture ingests");
        let mut index = SessionIndex::default();
        index.add(Agent::Cursor, &ingested).expect("index");
        let selected = index
            .select(&SelectionQuery {
                sessions: vec!["11111111-1111-4111-8111-111111111111".into()],
                ..SelectionQuery::default()
            })
            .expect("select composer");
        assert_eq!(selected.len(), 1);
        assert_eq!(
            index.get(selected.first().expect("one")).expect("indexed").agent,
            Agent::Cursor
        );
    }

    #[test]
    fn sqlite_accepts_text_timestamps_and_composer_model_fallback() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("state.vscdb");
        let connection = rusqlite::Connection::open(&path).expect("open sqlite");
        connection
            .execute("CREATE TABLE cursorDiskKV (key TEXT NOT NULL, value TEXT NOT NULL)", [])
            .expect("create table");
        connection
            .execute(
                "INSERT INTO cursorDiskKV (key, value) VALUES (?1, ?2)",
                [
                    "composerData:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                    r#"{
                      "composerId":"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                      "createdAt":"2026-09-18T00:00:00Z",
                      "lastUpdatedAt":"2026-09-18T01:00:00Z",
                      "unifiedMode":"agent",
                      "modelConfig":{"modelName":"gemini-3-pro"}
                    }"#,
                ],
            )
            .expect("insert composer");
        connection
            .execute(
                "INSERT INTO cursorDiskKV (key, value) VALUES (?1, ?2)",
                [
                    "bubbleId:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa:bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
                    r#"{
                      "requestId":"req-gemini",
                      "createdAt":"2026-09-18T00:00:01Z",
                      "thinkingStyle":"2",
                      "tokenCount":{"inputTokens":4,"outputTokens":1}
                    }"#,
                ],
            )
            .expect("insert bubble");
        drop(connection);

        let ingested = ingest_root(&path).expect("text timestamps ingest");
        assert_eq!(ingested.threads.len(), 1);
        assert_eq!(ingested.ledger.requests.len(), 1);
        let request = ingested.ledger.requests.values().next().expect("request");
        assert_eq!(request.model.as_ref().map(|model| model.name.as_str()), Some("gemini-3-pro"));
        assert_eq!(request.effort.map(crate::ledger::names::Name::as_str), Some("style-2"));
        assert!(request.first_seen.is_some());
        assert_eq!(provider_for(Agent::Cursor, Some("gemini-3-pro")).value(), Some(&"google"));

        connection_extra_unrelated(&path);
        let filtered = ingest_roots(
            std::slice::from_ref(&path),
            true,
            &crate::ledger::capacity::ObservationCapacity::default(),
            Some(&["aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_owned()]),
        )
        .expect("key lookup ingest");
        assert_eq!(filtered.threads.len(), 1);
        assert_eq!(filtered.ledger.requests.len(), 1);
    }

    fn connection_extra_unrelated(path: &std::path::Path) {
        let connection = rusqlite::Connection::open(path).expect("reopen sqlite");
        connection
            .execute(
                "INSERT INTO cursorDiskKV (key, value) VALUES (?1, ?2)",
                [
                    "composerData:22222222-2222-4222-8222-222222222222",
                    r#"{"composerId":"22222222-2222-4222-8222-222222222222","unifiedMode":"chat"}"#,
                ],
            )
            .expect("insert unrelated composer");
        connection
            .execute(
                "INSERT INTO cursorDiskKV (key, value) VALUES (?1, ?2)",
                [
                    "bubbleId:22222222-2222-4222-8222-222222222222:cccccccccccccccc-cccc-4ccc-8ccc-cccccccccccc",
                    r#"{"requestId":"req-other","tokenCount":{"inputTokens":9,"outputTokens":3}}"#,
                ],
            )
            .expect("insert unrelated bubble");
    }

    #[test]
    fn picker_concatenation_is_not_a_model_label() {
        assert_eq!(super::attributed_model_name(Some("default")), Some("auto".to_owned()));
        assert_eq!(
            super::attributed_model_name(Some("gpt-5.1-codex-high,gpt-5.1-codex-high")),
            Some("gpt-5.1-codex-high".to_owned())
        );
        assert_eq!(
            super::attributed_model_name(Some(
                "claude-4.5-opus-high-thinking,gpt-5.1-codex-max-high,grok-code-fast-1"
            )),
            None
        );
        let body = r#"{
          "format": "urollup-cursor-state/v1",
          "composers": [
            {
              "composerId": "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
              "modelName": "o3-pro,claude-4.5-opus-high-thinking,gpt-5-pro",
              "bubbles": [
                {
                  "bubbleId": "b1",
                  "requestId": "req-mixed",
                  "inputTokens": 4,
                  "outputTokens": 1
                }
              ]
            }
          ]
        }"#;
        let (_dir, path) = write_fixture(body);
        let ingested = ingest_root(&path).expect("mixed picker ingest");
        let request = ingested.ledger.requests.values().next().expect("request");
        assert!(request.model.is_none());
        assert_eq!(
            provider_for(Agent::Cursor, request.model.as_ref().map(|model| model.name.as_str()))
                .value(),
            None
        );
    }

    #[test]
    fn best_of_n_siblings_keep_spawn_edges_and_independent_usage() {
        let body = r#"{
          "format": "urollup-cursor-state/v1",
          "composers": [
            {
              "composerId": "11111111-1111-4111-8111-111111111111",
              "unifiedMode": "agent",
              "subComposerIds": [
                "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"
              ],
              "bubbles": [{
                "bubbleId": "b-parent",
                "requestId": "req-parent",
                "modelName": "cursor-grok-4.6-xhigh-fast",
                "inputTokens": 5,
                "outputTokens": 1
              }]
            },
            {
              "composerId": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
              "isBestOfNSubcomposer": true,
              "parentComposerId": "11111111-1111-4111-8111-111111111111",
              "bubbles": [{
                "bubbleId": "b-a",
                "requestId": "req-a",
                "modelName": "claude-4.5-opus-high-thinking",
                "inputTokens": 3,
                "outputTokens": 1
              }]
            },
            {
              "composerId": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
              "isBestOfNSubcomposer": true,
              "parentComposerId": "11111111-1111-4111-8111-111111111111",
              "bubbles": [{
                "bubbleId": "b-b",
                "requestId": "req-b",
                "modelName": "gemini-3-pro",
                "inputTokens": 2,
                "outputTokens": 1
              }]
            }
          ]
        }"#;
        let (_dir, path) = write_fixture(body);
        let ingested = ingest_root(&path).expect("best-of-n fixture ingests");
        assert_eq!(ingested.threads.len(), 3);
        assert_eq!(ingested.ledger.requests.len(), 3);
        assert!(ingested.relationships.iter().all(|relationship| {
            matches!(relationship.kind, crate::ledger::entities::RelationshipKind::Spawn)
        }));
        let mut index = SessionIndex::default();
        index.add(Agent::Cursor, &ingested).expect("index");
        let selected = index
            .select(&SelectionQuery {
                sessions: vec!["11111111-1111-4111-8111-111111111111".into()],
                ..SelectionQuery::default()
            })
            .expect("select parent with default descendants");
        assert_eq!(selected.len(), 3);
        let timezone = ResolvedTimeZone::resolve(Some("UTC")).expect("utc");
        let document = report(
            &[QuerySource { agent: Agent::Cursor, ingested: &ingested }],
            &index,
            &selected,
            false,
            QueryMetadata::new(
                "report",
                "session",
                crate::selection::Scope::Descendants,
                &timezone,
            ),
            &BTreeSet::from([GroupBy::Provider, GroupBy::Model]),
        )
        .expect("report");
        assert_eq!(document.totals.requests.owned, 3);
        let providers: BTreeSet<_> = document
            .breakdowns
            .get("provider")
            .expect("provider")
            .iter()
            .map(|row| row.value.as_str())
            .collect();
        assert_eq!(providers, BTreeSet::from(["anthropic", "cursor", "google"]));
    }

    #[test]
    fn selected_composer_family_keeps_descendants_and_drops_unrelated() {
        let body = r#"{
          "format": "urollup-cursor-state/v1",
          "composers": [
            {
              "composerId": "11111111-1111-4111-8111-111111111111",
              "workspaceId": "ws-alpha",
              "subagentComposerIds": ["aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"],
              "bubbles": [{
                "bubbleId": "b-parent",
                "requestId": "req-parent",
                "modelName": "cursor-grok-4.6-xhigh-fast",
                "inputTokens": 5,
                "outputTokens": 1
              }]
            },
            {
              "composerId": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
              "parentComposerId": "11111111-1111-4111-8111-111111111111",
              "workspaceId": "ws-alpha",
              "bubbles": [{
                "bubbleId": "b-child",
                "requestId": "req-child",
                "modelName": "gemini-3-pro",
                "inputTokens": 2,
                "outputTokens": 1
              }]
            },
            {
              "composerId": "22222222-2222-4222-8222-222222222222",
              "workspaceId": "ws-other",
              "bubbles": [{
                "bubbleId": "b-other",
                "requestId": "req-other",
                "modelName": "gpt-5.1-codex-high",
                "inputTokens": 9,
                "outputTokens": 3
              }]
            }
          ]
        }"#;
        let (_dir, path) = write_fixture(body);
        let family = ingest_roots(
            std::slice::from_ref(&path),
            true,
            &crate::ledger::capacity::ObservationCapacity::default(),
            Some(&["11111111-1111-4111-8111-111111111111".to_owned()]),
        )
        .expect("filtered ingest");
        assert_eq!(family.threads.len(), 2);
        assert_eq!(family.ledger.requests.len(), 2);
        let projects: BTreeSet<_> = family
            .threads
            .values()
            .filter_map(|thread| thread.project.value().map(String::as_str))
            .collect();
        assert_eq!(projects, BTreeSet::from(["ws-alpha"]));

        let all = ingest_root(&path).expect("full ingest");
        assert_eq!(all.threads.len(), 3);
        assert_eq!(all.ledger.requests.len(), 3);
        let mut index = SessionIndex::default();
        index.add(Agent::Cursor, &all).expect("index");
        let selected = index
            .select(&SelectionQuery { all: true, ..SelectionQuery::default() })
            .expect("select");
        let timezone = ResolvedTimeZone::resolve(Some("UTC")).expect("utc");
        let document = report(
            &[QuerySource { agent: Agent::Cursor, ingested: &all }],
            &index,
            &selected,
            true,
            QueryMetadata::new("report", "all", crate::selection::Scope::SelfOnly, &timezone),
            &BTreeSet::from([GroupBy::Project]),
        )
        .expect("report");
        let projects: BTreeSet<_> = document
            .breakdowns
            .get("project")
            .expect("project")
            .iter()
            .map(|row| row.value.as_str())
            .collect();
        assert_eq!(projects, BTreeSet::from(["ws-alpha", "ws-other"]));
    }
}
