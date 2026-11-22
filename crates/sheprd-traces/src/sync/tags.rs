//! Replicated device-tag projections. Full values permit bounded deltas and
//! preserve clears; unchanged values never receive another local sequence.
use super::{Error, MAX_RECORDS};
use crate::{Result, Store};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sheprd_client::{
    device_tags::{ComputerTags, TaggedHost},
    edited::Edited,
};

pub(crate) fn migrate(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch(
        "CREATE TABLE trace_host_tags (
        position INTEGER PRIMARY KEY AUTOINCREMENT, host_id TEXT UNIQUE NOT NULL, tags TEXT NOT NULL
    );
    ALTER TABLE trace_peers ADD COLUMN host_id TEXT;
    ALTER TABLE trace_peers ADD COLUMN tags_received INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE trace_peers ADD COLUMN tags_sent INTEGER NOT NULL DEFAULT 0;",
    )?;
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    position: i64,
    host_id: String,
    tags: ComputerTags,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Batch {
    pub start: i64,
    pub through: i64,
    pub more: bool,
    entries: Vec<Entry>,
}

pub(super) fn local_host(connection: &Connection) -> Result<Option<String>> {
    Ok(connection.query_row("SELECT host_id FROM hosts WHERE kind='local' ORDER BY last_seen_at DESC,host_id LIMIT 1", [], |row| row.get(0)).optional()?)
}

impl Store {
    pub fn local_host_id(&self) -> Result<Option<String>> {
        local_host(&self.connection)
    }

    pub fn hosts(&self) -> Result<Vec<TaggedHost>> {
        let mut query = self.connection.prepare_cached("SELECT host_id,label,t.tags FROM hosts h LEFT JOIN trace_host_tags t USING(host_id) ORDER BY lower(label),host_id LIMIT ?1")?;
        let rows = query.query_map([crate::MAX_LIMIT as i64], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get::<_, Option<String>>(2)?))
        })?;
        rows.map(|row| {
            let (id, label, tags) = row?;
            Ok(TaggedHost {
                id,
                label,
                tags: parse(tags)?,
            })
        })
        .collect()
    }

    /// Mirror an existing device-tag edit, not a separate classification system.
    pub fn set_host_tags(
        &mut self,
        host_id: &str,
        offer: ComputerTags,
        now_ms: u64,
    ) -> Result<bool> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = get(&tx, host_id)?;
        let next = current.clone().merged(offer, now_ms);
        let changed = next != current;
        if changed {
            put(&tx, host_id, &next)?;
        }
        tx.commit()?;
        Ok(changed)
    }
}

fn parse(text: Option<String>) -> Result<ComputerTags> {
    text.map(|text| serde_json::from_str(&text).map_err(|error| Error::Json(error).into()))
        .unwrap_or_else(|| Ok(ComputerTags::default()))
}

fn get(connection: &Connection, host: &str) -> Result<ComputerTags> {
    parse(
        connection
            .query_row(
                "SELECT tags FROM trace_host_tags WHERE host_id=?1",
                [host],
                |row| row.get(0),
            )
            .optional()?,
    )
}

fn put(tx: &Transaction<'_>, host_id: &str, tags: &ComputerTags) -> Result<()> {
    if host_id.is_empty() || host_id.len() > 512 || host_id.contains('\0') {
        return Err(Error::Limit.into());
    }
    let value = serde_json::to_string(tags).map_err(Error::Json)?;
    tx.execute(
        "INSERT OR REPLACE INTO trace_host_tags(host_id,tags) VALUES (?1,?2)",
        params![host_id, value],
    )?;
    Ok(())
}

pub(super) fn export(connection: &Connection, after: i64) -> Result<Batch> {
    let end: i64 = connection.query_row(
        "SELECT coalesce(max(position),0) FROM trace_host_tags",
        [],
        |row| row.get(0),
    )?;
    if after < 0 || after > end {
        return Err(Error::Invalid("tag cursor").into());
    }
    let mut statement = connection.prepare_cached("SELECT position,host_id,tags FROM trace_host_tags WHERE position > ?1 ORDER BY position LIMIT ?2")?;
    let rows = statement.query_map(params![after, MAX_RECORDS as i64], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get::<_, String>(2)?))
    })?;
    let mut entries = Vec::new();
    for row in rows {
        let (position, host_id, text) = row?;
        entries.push(Entry {
            position,
            host_id,
            tags: parse(Some(text))?,
        });
    }
    let through = entries.last().map_or(after, |entry| entry.position);
    Ok(Batch {
        start: after,
        through,
        more: through < end,
        entries,
    })
}

pub(super) fn import(tx: &Transaction<'_>, batch: &Batch) -> Result<()> {
    if batch.entries.len() > MAX_RECORDS || batch.start < 0 || batch.through < batch.start {
        return Err(Error::Limit.into());
    }
    let mut at = batch.start;
    for entry in &batch.entries {
        if entry.position <= at
            || entry.position > batch.through
            || entry.host_id.is_empty()
            || entry.host_id.len() > 512
            || entry.host_id.contains('\0')
            || entry.tags.tags.len() > sheprd_client::device_tags::MAX_TAGS
            || entry
                .tags
                .tags
                .iter()
                .any(|tag| tag.len() > sheprd_client::device_tags::MAX_TAG_CHARS * 4)
            || ComputerTags::new(&entry.tags.tags, entry.tags.updated_ms) != entry.tags
        {
            return Err(Error::Invalid("tag entry").into());
        }
        at = entry.position;
        let current = get(tx, &entry.host_id)?;
        // Timestamped whole values, with a deterministic tie for copies that
        // reached different daemons first. A clear remains a timestamped row.
        if (entry.tags.updated_ms, &entry.tags.tags) > (current.updated_ms, &current.tags) {
            put(tx, &entry.host_id, &entry.tags)?;
        }
    }
    if at != batch.through {
        return Err(Error::Invalid("tag batch cursor").into());
    }
    Ok(())
}
