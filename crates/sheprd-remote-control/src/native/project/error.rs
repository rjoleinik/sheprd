//! Why a new project was not made. Git's own words, when it refused, are
//! the last lines it wrote, bounded.

use axum::http::StatusCode;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    // keep-sorted start
    #[error("another project is still being made on this computer; try again shortly")]
    Busy,
    #[error("{name} already exists there")]
    Exists { name: String },
    #[error("git could not run on this computer")]
    Git(#[source] sheprd_client::Error),
    #[error("that is not a folder name a project can have")]
    Name,
    #[error("the new project could not be put in place")]
    Place(#[source] std::io::Error),
    #[error("git refused: {output}")]
    Refused { output: String },
    #[error("that is not a folder new projects go in")]
    Root,
    #[error("git took too long and was stopped")]
    TimedOut,
    #[error("that is not a repository address this computer clones")]
    Url,
    #[error("the new project could not be made")]
    Worker,
    // keep-sorted end
}

impl Error {
    pub(crate) fn status(&self) -> StatusCode {
        match self {
            Self::Busy => StatusCode::TOO_MANY_REQUESTS,
            Self::Exists { .. } => StatusCode::CONFLICT,
            Self::Name | Self::Root | Self::Url => StatusCode::BAD_REQUEST,
            Self::Refused { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::TimedOut => StatusCode::GATEWAY_TIMEOUT,
            Self::Git(_) | Self::Place(_) | Self::Worker => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
