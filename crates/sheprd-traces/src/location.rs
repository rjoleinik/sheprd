//! Where the index lives: `traces/traces.sqlite3` in the app's state
//! directory, one file per host holding every session that host indexed.
use std::path::PathBuf;

pub const FILE: &str = "traces.sqlite3";

/// The index file, or `None` without a state directory.
pub fn path() -> Option<PathBuf> {
    Some(
        sheprd_client::local_path::state_dir()?
            .join("traces")
            .join(FILE),
    )
}
