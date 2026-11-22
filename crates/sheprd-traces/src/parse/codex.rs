//! Codex's `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` records. Each is
//! `{timestamp, type, payload}`; `event_msg` often repeats `response_item`
//! text, so messages are read from `response_item` only.
use super::{Out, Parser, field, int, string, text_of};
use crate::model::EventKind;
use serde_json::Value;

/// Context Codex adds to the first user message, not a person's prompt.
const INJECTED: [&str; 4] = [
    "<environment_context>",
    "<user_instructions>",
    "# AGENTS.md",
    "<permissions",
];

impl Parser {
    pub(super) fn codex(&mut self, record: &Value, out: &mut Out<'_>) {
        let payload = &record["payload"];
        match string(record, "type") {
            Some("session_meta") => {
                self.meta.native_id = field(payload, "id").or_else(|| field(payload, "session_id"));
                self.meta.cwd = field(payload, "cwd").or(self.meta.cwd.take());
                self.meta.agent_version =
                    field(payload, "cli_version").or(self.meta.agent_version.take());
                if let Some(branch) = field(&payload["git"], "branch") {
                    self.meta.branch = Some(branch);
                }
            }
            Some("turn_context") => {
                if let Some(model) = field(payload, "model") {
                    self.meta.model = Some(model);
                }
            }
            Some("event_msg") if string(payload, "type") == Some("token_count") => {
                // Totals are cumulative; the latest replaces the last.
                let total = &payload["info"]["total_token_usage"];
                if total.is_object() {
                    self.meta.tokens_in = int(total, "input_tokens");
                    self.meta.tokens_out = int(total, "output_tokens");
                }
            }
            Some("response_item") => self.codex_item(payload, out),
            _ => {}
        }
    }

    fn codex_item(&mut self, payload: &Value, out: &mut Out<'_>) {
        match string(payload, "type") {
            Some("message") => {
                let kind = match (string(payload, "role"), string(payload, "channel")) {
                    (_, Some("analysis")) => EventKind::Thinking,
                    (Some("user"), _) => EventKind::User,
                    (Some("assistant"), _) => EventKind::Agent,
                    _ => return,
                };
                let text = text_of(&payload["content"], out.redactor);
                if kind == EventKind::User {
                    if INJECTED
                        .iter()
                        .any(|prefix| text.trim_start().starts_with(prefix))
                    {
                        return;
                    }
                    self.title_from(&text, out.redactor);
                }
                out.push(kind, None, &text);
            }
            Some("reasoning") => {
                out.push(
                    EventKind::Thinking,
                    None,
                    &text_of(&payload["summary"], out.redactor),
                );
            }
            Some("function_call") => out.push(
                EventKind::ToolCall,
                string(payload, "name"),
                &text_of(&payload["arguments"], out.redactor),
            ),
            Some("custom_tool_call") => out.push(
                EventKind::ToolCall,
                string(payload, "name"),
                &text_of(&payload["input"], out.redactor),
            ),
            Some("function_call_output" | "custom_tool_call_output") => {
                let output = &payload["output"];
                // Older rollouts nest `{content, success}`.
                let (text, failed) = match output.get("content") {
                    Some(content) => (text_of(content, out.redactor), output["success"] == false),
                    None => (text_of(output, out.redactor), false),
                };
                let kind = if failed {
                    EventKind::Error
                } else {
                    EventKind::ToolResult
                };
                out.push(kind, None, &text);
            }
            _ => {}
        }
    }
}
