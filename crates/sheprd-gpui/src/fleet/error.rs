//! Why the fleet could not do what was asked.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // keep-sorted start
    #[error("The host's answer is not one this app can read")]
    Answer(#[source] serde_json::Error),
    #[error("The remote control daemon refused: {0}")]
    Daemon(String),
    #[error("Remote control is off on this computer")]
    NoDaemon,
    #[error("The host has no Sheprd remote control to ask")]
    NoProgram,
    #[error("The other computer's fleet did not admit this one")]
    NotAdmitted,
    #[error("Could not reach the remote control daemon")]
    Request(#[source] sheprd_remote_control::control::Error),
    #[error("Could not run sheprd-remote-control on the host")]
    Run(#[source] sheprd_client::Error),
    #[error("The fleet's log is larger than a daemon keeps")]
    TooLarge,
    #[error("The remote control daemon answered something else")]
    Unexpected,
    #[error("Could not remove all fleet access on the host; some may remain")]
    Unscrubbed,
    #[error("The fleet cannot reach this kind of device")]
    Unsupported,
    // keep-sorted end
}
