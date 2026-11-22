//! Pi's (and omp's) `~/.pi/agent/sessions/--<cwd>--/<time>_<id>.jsonl`
//! entries. Entries form a tree through `parentId`; every branch is kept,
//! since each was work the agent did. System messages (the prompt and tool
//! loadout) are not events.
use super::{Out, Parser, field, int, string, text_of};
use crate::model::EventKind;
use serde_json::Value;

impl Parser {
    pub(super) fn pi(&mut self, record: &Value, out: &mut Out<'_>) {
        match string(record, "type") {
            Some("session") => {
                self.meta.native_id = field(record, "id").or(self.meta.native_id.take());
                if let Some(cwd) = field(record, "cwd") {
                    self.meta.cwd = Some(cwd);
                }
            }
            Some("session_info") => {
                if let Some(name) = string(record, "name") {
                    self.meta.title = None;
                    self.title_from(name, out.redactor);
                }
            }
            Some("model_change") => {
                if let Some(model) = field(record, "modelId") {
                    self.meta.model = Some(model);
                }
            }
            // Model work outside assistant messages still counts.
            Some("usage" | "compaction" | "branch_summary") => self.pi_usage(&record["usage"]),
            Some("message") => self.pi_message(&record["message"], out),
            _ => {}
        }
    }

    fn pi_message(&mut self, message: &Value, out: &mut Out<'_>) {
        match string(message, "role") {
            Some("user") => {
                let text = text_of(&message["content"], out.redactor);
                self.title_from(&text, out.redactor);
                out.push(EventKind::User, None, &text);
            }
            Some("assistant") => {
                if let Some(model) = field(message, "model") {
                    self.meta.model = Some(model);
                }
                self.pi_usage(&message["usage"]);
                for block in message["content"].as_array().into_iter().flatten() {
                    match string(block, "type") {
                        Some("text") => out.push(
                            EventKind::Agent,
                            None,
                            string(block, "text").unwrap_or_default(),
                        ),
                        Some("thinking") => out.push(
                            EventKind::Thinking,
                            None,
                            string(block, "thinking").unwrap_or_default(),
                        ),
                        Some("toolCall") => out.push(
                            EventKind::ToolCall,
                            string(block, "name"),
                            &text_of(&block["arguments"], out.redactor),
                        ),
                        _ => {}
                    }
                }
                if string(message, "stopReason") == Some("error") {
                    let error = string(message, "errorMessage").unwrap_or("The model failed");
                    out.push(EventKind::Error, None, error);
                }
            }
            Some("toolResult") => {
                self.pi_usage(&message["usage"]);
                let kind = if message["isError"] == true {
                    EventKind::Error
                } else {
                    EventKind::ToolResult
                };
                out.push(
                    kind,
                    string(message, "toolName"),
                    &text_of(&message["content"], out.redactor),
                );
            }
            _ => {}
        }
    }

    fn pi_usage(&mut self, usage: &Value) {
        if !usage.is_object() {
            return;
        }
        self.meta.tokens_in +=
            int(usage, "input") + int(usage, "cacheRead") + int(usage, "cacheWrite");
        self.meta.tokens_out += int(usage, "output");
    }
}
