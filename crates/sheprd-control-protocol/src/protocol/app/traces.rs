//! Replication progress. Internal users retain typed failures until display.
use serde::{Deserialize, Serialize};
pub use sheprd_client::device_tags::TaggedHost;

/// Scope of the Traces table, totals, and daily chart.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TraceFilter {
    // keep-sorted start
    #[default]
    All,
    Host {
        id: String,
    },
    Tag {
        tag: String,
    },
    // keep-sorted end
}

impl TraceFilter {
    pub fn includes(&self, host_id: &str, hosts: &[TaggedHost]) -> bool {
        match self {
            Self::All => true,
            Self::Host { id } => id == host_id,
            Self::Tag { tag } => hosts
                .iter()
                .find(|host| host.id == host_id)
                .is_some_and(|host| {
                    host.tags
                        .tags
                        .iter()
                        .any(|own| sheprd_client::device_tags::same(own, tag))
                }),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum TraceSyncState<E = String> {
    // keep-sorted start
    Failed { error: E },
    Ready,
    Syncing,
    Waiting,
    // keep-sorted end
}

impl<E> TraceSyncState<E> {
    pub fn map_error<F>(self, map: impl FnOnce(E) -> F) -> TraceSyncState<F> {
        match self {
            Self::Failed { error } => TraceSyncState::Failed { error: map(error) },
            Self::Ready => TraceSyncState::Ready,
            Self::Syncing => TraceSyncState::Syncing,
            Self::Waiting => TraceSyncState::Waiting,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TracePeer<E = String> {
    pub host: String,
    pub state: TraceSyncState<E>,
}
