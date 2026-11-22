//! Traces failures.
use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Traces database failed: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Could not read {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(
        "This traces index is schema {found}; this build understands up to {known}. Update Sheprd to use it."
    )]
    NewerSchema { found: i64, known: i64 },
    #[error("No state directory is available for traces")]
    NoStateDirectory,
    #[error(transparent)]
    Private(#[from] sheprd_client::private_file::Error),
    #[error("Saved parser state is not valid: {0}")]
    State(#[source] serde_json::Error),
    #[error(transparent)]
    Sync(#[from] crate::sync::Error),
    #[error("Could not watch session files: {0}")]
    Watch(#[from] notify::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
