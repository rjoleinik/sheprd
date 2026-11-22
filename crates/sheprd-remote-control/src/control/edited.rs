//! Settings a computer's own daemon keeps for every phone and desktop that
//! reaches it, such as its tags and the composer's text shortcuts. A client
//! offers its copy with the time it was edited and the daemon answers with
//! the copy that stands: the latest edit wins, whole.

pub use sheprd_client::edited::Edited;
