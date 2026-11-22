use super::{Batch, Error, MAX_EVENTS, MAX_RECORDS, PROTOCOL, Peer, storage, tags};
use crate::{Result, Store};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::io::{Read, Write};

/// Bounds both stdin and stdout, including JSON escaping and metadata.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    protocol: u32,
    id: String,
    sender: String,
    receiver: Option<String>,
    pull: i64,
    push: Batch,
    tags_pull: i64,
    tags_push: tags::Batch,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    protocol: u32,
    id: String,
    replica: String,
    host_id: Option<String>,
    body: Reply,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Reply {
    Reset,
    Exchanged {
        accepted: i64,
        batch: Batch,
        tags_accepted: i64,
        tags: tags::Batch,
    },
}

pub fn encode(value: &impl Serialize) -> std::result::Result<Vec<u8>, Error> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > MAX_BYTES {
        return Err(Error::Limit);
    }
    Ok(bytes)
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> std::result::Result<T, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::Limit);
    }
    Ok(serde_json::from_slice(bytes)?)
}

/// One exchange over the caller's authenticated channel. Never listens on a port.
pub fn serve(store: &mut Store, input: impl Read, mut output: impl Write) -> Result<()> {
    let mut bytes = Vec::new();
    input
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(Error::Io)?;
    let response = store.sync_respond(decode(&bytes)?)?;
    output.write_all(&encode(&response)?).map_err(Error::Io)?;
    output.flush().map_err(Error::Io)?;
    Ok(())
}

impl Store {
    /// Last acknowledged exchange for every host route this index has contacted.
    pub fn sync_peers(&self) -> Result<Vec<Peer>> {
        let mut statement = self.connection.prepare_cached("SELECT route,replica,received,sent,synced_at,host_id,tags_received,tags_sent FROM trace_peers ORDER BY route LIMIT 256")?;
        Ok(statement
            .query_map([], peer_row)?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn sync_request(&self, route: &str) -> Result<Request> {
        if route.len() > 4096 {
            return Err(Error::Limit.into());
        }
        let peer = peer(&self.connection, route)?;
        Ok(Request {
            protocol: PROTOCOL,
            id: uuid::Uuid::new_v4().to_string(),
            sender: storage::identity(&self.connection)?,
            receiver: peer.as_ref().map(|peer| peer.replica.clone()),
            pull: peer.as_ref().map_or(0, |peer| peer.received),
            push: storage::export(&self.connection, peer.as_ref().map_or(0, |peer| peer.sent))?,
            tags_pull: peer.as_ref().map_or(0, |peer| peer.tags_received),
            tags_push: tags::export(
                &self.connection,
                peer.as_ref().map_or(0, |peer| peer.tags_sent),
            )?,
        })
    }

    pub fn sync_respond(&mut self, request: Request) -> Result<Response> {
        if request.protocol != PROTOCOL {
            return Err(Error::Protocol(request.protocol).into());
        }
        uuid(&request.id)?;
        uuid(&request.sender)?;
        if let Some(receiver) = &request.receiver {
            uuid(receiver)?
        }
        validate(&request.push)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let replica = storage::identity(&tx)?;
        if request.sender == replica {
            return Err(Error::Invalid("cannot sync a store with itself").into());
        }
        let body = if request.receiver.as_deref() != Some(replica.as_str()) {
            Reply::Reset
        } else {
            storage::import(&tx, &request.push)?;
            tags::import(&tx, &request.tags_push)?;
            Reply::Exchanged {
                accepted: request.push.through,
                batch: storage::export(&tx, request.pull)?,
                tags_accepted: request.tags_push.through,
                tags: tags::export(&tx, request.tags_pull)?,
            }
        };
        let response = Response {
            protocol: PROTOCOL,
            id: request.id,
            replica,
            host_id: tags::local_host(&tx)?,
            body,
        };
        // Never commit a response that cannot be sent within the wire bound.
        encode(&response)?;
        tx.commit()?;
        Ok(response)
    }

    pub fn sync_complete(
        &mut self,
        route: &str,
        request: &Request,
        response: Response,
        now: i64,
    ) -> Result<bool> {
        if response.protocol != PROTOCOL {
            return Err(Error::Protocol(response.protocol).into());
        }
        uuid(&response.replica)?;
        if let Some(id) = &response.host_id {
            text(id, 512)?;
        }
        if response.id != request.id || response.replica == request.sender {
            return Err(Error::Correlation.into());
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = peer(&tx, route)?;
        if current.as_ref().map(|peer| peer.replica.as_str()) != request.receiver.as_deref()
            || current.as_ref().map_or(0, |peer| peer.sent) != request.push.start
            || current.as_ref().map_or(0, |peer| peer.received) != request.pull
            || current.as_ref().map_or(0, |peer| peer.tags_received) != request.tags_pull
            || current.as_ref().map_or(0, |peer| peer.tags_sent) != request.tags_push.start
        {
            return Err(Error::Correlation.into());
        }
        let (received, sent, tags_received, tags_sent, synced_at, more) = match response.body {
            Reply::Reset => {
                if request.receiver.as_deref() == Some(response.replica.as_str()) {
                    return Err(Error::Correlation.into());
                }
                (0, 0, 0, 0, None, true)
            }
            Reply::Exchanged {
                accepted,
                batch,
                tags_accepted,
                tags,
            } => {
                if request.receiver.as_deref() != Some(response.replica.as_str())
                    || accepted != request.push.through
                    || batch.start != request.pull
                    || tags_accepted != request.tags_push.through
                    || tags.start != request.tags_pull
                {
                    return Err(Error::Correlation.into());
                }
                storage::import(&tx, &batch)?;
                tags::import(&tx, &tags)?;
                (
                    batch.through,
                    accepted,
                    tags.through,
                    tags_accepted,
                    Some(now),
                    batch.more || request.push.more || tags.more || request.tags_push.more,
                )
            }
        };
        tx.execute(
            "INSERT INTO trace_peers(route,replica,received,sent,synced_at,host_id,tags_received,tags_sent) VALUES (?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(route) DO UPDATE SET replica=?2,received=?3,sent=?4,synced_at=?5,host_id=?6,tags_received=?7,tags_sent=?8",
            params![route,response.replica,received,sent,synced_at,response.host_id,tags_received,tags_sent],
        )?;
        let pending: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM trace_journal WHERE position > ?1)",
            [sent],
            |row| row.get(0),
        )?;
        let tags_pending: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM trace_host_tags WHERE position > ?1)",
            [tags_sent],
            |row| row.get(0),
        )?;
        tx.commit()?;
        Ok(more || pending || tags_pending)
    }
}

fn peer(connection: &rusqlite::Connection, route: &str) -> Result<Option<Peer>> {
    Ok(connection
        .query_row(
            "SELECT route,replica,received,sent,synced_at,host_id,tags_received,tags_sent FROM trace_peers WHERE route=?1",
            [route],
            peer_row,
        )
        .optional()?)
}

fn peer_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Peer> {
    Ok(Peer {
        route: row.get(0)?,
        replica: row.get(1)?,
        received: row.get(2)?,
        sent: row.get(3)?,
        synced_at: row.get(4)?,
        host_id: row.get(5)?,
        tags_received: row.get(6)?,
        tags_sent: row.get(7)?,
    })
}

fn uuid(text: &str) -> std::result::Result<(), Error> {
    if text.len() != 36 || uuid::Uuid::parse_str(text).is_err() {
        return Err(Error::Invalid("replica identity"));
    }
    Ok(())
}

fn text(value: &str, limit: usize) -> std::result::Result<(), Error> {
    if value.len() > limit || value.contains('\0') {
        return Err(Error::Limit);
    }
    Ok(())
}

pub(super) fn validate(batch: &Batch) -> std::result::Result<(), Error> {
    if batch.records.len() > MAX_RECORDS
        || batch.start < 0
        || batch.start.checked_add(batch.records.len() as i64) != Some(batch.through)
    {
        return Err(Error::Invalid("batch cursor or length"));
    }
    for record in &batch.records {
        uuid(&record.origin)?;
        uuid(&record.generation.replica)?;
        if record.sequence <= 0
            || record.version.clock <= 0
            || record.generation.clock <= 0
            || record.version.replica != record.origin
            || record.generation > record.version
            || record.events.len() > MAX_EVENTS
        {
            return Err(Error::Invalid("record version"));
        }
        let s = &record.session;
        if [
            s.messages,
            s.user_messages,
            s.tool_calls,
            s.errors,
            s.tokens_in,
            s.tokens_out,
        ]
        .into_iter()
        .any(|count| !(0..=1_000_000_000_000).contains(&count))
        {
            return Err(Error::Limit);
        }
        if s.id.is_empty()
            || s.host_id != record.host.id
            || s.host != record.host.label
            || super::account(&s.user) != super::account(&record.host.user)
        {
            return Err(Error::Invalid("session host"));
        }
        for value in [&s.id, &s.host_id, &s.host, &s.user, &record.host.os] {
            text(value, 512)?
        }
        text(&s.native_id, 4096)?;
        for value in [&s.title, &s.model, &s.cwd, &s.branch, &s.agent_version]
            .into_iter()
            .flatten()
        {
            text(value, 512)?
        }
        for event in &record.events {
            if event.seq < 0 || event.seq == i64::MAX {
                return Err(Error::Invalid("event sequence"));
            }
            text(&event.text, crate::parse::MAX_TEXT)?;
            if let Some(tool) = &event.tool {
                text(tool, 512)?
            }
        }
        if record
            .events
            .windows(2)
            .any(|pair| pair[0].seq >= pair[1].seq)
        {
            return Err(Error::Invalid("event order"));
        }
    }
    Ok(())
}
