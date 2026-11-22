//! The device-tag values the desktop, remote daemon, and trace index share.
use crate::edited::Edited;
use serde::{Deserialize, Serialize};

pub const MAX_TAGS: usize = 8;
pub const MAX_TAG_CHARS: usize = 24;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ComputerTags {
    pub tags: Vec<String>,
    pub updated_ms: u64,
}

impl ComputerTags {
    pub fn new<T: AsRef<str>>(tags: impl IntoIterator<Item = T>, updated_ms: u64) -> Self {
        let mut kept: Vec<String> = Vec::new();
        for tag in tags {
            let Some(tag) = normalized(tag.as_ref()) else {
                continue;
            };
            if kept.len() < MAX_TAGS && !kept.iter().any(|other| same(other, &tag)) {
                kept.push(tag);
            }
        }
        Self {
            tags: kept,
            updated_ms,
        }
    }
}

impl Edited for ComputerTags {
    fn updated_ms(&self) -> u64 {
        self.updated_ms
    }
    fn bounded(self, updated_ms: u64) -> Self {
        Self::new(self.tags, updated_ms)
    }
}

pub fn normalized(text: &str) -> Option<String> {
    let words = text
        .split_whitespace()
        .filter(|word| !word.chars().any(char::is_control))
        .collect::<Vec<_>>()
        .join(" ");
    let tag: String = words.chars().take(MAX_TAG_CHARS).collect();
    let tag = tag.trim_end();
    (!tag.is_empty()).then(|| tag.to_owned())
}

pub fn same(one: &str, other: &str) -> bool {
    one.to_lowercase() == other.to_lowercase()
}

/// A stable computer identity and the labels a person assigned to it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaggedHost {
    pub id: String,
    pub label: String,
    pub tags: ComputerTags,
}
