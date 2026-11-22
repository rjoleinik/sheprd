//! Projects native session records into bounded, redacted events and the
//! session's metadata. One [`Parser`] follows one file; its state is saved
//! with the file's cursor, so the next scan continues where this one stopped.
//!
//! Records are data: links, tool calls, and paths in them are never run or
//! followed. A record that does not parse is skipped, never an error, since
//! agents append partial or newer records.

use crate::{
    model::{Event, EventKind},
    redact::Redactor,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sheprd_client::herdr::AgentKind;

mod claude;
mod codex;
mod pi;

/// The longest event text kept, in bytes. The full text stays in the
/// agent's own file on its host.
pub(crate) const MAX_TEXT: usize = 4096;
/// The longest title kept, in bytes.
const MAX_TITLE: usize = 160;
/// The longest identifier, model, branch, or version kept, in bytes.
const MAX_FIELD: usize = 512;
/// The most events one record yields; `seq` leaves room for exactly these.
pub(crate) const BLOCKS: i64 = 64;

/// What is known about a session so far.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Meta {
    pub native_id: Option<String>,
    pub title: Option<String>,
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub branch: Option<String>,
    pub agent_version: Option<String>,
    pub started_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub tokens_in: i64,
    pub tokens_out: i64,
}

/// One file's parser and what it has learned.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Parser {
    pub meta: Meta,
    /// Claude writes one record per content block of a message, each with
    /// the message's usage; usage counts once per message.
    last_message: Option<String>,
}

impl Parser {
    pub(crate) fn new() -> Self {
        Self {
            meta: Meta::default(),
            last_message: None,
        }
    }

    /// Reads the record that starts at byte `offset` of the file, appending
    /// its events to `events`.
    pub(crate) fn record(
        &mut self,
        agent: AgentKind,
        offset: u64,
        line: &str,
        redactor: &Redactor,
        events: &mut Vec<Event>,
    ) {
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            return;
        };
        let mut out = Out {
            base: i64::try_from(offset).unwrap_or(i64::MAX / BLOCKS) * BLOCKS,
            at: string(&record, "timestamp").and_then(millis),
            redactor,
            events,
            count: 0,
        };
        if let Some(at) = out.at {
            self.meta.started_at = Some(self.meta.started_at.map_or(at, |s| s.min(at)));
            self.meta.updated_at = Some(self.meta.updated_at.map_or(at, |u| u.max(at)));
        }
        match agent {
            AgentKind::Claude => self.claude(&record, &mut out),
            AgentKind::Codex => self.codex(&record, &mut out),
            AgentKind::Pi | AgentKind::Omp => self.pi(&record, &mut out),
            _ => {}
        }
    }

    /// Takes `text`, the first prompt, as the title when none is known.
    fn title_from(&mut self, text: &str, redactor: &Redactor) {
        if self.meta.title.is_some() {
            return;
        }
        let line = text.lines().map(str::trim).find(|line| !line.is_empty());
        if let Some(line) = line.filter(|line| !line.starts_with('<')) {
            self.meta.title = Some(bounded(&redactor.redact(line), MAX_TITLE));
        }
    }
}

/// Where a record's events go, with the record's time and `seq` base.
struct Out<'a> {
    base: i64,
    at: Option<i64>,
    redactor: &'a Redactor,
    events: &'a mut Vec<Event>,
    count: i64,
}

impl Out<'_> {
    fn push(&mut self, kind: EventKind, tool: Option<&str>, text: &str) {
        if self.count >= BLOCKS || (text.trim().is_empty() && tool.is_none()) {
            return;
        }
        self.events.push(Event {
            seq: self.base + self.count,
            at: self.at,
            kind,
            tool: tool.map(|tool| bounded(tool, MAX_FIELD)),
            text: bounded(&self.redactor.redact(text), MAX_TEXT),
        });
        self.count += 1;
    }
}

fn string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key)?.as_str().filter(|value| !value.is_empty())
}

fn field(value: &Value, key: &str) -> Option<String> {
    string(value, key).map(|value| bounded(value, MAX_FIELD))
}

fn int(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or_default()
}

/// Unix milliseconds of an RFC 3339 time.
fn millis(text: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|time| time.timestamp_millis())
}

/// The text of a tool's input or output: strings as they are, text blocks
/// joined, anything else as compact JSON.
fn text_of(value: &Value, redactor: &Redactor) -> String {
    match value {
        Value::String(text) => bounded(&redactor.redact(text), MAX_TEXT),
        Value::Array(blocks) => {
            let mut text = String::new();
            for block in blocks.iter().take(BLOCKS as usize) {
                let part = string(block, "text")
                    .map_or_else(|| text_of(block, redactor), |text| redactor.redact(text));
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&part);
                if text.len() >= MAX_TEXT {
                    break;
                }
            }
            bounded(&text, MAX_TEXT)
        }
        Value::Null => String::new(),
        other => {
            let mut clean = other.clone();
            redactor.json(&mut clean, 0);
            bounded(&clean.to_string(), MAX_TEXT)
        }
    }
}

/// `text` cut to at most `max` bytes on a character boundary.
pub(crate) fn bounded(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_owned();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}
