//! The tags a person gives a computer, such as `personal` or `work`. Each
//! computer's own daemon holds them, so every phone and desktop that reaches
//! it sees the same set, and no client has to match its name for the
//! computer to anyone else's.
//!
//! A client offers its copy with the time it was edited and the daemon
//! answers with the set that stands: the latest edit wins, whole, because
//! tags are low stakes and a merge could never remove one.

#[cfg(test)]
use super::edited::Edited;
pub use sheprd_client::device_tags::{ComputerTags, MAX_TAG_CHARS, MAX_TAGS, normalized, same};

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
