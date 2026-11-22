//! Whole-value settings shared by a computer's clients.
use serde::{Serialize, de::DeserializeOwned};

pub trait Edited: Clone + Default + Eq + Serialize + DeserializeOwned {
    fn updated_ms(&self) -> u64;
    #[must_use]
    fn bounded(self, updated_ms: u64) -> Self;

    /// Latest edit wins; an ahead-of-time offer cannot outlast later edits.
    /// Ties keep the stored copy, as the device settings exchange requires.
    #[must_use]
    fn merged(self, offer: Self, now_ms: u64) -> Self {
        let latest = now_ms.max(self.updated_ms().saturating_add(1));
        let updated_ms = offer.updated_ms().min(latest);
        if updated_ms > self.updated_ms() {
            offer.bounded(updated_ms)
        } else {
            self
        }
    }
}
