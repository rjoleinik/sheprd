//! Bidirectional store-and-forward replication over an authenticated host command.
//!
//! Imported records enter the same journal as local records. Origin/sequence
//! deduplication makes cycles harmless; per-session logical generations keep
//! a replaced file from resurrecting old events. A peer cursor advances only
//! in the transaction that imports its reply. No network or background runtime
//! is owned here: the app supplies its existing authenticated host transport.
mod error;
mod protocol;
pub(crate) mod storage;
pub(crate) mod tags;

#[cfg(test)]
mod tests;

use crate::{Result, Store};
pub use error::Error;
pub use protocol::{MAX_BYTES, Request, Response, decode, encode, serve};
use serde::{Deserialize, Serialize};

pub(crate) const MAX_RECORDS: usize = 64;
pub(crate) const MAX_EVENTS: usize = 32;
pub(crate) const BATCH_BYTES: usize = 512 * 1024;
pub const PROTOCOL: u32 = 2;

// Host labels may change; the account before the final @ does not.
fn account(user: &str) -> &str {
    user.rsplit_once('@').map_or(user, |(account, _)| account)
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Stamp {
    clock: i64,
    replica: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    origin: String,
    sequence: i64,
    version: Stamp,
    generation: Stamp,
    host: crate::Host,
    session: crate::Session,
    events: Vec<crate::Event>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Batch {
    start: i64,
    through: i64,
    more: bool,
    records: Vec<Record>,
}

/// Durable progress for one route. Routes are local; never forwarded to peers.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Peer {
    pub route: String,
    pub replica: String,
    pub received: i64,
    pub sent: i64,
    pub synced_at: Option<i64>,
    pub host_id: Option<String>,
    pub tags_received: i64,
    pub tags_sent: i64,
}

/// One bounded exchange. A new/rebuilt peer first identifies itself; the next
/// call starts at zero. Disconnects and lost replies replay safely.
pub fn exchange(
    store: &mut Store,
    route: &str,
    now: i64,
    transport: impl FnOnce(&[u8]) -> std::result::Result<Vec<u8>, Error>,
) -> Result<bool> {
    let request = store.sync_request(route)?;
    let bytes = encode(&request)?;
    let reply: Response = decode(&transport(&bytes)?)?;
    store.sync_complete(route, &request, reply, now)
}
