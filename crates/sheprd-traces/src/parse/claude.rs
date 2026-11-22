//! Claude Code's `~/.claude/projects/<cwd>/<session>.jsonl` records.
use super::{Out, Parser, field, int, string, text_of};
use crate::model::EventKind;
use serde_json::Value;

impl Parser {
    pub(super) fn claude(&mut self, record: &Value, out: &mut Out<'_>) {
        if self.meta.native_id.is_none() {
            self.meta.native_id = field(record, "sessionId");
        }
        if let Some(cwd) = field(record, "cwd") {
            self.meta.cwd = Some(cwd);
        }
        if let Some(branch) = field(record, "gitBranch").filter(|branch| branch != "HEAD") {
            self.meta.branch = Some(branch);
        }
        if let Some(version) = field(record, "version") {
            self.meta.agent_version = Some(version);
        }
        // Subagent runs share the file; they are the parent's tool call.
        if record["isSidechain"] == true || record["isMeta"] == true {
            return;
        }
        match string(record, "type") {
            Some("summary" | "custom-title" | "ai-title") => {
                let title = string(record, "summary")
                    .or_else(|| string(record, "customTitle"))
                    .or_else(|| string(record, "title"));
                if let Some(title) = title {
                    self.meta.title = None;
                    self.title_from(title, out.redactor);
                }
            }
            Some("user") => self.claude_user(&record["message"], out),
            Some("assistant") => self.claude_assistant(&record["message"], out),
            _ => {}
        }
    }

    fn claude_user(&mut self, message: &Value, out: &mut Out<'_>) {
        match &message["content"] {
            Value::String(text) => {
                self.title_from(text, out.redactor);
                out.push(EventKind::User, None, text);
            }
            Value::Array(blocks) => {
                for block in blocks {
                    match string(block, "type") {
                        Some("text") => {
                            let text = string(block, "text").unwrap_or_default();
                            self.title_from(text, out.redactor);
                            out.push(EventKind::User, None, text);
                        }
                        Some("tool_result") => {
                            let kind = if block["is_error"] == true {
                                EventKind::Error
                            } else {
                                EventKind::ToolResult
                            };
                            out.push(kind, None, &text_of(&block["content"], out.redactor));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn claude_assistant(&mut self, message: &Value, out: &mut Out<'_>) {
        if let Some(model) = field(message, "model").filter(|model| model != "<synthetic>") {
            self.meta.model = Some(model);
        }
        let id = string(message, "id").map(str::to_owned);
        if id.is_none() || id != self.last_message {
            let usage = &message["usage"];
            self.meta.tokens_in += int(usage, "input_tokens")
                + int(usage, "cache_creation_input_tokens")
                + int(usage, "cache_read_input_tokens");
            self.meta.tokens_out += int(usage, "output_tokens");
            self.last_message = id;
        }
        let Some(blocks) = message["content"].as_array() else {
            return;
        };
        for block in blocks {
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
                Some("tool_use") => out.push(
                    EventKind::ToolCall,
                    string(block, "name"),
                    &text_of(&block["input"], out.redactor),
                ),
                _ => {}
            }
        }
    }
}
