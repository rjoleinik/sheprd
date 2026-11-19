//! Change history failures. Refusals shared with checkpoints (no checkout of
//! the branch, scripts that failed) arrive as `checkpoint::Error`.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // keep-sorted start
    #[error("The branch's change history was not understood.")]
    Log,
    #[error("Change history output was not understood.")]
    Output,
    #[error("The change history kept changing while it was written.")]
    Raced,
    // keep-sorted end
}
