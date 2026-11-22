//! Replication failures retain protocol, transport, and decoding causes.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Traces replication worker stopped")]
    WorkerStopped,
    #[error("Unsupported Traces replication protocol {0}")]
    Protocol(u32),
    #[error("Traces replication message exceeds its bounds")]
    Limit,
    #[error("Invalid Traces replication record: {0}")]
    Invalid(&'static str),
    #[error("Traces replication history is missing an earlier record")]
    Gap,
    #[error("Traces replication record conflicts with an accepted record")]
    Conflict,
    #[error("Traces replication reply does not match the request")]
    Correlation,
    #[error("Traces replication JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Traces replication I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("Traces host command failed: {0}")]
    Transport(#[from] sheprd_client::Error),
}
